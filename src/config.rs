use tokio::fs;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

/// Default language for bot messages.
pub const DEFAULT_LANG: &str = "en";
/// Default times
pub const DEFAULT_MORNING_TIME: &str = "09:00";
pub const DEFAULT_AFTERNOON_TIME: &str = "14:00";
pub const DEFAULT_EVENING_TIME: &str = "19:00";


/// Config for AppContext
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub auth: MatrixConfig,
    pub recovery: Option<RecoveryConfig>,
    pub bot: BotConfig,
}

/// Config for matrix server authentication
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct MatrixConfig {
    pub homeserver: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub device: Option<String>,
    pub recovery: Option<String>,
}

/// Recovery for account
#[derive(Debug, Serialize, Deserialize)]
pub struct RecoveryConfig {
    pub recovery_key: String,
    pub created_at: String,
}

/// Bot config
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct BotConfig {
    #[serde(default = "BotConfig::default_lang")]
    pub lang: String,
    pub command: Option<String>,
    pub on_command: Option<bool>,
    pub on_mention: Option<bool>,
    #[serde(default = "BotConfig::default_morning_time")]
    pub morning_time: String,
    #[serde(default = "BotConfig::default_afternoon_time")]
    pub afternoon_time: String,
    #[serde(default = "BotConfig::default_evening_time")]
    pub evening_time: String,
}

impl BotConfig {
    fn default_lang() -> String {
        DEFAULT_LANG.into()
    }
    fn default_morning_time() -> String {
        DEFAULT_MORNING_TIME.into()
    }
    fn default_afternoon_time() -> String {
        DEFAULT_AFTERNOON_TIME.into()
    }
    fn default_evening_time() -> String {
        DEFAULT_EVENING_TIME.into()
    }

    /// Create config.yaml from loaded BotConfig
    pub async fn save_to_file(&self) -> anyhow::Result<()> {
        let yaml_path = dirs::data_dir()
            .context("No data_dir directory found")?
            .join(super::APP_FOLDER).join("config.yaml");
        if yaml_path.exists() {
            warn!("Configuration file already exists at {}", yaml_path.display());
            return Ok(());
        }

        let yaml_str = serde_yaml_ng::to_string(&self)
            .context("Failed to serialize bot config")?;

        fs::write(&yaml_path, yaml_str).await.context("Failed to write config.yaml")?;

        info!(file = %yaml_path.display(), "Created config.yaml");

        Ok(())
    }
}

impl AppConfig {
    pub async fn load() -> Result<Self> {
        //dotenvy::dotenv().ok();

        // .env
        let auth: MatrixConfig = envy::prefixed("MATRIX_")
            .from_env()
            .map_err(|e| anyhow::anyhow!("Environment error: {}", e))?;

        // recovery.json
        let data_dir = dirs::data_dir().context("No data_dir directory found")?.join(super::APP_FOLDER);
        let recovery_file = data_dir.join("recovery.json");

        let recovery = if recovery_file.exists() {
            let serialized = fs::read_to_string(&recovery_file).await
                .context("Error reading recovery.json")?;
            let data: RecoveryConfig = serde_json::from_str(&serialized)
                .context("File recovery.json has invalid JSON")?;
            Some(data)
        } else {
            None
        };

        // config.yaml or .env
        // yaml and yml files are supported
        let yaml_path = data_dir.join("config.yaml");
        let yml_path = data_dir.join("config.yml");
        let config_path = if yaml_path.exists() {
            Some(yaml_path)
        } else if yml_path.exists() {
            Some(yml_path)
        } else {
            None
        };
        
        let bot: BotConfig = if let Some(path) = config_path {
            let yaml_str = fs::read_to_string(&path).await
                .with_context(|| format!("Error reading {}", path.display()))?;
            
            match serde_yaml_ng::from_str::<BotConfig>(&yaml_str) {
                Ok(config) => {
                    info!(file = %path.display(), "Successfully loaded bot configuration");
                    config
                },
                Err(e) => {
                    error!(error = %e, "Failed to parse YAML config. Falling back to .env");
                    envy::prefixed("BOT_")
                        .from_env()
                        .map_err(|e| anyhow::anyhow!("Environment error: {}", e))?
                }
            }
        } else {
            warn!("No config.yaml or config.yml found, using .env variables");
            envy::prefixed("BOT_")
                .from_env()
                .map_err(|e| anyhow::anyhow!("Environment error: {}", e))?
        };

        Ok(Self { auth, recovery, bot })
    }
}

