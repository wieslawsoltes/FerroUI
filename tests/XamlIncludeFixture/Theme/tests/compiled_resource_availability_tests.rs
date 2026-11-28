//! Availability is exported by the real build, rather than hand-supplied in a test.
use ferroui_base::{controls::ResourceDictionary, data::core::ValueTypes, utilities::UriRef};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_markup_xaml::FerroXamlLoader;

#[test]
fn generated_public_index_and_runtime_include_retain_private_document_boundaries() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    let public = UriRef::parse_absolute("ferres://Tests/Runtime/Public.xaml").unwrap();
    let private = UriRef::parse_absolute("ferres://Tests/Runtime/Private.xaml").unwrap();
    assert!(
        crate::compiled_markup::PUBLIC_COMPILED_DOCUMENTS
            .iter()
            .any(|p| *p == "ferres://Tests/Runtime/Public.xaml")
    );
    assert!(
        !crate::compiled_markup::PUBLIC_COMPILED_DOCUMENTS
            .iter()
            .any(|p| *p == "ferres://Tests/Runtime/Private.xaml")
    );
    assert!(FerroXamlLoader::has_public_compiled_document(&public.into(), None));
    assert!(!FerroXamlLoader::has_public_compiled_document(&private.into(), None));
    let class_uri = crate::documents::STYLE_WITH_SERVICE_PROVIDER.0;
    let class_uri = UriRef::parse_absolute(&format!("ferres://Tests{}", class_uri)).unwrap();
    assert!(FerroXamlLoader::has_public_compiled_document(&class_uri.into(), None));
    let xaml = "<ResourceDictionary xmlns='https://github.com/ferroui'><ResourceDictionary.MergedDictionaries><ResourceInclude Source='ferres://Tests/Runtime/Public.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>";
    let root = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(xaml, None).unwrap();
    let dictionary = ValueTypes::as_object(&*root)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    assert_eq!(
        dictionary
            .try_get_resource(&"Availability".into(), None)
            .flatten()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "Public"
    );
    let error =
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(&xaml.replace("Public.xaml", "Private.xaml"), None)
            .unwrap_err();
    assert!(error.message().contains("Private.xaml"), "{error:?}");
}
