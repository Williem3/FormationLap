use std::{env, fs, path::PathBuf, process::ExitCode};

fn execute() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args().skip(1);
    if arguments.next().as_deref() != Some("--signal") {
        return Err("expected --signal followed by a path".into());
    }
    let signal_path = PathBuf::from(arguments.next().ok_or("missing signal path")?);
    if arguments.next().is_some() {
        return Err("unexpected stop-fixture argument".into());
    }
    fs::write(signal_path, b"stop")?;
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
