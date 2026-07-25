#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    let pipe_argument = arguments.next().and_then(|value| value.into_string().ok());
    let extra_argument = arguments.next().and_then(|value| value.into_string().ok());
    if arguments.next().is_some() {
        return std::process::ExitCode::FAILURE;
    }
    let Some(pipe_argument) = pipe_argument else {
        return std::process::ExitCode::FAILURE;
    };
    #[cfg(feature = "process-fixtures")]
    if extra_argument.as_deref() == Some("--allow-development-test-caller") {
        return match formation_lap_lib::run_elevated_helper_for_test(&pipe_argument) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(_) => std::process::ExitCode::FAILURE,
        };
    }
    if extra_argument.is_some() {
        return std::process::ExitCode::FAILURE;
    }
    match formation_lap_lib::run_elevated_helper(&pipe_argument) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(_) => std::process::ExitCode::FAILURE,
    }
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    std::process::ExitCode::FAILURE
}
