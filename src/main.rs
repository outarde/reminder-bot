use std::{
    //io::{self, Write},
    path::{Path, PathBuf},
};
use matrix_sdk::{
    Client, 
    config::SyncSettings,
};
use anyhow::{Context, Result};
use tracing_subscriber;
use tracing::{info, warn, error, instrument, Level};
//use dotenvy::dotenv;
//use std::env;
use tokio::signal;
use tokio_rusqlite::Connection;
use clap::{Parser, Subcommand};
use rust_i18n::t;

rust_i18n::i18n!("locales", fallback = "en");

mod config;
mod auth;
mod handlers;
mod reminder;

/// Folder for storing session files: session.json, database for persist session. recovery.json
/// and app sqlite database: reminders.db
/// Located in dirs::data_dir() directory.
pub const APP_FOLDER: &str = "reminder_bot";

/// Reminder Bot will send reminders for anything you ask 
/// at any time on your [matrix] server
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    /// The sub-command to run.
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Recover bot's cross-signing verification by recovery key or passphrase
    /// to put away red exclamation mark on each bot message ("unverified device").
    /// It also recoveres access to backup.
    Recover { 
        /// Your recovery key for backup. You can get it via
        /// matrix clients like Element Web, Element X, Fluffy Chat, etc.
        /// The key is accepted in the following priority: recovery-key flag, .env, recovery.json.
        #[arg(short, long)]
        recovery_key: Option<String>,

        /// This fixes backup key (not recovery key) in your Recovery if it is damaged.
        /// (Backup key used to decrypt keys for rooms.)
        #[arg(long)]
        fix_backup: bool,
    },

    /// Print list of your devices with verification statuses
    DevicesList,

    /// Verify other bot devices to put away warning messages in some clients
    VerifySomeDevice { 
        /// ID of device with letters and numbers. You can get it by devices-list command
        device_id: String,
    },

    /// Read and print recovery key from json file if you got it before
    RecallRecoveryKey,

    /// Clean your db
    ResetRecovery { 
        /// Reset your key with backup
        #[arg(short, long)]
        backup: bool,

        /// Your active recovery key if you select "backup" option
        #[arg(short, long)]
        recovery_key: Option<String>,

        /// Confirm this action
        #[arg(long)]
        confirmation: bool,
    },

    /// Clean your db
    Cleanup { 
        /// Level of cleanup. From 1 to 5
        level: String,
    },
}

// ===== App Context =====

struct AppContext {
    config: config::AppConfig,
    data_dir: PathBuf,
    session_file: PathBuf,
    client: Option<Client>,
    sync_settings: Option<SyncSettings>,
    sync_token: Option<String>,
    db_conn: Option<Connection>
}

impl AppContext {
    async fn new() -> Result<Self> {
        let config = config::AppConfig::load().await?;

        rust_i18n::set_locale(&config.i18n.app);

        let data_dir = dirs::data_dir().expect("No data_dir directory found").join(APP_FOLDER);
        let session_file = data_dir.join("session.json");

        Ok(Self {
            config,
            data_dir,
            session_file,
            client: None,
            sync_settings: None,
            sync_token: None,
            db_conn: None,
        })
    }

    /// Client inizialization: new login or persisted session.
    async fn init_client(&mut self) -> Result<()> {
        if self.client.is_some() {
            return Ok(());
        }

        let (client, sync_token) = if self.session_file.exists() {
            auth::restore_session(&self.session_file).await?
        } else {
            // Try to get recovery key for recover backup and cross-signing keys in login()
            let key = auth::pick_recovery_key(None, &self.config.auth, &self.config.recovery).await;
            
            (auth::login(
                &self.data_dir, 
                &self.session_file, 
                &self.config.auth,
                key.as_deref()
            ).await?, None)
        };

        self.client = Some(client);
        self.sync_token = sync_token;
        self.sync_settings = Some(auth::get_sync(
            &self.client.as_ref().unwrap(), 
            &self.sync_token, 
            &self.session_file
        ).await?);

        Ok(())
    }

    /// Returns link to client
    fn client(&self) -> Result<&Client> {
        self.client
            .as_ref()
            .context("Call init_client() at first")
    }

    // === CLI Commands ===

    pub async fn recover(&self, key: Option<&str>) -> Result<()> {
        let client = self.client()?.clone();

        if let Some(recovery_key) = auth::pick_recovery_key(key, &self.config.auth, &self.config.recovery).await {
            auth::recover_device(client, &recovery_key).await?;
        } else {
            warn!("You don't have active recovery key");
        }
        
        Ok(())
    }

