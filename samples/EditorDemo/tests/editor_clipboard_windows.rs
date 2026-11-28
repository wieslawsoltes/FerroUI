//! Native sample fixtures for the isolated Windows clipboard and rendering checks.

#[cfg(windows)]
mod native_editor_fixture;
#[cfg(windows)]
mod windows_clipboard_station;
#[cfg(windows)]
mod windows_inline_control;
#[cfg(windows)]
mod windows_pointer_render;
#[cfg(windows)]
mod windows_text_automation;

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    use native_editor_fixture::EditorFixture;
    use std::process::ExitCode;

    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.first().is_some_and(|arg| arg == "--peer") {
        let [_, station, check @ ..] = arguments.as_slice() else {
            return ExitCode::FAILURE;
        };
        if let Err(error) = windows_clipboard_station::verify_peer(station) {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
        let Some(check) = EditorFixture::from_args(check) else {
            return ExitCode::FAILURE;
        };
        return check.run();
    }
    // Ordinary cargo test invocations have no browser test environment.
    if std::env::var_os("FERROUI_TEST_WINDOWS_DESKTOP").is_none() {
        return ExitCode::SUCCESS;
    }
    match windows_clipboard_station::launch(&arguments) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(code) => {
            eprintln!("Private editor fixture exited with status {code}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn main() {}
