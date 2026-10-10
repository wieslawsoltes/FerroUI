use std::env;
use std::path::Path;
use std::process::Command;

/// Three things depend on the target:
///
/// - Which GPU back end of Skia the build has. The Skia dependency is built
///   with Ganesh on OpenGL where the manifest of this crate asks for it; the
///   `ferro_skia_ganesh_gl` configuration tells the sources.
/// - Skia's prebuilt archive for the browser uses Emscripten's script-based
///   setjmp and longjmp, which a module that unwinds with WebAssembly
///   exceptions (Rust 1.93 and later) does not provide:
///   `emscripten/emscripten_sjlj.cpp` provides them.
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
        build_emscripten_sjlj();
    }
    if env::var("CARGO_CFG_WINDOWS").is_ok() {
        println!("cargo:rustc-cfg=ferro_skia_ganesh_gl");
    }
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-cfg=ferro_skia_ganesh_gl");
    }

    if env::var("CARGO_CFG_TARGET_VENDOR").as_deref() != Ok("apple") {
        return;
    }

    // The runtime of the iOS simulator is a library of its own: the one of
    // the devices has no slice for it, and the linker refuses the archive.
    let simulator = env::var("CARGO_CFG_TARGET_ABI").as_deref() == Ok("sim")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("sim")
        || env::var("TARGET").is_ok_and(|target| target == "x86_64-apple-ios");
    let runtime = match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => "clang_rt.osx",
        Ok("ios") if simulator => "clang_rt.iossim",
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
        // The runtime is a universal archive. The compiler takes the slice
        // of the target out of one when it puts a static library into the
        // library of this crate, for macOS only; for iOS the archive is
        // left to the linker of the final link, which reads universal
        // archives itself.
        if runtime == "clang_rt.osx" {
            println!("cargo:rustc-link-lib=static={runtime}");
        } else {
            println!("cargo:rustc-link-lib=static:-bundle={runtime}");
        }
    }
}

/// Compiles the setjmp and longjmp support of Skia's archive (see the file) for
/// wasm32-unknown-emscripten. Rust 1.93 is the first release whose standard
/// library for the target unwinds with WebAssembly exceptions; with an earlier
/// one the module unwinds with script exceptions, and Emscripten's runtime
/// provides what this file defines.
fn build_emscripten_sjlj() {
    const SOURCE: &str = "emscripten/emscripten_sjlj.cpp";
    println!("cargo:rerun-if-changed={SOURCE}");

    // "rustc 1.99.0 (b940084d7 2026-09-28)"
    let rustc = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = Command::new(rustc).arg("--version").output().ok();
    let minor = version
        .as_ref()
        .and_then(|output| String::from_utf8_lossy(&output.stdout).split_whitespace().nth(1).map(str::to_string))
        .and_then(|release| release.split('.').nth(1).and_then(|minor| minor.parse::<u32>().ok()));
    if let Some(minor) = minor.filter(|minor| *minor < 93) {
        panic!(
            "the browser build needs Rust 1.93 or later, which unwinds with WebAssembly exceptions on wasm32-unknown-emscripten (this is 1.{minor}); see scripts/browser/setup.sh"
        );
    }

    // The exception model of the module: the jump is a WebAssembly exception.
    cc::Build::new()
        .cpp(true)
        .file(SOURCE)
        .flag("-fwasm-exceptions")
        // EMCC_CFLAGS of the browser build carries a link setting for Skia.
        .flag("-Wno-unused-command-line-argument")
        .warnings(true)
        .compile("ferroui_emscripten_sjlj");
}
