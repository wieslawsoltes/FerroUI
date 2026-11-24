//! Compiles a page against the metadata the library exports.
fn main() {
    ferroui_build::Build::from_env().type_system(ferroui_build::TypeSystem::Model).compile_xaml().run();
}
