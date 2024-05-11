use crate::key::KeyMgr;
use clap::{ArgMatches, Command};
use std::process::ExitCode;
use std::sync::Arc;

/// Command to manage file encryption keys.
pub struct Key {
    keymgr: Arc<KeyMgr>,
}

impl Key {
    pub const NAME: &'static str = "key";

    pub fn new(keymgr: Arc<KeyMgr>) -> Self {
        Self { keymgr }
    }

    fn ls(&self, _: &ArgMatches) -> ExitCode {
        let mut t = tabled::builder::Builder::new();

        t.push_record(["ID"]);

        for k in self.keymgr.keys() {
            t.push_record([k.id().to_string()]);
        }

        println!("{}", t.build());

        ExitCode::SUCCESS
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

    fn exec(&self, args: &ArgMatches) -> ExitCode {
        match args.subcommand().unwrap() {
            ("ls", args) => self.ls(args),
            _ => unreachable!(),
        }
    }
}
