use crate::cmd::{Command, Init};
use crate::config::AppConfig;
use dirs::home_dir;
use erdp::ErrorDisplay;
use std::fs::File;
use std::io::BufReader;
use std::process::ExitCode;
use std::sync::Arc;

mod cmd;
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

    // Setup commands.
    let mut args = clap::Command::new("warp");
    let config = Arc::new(config);
    let commands: Vec<Box<dyn Command>> = vec![Box::new(Init::new(config.clone()))];

    for cmd in &commands {
        args = args.subcommand(cmd.definition());
    }

    // Execute the command.
    let args = args.get_matches();
    let (name, args) = match args.subcommand() {
        Some(v) => v,
        None => return warp(),
    };

    for cmd in commands {
        if cmd.is_matched(name) {
            return cmd.exec(args);
        }
    }

    unreachable!()
}

fn warp() -> ExitCode {
    // Get current shell.
    let shell = match std::env::var_os("SHELL") {
        Some(v) => v,
        None => {
            eprintln!("No SHELL environment variable.");
            return ExitCode::FAILURE;
        }
    };

    // Prepare to launch the shell.
    let mut cmd = std::process::Command::new(&shell);

    // Launch the shell.
    if let Err(e) = cmd.status() {
        eprintln!("Failed to launch {}: {}.", shell.to_string_lossy(), e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
