use crate::config::AppConfig;
use clap::{value_parser, Arg, ArgMatches, Command};
use dirs::home_dir;
use erdp::ErrorDisplay;
use std::fs::File;
use std::io::BufReader;
use std::{path::PathBuf, process::ExitCode};

mod config;

fn main() -> ExitCode {
    // Get our home directory.
    let mut home = match home_dir() {
        Some(v) => v,
        None => {
            eprintln!("Failed to locate home directory.");
            return ExitCode::FAILURE;
        }
    };

    home.push(".warp");

    // Create our home if not exists.
    if let Err(e) = std::fs::create_dir(&home) {
        if e.kind() != std::io::ErrorKind::AlreadyExists {
            eprintln!("Failed to create {}: {}.", home.display(), e.display());
            return ExitCode::FAILURE;
        }
    }

    // Load application configurations.
    let path = home.join("config.yml");
    let config = match File::open(&path) {
        Ok(v) => match serde_yaml::from_reader::<_, AppConfig>(BufReader::new(v)) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to load {}: {}.", path.display(), e.display());
                return ExitCode::FAILURE;
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => AppConfig::default(),
        Err(e) => {
            eprintln!("Failed to open {}: {}.", path.display(), e.display());
            return ExitCode::FAILURE;
        }
    };

    // Parse arguments.
    let args = Command::new("warp")
        .subcommand(
            Command::new("init")
                .about("Setup an existing directory to be resume on another computer")
                .arg(Arg::new("name").help(
                    "Unique name of this directory on the server (default to directory name)",
                ).long("name").value_name("NAME"))
                .arg(
                    Arg::new("server")
                        .help(format!("URL of the server to use (default to {})", config.default_server)).long("server").value_name("URL"),
                ).arg(Arg::new("directory").help("The directory to setup (default to current directory)").value_name("DIRECTORY").value_parser(value_parser!(PathBuf))),
        )
        .get_matches();

    // Execute the command.
    let res = match args.subcommand() {
        Some(("init", args)) => init(args),
        _ => wrap(),
    };

    match res {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => e,
    }
}

fn init(_: &ArgMatches) -> Result<(), ExitCode> {
    todo!()
}

fn wrap() -> Result<(), ExitCode> {
    // Get current shell.
    let shell = match std::env::var_os("SHELL") {
        Some(v) => v,
        None => {
            eprintln!("No SHELL environment variable.");
            return Err(ExitCode::FAILURE);
        }
    };

    // Prepare to launch the shell.
    let mut cmd = std::process::Command::new(&shell);

    // Launch the shell.
    if let Err(e) = cmd.status() {
        eprintln!("Failed to launch {}: {}.", shell.to_string_lossy(), e);
        return Err(ExitCode::FAILURE);
    }

    Ok(())
}
