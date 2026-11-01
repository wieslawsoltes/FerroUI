fn main() {
    ferroui_qml_build::compile_file("delegates/Main.qml", "delegates.rs")
        .expect("compile the delegate fixture");
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("delegates/Main.qml", "delegates_js.rs")
        .expect("compile the WebScene delegate fixture");
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .qml_entry("Part.qml")
        .qml_entry("sub/Part.qml")
        .qml_entry("sub/scripts/Part.qml")
        .compile_file("creation/origin/Main.qml", "catalog_origin.rs")
        .expect("compile nested factories");
    ferroui_qml_build::Compiler::new()
        .compile_file("loader/Main.qml", "loader_model.rs")
        .expect("compile Loader fixture");
    #[cfg(feature = "webscene")]
    for (source, output) in [
        ("loader/Main.qml", "loader_js.rs"),
        ("loader/Bindings.qml", "loader_initial_bindings.rs"),
    ] {
        ferroui_qml_build::Compiler::new()
            .with_javascript()
            .compile_file(source, output)
            .expect("compile Loader for V8");
    }

    ferroui_qml_build::Compiler::new()
        .qml_entry("Part.qml")
        .compile_file("creation/catalog/Main.qml", "catalog_native.rs")
        .expect("compile file factories");
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .qml_entry("Part.qml")
        .compile_file("creation/catalog/Main.qml", "catalog_guest.rs")
        .expect("compile guest file factories");
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("creation/Initial.qml", "component_properties.rs")
        .expect("compile initial bindings");
    ferroui_qml_build::compile_file("creation/urls/Main.qml", "url_utilities.rs")
        .expect("compile URL utilities");
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("creation/urls/Main.qml", "url_utilities_guest.rs")
        .expect("compile guest URL utilities");
    ferroui_qml_build::compile_file("creation/Main.qml", "dynamic_component.rs")
        .expect("compile dynamic component");
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .compile_file("creation/Main.qml", "dynamic_guest.rs")
        .expect("compile guest dynamic component");
    #[cfg(feature = "webscene")]
    ferroui_qml_build::Compiler::new()
        .with_javascript()
        .worker_script_entry("loose.js")
        .worker_script_entry("nested/nested-only.js")
        .compile_file("workers/Main.qml", "workers.rs")
        .expect("compile WorkerScript fixture");

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
