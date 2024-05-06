pub use self::default::*;
use super::Key;
use std::error::Error;
use std::sync::Arc;

mod default;

/// Storage to keep encryption keys.
pub trait Keystore: Send + Sync {
    fn id(&self) -> &'static str;

    fn list(self: &Arc<Self>) -> impl Iterator<Item = Result<Key, Box<dyn Error>>>
    where
        Self: Sized;

    fn generate(self: Arc<Self>) -> Result<Key, Box<dyn Error>>;
}
