use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, warn};
use matrix_sdk::Client;

use crate::{AppConfig, BotRuntime, BotManager};
use crate::auth;

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

    /// Generate configuration file interactive
    SetupConfig,
}

// === CLI Commands ===
    
async fn recover(client: Client, config: &AppConfig, key: Option<&str>) -> Result<()> {
    if let Some(recovery_key) = auth::pick_recovery_key(key, &config.auth, &config.recovery).await {
        auth::recover_device(client, &recovery_key).await?;
    } else {
        warn!("You do not have active recovery key");
    }
    
    Ok(())
}

async fn recover_with_fix(client: Client, config: &AppConfig, key: Option<&str>) -> Result<()> {
    if let Some(recovery_key) = auth::pick_recovery_key(key, &config.auth, &config.recovery).await {
        auth::recover_device_with_fix(client, &recovery_key).await?;
    } else {
        warn!("You do not have active recovery key");
    }

    Ok(())
}

async fn devices_list(client: Client) -> Result<()> {
    auth::devices_list(client).await?;
    Ok(())
}

async fn verify_some_device(client: Client, device_id: &str) -> Result<()> {
    auth::verify_some_device(client, device_id).await?;
    Ok(())
}

async fn recall_recovery_key() -> Result<()> {
    auth::recall_recovery_key().await?;
    Ok(())
}

async fn reset_recovery(client: Client) -> Result<()> {
    auth::reset_recovery(client).await?;
    Ok(())
}

async fn reset_recovery_with_backup(client: Client, config: &AppConfig, key: Option<&str>) -> Result<()> {
    if let Some(recovery_key) = auth::pick_recovery_key(key, &config.auth, &config.recovery).await {
        auth::reset_recovery_with_backup(client, &recovery_key).await?;
    } else {
        warn!("You do not have active recovery key");
    }

    Ok(())
}

fn cleanup(level: &str) -> Result<()> {
    info!("DB cleaning module is under development (level: {}).", level);
    // dummy. works directly with the database without a client.
    Ok(())
}

pub async fn run(config: &mut AppConfig) -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // Commands without Client or BotRuntime
        Some(Commands::Cleanup { level }) => {
            cleanup(&level)?;
            return Ok(());
        }
        Some(Commands::SetupConfig) => {
            config.bot.setup_config(&config.data_dir)?;
            return Ok(());
        }
        
        // Other Commands
        other_command => {
            let runtime = BotRuntime::init(&config).await?;
            let client = runtime.client.clone();

            match other_command {
                Some(Commands::Recover { recovery_key, fix_backup }) => {
                    if fix_backup {
                        recover_with_fix(client, &config, recovery_key.as_deref()).await?;
                    } else {
                        recover(client, &config, recovery_key.as_deref()).await?;
                    }
                }
                Some(Commands::DevicesList) => {
                    devices_list(client).await?;
                }
                Some(Commands::VerifySomeDevice { device_id }) => {
                    verify_some_device(client, &device_id).await?;
                }
                Some(Commands::RecallRecoveryKey) => {
                    recall_recovery_key().await?;
                }
                Some(Commands::ResetRecovery { backup, recovery_key, confirmation }) => {
                    if !confirmation {
                        warn!("Use --confirmation flag");
                        return Ok(())
                    }
                    if !backup {
                        reset_recovery(client).await?;
                    } else {
                        reset_recovery_with_backup(client, &config, recovery_key.as_deref()).await?;
                    }
                    
                }
                
                None => {
                    let manager = BotManager::new(&runtime, config).await?;
                    manager.start(&runtime).await?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}