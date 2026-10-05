//! `microcom-codegen <file.idl> [--cpp <out.h>] [--rust <out.rs>]`

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (mut idl_path, mut cpp_out, mut rust_out) = (None, None, None);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--cpp" => cpp_out = args.next(),
            "--rust" => rust_out = args.next(),
            "-h" | "--help" => {
                println!("usage: microcom-codegen <file.idl> [--cpp <out.h>] [--rust <out.rs>]");
                return ExitCode::SUCCESS;
            }
            _ if idl_path.is_none() => idl_path = Some(a),
            _ => {
                eprintln!("unexpected argument '{a}'");
                return ExitCode::from(2);
            }
        }
    }
    let Some(idl_path) = idl_path else {
        eprintln!("usage: microcom-codegen <file.idl> [--cpp <out.h>] [--rust <out.rs>]");
        return ExitCode::from(2);
    };
    match run(&idl_path, cpp_out.as_deref(), rust_out.as_deref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{idl_path}: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(idl_path: &str, cpp_out: Option<&str>, rust_out: Option<&str>) -> Result<(), String> {
    let src = std::fs::read_to_string(idl_path).map_err(|e| e.to_string())?;
    let idl = microcom_codegen::parse(&src).map_err(|e| e.to_string())?;
    if let Some(p) = cpp_out {
        let h = microcom_codegen::cpp::generate(&idl)?;
        std::fs::write(p, h).map_err(|e| format!("{p}: {e}"))?;
    }
    if let Some(p) = rust_out {
        let r = microcom_codegen::rust::generate(&idl, &Default::default())?;
        std::fs::write(p, r).map_err(|e| format!("{p}: {e}"))?;
    }
    if cpp_out.is_none() && rust_out.is_none() {
        println!(
            "{} enums, {} structs, {} interfaces",
            idl.enums.len(),
            idl.structs.len(),
            idl.interfaces.len()
        );
    }
    Ok(())
}
