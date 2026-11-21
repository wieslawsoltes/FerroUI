//! Regression for runtime ResourceInclude of a public registered compiled library
//! document.
use ferroui_base::{
    controls::ResourceDictionary, data::core::ValueTypes, threading::CancellationToken,
    utilities::UriRef,
};
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "FerroRibbon.CompiledResource.Gap",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn registered_public_compiled_dictionary_must_resolve_in_runtime_resource_include() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY)).dispatch(|| {
        ferroribbon::register_types();
        let uri = UriRef::from(UriRef::parse_absolute("ferres://FerroRibbon/Themes/ScreenTips.xaml").unwrap());
        let direct=ferroui_markup_xaml::FerroXamlLoader::load(&uri,None).expect("registered library compiled URI loads directly");
        assert!(ValueTypes::as_object(&*direct).unwrap().cast::<ResourceDictionary>().is_some());
        let xaml=r#"<ResourceDictionary xmlns="https://github.com/ferroui"><ResourceDictionary.MergedDictionaries><ResourceInclude Source="ferres://FerroRibbon/Themes/ScreenTips.xaml" /></ResourceDictionary.MergedDictionaries></ResourceDictionary>"#;
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(xaml,None)
            .expect("runtime include must reach that same public compiled dictionary");
    },CancellationToken::none()).wait().expect("framework resource inclusion");
}

#[test]
fn generated_unicode_dictionary_loads_directly_and_through_runtime_include() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            || {
                ferroribbon::register_types();
                ferroribbon_markup_consumer::compiled_markup::register();
                let uri = UriRef::parse(
                    "ferres://ferroribbon_markup_consumer/Caf%C3%89%20menu.xaml",
                )
                .unwrap();
                let generated = ferroribbon_markup_consumer::compiled_xaml::try_load(
                    None,
                    &uri.to_string(),
                )
                .unwrap()
                .expect("generated loader matches the encoded URI");
                let direct = ferroui_markup_xaml::FerroXamlLoader::load(&uri, None).unwrap();
                let source = format!(
                    r#"<ResourceDictionary xmlns="https://github.com/ferroui">
                        <ResourceDictionary.MergedDictionaries>
                            <ResourceInclude Source="{uri}" />
                        </ResourceDictionary.MergedDictionaries>
                    </ResourceDictionary>"#
                );
                assert!(ferroui_markup_xaml::FerroXamlLoader::has_public_compiled_document(
                    &uri, None,
                ));
                let included =
                    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(&source, None).unwrap();
                for value in [generated, direct, included] {
                    let dictionary = ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<ResourceDictionary>()
                        .unwrap();
                    let title = dictionary
                        .try_get_resource(&"MenuTitle".into(), None)
                        .flatten()
                        .expect("resource from the compiled Unicode document");
                    assert_eq!(title.downcast_ref::<String>().unwrap(), "Compiled café menu");
                }
            },
            CancellationToken::none(),
        )
        .wait()
        .expect("generated Unicode dictionary");
}
