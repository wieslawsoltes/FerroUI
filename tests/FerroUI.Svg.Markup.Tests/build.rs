//! Compiles a page against the metadata `ferroui-svg` exports.
fn main() {
    ferroui_build::Build::from_env().type_system(ferroui_build::TypeSystem::Model).compile_xaml().run();
}
