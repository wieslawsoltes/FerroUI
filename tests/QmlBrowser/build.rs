fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        ferroui_qml_build::Compiler::new()
            .with_javascript()
            .compile_file("Main.qml", "browser.rs")
            .expect("compile browser script fixture");
        ferroui_qml_build::Compiler::new()
            .with_javascript()
            .worker_script_entry("loose.js")
            .worker_script_entry("nested/nested-only.js")
            .compile_file("../QmlCompiled/workers/Main.qml", "browser_workers.rs")
            .expect("compile browser WorkerScript fixture");
    }
}
