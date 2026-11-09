//! Build script of `ferroui-native` (macOS only):
//!
//! 1. runs `microcom-codegen` on `frn.idl`, producing the C++ header
//!    `$OUT_DIR/inc/ferro-native.h` and the Rust bindings
//!    `$OUT_DIR/frn_interop.rs` (included by the `interop` module);
//! 2. writes `$OUT_DIR/native_sources.rs`, a fingerprint of the sources of the
//!    native library (included by the `interop` module);
//! 3. compiles the Objective-C++ backend in `native/FerroUI.Native/src/OSX`
//!    into static libraries and links them plus the system frameworks.
//!
//! The compiled crate carries the static libraries (`+whole-archive`), so its
//! compilation depends on them, and nothing in the command line of the Rust
//! compiler says so: a compiler cache keyed on the Rust sources and that
//! command line gives a crate built before a change of the native sources
//! back after it. The fingerprint is a Rust source of the crate, which every
//! such cache reads.
//!
//! The native sources are reached through `native`, a symbolic link of the
//! crate directory to `native/FerroUI.Native` of the repository: the package
//! of the published crate holds the files behind it, so the crate builds
//! from its package as it does in the repository.
//!
//! On docs.rs (`DOCS_RS`) the third step is left out: the documentation of
//! the macOS target is generated on a Linux host, which has no compiler for
//! Objective-C++ against the macOS SDK, and generating it links nothing.

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // A checkout without symbolic links (the link is then a file) has the sources at their place
    // in the repository.
    let linked = manifest_dir.join("native");
    let native_dir = if linked.is_dir() { linked } else { manifest_dir.join("../../native/FerroUI.Native") };
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

    // --- 2. fingerprint of the native sources -------------------------------
    println!("cargo:rerun-if-changed={}", inc_dir.display());
    println!("cargo:rerun-if-changed={}", src_dir.display());
    let mut inputs: Vec<PathBuf> = [&inc_dir, &src_dir]
        .into_iter()
        .flat_map(|dir| fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .collect();
    inputs.sort();
    // What the library is compiled from: this script (the flags), the generated header and the
    // sources.
    let mut fingerprint = Fnv1a::new();
    fingerprint.write(include_bytes!("build.rs"));
    fingerprint.write(env::var("PROFILE").unwrap_or_default().as_bytes());
    fingerprint.write(header.as_bytes());
    for path in &inputs {
        println!("cargo:rerun-if-changed={}", path.display());
        fingerprint.write(path.file_name().unwrap().as_encoded_bytes());
        fingerprint.write(&fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display())));
    }
    write_if_changed(
        &out_dir.join("native_sources.rs"),
        &format!(
            "/// A fingerprint of the sources the native library of this build was compiled from.\n\
             pub const NATIVE_SOURCES: u64 = {:#018x};\n",
            fingerprint.finish()
        ),
    );

    // --- 3. native library --------------------------------------------------
    if env::var_os("DOCS_RS").is_some() {
        return;
    }
    let mut arc_sources = Vec::new();
    let mut noarc_sources = Vec::new();
    for path in inputs.into_iter().filter(|path| path.starts_with(&src_dir)) {
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

/// FNV-1a, 64 bits: the same value for the same bytes on every host and toolchain.
struct Fnv1a(u64);

impl Fnv1a {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
        // The end of the piece, so that two pieces do not read as one.
        self.0 = (self.0 ^ 0xff).wrapping_mul(0x0000_0100_0000_01b3);
    }

    fn finish(&self) -> u64 {
        self.0
    }
}
