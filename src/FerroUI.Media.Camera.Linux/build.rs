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
        let lib = pkg_config::Config::new()
            .atleast_version(version)
            .probe(package)
            .unwrap_or_else(|e| panic!("Linux portal camera needs {package}: {e}"));
        for path in lib.include_paths {
            build.include(path);
        }
    }
    build
        .file("native/portal.c")
        .flag_if_supported("-std=c11")
        .compile("ferroui_media_linux_portal");
}
