fn main() {
    println!("cargo:rerun-if-changed=native/buffer.m");
    if std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() != Ok("apple") {
        return;
    }
    cc::Build::new()
        .file("native/buffer.m")
        .flag("-fobjc-arc")
        .flag("-fblocks")
        .compile("ferroui_media_apple");
    for framework in ["Foundation", "CoreVideo", "IOSurface"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
