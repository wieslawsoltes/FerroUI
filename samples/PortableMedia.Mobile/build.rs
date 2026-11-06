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
}
