fn main() {
    ferroui_qml_build::Compiler::new()
        .import_path("singletons/imports")
        .compile_file("singletons/Main.qml", "singletons.rs")
        .expect("compile source singleton fixture");
    ferroui_qml_build::compile_file("ui/Main.qml", "component.rs").expect("compile test component");
}
