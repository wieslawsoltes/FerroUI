fn main() {
    println!("cargo:rerun-if-changed=native/player.mm");
    if std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() != Ok("apple") {
        return;
    }
    cc::Build::new()
        .cpp(true)
        .flag("-std=c++17")
        .file("native/player.mm")
        .flag("-fobjc-arc")
        .flag("-fblocks")
        .compile("ferroui_media_video_apple");
    for framework in ["Foundation", "AVFoundation", "CoreMedia", "CoreVideo"] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