    pub async fn recover_with_fix(&self, key: Option<&str>) -> Result<()> {
        let client = self.client()?.clone();

        if let Some(recovery_key) = auth::pick_recovery_key(key, &self.config.auth, &self.config.recovery).await {
            auth::recover_device_with_fix(client, &recovery_key).await?;
        } else {
            warn!("You don't have active recovery key");
        }

        Ok(())
    }

    pub async fn devices_list(&self) -> Result<()> {
        let client = self.client()?.clone();
        auth::devices_list(client).await?;
        Ok(())
    }

    pub async fn verify_some_device(&self, device_id: &str) -> Result<()> {
        let client = self.client()?.clone();
        auth::verify_some_device(client, device_id).await?;
        Ok(())
    }

    pub async fn recall_recovery_key(&self) -> Result<()> {
        auth::recall_recovery_key().await?;
        Ok(())
    }

    pub async fn reset_recovery(&self) -> Result<()> {
        let client = self.client()?.clone();
        auth::reset_recovery(client).await?;
        Ok(())
    }

    pub async fn reset_recovery_with_backup(&self, key: Option<&str>) -> Result<()> {
        let client = self.client()?.clone();

        if let Some(recovery_key) = auth::pick_recovery_key(key, &self.config.auth, &self.config.recovery).await {
            auth::reset_recovery_with_backup(client, &recovery_key).await?;
        } else {
            warn!("You don't have active recovery key");
        }

        Ok(())
    }

    pub async fn cleanup(&self, level: &str) -> Result<()> {
        info!("DB cleaning module is under development (level: {}).", level);
        // dummy. work directly with the database without a client.
        Ok(())
    }

    /// Bot logic: restoring reminders, setting events handlers.
    pub async fn run_bot(&mut self) -> Result<()> {
        // Initialization check
        if self.client.is_none() {
            self.init_client().await?;
        }
        let client = self.client.as_ref().unwrap();

        // DB
        let db_conn = reminder::init_db().await?;
        self.db_conn = Some(db_conn);

        // Restore reminders
        reminder::restore_reminders(client.clone(), self.db_conn.as_ref().unwrap().clone()).await?;

        // Handlers
        let client_clone = client.clone();
        let db_clone = self.db_conn.as_ref().unwrap().clone();
        client.add_event_handler(move |event, room| {
            handlers::on_room_message(event, room, client_clone.clone(), db_clone.clone())
        });
        client.add_event_handler(handlers::on_stripped_state_member);

        // Getting synchronization settings
        //let sync_settings = auth::get_sync(client, &self.sync_token, &self.session_file).await?;
        let sync_settings = self.sync_settings.as_ref().unwrap();

        // Synchronization and Ctrl+C break-up
        tokio::select! {
            result = auth::sync(client.clone(), sync_settings.clone(), &self.session_file) => {
                result?;
            }
            _ = signal::ctrl_c() => {
                info!("🛑 The application is terminating...");
                // https://docs.rs/matrix-sdk/latest/matrix_sdk/struct.Client.html#method.sync
                //
                //client.sync_service().stop().await?;

                // 
                // drop(client); или let _ = client;
            }
        }

        Ok(())
    }
}

// ===== main =====
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let cli = Cli::parse();
    let mut app = AppContext::new().await?;

    match cli.command {
        Some(Commands::Recover { recovery_key, fix_backup }) => {
            app.init_client().await?;
            if fix_backup {
                app.recover_with_fix(recovery_key.as_deref()).await?;
            } else {
                app.recover(recovery_key.as_deref()).await?;
            }
        }
        Some(Commands::DevicesList) => {
            app.init_client().await?;
            app.devices_list().await?;
            return Ok(())
        }
        Some(Commands::VerifySomeDevice { device_id }) => {
            app.init_client().await?;
            app.verify_some_device(&device_id).await?;
        }
        Some(Commands::RecallRecoveryKey) => {
            app.recall_recovery_key().await?;
        }
        Some(Commands::ResetRecovery { backup, recovery_key, confirmation }) => {
            if !confirmation {
                warn!("Use --confirmation flag");
                return Ok(())
            }
            if !backup {
                app.reset_recovery().await?;
            } else {
                app.reset_recovery_with_backup(recovery_key.as_deref()).await?;
            }
            
        }
        Some(Commands::Cleanup { level }) => {
            // without init_client
            app.cleanup(&level).await?;
        }
        None => {
            app.run_bot().await?;
        }
    }

    Ok(())
}