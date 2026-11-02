fn main() {
    println!("cargo:rustc-check-cfg=cfg(ferro_skia_apple_ganesh)");
    println!("cargo:rerun-if-changed=Info.plist");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=macos_diagnostics.m");
        if std::env::var_os("CARGO_FEATURE_NATIVE_PIXEL_TESTS").is_some() {
            cc::Build::new()
                .file("macos_diagnostics.m")
                .flag("-fobjc-arc")
                .flag("-fblocks")
                .compile("portable_media_macos_diagnostics");
            println!("cargo:rustc-link-arg=-Wl,-weak_framework,ScreenCaptureKit");
            for framework in ["CoreGraphics", "AppKit", "CoreVideo"] {
                println!("cargo:rustc-link-lib=framework={framework}");
            }
        }
        println!("cargo:rerun-if-changed=../PortableMedia.Mobile/plane.m");
        cc::Build::new()
            .file("../PortableMedia.Mobile/plane.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("portable_media_plane");
        println!("cargo:rustc-link-lib=framework=Metal");
        let path = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("Info.plist");
        println!(
            "cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,{}",
            path.display()
        );
    }
}
