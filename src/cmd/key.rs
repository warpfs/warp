use crate::key::KeyMgr;
use clap::{ArgMatches, Command};
use std::process::ExitCode;
use std::sync::Arc;
use time::format_description::well_known::Rfc2822;
use time::{OffsetDateTime, UtcOffset};

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
        let mut table = tabled::builder::Builder::new();
        let local = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);

        table.push_record(["ID", "Imported Date"]);

        for key in self.keymgr.keys() {
            let id = key.id();
            let imported = OffsetDateTime::from(key.created()).to_offset(local);

            table.push_record([id.to_string(), imported.format(&Rfc2822).unwrap()]);
        }

        println!("{}", table.build());

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
