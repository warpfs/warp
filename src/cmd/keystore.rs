use crate::key::KeyMgr;
use clap::{ArgMatches, Command};
use std::process::ExitCode;
use std::sync::Arc;

/// Command to manage file encryption keystores.
pub struct Keystore {
    keymgr: Arc<KeyMgr>,
}

impl Keystore {
    pub const NAME: &'static str = "keystore";

    pub fn new(keymgr: Arc<KeyMgr>) -> Self {
        Self { keymgr }
    }

    fn ls(&self) -> ExitCode {
        let mut t = tabled::builder::Builder::new();

        t.push_record(["ID"]);

        for s in self.keymgr.stores() {
            t.push_record([s.id()]);
        }

        println!("{}", t.build());

        ExitCode::SUCCESS
    }
}

impl super::Command for Keystore {
    fn is_matched(&self, name: &str) -> bool {
        name == Self::NAME
    }

    fn definition(&self) -> Command {
        Command::new(Self::NAME)
            .about("Manage file encryption keystores")
            .subcommand_required(true)
            .subcommand(Command::new("ls").about("List all enabled keystores"))
    }

    fn exec(&self, args: &ArgMatches) -> ExitCode {
        match args.subcommand().unwrap() {
            ("ls", _) => self.ls(),
            _ => unreachable!(),
        }
    }
}
