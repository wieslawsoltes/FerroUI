fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        ferroui_qml_build::Compiler::new()
            .with_javascript()
            .compile_file("Main.qml", "browser.rs")
            .expect("compile browser script fixture");
    }
}
