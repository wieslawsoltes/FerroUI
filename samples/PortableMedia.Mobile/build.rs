fn main() {
    println!("cargo:rerun-if-changed=plane.m");
    if std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() == Ok("apple") {
        cc::Build::new()
            .file("plane.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("portable_media_plane");
        println!("cargo:rustc-link-lib=framework=Metal");
    }
    // The units of the WinUI theme the application needs (docs/xaml-themes.md, section 3): the
    // build reads the sources of the crate (`lib.rs` and `app.rs`, and `main.rs`), finds the
    // controls they make, and writes the table `app.rs` constructs the theme with.
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .theme("WinUITheme")
        .run();
}
