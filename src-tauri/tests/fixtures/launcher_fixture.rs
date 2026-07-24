use std::{env, path::PathBuf, process::Command, process::ExitCode};

fn execute() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args().skip(1);
    if arguments.next().as_deref() != Some("--target") {
        return Err("expected --target followed by an executable path".into());
    }
    let target = PathBuf::from(arguments.next().ok_or("missing target executable")?);
    Command::new(target).args(arguments).spawn()?;
    Ok(())
}

fn main() -> ExitCode {
    match execute() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
