fn main() {
    println!("cargo:rerun-if-changed=native/media.c");
    println!("cargo:rerun-if-changed=native/media.h");
    println!("cargo:rerun-if-changed=native/frame_tests.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }
    println!("cargo:rerun-if-env-changed=FERROUI_NATIVE_MEDIA_CHECK_ONLY");
    if std::env::var_os("FERROUI_NATIVE_MEDIA_CHECK_ONLY").is_some() {
        println!(
            "cargo:warning=FFI binding check only: native Linux media is not compiled or linkable"
        );
        return;
    }
    let mut build = cc::Build::new();
    if std::env::var_os("CARGO_FEATURE_NATIVE_FRAME_TESTS").is_some() {
        build.define("FERROUI_NATIVE_FRAME_TESTS", None);
    }
    let packages = [
        "gstreamer-app-1.0",
        "gstreamer-video-1.0",
        "gstreamer-allocators-1.0",
        "egl",
        "glesv2",
    ];
    let mut libraries = Vec::new();
    for package in packages {
        let probe = pkg_config::Config::new()
            .cargo_metadata(false)
            .atleast_version(if package.starts_with("gstreamer") {
                "1.24"
            } else {
                "1.0"
            })
            .probe(package);
        match probe {
            Ok(library) => libraries.push(library),
            Err(error) => {
                // Without its system libraries the crate builds the bridge that opens no session:
                // the application's media report a native failure, the workspace still builds.
                println!(
                    "cargo:warning=native Linux media is unavailable in this build (it needs system {package}: {})",
                    error.to_string().lines().next().unwrap_or_default()
                );
                println!("cargo:rerun-if-changed=native/media_unavailable.c");
                cc::Build::new()
                    .file("native/media_unavailable.c")
                    .flag_if_supported("-std=c11")
                    .compile("ferroui_media_linux");
                return;
            }
        }
    }
    for library in libraries {
        for path in library.include_paths {
            build.include(path);
        }
    }
    build
        .file("native/media.c")
        .flag_if_supported("-std=c11")
        .compile("ferroui_media_linux");
    // Keep native dependencies after the static bridge. With --as-needed,
    // emitting them first drops libraries needed by this crate's own test bin.
    for package in packages {
        pkg_config::Config::new()
            .probe(package)
            .expect("already probed Linux media dependency");
    }
    println!("cargo:rustc-link-lib=m");
}
