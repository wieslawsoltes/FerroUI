fn main() {
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .compile_xaml()
        .compile_group(
            ferroui_build::XamlGroup::new("compiled_with_source_info")
                .documents(&[("Value.xaml", include_str!("Value.xaml"))])
                .create_source_info(true)
                .internal(),
        )
        .run();
}
