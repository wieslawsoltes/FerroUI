//! Runtime inclusion must query exact compiled availability without probing a build.
use ferroui_base::{
    BoxedValue, FerroLocator,
    controls::ResourceDictionary,
    data::core::ValueTypes,
    platform::{IAssetLoader, StandardAssetLoader, register_assets},
    utilities::UriRef,
};
use ferroui_markup_xaml::{FerroXamlLoader, XamlLoadError};
use std::{cell::Cell, rc::Rc};
thread_local! {static BUILDS:Cell<usize>=const{Cell::new(0)};}
const DOCUMENTS: &[&str] = &[
    "ferres://compiled.runtime.tests/Public.xaml",
    "ferres://compiled.runtime.tests/BadRoot.xaml",
    "ferres://compiled.runtime.tests/Failure.xaml",
    "ferres://compiled.runtime.tests/Café menu.xaml",
];
fn loader(
    _: Option<&Rc<dyn ferroui_base::metadata::IServiceProvider>>,
    uri: &str,
) -> Result<Option<BoxedValue>, XamlLoadError> {
    use ferroui_markup_xaml::xaml_il::runtime::compiled::uri_equals;
    if !DOCUMENTS.iter().any(|doc| uri_equals(uri, doc, "")) {
        return Ok(None);
    }
    BUILDS.with(|n| n.set(n.get() + 1));
    if uri.ends_with("BadRoot.xaml") {
        return Ok(Some(Rc::new(42i32)));
    }
    if uri.ends_with("Failure.xaml") {
        return Err(XamlLoadError::with_message("compiled build failed"));
    }
    let d = ResourceDictionary::new();
    d.set("Result", Some(Rc::new("Loaded".to_owned())));
    Ok(Some(Rc::new(d)))
}
fn markup(path: &str) -> String {
    format!(
        "<ResourceDictionary xmlns=\"https://github.com/ferroui\"><ResourceDictionary.MergedDictionaries><ResourceInclude Source=\"ferres://compiled.runtime.tests/{path}\" /></ResourceDictionary.MergedDictionaries></ResourceDictionary>"
    )
}
#[test]
fn public_compiled_include_constructs_once_per_load_and_rejects_missing_private_and_bad_roots() {
    let scope = FerroLocator::enter_scope();
    let assets: Rc<dyn IAssetLoader> = Rc::new(StandardAssetLoader::new(None));
    FerroLocator::current_mutable()
        .bind::<dyn IAssetLoader>()
        .to_constant(assets);
    ferroui_markup_xaml::register_types();
    register_assets("compiled.runtime.tests", &[]);
    FerroXamlLoader::register_compiled_xaml_with_documents("compiled.runtime.tests", loader, DOCUMENTS);
    BUILDS.with(|n| n.set(0));
    assert!(FerroXamlLoader::has_public_compiled_document(
        &UriRef::parse(DOCUMENTS[0]).unwrap(),
        None
    ));
    assert_eq!(BUILDS.with(Cell::get), 0);
    let first = crate::FerroRuntimeXamlLoader::parse(&markup("Public.xaml"), None).unwrap();
    let second = crate::FerroRuntimeXamlLoader::parse(&markup("Public.xaml"), None).unwrap();
    // ResourceInclude is lazy while its dictionary has no owner. Transformation
    // and interpretation must not probe construction as an existence check.
    assert_eq!(BUILDS.with(Cell::get), 0);
    let first = ValueTypes::as_object(&*first)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    let second = ValueTypes::as_object(&*second)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    assert_ne!(first, second);
    let a = first.try_get_resource(&"Result".into(), None).flatten().unwrap();
    let b = second.try_get_resource(&"Result".into(), None).flatten().unwrap();
    assert_eq!(a.downcast_ref::<String>().unwrap(), "Loaded");
    assert!(!Rc::ptr_eq(&a, &b));
    assert!(Rc::ptr_eq(
        &a,
        &first.try_get_resource(&"Result".into(), None).flatten().unwrap()
    ));
    assert_eq!(BUILDS.with(Cell::get), 2);
    for path in ["Private.xaml", "Missing.xaml"] {
        assert!(crate::FerroRuntimeXamlLoader::parse(&markup(path), None).is_err());
        assert_eq!(BUILDS.with(Cell::get), 2);
    }
    let include = |path: &str| {
        let xaml = format!(
            "<ResourceInclude xmlns=\"https://github.com/ferroui\" Source=\"ferres://compiled.runtime.tests/{path}\" />"
        );
        let value = crate::FerroRuntimeXamlLoader::parse(&xaml, None).unwrap();
        ferroui_base::metadata::from_markup_value::<Rc<ferroui_markup_xaml::styling::ResourceInclude>>(&Some(value))
            .unwrap()
    };
    let root = include("BadRoot.xaml")
        .try_loaded()
        .err()
        .expect("invalid compiled document must fail");
    assert!(root.message().contains("IResourceDictionary"), "{root:?}");
    let failure = include("Failure.xaml")
        .try_loaded()
        .err()
        .expect("invalid compiled document must fail");
    assert!(failure.message().contains("compiled build failed"), "{failure:?}");
    assert_eq!(BUILDS.with(Cell::get), 4);

    let encoded = "Caf%C3%89%20menu.xaml";
    let dictionary = crate::FerroRuntimeXamlLoader::parse(&markup(encoded), None).unwrap();
    assert_eq!(BUILDS.with(Cell::get), 4);
    let dictionary = ValueTypes::as_object(&*dictionary)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    let value = dictionary.try_get_resource(&"Result".into(), None).flatten().unwrap();
    assert_eq!(value.downcast_ref::<String>().unwrap(), "Loaded");
    assert_eq!(BUILDS.with(Cell::get), 5);

    FerroXamlLoader::unregister_compiled_xaml("compiled.runtime.tests");
    scope.dispose();
}
