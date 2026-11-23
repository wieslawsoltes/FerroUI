//! Compiles the D3D11 completion helper of the renderer benchmarks (`gpu_completion.cpp`). Only the
//! feature `renderer-benchmarks` on a Windows MSVC target needs it: every other build does nothing
//! here and has no `cc` to build.

fn main() {
    println!("cargo:rerun-if-changed=gpu_completion.cpp");
    #[cfg(feature = "renderer-benchmarks")]
    compile_gpu_completion();
}

/// The target comes from the environment cargo gives a build script: `cfg` here is the host.
#[cfg(feature = "renderer-benchmarks")]
fn compile_gpu_completion() {
    let target = |name: &str| std::env::var(name).unwrap_or_default();
    if target("CARGO_CFG_TARGET_OS") != "windows" || target("CARGO_CFG_TARGET_ENV") != "msvc" {
        return;
    }
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .define("NOMINMAX", None)
        .file("gpu_completion.cpp")
        .compile("tdg_gpu_completion");
}
