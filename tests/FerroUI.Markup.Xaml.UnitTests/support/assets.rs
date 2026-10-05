//! The markup documents the test project embeds as assets, addressable as
//! `ferres://FerroUI.Markup.Xaml.UnitTests/Xaml/<name>.xaml`.

use ferroui_base::platform::register_assets;

/// The assets of the test assembly, by path.
const ASSETS: &[(&str, &[u8])] = &[
    ("/Xaml/Style1.xaml", include_bytes!("../assets/Xaml/Style1.xaml")),
    ("/Xaml/Style2.xaml", include_bytes!("../assets/Xaml/Style2.xaml")),
    ("/Xaml/StyleWithServiceProvider.xaml", include_bytes!("../assets/Xaml/StyleWithServiceProvider.xaml")),
    ("/Xaml/XamlIlClassWithCustomProperty.xaml", include_bytes!("../assets/Xaml/XamlIlClassWithCustomProperty.xaml")),
    ("/Xaml/XamlIlClassWithPrecompiledXaml.xaml", include_bytes!("../assets/Xaml/XamlIlClassWithPrecompiledXaml.xaml")),
];

/// Registers the embedded documents under the name of the test assembly.
pub(crate) fn register() {
    register_assets(crate::ASSEMBLY.name, ASSETS);
    // The classes with markup: until markup is compiled, the run-time loader populates
    // their instances from the documents of the classes.
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
    const ROOT: &str = "ferres://FerroUI.Markup.Xaml.UnitTests/Xaml";
    FerroRuntimeXamlLoader::register_class_document(
        crate::support::xaml::style_include_tests::StyleWithServiceProvider::TYPE,
        &format!("{ROOT}/StyleWithServiceProvider.xaml"),
    );
    FerroRuntimeXamlLoader::register_class_document(
        crate::support::xaml_il_tests::XamlIlClassWithCustomProperty::TYPE,
        &format!("{ROOT}/XamlIlClassWithCustomProperty.xaml"),
    );
    FerroRuntimeXamlLoader::register_class_document(
        crate::support::xaml_il_tests::XamlIlClassWithPrecompiledXaml::TYPE,
        &format!("{ROOT}/XamlIlClassWithPrecompiledXaml.xaml"),
    );
}
