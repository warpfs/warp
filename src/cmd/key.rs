use clap::{ArgMatches, Command};
use std::process::ExitCode;

/// Command to manage file encryption keys.
pub struct Key {}

impl Key {
    pub const NAME: &'static str = "key";

    pub fn new() -> Self {
        Self {}
    }
}

impl super::Command for Key {
    fn is_matched(&self, name: &str) -> bool {
        name == Self::NAME
    }

    fn definition(&self) -> Command {
        Command::new(Self::NAME)
            .about("Manage file encryption keys")
            .subcommand_required(true)
            .subcommand(Command::new("ls").about("List all available keys"))
    }

    fn exec(&self, _: &ArgMatches) -> ExitCode {
        todo!()
    }
}
