fn main() {
    ferroui_qml_build::compile_file("ui/Main.qml", "component.rs").expect("compile test component");
}
