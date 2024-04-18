use crate::config::AppConfig;
use clap::{value_parser, Arg, ArgMatches, Command};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

/// Command to initialize a new respotiroy.
pub struct Init {
    config: Arc<AppConfig>,
}

impl Init {
    pub const NAME: &'static str = "init";

    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
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
        todo!()
    }
}
