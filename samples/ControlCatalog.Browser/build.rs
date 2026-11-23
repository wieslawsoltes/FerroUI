use std::{env, fs, path::PathBuf};
fn main() {
    // The WebGPU of the page, which the engine of the ProGPU backend is compiled against. A
    // dependency cannot pass this to the link of the module.
    if env::var_os("CARGO_FEATURE_PROGPU").is_some()
        && env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten")
    {
        println!("cargo:rustc-link-arg=--use-port=emdawnwebgpu:cpp_bindings=false");
    }
    // Intentionally outside the catalog asset manifest: neither startup loads,
    // page navigation nor its idle prefetch may request the video.
    let source =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../ControlCatalog/Media");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("browser-site/media");
    fs::create_dir_all(&output).unwrap();
    for name in ["sintel-sample.mp4", "README.md", "LICENSE-CC-BY-3.0.txt"] {
        let file = source.join(name);
        println!("cargo:rerun-if-changed={}", file.display());
        fs::copy(file, output.join(name)).unwrap();
    }
}
