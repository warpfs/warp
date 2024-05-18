use super::Key;
use crate::config::AppConfig;
use crate::key::KeyMgr;
use clap::{value_parser, Arg, ArgMatches, Command};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

/// Command to initialize a new respotiroy.
pub struct Init {
    config: Arc<AppConfig>,
    keymgr: Arc<KeyMgr>,
}

impl Init {
    pub const NAME: &'static str = "init";

    pub fn new(config: Arc<AppConfig>, keymgr: Arc<KeyMgr>) -> Self {
        Self { config, keymgr }
    }
}

impl super::Command for Init {
    fn is_matched(&self, name: &str) -> bool {
        name == Self::NAME
    }

    fn definition(&self) -> clap::Command {
        Command::new(Self::NAME)
            .about("Setup an existing directory to be resume on another computer")
            .arg(
                Arg::new("name")
                    .help("Unique name of this directory on the server (default to directory name)")
                    .long("name")
                    .value_name("NAME"),
            )
            .arg(
                Arg::new("server")
                    .help(format!(
                        "URL of the server to use (default to {})",
                        self.config.default_server
                    ))
                    .long("server")
                    .value_name("URL"),
            )
            .arg(
                Arg::new("directory")
                    .help("The directory to setup (default to current directory)")
                    .value_name("DIRECTORY")
                    .value_parser(value_parser!(PathBuf)),
            )
    }

    fn exec(&self, _: &ArgMatches) -> ExitCode {
        // Check if we have at least one key to encrypt.
        if !self.keymgr.has_keys() {
            eprintln!("No file encryption keys available, invoke Warp with '{} --help' to see how to create a new key.", Key::NAME);
            return ExitCode::FAILURE;
        }

        todo!()
    }
}
