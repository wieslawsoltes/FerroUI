//! Compiles the source splitter theme and exports library class metadata.
fn main() {
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .default_compile_bindings(true)
        .compile_xaml()
        .run();
}
