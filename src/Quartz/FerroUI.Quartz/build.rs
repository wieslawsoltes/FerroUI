fn main() {
    println!("cargo:rerun-if-changed=native.m");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if os != "macos" && os != "ios" {
        return;
    }
    let target = std::env::var("TARGET").unwrap_or_default();
    let simulator = target.ends_with("-sim") || target.starts_with("x86_64-apple-ios");
    let mut build = cc::Build::new();
    build.file("native.m").flag("-fobjc-arc");
    if std::env::var_os("CARGO_FEATURE_METAL").is_some() {
        for file in ["metal.m", "metal.h", "shaders.metal"] {
            println!("cargo:rerun-if-changed={file}");
        }
        let source = std::fs::read_to_string("shaders.metal").unwrap();
        let source = source.replace("\r\n", "\n").replace('\r', "\n");
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
        // MPS is absent from the simulator SDK. Its blur path uses the same
        // Metal compute Gaussian implementation as general affine blur.
        if !simulator {
            println!("cargo:rustc-link-lib=framework=MetalPerformanceShaders");
        }
    }
    if std::env::var_os("CARGO_FEATURE_MEDIA_TEST_FIXTURES").is_some() {
        build.define("QUARTZ_MEDIA_TEST_FIXTURES", None);
        println!("cargo:rustc-link-lib=framework=CoreVideo");
        println!("cargo:rustc-link-lib=framework=IOSurface");
    }
    build.compile("ferroui_quartz");
    if os == "ios" {
        // @available guards need Clang's availability runtime even when Skia
        // is absent. Keep the universal archive for the final Apple linker.
        let runtime = if simulator {
            "clang_rt.iossim"
        } else {
            "clang_rt.ios"
        };
        let output = std::process::Command::new("clang")
            .arg(format!("--print-file-name=lib{runtime}.a"))
            .output()
            .expect("Xcode clang is required for Quartz on iOS");
        let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let path = std::path::Path::new(&path);
        assert!(
            output.status.success() && path.is_file(),
            "Missing iOS Clang runtime: {runtime}"
        );
        println!(
            "cargo:rustc-link-search=native={}",
            path.parent().unwrap().display()
        );
        println!("cargo:rustc-link-lib=static:-bundle={runtime}");
    }
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
