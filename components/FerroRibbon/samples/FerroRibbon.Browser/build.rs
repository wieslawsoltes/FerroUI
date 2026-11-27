use std::{env, fs, path::PathBuf};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("emscripten") {
        return;
    }

    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
    let root = manifest.join("../..");
    for input in ["LICENSE", "NOTICE.md", "docs/licenses"] {
        println!("cargo::rerun-if-changed={}", root.join(input).display());
    }
    let staging = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output")).join("browser-site");
    let icon = root.join("samples/FerroRibbon.Sample/windows/icon.ico");
    let splash = manifest.join("assets/splash_screen.scale-200.png");
    println!("cargo::rerun-if-changed={}", icon.display());
    println!("cargo::rerun-if-changed={}", splash.display());
    fs::create_dir_all(&staging).expect("create browser site staging directory");
    fs::copy(icon, staging.join("favicon.ico")).expect("copy the source gallery icon");
    fs::copy(splash, staging.join("splash_screen.scale-200.png"))
        .expect("copy the source browser splash");
    let licenses = staging.join("licenses");
    fs::create_dir_all(&licenses).expect("create the license directory");
    for name in ["LICENSE", "NOTICE.md"] {
        fs::copy(root.join(name), licenses.join(name)).expect("copy the component notice");
    }
    for entry in fs::read_dir(root.join("docs/licenses")).expect("license directory") {
        let path = entry.expect("license entry").path();
        fs::copy(&path, licenses.join(path.file_name().expect("license file"))).expect("copy a license");
    }
    let framework = root.join("../..");
    fs::copy(framework.join("LICENSE"), licenses.join("FerroUI-LICENSE")).expect("copy the FerroUI license");
}
