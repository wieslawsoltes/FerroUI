//! Compiles settings markup independently of the docking UI assembly.
fn main() {
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .default_compile_bindings(true)
        .compile_xaml()
        .run();
}
