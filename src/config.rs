use std::{
    path::{Path, PathBuf},
};
use tokio::fs;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Config for AppContext
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub auth: MatrixConfig,
    pub recovery: Option<RecoveryConfig>,
    pub i18n: I18nConfig,
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

/// Language
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct I18nConfig {
    pub app: String,
    pub bot_command: String,
}

impl AppConfig {
    pub async fn load() -> Result<Self> {
        //dotenvy::dotenv().ok();

        let auth: MatrixConfig = envy::prefixed("MATRIX_")
            .from_env()
            .map_err(|e| anyhow::anyhow!("Environment error: {}", e))?;

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

        let i18n: I18nConfig = envy::prefixed("LANG_")
            .from_env()
            .map_err(|e| anyhow::anyhow!("Environment error: {}", e))?;

        Ok(Self { auth, recovery, i18n })
    }
}