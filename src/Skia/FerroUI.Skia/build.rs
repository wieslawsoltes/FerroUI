use std::env;
use std::path::Path;
use std::process::Command;

/// Skia's Metal code uses `@available` checks, which compile to calls into
/// the compiler runtime (`__isPlatformVersionAtLeast`). Rust links without
/// the default libraries, so the clang runtime has to be named explicitly
/// on Apple targets.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if env::var("CARGO_CFG_TARGET_VENDOR").as_deref() != Ok("apple") {
        return;
    }

    let runtime = match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => "clang_rt.osx",
        Ok("ios") => "clang_rt.ios",
        _ => return,
    };

    let output = Command::new("clang").arg(format!("--print-file-name=lib{runtime}.a")).output();
    let Ok(output) = output else {
        return;
    };

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = Path::new(&path);

    // Without a directory clang did not find the library and echoed the name.
    if let Some(directory) = path.parent().filter(|directory| directory.is_dir() && path.is_file()) {
        println!("cargo:rustc-link-search=native={}", directory.display());
        println!("cargo:rustc-link-lib=static={runtime}");
    }
}
