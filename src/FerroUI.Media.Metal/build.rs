fn main() {
    println!("cargo:rerun-if-changed=native/frames.m");
    if std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() != Ok("apple") {
        return;
    }
    cc::Build::new()
        .file("native/frames.m")
        .flag("-fobjc-arc")
        .flag("-fblocks")
        .compile("ferroui_media_metal");
    for framework in ["Foundation", "Metal", "CoreVideo", "IOSurface"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
