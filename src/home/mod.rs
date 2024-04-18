use dirs::home_dir;
use std::path::PathBuf;
use thiserror::Error;

/// Encapsulate application home directory.
pub struct Home(PathBuf);

impl Home {
    pub fn new() -> Result<Self, HomeError> {
        // Get our home directory.
        let mut path = match home_dir() {
            Some(v) => v,
            None => return Err(HomeError::GetUserHomeFailed),
        };

        path.push(".warp");

        // Create our home if not exists.
        if let Err(e) = std::fs::create_dir(&path) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(HomeError::CreateDirectoryFailed(path, e));
            }
        }

        Ok(Self(path))
    }

    pub fn config(&self) -> PathBuf {
        self.0.join("config.yml")
    }
}

/// Represents an error when [`Home::new()`] fails.
#[derive(Debug, Error)]
pub enum HomeError {
    #[error("couldn't get path to user home")]
    GetUserHomeFailed,

    #[error("couldn't create {0}")]
    CreateDirectoryFailed(PathBuf, #[source] std::io::Error),
}
