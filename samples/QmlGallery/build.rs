fn main() {
    ferroui_qml_build::compile_file("ui/Main.qml", "main_qml.rs").expect("compile ui/Main.qml");
}
