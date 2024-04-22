pub use self::init::*;
pub use self::key::*;
pub use self::keystore::*;
use std::process::ExitCode;

mod init;
mod key;
mod keystore;

/// A single command passed from a command line argument.
pub trait Command {
    fn is_matched(&self, name: &str) -> bool;
    fn definition(&self) -> clap::Command;
    fn exec(&self, args: &clap::ArgMatches) -> ExitCode;
}
