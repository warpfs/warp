use serde::Deserialize;
use url::Url;

/// Application configurations.
#[derive(Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub default_server: Url,
    pub key: Key,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_server: Url::parse("https://api.warpgate.sh").unwrap(),
            key: Key::default(),
        }
    }
}

/// Configurations for encryption key.
#[derive(Deserialize)]
#[serde(default)]
pub struct Key {
    pub default_storage: bool,
}

impl Default for Key {
    fn default() -> Self {
        Self {
            default_storage: true,
        }
    }
}
