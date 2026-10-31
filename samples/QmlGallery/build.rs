fn main() {
    let compiler = ferroui_qml_build::Compiler::new();
    #[cfg(feature = "webscene")]
    let compiler = compiler.with_javascript();
    compiler
        .compile_file("ui/Main.qml", "main_qml.rs")
        .expect("compile ui/Main.qml");
}
