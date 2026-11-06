fn main() {
    println!("cargo:rerun-if-changed=native/recognize.m");
    if std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() != Ok("apple") {
        return;
    }
    cc::Build::new()
        .file("native/recognize.m")
        .flag("-fobjc-arc")
        .compile("ferroui_media_qr_apple");
    for framework in ["Foundation", "Vision", "CoreImage", "CoreVideo", "ImageIO"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
