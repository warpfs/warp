pub use self::init::*;
use std::process::ExitCode;

mod init;

/// A single command passed from a command line argument.
pub trait Command {
    fn is_matched(&self, name: &str) -> bool;
    fn definition(&self) -> clap::Command;
    fn exec(&self, args: &clap::ArgMatches) -> ExitCode;
}
