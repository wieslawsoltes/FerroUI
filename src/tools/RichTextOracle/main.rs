//! The Msftedit oracle: characterises the system RichEdit (`Msftedit.dll`) through its text object
//! model (TOM) and prints one `SYSTEM_MSFTEDIT_*` line per observation on standard output.
//!
//! It asserts nothing about FerroUI. The observation extractor
//! (`scripts/winui-rich-text/rich-text-tom-observations.py`) turns the output into a record, and
//! its `check-cases` command compares the record with the case tables of the editor's tests
//! (`src/FerroUI.Controls/rich_text/*_native_cases.rs`). See docs/porting/winui-rich-text.md,
//! section 8.
//!
//! ```text
//! cargo run --locked -p ferroui-rich-text-oracle > msftedit.log
//! cargo run --locked -p ferroui-rich-text-oracle -- link gravity > msftedit.log
//! ```
//!
//! With no argument every probe runs, one after another, each on a thread of its own (the OLE
//! apartment of a probe is its thread's). Arguments name the probes to run.

#[cfg(windows)]
mod probes;

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    use std::process::ExitCode;

    let requested: Vec<String> = std::env::args().skip(1).collect();
    if requested.iter().any(|argument| argument == "--help" || argument == "-h") {
        let names: Vec<&str> = probes::PROBES.iter().map(|probe| probe.name).collect();
        eprintln!("usage: ferroui-rich-text-oracle [PROBE...]\nprobes: {}", names.join(" "));
        return ExitCode::SUCCESS;
    }
    if let Some(unknown) = requested
        .iter()
        .find(|name| !probes::PROBES.iter().any(|probe| probe.name == name.as_str()))
    {
        eprintln!("ferroui-rich-text-oracle: no probe named {unknown:?} (see --help)");
        return ExitCode::from(2);
    }
    let mut failed = false;
    for probe in probes::PROBES {
        if !requested.is_empty() && !requested.iter().any(|name| name == probe.name) {
            continue;
        }
        let outcome = std::thread::Builder::new()
            .name(probe.thread.into())
            .spawn(probe.run)
            .map_err(|error| format!("could not start the thread: {error}"))
            .and_then(|thread| thread.join().map_err(|_| "the probe panicked".to_string()))
            .and_then(|result| result);
        match outcome {
            Ok(()) => eprintln!("probe {}: done", probe.name),
            Err(error) => {
                eprintln!("probe {}: failed: {error}", probe.name);
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("ferroui-rich-text-oracle characterises the Msftedit of Windows and runs on Windows only");
    std::process::ExitCode::FAILURE
}
