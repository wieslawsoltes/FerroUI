fn main() {
    println!("cargo:rerun-if-changed=native/media.cpp");
    println!("cargo:rerun-if-changed=native/media.h");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    println!("cargo:rerun-if-env-changed=FERROUI_NATIVE_MEDIA_CHECK_ONLY");
    if std::env::var_os("FERROUI_NATIVE_MEDIA_CHECK_ONLY").is_some() {
        println!("cargo:warning=FFI binding check only: native Windows media is not compiled or linkable");
        return;
    }
    cc::Build::new()
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
