fn main() {
    println!("cargo:rerun-if-changed=gpu_completion.cpp");
    if std::env::var_os("CARGO_FEATURE_RENDERER_BENCHMARKS").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
    {
        cc::Build::new()
            .cpp(true)
            .std("c++17")
            .define("NOMINMAX", None)
            .file("gpu_completion.cpp")
            .compile("tdg_gpu_completion");
    }
}
