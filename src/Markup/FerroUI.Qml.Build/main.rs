use ferroui_qml_build::Compiler;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "Usage: ferroui-qmlc [-I MODULE_ROOT]... [--check] [--javascript] INPUT.qml\nEmits a Rust module on stdout; --check only validates source and component linkage.";

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let mut compiler = Compiler::new();
    let mut input: Option<OsString> = None;
    let mut check = false;
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!("{USAGE}");
            return Ok(());
        }
        if arg == "-I" {
            compiler = compiler.import_path(PathBuf::from(
                args.next().ok_or("-I requires a module root")?,
            ));
        } else if arg == "--javascript" {
            #[cfg(feature = "javascript")]
            {
                compiler = compiler.with_javascript();
            }
            #[cfg(not(feature = "javascript"))]
            return Err("--javascript requires ferroui-qml-build's javascript feature".into());
        } else if arg == "--check" {
            check = true;
        } else if arg.to_string_lossy().starts_with('-') || input.replace(arg).is_some() {
            return Err(USAGE.into());
        }
    }
    let output = compiler.compile(PathBuf::from(input.ok_or(USAGE)?))?;
    if !check {
        use std::io::Write;
        std::io::stdout().lock().write_all(output.rust.as_bytes())?;
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
