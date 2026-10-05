use std::env;
use std::path::Path;
use std::process::Command;

/// Two things depend on the target:
///
/// - Which GPU back end of Skia the build has. The Skia dependency is built
///   with Ganesh on OpenGL where the manifest of this crate asks for it; the
///   `ferro_skia_ganesh_gl` configuration tells the sources.
/// - Skia's Metal code uses `@available` checks, which compile to calls into
///   the compiler runtime (`__isPlatformVersionAtLeast`). Rust links without
///   the default libraries, so the clang runtime has to be named explicitly
///   on Apple targets.
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-check-cfg=cfg(ferro_skia_ganesh_gl)");

    // Keep in step with the target tables of the manifest.
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten") {
        println!("cargo:rustc-cfg=ferro_skia_ganesh_gl");
    }

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
