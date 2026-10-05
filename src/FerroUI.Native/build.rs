//! Build script of `ferroui-native` (macOS only):
//!
//! 1. runs `microcom-codegen` on `frn.idl`, producing the C++ header
//!    `$OUT_DIR/inc/ferro-native.h` and the Rust bindings
//!    `$OUT_DIR/frn_interop.rs` (included by the `interop` module);
//! 2. compiles the Objective-C++ backend in `native/FerroUI.Native/src/OSX`
//!    into static libraries and links them plus the system frameworks.

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let native_dir = manifest_dir.join("../../native/FerroUI.Native");
    let inc_dir = native_dir.join("inc");
    let src_dir = native_dir.join("src/OSX");

    // --- 1. code generation -------------------------------------------------
    let idl_path = manifest_dir.join("frn.idl");
    println!("cargo:rerun-if-changed={}", idl_path.display());
    let idl_src = fs::read_to_string(&idl_path).expect("read frn.idl");
    let idl = microcom_codegen::parse(&idl_src)
        .unwrap_or_else(|e| panic!("{}:{e}", idl_path.display()));
    let header = microcom_codegen::cpp::generate(&idl).expect("generate C++ header");
    let bindings = microcom_codegen::rust::generate(&idl, &Default::default())
        .expect("generate Rust bindings");
    let gen_inc_dir = out_dir.join("inc");
    fs::create_dir_all(&gen_inc_dir).unwrap();
    write_if_changed(&gen_inc_dir.join("ferro-native.h"), &header);
    write_if_changed(&out_dir.join("frn_interop.rs"), &bindings);

    // --- 2. native library --------------------------------------------------
    println!("cargo:rerun-if-changed={}", inc_dir.display());
    println!("cargo:rerun-if-changed={}", src_dir.display());
    let mut arc_sources = Vec::new();
    let mut noarc_sources = Vec::new();
    for entry in fs::read_dir(&src_dir).expect("native/FerroUI.Native/src/OSX") {
        let path = entry.unwrap().path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path.extension().is_some_and(|e| e == "mm") {
            // Same per-file setting as the upstream Xcode project: everything
            // is built with ARC except noarc.mm.
            if path.file_name().is_some_and(|n| n == "noarc.mm") {
                noarc_sources.push(path);
            } else {
                arc_sources.push(path);
            }
        }
    }
    arc_sources.sort();

    // Deployment target of the upstream Xcode target; arm64 starts at 11.0.
    if env::var_os("MACOSX_DEPLOYMENT_TARGET").is_none() {
        let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
        let min = if arch == "x86_64" { "10.13" } else { "11.0" };
        env::set_var("MACOSX_DEPLOYMENT_TARGET", min);
    }

    let debug = env::var("PROFILE").as_deref() == Ok("debug");
    let configure = |arc: bool| {
        let mut b = cc::Build::new();
        b.cpp(true)
            .cargo_metadata(false)
            .warnings(false)
            .include(&gen_inc_dir)
            .include(&inc_dir)
            .include(&src_dir)
            // CLANG_CXX_LANGUAGE_STANDARD / CLANG_CXX_LIBRARY of the Xcode project.
            .flag("-std=gnu++0x")
            .flag("-stdlib=libc++")
            // CLANG_ENABLE_MODULES is YES upstream, but Xcode does not enable
            // modules for Objective-C++ sources, so no -fmodules here (with
            // it CoreFoundation's own IUnknown/REFIID clash with com.h).
            .flag(if arc { "-fobjc-arc" } else { "-fno-objc-arc" });
        if arc {
            b.flag("-fobjc-weak");
        }
        if debug {
            b.define("DEBUG", "1");
        } else {
            b.define("NS_BLOCK_ASSERTIONS", "1");
        }
        b
    };

    configure(true).files(&arc_sources).compile("ferro_native_osx");
    configure(false).files(&noarc_sources).compile("ferro_native_osx_noarc");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    // whole-archive: the Objective-C classes and categories are only reached
    // through the ObjC runtime, so the linker must not drop their objects.
    println!("cargo:rustc-link-lib=static:+whole-archive=ferro_native_osx");
    println!("cargo:rustc-link-lib=static:+whole-archive=ferro_native_osx_noarc");
    println!("cargo:rustc-link-lib=c++");
    // `@available(...)` checks compile to calls into clang's builtins runtime,
    // which rustc (linking with -nodefaultlibs) does not add on its own.
    let rt = configure(true)
        .get_compiler()
        .to_command()
        .arg("--print-file-name=libclang_rt.osx.a")
        .output()
        .ok()
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
        .filter(|p| p.is_absolute() && p.exists())
        .expect("locate libclang_rt.osx.a (clang builtins runtime)");
    println!("cargo:rustc-link-search=native={}", rt.parent().unwrap().display());
    println!("cargo:rustc-link-lib=static=clang_rt.osx");
    for framework in [
        "AppKit",
        "Foundation",
        "CoreFoundation",
        "CoreGraphics",
        "CoreVideo",
        "QuartzCore",
        "IOSurface",
        "IOKit",
        "OpenGL",
        "Carbon",
        "Metal",
        "UniformTypeIdentifiers",
    ] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}

fn write_if_changed(path: &Path, content: &str) {
    if fs::read_to_string(path).is_ok_and(|old| old == content) {
        return;
    }
    fs::write(path, content).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}
