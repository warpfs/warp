use std::path::{Path, PathBuf};
use thiserror::Error;

/// Represents a single repository that loaded from `.warp` directory.
pub struct Repo {}

impl Repo {
    /// `path` is a path to a directory that contains `.warp` directory.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, RepoLoadError> {
        // Check if .warp exists.
        let path = path.as_ref().join(".warp");
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(v) => v,
            Err(e) => {
                return Err(if e.kind() == std::io::ErrorKind::NotFound {
                    RepoLoadError::NotWarpRepo
                } else {
                    RepoLoadError::GetMetaDataFailed(path, e)
                });
            }
        };

        // Check if .warp a directory.
        if !meta.is_dir() {
            return Err(RepoLoadError::NotWarpRepo);
        }

        todo!()
    }
}

/// Represents an error when [`Repo`] fails to load.
#[derive(Debug, Error)]
pub enum RepoLoadError {
    #[error("couldn't get metadata of {0}")]
    GetMetaDataFailed(PathBuf, #[source] std::io::Error),

    #[error("the specified path is not a Warp repository")]
    NotWarpRepo,
}
