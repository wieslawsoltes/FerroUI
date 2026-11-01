fn main() {
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("webscene/Bindings.qml", "webscene_bindings.rs")
        .expect("compile imperative property bindings");

    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .javascript_entry("dual.js")
        .javascript_entry("plugins/en/index.js")
        .javascript_entry("plugins/pl/index.js")
        .javascript_entry("plugins/en/leaf.mjs")
        .compile_file("computed/Main.qml", "computed.rs")
        .expect("compile explicitly bundled dynamic modules");

    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("webscene/Signals.qml", "webscene_signals.rs")
        .expect("compile dynamic signal connections");

    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("reads/Main.qml", "direct_reads.rs")
        .expect("compile native property read fixture");
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("reference/Probe.qml", "webscene_reference.rs")
        .expect("compile baseline Qt QML probes for WebScene");

    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("webscene/Async.qml", "async_model.rs")
        .expect("compile async JavaScript fixture");
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("webscene/App.qml", "webscene.rs")
        .expect("compile the WebScene components and JavaScript module graph");
    ferroui_qml_build::compile_file("host/Main.qml", "native_host.rs")
        .expect("compile Rust host callback fixture");
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("host/Main.qml", "webscene_host.rs")
        .expect("embed Rust host callback fixture");
    ferroui_qml_build::compile_file("models/Inventory.qml", "inventory.rs")
        .expect("compile list model");
    ferroui_qml_build::compile_file("shadows/Final.qml", "shadow.rs")
        .expect("compile lexical shadow fixture");
    ferroui_qml_build::compile_file("reference/Probe.qml", "reference.rs")
        .expect("compile Qt reference fixture");
    let types = ferroui_qml::TypeRegistry::new();
    ferroui_qml_build::Compiler::new()
        .check_with(|doc| types.check(doc))
        .compile_file("scripts/ControlFlow.qml", "scripts.rs")
        .expect("compile control-flow fixture");
    ferroui_qml_build::Compiler::new()
        .check_with(|doc| types.check(doc))
        .import_path("singletons/imports")
        .compile_file("singletons/Main.qml", "singletons.rs")
        .expect("compile source singleton fixture");
    ferroui_qml_build::Compiler::new()
        .check_with(|doc| types.check(doc))
        .compile_file("ui/Main.qml", "component.rs")
        .expect("compile test component");
}
