fn main() {
    println!("cargo:rerun-if-changed=native/portal.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("linux") {
        return;
    }
    println!("cargo:rerun-if-env-changed=FERROUI_NATIVE_MEDIA_CHECK_ONLY");
    if std::env::var_os("FERROUI_NATIVE_MEDIA_CHECK_ONLY").is_some() {
        println!(
            "cargo:warning=FFI binding check only: Linux camera portal is not compiled or linkable"
        );
        return;
    }
    let mut build = cc::Build::new();
    for (package, version) in [("gio-unix-2.0", "2.56"), ("libpipewire-0.3", "0.3")] {
        let lib = match pkg_config::Config::new().cargo_metadata(false).atleast_version(version).probe(package) {
            Ok(lib) => lib,
            Err(error) => {
                // Without its system libraries the crate builds the bridge without a portal: the
                // camera reports itself unavailable, the workspace still builds.
                println!(
                    "cargo:warning=the Linux camera portal is unavailable in this build (it needs {package}: {})",
                    error.to_string().lines().next().unwrap_or_default()
                );
                println!("cargo:rerun-if-changed=native/portal_unavailable.c");
                cc::Build::new()
                    .file("native/portal_unavailable.c")
                    .flag_if_supported("-std=c11")
                    .compile("ferroui_media_linux_portal");
                return;
            }
        };
        for path in lib.include_paths {
            build.include(path);
        }
    }
    build
        .file("native/portal.c")
        .flag_if_supported("-std=c11")
        .compile("ferroui_media_linux_portal");
    for package in ["gio-unix-2.0", "libpipewire-0.3"] {
        pkg_config::Config::new()
            .probe(package)
            .expect("already probed Linux camera dependency");
    }
}
