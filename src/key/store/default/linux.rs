use crate::key::Key;
use std::error::Error;

/// Iterator to list all keys from libsecret.
#[derive(Default)]
pub struct KeyList {}

impl Iterator for KeyList {
    type Item = Result<Key, Box<dyn Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
