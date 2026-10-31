fn main() {
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
