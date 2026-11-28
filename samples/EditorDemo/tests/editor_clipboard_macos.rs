//! AppKit peer for the Chromium and native editor clipboard round trip.

#[cfg(target_os = "macos")]
mod macos_text_automation;

#[cfg(target_os = "macos")]
mod native_editor_fixture;

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    use native_editor_fixture::EditorFixture;
    use std::process::ExitCode;

    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() {
        return ExitCode::SUCCESS;
    }
    if let [flag, args @ ..] = args.as_slice() {
        if flag == "--fixture" {
            if let Some(check) = EditorFixture::from_args(args) {
                return check.run();
            }
        }
    }
    eprintln!("Usage: editor_clipboard_macos --fixture (text-automation | (copy|paste) (rectangle|line))");
    ExitCode::FAILURE
}

#[cfg(not(target_os = "macos"))]
fn main() {}
