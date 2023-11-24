use clap::Command;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Parse arguments.
    let args = Command::new("warp").get_matches();

    // Execute the command.
    let res = match args.subcommand() {
        Some(_) => todo!(),
        None => wrap(),
    };

    match res {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => e,
    }
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
