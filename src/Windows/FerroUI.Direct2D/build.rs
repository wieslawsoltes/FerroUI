fn main() {
    println!("cargo:rerun-if-changed=native.cpp");
    println!("cargo:rerun-if-changed=effects.hlsl");
    println!("cargo:rerun-if-env-changed=FXC");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // The native part is C++ against the Windows SDK and links the MSVC runtime.
    assert!(
        std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc"),
        "ferroui-direct2d builds for the windows-msvc targets only (the target is {})",
        std::env::var("TARGET").unwrap_or_default()
    );
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let fxc = std::env::var_os("FXC")
        .map(std::path::PathBuf::from)
        .or_else(sdk_shader_compiler)
        .expect("Install the Windows SDK (it has the shader compiler fxc.exe), or set FXC to it");
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

/// `fxc.exe` of the newest installed Windows SDK, the one of the host architecture when the SDK
/// has it (the x64 one runs on an ARM64 host too, emulated).
fn sdk_shader_compiler() -> Option<std::path::PathBuf> {
    let host = std::env::var("HOST").unwrap_or_default();
    let architectures: &[&str] = match host.split('-').next() {
        Some("aarch64") => &["arm64", "x64", "x86"],
        Some("x86_64") => &["x64", "x86"],
        _ => &["x86", "x64"],
    };
    let version = |path: &std::path::Path| -> Vec<u64> {
        let name = path.file_name().and_then(|name| name.to_str());
        name.unwrap_or_default()
            .split('.')
            .map(|part| part.parse().unwrap_or(0))
            .collect()
    };
    ["ProgramFiles(x86)", "ProgramFiles"]
        .iter()
        .filter_map(std::env::var_os)
        .filter_map(|root| {
            std::fs::read_dir(std::path::PathBuf::from(root).join("Windows Kits/10/bin")).ok()
        })
        .flat_map(|versions| versions.filter_map(Result::ok).map(|entry| entry.path()))
        .filter_map(|sdk| {
            let compiler = architectures
                .iter()
                .map(|architecture| sdk.join(architecture).join("fxc.exe"))
                .find(|compiler| compiler.is_file())?;
            Some((version(&sdk), compiler))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, compiler)| compiler)
}
