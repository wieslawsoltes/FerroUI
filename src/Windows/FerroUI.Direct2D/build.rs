fn main() {
    println!("cargo:rerun-if-changed=native.cpp");
    println!("cargo:rerun-if-changed=effects.hlsl");
    println!("cargo:rerun-if-env-changed=FXC");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let fxc = std::env::var_os("FXC")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let sdk = std::path::PathBuf::from(
                std::env::var_os("ProgramFiles(x86)").expect("Windows SDK location"),
            )
            .join("Windows Kits/10/bin");
            let mut versions: Vec<_> = std::fs::read_dir(sdk)
                .expect("Windows 10 SDK with fxc.exe")
                .filter_map(Result::ok)
                .map(|v| v.path().join("x64/fxc.exe"))
                .filter(|p| p.is_file())
                .collect();
            versions.sort();
            versions
                .pop()
                .expect("Install the Windows SDK shader compiler or set FXC")
        });
    let status = std::process::Command::new(fxc)
        .args(["/nologo", "/T", "cs_5_0", "/E", "main", "/O3", "/Fh"])
        .arg(output.join("effects_shader.h"))
        .args(["/Vn", "ferro_effect_shader", "effects.hlsl"])
        .status()
        .expect("Run Windows SDK shader compiler");
    assert!(status.success(), "Compile native GPU effects");
    let mut native = cc::Build::new();
    if std::env::var_os("CARGO_FEATURE_RUNTIME_SHADERS").is_some() {
        native.define("FERRO_RUNTIME_SHADERS", None);
    }
    native
        .cpp(true)
        .file("native.cpp")
        .include(output)
        .flag_if_supported("/std:c++17")
        .flag_if_supported("/EHsc")
        .define("NOMINMAX", None)
        .define("WIN32_LEAN_AND_MEAN", None)
        .compile("ferroui_direct2d");
    for library in [
        "d3d11",
        "dxgi",
        "dcomp",
        "dxguid",
        "d2d1",
        "dwrite",
        "windowscodecs",
        "ole32",
        "user32",
    ] {
        println!("cargo:rustc-link-lib={library}");
    }
}
