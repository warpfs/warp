pub use self::default::*;
use super::Key;
use std::error::Error;
use std::sync::Arc;

mod default;

/// Storage to keep encryption keys.
pub trait Keystore {
    fn id(&self) -> &'static str;

    fn list(self: &Arc<Self>) -> impl Iterator<Item = Key>
    where
        Self: Sized;

    fn new(self: Arc<Self>) -> Result<Key, Box<dyn Error>>;
}
