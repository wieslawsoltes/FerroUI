//! Compiles the document that names the controls without an XML prefix.
fn main() {
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .compile_xaml()
        .run();
}
