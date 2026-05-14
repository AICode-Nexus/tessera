use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TesseraConfig {
    pub data_dir: Option<String>,
    #[serde(default)]
    pub providers: Vec<ProviderProfile>,
}

impl TesseraConfig {
    pub fn default_with_mock() -> Self {
        Self {
            data_dir: default_data_dir().map(|path| path.to_string_lossy().to_string()),
            providers: vec![ProviderProfile {
                id: "mock".to_string(),
                kind: "mock".to_string(),
                default_model: "mock-chat".to_string(),
                api_key_env: None,
            }],
        }
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let config = toml::from_str(&raw).map_err(std::io::Error::other)?;
        Ok(config)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: String,
    pub kind: String,
    pub default_model: String,
    pub api_key_env: Option<String>,
}

pub fn default_data_dir() -> Option<PathBuf> {
    ProjectDirs::from("dev", "tessera", "tessera").map(|dirs| dirs.data_dir().to_path_buf())
}
