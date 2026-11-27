use std::{env, fs, path::PathBuf, process::Command};

// Cargo content stamp for audited native packages; no runtime compatibility layer.
fn main() {
    println!("cargo::rerun-if-env-changed=FERRORIBBON_BUILD_FINGERPRINT");
    if let Ok(value) = std::env::var("FERRORIBBON_BUILD_FINGERPRINT") {
        assert!(
            value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid package source fingerprint"
        );
        println!("cargo::rustc-env=FERRORIBBON_BUILD_FINGERPRINT={value}");
    }
    println!("cargo::rerun-if-changed=build.rs");
    match env::var("CARGO_CFG_TARGET_OS").as_deref() {
        Ok("emscripten") => configure_browser_icu(),
        Ok("linux") => configure_linux_icu(),
        _ => {}
    }
}

fn configure_linux_icu() {
    println!("cargo::rerun-if-changed=linux_icu.c");
    let icu = pkg_config::Config::new()
        .probe("icu-i18n")
        .expect("install the target's ICU development package and pkg-config");
    let mut shim = cc::Build::new();
    shim.includes(icu.include_paths).file("linux_icu.c");
    for (name, value) in &icu.defines {
        shim.define(name, value.as_deref());
    }
    shim.compile("ferroribbon_linux_icu");
}

fn configure_browser_icu() {
    println!("cargo::rerun-if-env-changed=EM_CACHE");
    println!("cargo::rerun-if-env-changed=EM_CONFIG");
    println!("cargo::rerun-if-env-changed=EM_PORTS");
    let threads = env::var("CARGO_CFG_TARGET_FEATURE")
        .unwrap_or_default()
        .split(',')
        .any(|feature| feature == "atomics");
    let port = if threads { "icu-mt" } else { "icu" };
    let status = Command::new("embuilder")
        .args(["build", port])
        .status()
        .expect("run embuilder from the pinned Emscripten SDK");
    assert!(status.success(), "build the Emscripten ICU port");

    let cache = Command::new("em-config")
        .arg("CACHE")
        .output()
        .expect("locate the Emscripten cache");
    assert!(cache.status.success(), "read the Emscripten cache location");
    let cache = PathBuf::from(
        String::from_utf8(cache.stdout)
            .expect("Emscripten cache path")
            .trim(),
    );
    let ports = env::var_os("EM_PORTS")
        .map(PathBuf::from)
        .unwrap_or_else(|| cache.join("ports"));
    let source = ports.join("icu/icu");
    let data = source.join("source/data/in/icudt68l.dat");
    let license = source.join("LICENSE");
    assert!(
        data.is_file(),
        "missing ICU 68 common data at {}",
        data.display()
    );
    println!("cargo::rerun-if-changed={}", data.display());
    println!("cargo::rerun-if-changed={}", license.display());
    println!(
        "cargo::rustc-env=FERRORIBBON_BROWSER_ICU_DATA={}",
        data.display()
    );

    let libs = cache.join("sysroot/lib/wasm32-emscripten");
    let suffix = if threads { "-mt" } else { "" };
    println!("cargo::rustc-link-search=native={}", libs.display());
    for library in ["icu_i18n", "icu_common", "icu_stubdata"] {
        println!("cargo::rustc-link-lib=static:-bundle={library}{suffix}");
    }

    let site = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output")).join("browser-site");
    fs::create_dir_all(&site).expect("create browser site assets directory");
    fs::copy(license, site.join("ICU-LICENSE.txt")).expect("package the ICU license");
}
