fn main() {
    println!("cargo:rerun-if-changed=native.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let mut build = cc::Build::new();
    build.file("native.m").flag("-fobjc-arc");
    if std::env::var_os("CARGO_FEATURE_METAL").is_some() {
        for file in ["metal.m", "metal.h", "shaders.metal"] {
            println!("cargo:rerun-if-changed={file}");
        }
        let source = std::fs::read_to_string("shaders.metal").unwrap();
        let escaped = source
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n\"\n\"");
        let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        std::fs::write(
            out.join("metal_source.h"),
            format!("static const char *q_metal_source = \"{escaped}\";\n"),
        )
        .unwrap();
        build
            .file("metal.m")
            .define("QUARTZ_METAL", None)
            .include(out);
        println!("cargo:rustc-link-lib=framework=Metal");
        println!("cargo:rustc-link-lib=framework=MetalPerformanceShaders");
    }
    if std::env::var_os("CARGO_FEATURE_MEDIA_TEST_FIXTURES").is_some() {
        build.define("QUARTZ_MEDIA_TEST_FIXTURES", None);
        println!("cargo:rustc-link-lib=framework=CoreVideo");
        println!("cargo:rustc-link-lib=framework=IOSurface");
    }
    build.compile("ferroui_quartz");
    for framework in [
        "CoreFoundation",
        "CoreGraphics",
        "CoreText",
        "ImageIO",
        "Foundation",
    ] {
        println!("cargo:rustc-link-lib=framework={framework}");
    }
}
