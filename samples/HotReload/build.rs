fn main() {
    // This development sample loads XAML at runtime. The build records sources
    // and contracts, but does not generate or compile view code.
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .hot_reload_manifest(true)
        .input(".")
        .run();
}
