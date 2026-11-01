fn main() {
    println!("cargo:rerun-if-changed=native/media.cpp");
    println!("cargo:rerun-if-changed=native/media.h");
    println!("cargo:rerun-if-changed=native/frame_tests.cpp");
    println!("cargo:rerun-if-changed=native/synthetic_camera.cpp");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    println!("cargo:rerun-if-env-changed=FERROUI_NATIVE_MEDIA_CHECK_ONLY");
    if std::env::var_os("FERROUI_NATIVE_MEDIA_CHECK_ONLY").is_some() {
        println!("cargo:warning=FFI binding check only: native Windows media is not compiled or linkable");
        return;
    }
    let mut build = cc::Build::new();
    if std::env::var_os("CARGO_FEATURE_SYNTHETIC_CAMERA").is_some() {
        build.define("FERROUI_SYNTHETIC_CAMERA", None);
    }
    if std::env::var_os("CARGO_FEATURE_NATIVE_FRAME_TESTS").is_some() {
        build.define("FERROUI_NATIVE_FRAME_TESTS", None);
    }
    build
        .cpp(true)
        .file("native/media.cpp")
        .flag_if_supported("/std:c++17")
        .flag_if_supported("/EHsc")
        .define("NOMINMAX", None)
        .compile("ferroui_media_windows");
    for library in [
        "mfplat",
        "mfreadwrite",
        "mfuuid",
        "mf",
        "ole32",
        "uuid",
        "propsys",
        "d3d11",
        "dxgi",
        "avrt",
    ] {
        println!("cargo:rustc-link-lib={library}");
    }
}
