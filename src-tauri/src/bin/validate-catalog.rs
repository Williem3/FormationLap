use formation_lap_lib::validate_catalog_documents;
use std::{env, fs, path::PathBuf, process::ExitCode};

fn execute() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    if arguments.next().as_deref() != Some("--sims") {
        return Err("expected --sims followed by the sim catalog path".to_owned());
    }
    let sims_path = PathBuf::from(
        arguments
            .next()
            .ok_or("missing sim catalog path after --sims")?,
    );
    if arguments.next().as_deref() != Some("--applications") {
        return Err(
            "expected --applications followed by the Supporting Application catalog path"
                .to_owned(),
        );
    }
    let applications_path = PathBuf::from(
        arguments
            .next()
            .ok_or("missing Supporting Application catalog path after --applications")?,
    );
    if arguments.next().is_some() {
        return Err("unexpected catalog validator argument".to_owned());
    }

    let sims = fs::read_to_string(&sims_path)
        .map_err(|error| format!("could not read {}: {error}", sims_path.display()))?;
    let applications = fs::read_to_string(&applications_path)
        .map_err(|error| format!("could not read {}: {error}", applications_path.display()))?;
    let snapshot =
        validate_catalog_documents(&sims, &applications).map_err(|error| error.to_string())?;
    println!(
        "Catalog validation passed: {} sims, {} Supporting Applications.",
        snapshot.primary_sims.len(),
        snapshot.supporting_applications.len()
    );
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
