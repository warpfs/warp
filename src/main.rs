use clap::{value_parser, Arg, ArgMatches, Command};
use std::{path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    // Parse arguments.
    let args = Command::new("warp")
        .subcommand(
            Command::new("init")
                .about("Setup an existing directory to be resuming on another computer")
                .arg(Arg::new("name").help(
                    "Unique name of this directory on the server (default to directory name)",
                ).long("name").value_name("NAME"))
                .arg(
                    Arg::new("server")
                        .help("URL of the server to use (default to https://api.warpgate.sh)").long("server").value_name("URL"),
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
