//! Links the Osprey application the host runs (docs/osprey-native-host.md, section 2).
//!
//! `OSPREY_NATIVE_APP_DIR` is the folder that holds `libOspreyApp.a`, the application compiled
//! with `osprey <app> --compile --target=macos -o <folder>/libOspreyApp.a`. On macOS the archive
//! is linked beside the Mac ARC runtime `libosprey_runtime_macos_arc.a` from `OSPREY_RUNTIME_DIR`
//! (the folder of the `osprey` compiler, which ships the runtime archives); a Linux or Windows
//! archive bundles its native runtime and needs only the system libraries that runtime calls.
//! More libraries an application needs are named in `OSPREY_NATIVE_LINK_SEARCH` (folders
//! separated by `:`) and `OSPREY_NATIVE_LINK_LIBS` (cargo link specs separated by `,`).
//!
//! Without `OSPREY_NATIVE_APP_DIR` nothing is linked and the configuration `osprey_app` is not
//! set: the library builds and its tests run without an application (the boundary is then a
//! stand-in the tests drive), and the binary says how to link one.

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(osprey_app)");
    for name in [
        "OSPREY_NATIVE_APP_DIR",
        "OSPREY_RUNTIME_DIR",
        "OSPREY_NATIVE_LINK_SEARCH",
        "OSPREY_NATIVE_LINK_LIBS",
    ] {
        println!("cargo::rerun-if-env-changed={name}");
    }
    let Some(app) = env::var_os("OSPREY_NATIVE_APP_DIR").map(PathBuf::from) else {
        return;
    };
    let archive = app.join("libOspreyApp.a");
    if !archive.is_file() {
        panic!(
            "OSPREY_NATIVE_APP_DIR names {}, which holds no libOspreyApp.a",
            app.display()
        );
    }
    println!("cargo::rerun-if-changed={}", archive.display());
    println!("cargo::rustc-cfg=osprey_app");
    println!("cargo::rustc-link-search=native={}", app.display());
    let search = env::var("OSPREY_NATIVE_LINK_SEARCH").unwrap_or_default();
    for folder in search.split(':').filter(|folder| !folder.is_empty()) {
        println!("cargo::rustc-link-search=native={folder}");
    }
    println!("cargo::rustc-link-lib=static=OspreyApp");
    match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("macos") => {
            let runtime = env::var_os("OSPREY_RUNTIME_DIR")
                .map(PathBuf::from)
                .expect("OSPREY_RUNTIME_DIR must name the folder of libosprey_runtime_macos_arc.a");
            println!(
                "cargo::rerun-if-changed={}",
                runtime.join("libosprey_runtime_macos_arc.a").display()
            );
            println!("cargo::rustc-link-search=native={}", runtime.display());
            println!("cargo::rustc-link-lib=static=osprey_runtime_macos_arc");
            // The runtime's GPU compute adapter is Metal through the Objective-C runtime.
            for framework in ["Metal", "Foundation"] {
                println!("cargo::rustc-link-lib=framework={framework}");
            }
            println!("cargo::rustc-link-lib=dylib=objc");
        }
        Ok("windows") => println!("cargo::rustc-link-lib=dylib=ws2_32"),
        _ => {
            for system in ["pthread", "dl", "m"] {
                println!("cargo::rustc-link-lib=dylib={system}");
            }
        }
    }
    let libs = env::var("OSPREY_NATIVE_LINK_LIBS").unwrap_or_default();
    for lib in libs.split(',').filter(|lib| !lib.is_empty()) {
        println!("cargo::rustc-link-lib={lib}");
    }
}
