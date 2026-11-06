fn main() {
    println!("cargo:rerun-if-changed=native/player.mm");
    if std::env::var("CARGO_CFG_TARGET_VENDOR").as_deref() != Ok("apple") {
        return;
    }
    let mut build = cc::Build::new();
    if std::env::var_os("CARGO_FEATURE_NATIVE_LIFETIME_TESTS").is_some() {
        build.define("FERROUI_VIDEO_LIFETIME_TESTS", None);
    }
    build
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
