use serde::Deserialize;
use url::Url;

/// Application configurations.
#[derive(Deserialize)]
pub struct AppConfig {
    #[serde(default = "AppConfig::default_default_server")]
    pub default_server: Url,
}

impl AppConfig {
    fn default_default_server() -> Url {
        Url::parse("https://api.warpgate.sh").unwrap()
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            default_server: Self::default_default_server(),
        }
    }
}
