use crate::config::AppConfig;
use crate::key::KeyMgr;
use clap::builder::NonEmptyStringValueParser;
use clap::{Arg, ArgMatches, Command};
use erdp::ErrorDisplay;
use std::process::ExitCode;
use std::sync::Arc;
use time::format_description::well_known::Rfc2822;
use time::{OffsetDateTime, UtcOffset};

/// Command to manage file encryption keys.
pub struct Key {
    config: Arc<AppConfig>,
    keymgr: Arc<KeyMgr>,
}

impl Key {
    pub const NAME: &'static str = "key";

    pub fn new(config: Arc<AppConfig>, keymgr: Arc<KeyMgr>) -> Self {
        Self { config, keymgr }
    }

    fn exec_new(&self, args: &ArgMatches) -> ExitCode {
        // Get target store.
        let store: &String = args
            .get_one("store")
            .unwrap_or(&self.config.key.default_store);

        // Generate.
        let key = match self.keymgr.generate(store) {
            Ok(Some(v)) => v,
            Ok(None) => {
                eprintln!("Unknown keystore '{store}'.");
                return ExitCode::FAILURE;
            }
            Err(e) => {
                eprintln!("Failed to create a key: {}.", e.display());
                return ExitCode::FAILURE;
            }
        };

        println!("{}", key.id());

        ExitCode::SUCCESS
    }

    fn exec_ls(&self, _: &ArgMatches) -> ExitCode {
        let mut table = tabled::builder::Builder::new();
        let local = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);

        table.push_record(["ID", "Imported Date"]);

        self.keymgr.for_each_key(|key| {
            let id = key.id();
            let imported = OffsetDateTime::from(key.created()).to_offset(local);

            table.push_record([id.to_string(), imported.format(&Rfc2822).unwrap()]);
        });

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
            .subcommand(
                Command::new("new").about("Create a new key").arg(
                    Arg::new("store")
                        .help(format!(
                            "Key store to use (default to '{}')",
                            self.config.key.default_store
                        ))
                        .long("store")
                        .value_name("ID")
                        .value_parser(NonEmptyStringValueParser::new()),
                ),
            )
            .subcommand(Command::new("ls").about("List all available keys"))
    }

    fn exec(&self, args: &ArgMatches) -> ExitCode {
        match args.subcommand().unwrap() {
            ("new", args) => self.exec_new(args),
            ("ls", args) => self.exec_ls(args),
            _ => unreachable!(),
        }
    }
}
