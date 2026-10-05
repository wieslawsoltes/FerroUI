//! Tests of the loader entry point and of the includes that load through
//! it: compiled documents registered per assembly, the runtime loader
//! service, and the includes as resource providers and styles.

use super::*;
use crate::test_support::{boxed, uri, TestAssetLoader, TestServiceProvider};
use crate::{
    register_types, FerroXamlLoader, IRuntimeXamlLoader, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    ServiceProviderExtensions, XamlLoadException, XamlResourceNode,
};
use ferroui_base::controls::{
    IResourceNode, IResourceProvider, IThemeVariantProvider, ResourceDictionary, ResourceHostRef, ResourceKey,
};
use ferroui_base::metadata::{from_markup_value, into_markup_value, IServiceProvider, MarkupTyped};
use ferroui_base::styling::{IStyle, Style, Styles, ThemeVariant};
use ferroui_base::{BoxedValue, FerroLocator, Ref};
use ferroui_controls::Border;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn key(text: &str) -> ResourceKey {
    text.into()
}

fn text(value: Option<Option<BoxedValue>>) -> String {
    value.flatten().and_then(|v| v.downcast_ref::<String>().cloned()).unwrap()
}

thread_local! {
    static LOADS: Cell<u32> = const { Cell::new(0) };
    static SEEN_PROVIDER: Cell<bool> = const { Cell::new(false) };
    static REENTRANT: RefCell<Option<Rc<ResourceInclude>>> = const { RefCell::new(None) };
}

fn compiled_resources(sp: Option<&Rc<dyn IServiceProvider>>, uri: &str) -> Result<Option<BoxedValue>, XamlLoadException> {
    LOADS.with(|loads| loads.set(loads.get() + 1));
    SEEN_PROVIDER.with(|seen| seen.set(sp.is_some()));
    let Some(name) = uri.strip_prefix("ferres://includes.resources/") else {
        return Ok(None);
    };
    if !name.ends_with(".xaml") {
        return Ok(None);
    }
    let dictionary = ResourceDictionary::new();
    dictionary.add("source", Some(boxed(name.to_string())));
    Ok(Some(boxed(dictionary)))
}

fn compiled_styles(_sp: Option<&Rc<dyn IServiceProvider>>, uri: &str) -> Result<Option<BoxedValue>, XamlLoadException> {
    Ok(match uri {
        "ferres://includes.styles/Style.xaml" => {
            let style = Style::new();
            style.resources().add("styled", Some(boxed("from style".to_string())));
            Some(boxed(style))
        }
        "ferres://includes.styles/Styles.xaml" => Some(boxed(Styles::new())),
        "ferres://includes.styles/NotAStyle.xaml" => Some(boxed(5i32)),
        _ => None,
    })
}

fn compiled_reentrant(_sp: Option<&Rc<dyn IServiceProvider>>, _uri: &str) -> Result<Option<BoxedValue>, XamlLoadException> {
    // While it is loading, an include answers no resource lookups.
    let include = REENTRANT.with(|include| include.borrow().clone()).unwrap();
    assert!(include.try_get_resource(&key("any"), None).is_none());
    Ok(Some(boxed(ResourceDictionary::new())))
}

#[test]
fn loading_an_object_without_compiled_markup_is_an_error() {
    register_types();
    let error = FerroXamlLoader::load_object(&boxed(Border::new())).unwrap_err();
    assert_eq!(
        error.message(),
        "No precompiled XAML found for FerroUI.Controls.Border, make sure to specify x:Class and include your XAML file as FerroResource"
    );
    let sp = TestServiceProvider::new().sp();
    assert!(FerroXamlLoader::load_object_with_service_provider(Some(&sp), &boxed(1i32)).is_err());
}

#[test]
#[should_panic(expected = "Could not create IAssetLoader")]
fn loading_without_an_asset_loader_panics() {
    let _scope = FerroLocator::enter_scope();
    let _ = FerroXamlLoader::load(&uri("ferres://includes.none/A.xaml"), None);
}

#[test]
#[should_panic(expected = "Cannot load relative Uri when BaseUri is null")]
fn loading_a_relative_uri_without_a_base_uri_panics() {
    let _scope = TestAssetLoader::new().install();
    let _ = FerroXamlLoader::load(&uri("A.xaml"), None);
}

#[test]
fn compiled_documents_are_loaded_through_the_table_of_their_assembly() {
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.resources", compiled_resources);

    let loaded = FerroXamlLoader::load(&uri("ferres://includes.resources/Colors.xaml"), None).unwrap();
    let dictionary = from_markup_value::<Ref<ResourceDictionary>>(&Some(loaded)).unwrap();
    assert_eq!(text(dictionary.try_get_value(&key("source"))), "Colors.xaml");
    assert!(!SEEN_PROVIDER.with(Cell::get));

    // A relative URI is resolved against the base URI; the service provider is passed on.
    let sp = TestServiceProvider::new().sp();
    let base = uri("ferres://includes.resources/Themes/Base.xaml");
    let loaded =
        FerroXamlLoader::load_with_service_provider(Some(&sp), &uri("Dark.xaml"), Some(&base)).unwrap();
    let dictionary = from_markup_value::<Ref<ResourceDictionary>>(&Some(loaded)).unwrap();
    assert_eq!(text(dictionary.try_get_value(&key("source"))), "Themes/Dark.xaml");
    assert!(SEEN_PROVIDER.with(Cell::get));

    // The table has no such document and there is no runtime loader.
    let error = FerroXamlLoader::load(&uri("ferres://includes.resources/Missing.txt"), None).unwrap_err();
    assert_eq!(
        error.message(),
        "No precompiled XAML found for ferres://includes.resources/Missing.txt (baseUri: ), make sure to specify x:Class and include your XAML file as FerroResource"
    );
    // An assembly without a table.
    assert!(FerroXamlLoader::load(&uri("ferres://includes.unknown/A.xaml"), None).is_err());
    scope.dispose();
}

struct RecordingRuntimeLoader {
    seen: RefCell<Vec<String>>,
}

impl IRuntimeXamlLoader for RecordingRuntimeLoader {
    fn load(
        &self,
        document: RuntimeXamlLoaderDocument,
        configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<BoxedValue, XamlLoadException> {
        let text = document.read_to_string().unwrap();
        self.seen.borrow_mut().push(format!(
            "{}|{}|{}|{}",
            document.base_uri.as_ref().unwrap().absolute_uri(),
            text,
            document.service_provider.is_some(),
            configuration.local_assembly.map_or("none", |a| a.name)
        ));
        if text.contains("fail") {
            return Err(XamlLoadException::with_message("bad document"));
        }
        let dictionary = ResourceDictionary::new();
        dictionary.add("text", Some(boxed(text)));
        Ok(boxed(dictionary))
    }
}

#[test]
fn documents_without_compiled_markup_go_to_the_runtime_loader_service() {
    register_types();
    let assets = TestAssetLoader::new();
    assets.add("ferres://FerroUI.Markup.Xaml/Runtime.xaml", "<ResourceDictionary/>");
    assets.add("ferres://includes.runtime/Broken.xaml", "fail");
    let scope = assets.install();
    let loader = Rc::new(RecordingRuntimeLoader { seen: RefCell::new(Vec::new()) });
    let service: Rc<dyn IRuntimeXamlLoader> = loader.clone();
    FerroLocator::current_mutable().bind::<dyn IRuntimeXamlLoader>().to_constant(service);

    let sp = TestServiceProvider::new().sp();
    let loaded =
        FerroXamlLoader::load_with_service_provider(Some(&sp), &uri("ferres://FerroUI.Markup.Xaml/Runtime.xaml"), None)
            .unwrap();
    assert!(from_markup_value::<Ref<ResourceDictionary>>(&Some(loaded)).is_some());
    // The document carries the absolute URI, the text, the service provider
    // and the assembly the asset belongs to.
    assert_eq!(
        loader.seen.borrow()[0],
        "ferres://ferroui.markup.xaml/Runtime.xaml|<ResourceDictionary/>|true|FerroUI.Markup.Xaml"
    );

    // The error of the runtime loader is the error of the load.
    let error = FerroXamlLoader::load(&uri("ferres://includes.runtime/Broken.xaml"), None).unwrap_err();
    assert_eq!(error.message(), "bad document");
    assert!(loader.seen.borrow()[1].ends_with("|false|none"));

    // An asset that does not exist cannot be opened.
    let error = FerroXamlLoader::load(&uri("ferres://includes.runtime/Missing.xaml"), None).unwrap_err();
    assert!(error.message().starts_with("Could not open the XAML document"));
    scope.dispose();
}

#[test]
fn resource_include_loads_its_dictionary_on_first_use() {
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.resources", compiled_resources);

    let include = ResourceInclude::new(Some(uri("ferres://includes.resources/Themes/Base.xaml")));
    assert!(include.source().is_none());
    include.set_source(Some(uri("Light.xaml")));
    let before = LOADS.with(Cell::get);

    assert_eq!(text(include.try_get_resource(&key("source"), None)), "Themes/Light.xaml");
    assert!(include.has_resources());
    assert!(include.try_get_resource(&key("missing"), None).is_none());
    // Loaded once.
    assert_eq!(LOADS.with(Cell::get), before + 1);
    assert!(std::ptr::addr_eq(Rc::as_ptr(&include.loaded()), Rc::as_ptr(&include.try_loaded().unwrap())));

    // The owner is the owner of the loaded dictionary.
    assert!(include.owner().is_none());
    let host = Border::new();
    let owner: ResourceHostRef = host.clone().into();
    let changes = Rc::new(Cell::new(0));
    let counter = changes.clone();
    let subscription = include.owner_changed(Rc::new(move || counter.set(counter.get() + 1)));
    include.add_owner(&owner);
    assert!(include.owner().unwrap() == owner);
    assert_eq!(changes.get(), 1);
    include.remove_owner(&owner);
    assert!(include.owner().is_none());
    subscription.dispose();

    // The theme variant key is a member of the include itself.
    assert!(include.key().is_none());
    include.set_key(Some(ThemeVariant::dark()));
    assert_eq!(include.key(), Some(ThemeVariant::dark()));
    scope.dispose();
}

#[test]
fn resource_include_takes_its_base_uri_from_the_service_provider() {
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.resources", compiled_resources);

    let sp = TestServiceProvider::new().with_base_uri("ferres://includes.resources/Views/Main.xaml").sp();
    let include = ResourceInclude::with_service_provider(sp);
    include.set_source(Some(uri("../Shared.xaml")));
    assert_eq!(text(include.try_get_resource(&key("source"), None)), "Shared.xaml");
    assert!(SEEN_PROVIDER.with(Cell::get));
    scope.dispose();
}

#[test]
fn resource_include_as_a_merged_dictionary_and_as_a_parent() {
    register_types();
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.resources", compiled_resources);

    let include = ResourceInclude::new(None);
    include.set_source(Some(uri("ferres://includes.resources/Merged.xaml")));

    let dictionary = ResourceDictionary::new();
    dictionary.add_merged_dictionary(include.as_resource_provider());
    assert_eq!(text(dictionary.try_get_resource(&key("source"), None)), "Merged.xaml");

    // Among the parents of a document it is a resource node, a resource
    // provider and a theme variant provider.
    let untyped: BoxedValue = include.clone();
    let sp = TestServiceProvider::new().with_parents(vec![untyped]).sp();
    assert!(sp.get_first_parent::<XamlResourceNode>().is_some());
    assert!(sp.get_first_parent::<Rc<dyn IResourceProvider>>().is_some());
    assert!(sp.get_first_parent::<Rc<dyn IThemeVariantProvider>>().is_some());
    let static_resource =
        crate::markup_extensions::StaticResourceExtension::with_resource_key(Some(boxed("source".to_string())));
    assert_eq!(text(Some(static_resource.provide_value(&sp).unwrap())), "Merged.xaml");
    scope.dispose();
}

#[test]
fn resource_include_answers_nothing_while_it_is_loading() {
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.reentrant", compiled_reentrant);

    let include = ResourceInclude::new(None);
    include.set_source(Some(uri("ferres://includes.reentrant/Self.xaml")));
    REENTRANT.with(|slot| *slot.borrow_mut() = Some(include.clone()));
    assert!(include.try_get_resource(&key("any"), None).is_none());
    REENTRANT.with(|slot| *slot.borrow_mut() = None);
    scope.dispose();
}

#[test]
fn resource_include_reports_a_document_that_cannot_be_loaded() {
    let scope = TestAssetLoader::new().install();
    let include = ResourceInclude::new(None);
    include.set_source(Some(uri("ferres://includes.nothing/A.xaml")));
    assert!(include.try_loaded().err().unwrap().message().starts_with("No precompiled XAML found"));
    scope.dispose();
}

#[test]
#[should_panic(expected = "ResourceInclude.Source must be set.")]
fn resource_include_requires_a_source() {
    let _ = ResourceInclude::new(None).loaded();
}

#[test]
#[should_panic(expected = "to type 'IResourceDictionary'")]
fn resource_include_requires_a_resource_dictionary() {
    let _scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.styles", compiled_styles);
    let include = ResourceInclude::new(None);
    include.set_source(Some(uri("ferres://includes.styles/NotAStyle.xaml")));
    let _ = include.loaded();
}

#[test]
fn merge_resource_include_behaves_like_a_resource_include() {
    register_types();
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.resources", compiled_resources);

    let include = MergeResourceInclude::new(Some(uri("ferres://includes.resources/Base.xaml")));
    include.set_source(Some(uri("Merge.xaml")));
    assert_eq!(text(include.try_get_resource(&key("source"), None)), "Merge.xaml");

    let sp = TestServiceProvider::new().with_base_uri("ferres://includes.resources/Base.xaml").sp();
    let other = MergeResourceInclude::with_service_provider(sp);
    other.set_source(Some(uri("Other.xaml")));
    assert!(other.loaded().has_resources());

    // It is a resource include: its base class part is reached by cast.
    let untyped: BoxedValue = include.clone();
    assert!(Rc::ptr_eq(&from_markup_value::<Rc<ResourceInclude>>(&Some(untyped.clone())).unwrap(), include.base()));
    let sp = TestServiceProvider::new().with_parents(vec![untyped]).sp();
    assert!(sp.get_first_parent::<Rc<dyn IResourceProvider>>().is_some());

    let markup = <MergeResourceInclude as MarkupTyped>::MARKUP;
    assert_eq!(markup.constructors.len(), 2);
    assert!(markup.base.is_some());
    scope.dispose();
}

#[test]
fn style_include_loads_its_style_on_first_use() {
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.styles", compiled_styles);

    let include = StyleInclude::new(Some(uri("ferres://includes.styles/App.xaml")));
    // Nothing is loaded before the style is asked for.
    assert!(include.children().is_empty());
    include.set_source(Some(uri("Style.xaml")));

    let loaded = include.loaded();
    assert!(loaded.as_object().unwrap().is::<Style>());
    let children = include.children();
    assert_eq!(children.len(), 1);
    assert!(ferroui_base::styling::style_ptr_eq(&children[0], &loaded));
    assert_eq!(text(IResourceNode::try_get_resource(&*include, &key("styled"), None)), "from style");
    assert!(IResourceNode::has_resources(&*include));

    // The owner is the owner of the loaded style.
    assert!(include.owner().is_none());
    let owner: ResourceHostRef = Border::new().into();
    let provider: &dyn IResourceProvider = &*include;
    let changes = Rc::new(Cell::new(0));
    let counter = changes.clone();
    let subscription = provider.owner_changed(Rc::new(move || counter.set(counter.get() + 1)));
    provider.add_owner(&owner);
    assert!(include.owner().unwrap() == owner);
    assert_eq!(changes.get(), 1);
    provider.remove_owner(&owner);
    assert!(provider.owner().is_none());
    subscription.dispose();
    scope.dispose();
}

#[test]
fn style_include_is_a_style_among_parents_and_in_a_styles_collection() {
    register_types();
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.styles", compiled_styles);

    let sp = TestServiceProvider::new().with_base_uri("ferres://includes.styles/App.xaml").sp();
    let include = StyleInclude::with_service_provider(sp);
    include.set_source(Some(uri("/Styles.xaml")));
    assert!(include.loaded().as_object().unwrap().is::<Styles>());

    let styles = Styles::new();
    styles.add(include.as_style());
    assert_eq!(styles.count(), 1);

    let untyped: BoxedValue = include;
    let sp = TestServiceProvider::new().with_parents(vec![untyped]).sp();
    assert!(sp.get_first_parent::<Rc<dyn IStyle>>().is_some());
    assert!(sp.get_first_parent::<Rc<dyn IResourceProvider>>().is_some());
    assert!(sp.get_first_parent::<XamlResourceNode>().is_some());
    // An include is not an object of the class model, so it cannot anchor a binding.
    assert!(sp.get_default_anchor().is_none());
    scope.dispose();
}

#[test]
#[should_panic(expected = "StyleInclude.Source must be set.")]
fn style_include_requires_a_source() {
    let _ = StyleInclude::new(None).loaded();
}

#[test]
#[should_panic(expected = "to type 'IStyle'")]
fn style_include_requires_a_style() {
    let _scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.styles", compiled_styles);
    let include = StyleInclude::new(None);
    include.set_source(Some(uri("ferres://includes.styles/NotAStyle.xaml")));
    let _ = include.loaded();
}

#[test]
fn includes_are_constructed_and_loaded_through_their_metadata() {
    register_types();
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.resources", compiled_resources);
    FerroXamlLoader::register_compiled_xaml("includes.styles", compiled_styles);

    let markup = <ResourceInclude as MarkupTyped>::MARKUP;
    assert_eq!(markup.namespace(), "FerroUI.Markup.Xaml.Styling");
    let base_uri = uri("ferres://includes.resources/Base.xaml");
    let include = (markup.constructors[0].invoke)(&[into_markup_value(base_uri)]).unwrap();
    let source = markup.find_property("Source").unwrap();
    (source.set.unwrap())(&[include.clone(), into_markup_value(uri("Meta.xaml"))]).unwrap();
    let typed = from_markup_value::<Rc<ResourceInclude>>(&include).unwrap();
    assert_eq!(text(typed.try_get_resource(&key("source"), None)), "Meta.xaml");
    assert!(markup.find_event("OwnerChanged").is_some());
    // The constructor without a base URI and the one taking a service provider.
    assert!((markup.constructors[0].invoke)(&[None]).unwrap().is_some());
    let sp = TestServiceProvider::new().with_base_uri("ferres://includes.resources/Base.xaml").sp();
    assert!((markup.constructors[1].invoke)(&[into_markup_value(sp.clone())]).unwrap().is_some());

    let markup = <StyleInclude as MarkupTyped>::MARKUP;
    let include = (markup.constructors[1].invoke)(&[into_markup_value(sp)]).unwrap();
    let source = markup.find_property("Source").unwrap();
    (source.set.unwrap())(&[include.clone(), into_markup_value(uri("ferres://includes.styles/Style.xaml"))]).unwrap();
    let typed = from_markup_value::<Rc<StyleInclude>>(&include).unwrap();
    assert!(typed.loaded().as_object().unwrap().is::<Style>());

    // The loader itself.
    let loader = <FerroXamlLoader as MarkupTyped>::MARKUP;
    let load = loader.find_methods("Load").find(|m| m.parameters.len() == 2 && m.return_type.is_some()).unwrap();
    let loaded = (load.invoke)(&[into_markup_value(uri("ferres://includes.styles/Style.xaml")), None]).unwrap();
    assert!(from_markup_value::<Ref<Style>>(&loaded).is_some());
    assert_eq!(loader.find_methods("Load").count(), 4);
    scope.dispose();
}

/// Not from upstream (the exception of a generated `TryLoad` propagates there): a compiled
/// document that fails to build is an error of the load, not a missing document.
#[test]
fn a_compiled_document_that_fails_to_build_is_an_error_of_the_load() {
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.failing", |_, _| {
        Err(XamlLoadException::with_message("The document failed to build"))
    });
    let error = FerroXamlLoader::load(&uri("ferres://includes.failing/A.xaml"), None).unwrap_err();
    assert_eq!(error.message(), "The document failed to build");
    assert!(FerroXamlLoader::unregister_compiled_xaml("includes.failing"));
    scope.dispose();
}

#[test]
fn a_registration_can_be_replaced_and_removed() {
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.replace", compiled_styles);
    FerroXamlLoader::register_compiled_xaml("includes.replace", |_, _| Ok(Some(boxed(1i32))));
    let loaded = FerroXamlLoader::load(&uri("ferres://includes.replace/A.xaml"), None).unwrap();
    assert_eq!(loaded.downcast_ref::<i32>(), Some(&1));

    assert!(FerroXamlLoader::unregister_compiled_xaml("includes.replace"));
    assert!(!FerroXamlLoader::unregister_compiled_xaml("includes.replace"));
    assert!(FerroXamlLoader::load(&uri("ferres://includes.replace/A.xaml"), None).is_err());
    scope.dispose();
}

#[test]
fn subscribing_to_owner_changed_through_metadata_reports_a_failing_load() {
    use ferroui_base::metadata::{MarkupDelegate, MarkupInvokeError};

    register_types();
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.resources", compiled_resources);
    FerroXamlLoader::register_compiled_xaml("includes.styles", compiled_styles);
    let handler = || into_markup_value(MarkupDelegate::new(|_| None));

    for markup in [<ResourceInclude as MarkupTyped>::MARKUP, <StyleInclude as MarkupTyped>::MARKUP] {
        let event = markup.find_event("OwnerChanged").unwrap();
        let source = markup.find_property("Source").unwrap();

        // No source.
        let include = (markup.constructors[0].invoke)(&[None]).unwrap();
        let expected = format!("{}.Source must be set.", markup.name);
        assert_eq!((event.add)(&[include.clone(), handler()]), Err(MarkupInvokeError::Failed(expected)));

        // A document that does not exist.
        (source.set.unwrap())(&[include.clone(), into_markup_value(uri("ferres://includes.nothing/A.xaml"))]).unwrap();
        assert!(matches!(
            (event.add)(&[include.clone(), handler()]),
            Err(MarkupInvokeError::Failed(message)) if message.starts_with("No precompiled XAML found")
        ));

        // A document of the wrong kind.
        (source.set.unwrap())(&[include.clone(), into_markup_value(uri("ferres://includes.styles/NotAStyle.xaml"))]).unwrap();
        assert!(matches!(
            (event.add)(&[include, handler()]),
            Err(MarkupInvokeError::Failed(message)) if message.starts_with("Unable to cast object of type")
        ));
    }

    // A document that loads: the handler is subscribed to the loaded object.
    let markup = <ResourceInclude as MarkupTyped>::MARKUP;
    let include = (markup.constructors[0].invoke)(&[None]).unwrap();
    (markup.find_property("Source").unwrap().set.unwrap())(&[
        include.clone(),
        into_markup_value(uri("ferres://includes.resources/Events.xaml")),
    ])
    .unwrap();
    let raised = Rc::new(Cell::new(0));
    let counter = raised.clone();
    let delegate = MarkupDelegate::new(move |_| {
        counter.set(counter.get() + 1);
        None
    });
    assert_eq!((markup.find_event("OwnerChanged").unwrap().add)(&[include.clone(), into_markup_value(delegate)]), Ok(None));
    let typed = from_markup_value::<Rc<ResourceInclude>>(&include).unwrap();
    let owner: ResourceHostRef = Border::new().into();
    typed.add_owner(&owner);
    assert_eq!(raised.get(), 1);
    scope.dispose();
}

#[test]
fn includes_are_added_to_dictionaries_and_styles_through_their_contracts() {
    use ferroui_base::controls::IResourceDictionary;

    register_types();
    let scope = TestAssetLoader::new().install();
    FerroXamlLoader::register_compiled_xaml("includes.resources", compiled_resources);
    FerroXamlLoader::register_compiled_xaml("includes.styles", compiled_styles);

    // An include in its untyped form is a resource provider and a theme
    // variant provider: what the merged and theme dictionaries take.
    let markup = <ResourceInclude as MarkupTyped>::MARKUP;
    let include = (markup.constructors[0].invoke)(&[None]).unwrap();
    (markup.find_property("Source").unwrap().set.unwrap())(&[
        include.clone(),
        into_markup_value(uri("ferres://includes.resources/Cast.xaml")),
    ])
    .unwrap();
    let provider = from_markup_value::<Rc<dyn IResourceProvider>>(&include).unwrap();
    let dictionary = ResourceDictionary::new();
    dictionary.merged_dictionaries().add(provider);
    assert_eq!(text(dictionary.try_get_resource(&key("source"), None)), "Cast.xaml");
    let themed = from_markup_value::<Rc<dyn IThemeVariantProvider>>(&include).unwrap();
    dictionary.add_theme_dictionary(ThemeVariant::dark(), themed);

    // `Loaded` and `Owner` through metadata.
    let loaded = (markup.find_property("Loaded").unwrap().get.unwrap())(&[include.clone()]).unwrap();
    assert!(from_markup_value::<Rc<dyn IResourceDictionary>>(&loaded).unwrap().has_resources());
    assert!((markup.find_property("Owner").unwrap().get.unwrap())(&[include]).is_ok());
    let unloadable = (markup.constructors[0].invoke)(&[None]).unwrap();
    assert!((markup.find_property("Loaded").unwrap().get.unwrap())(&[unloadable.clone()]).is_err());
    assert!((markup.find_property("Owner").unwrap().get.unwrap())(&[unloadable]).is_err());

    // A merging include is one too.
    let merge = <MergeResourceInclude as MarkupTyped>::MARKUP;
    let merging = (merge.constructors[0].invoke)(&[None]).unwrap();
    assert!(from_markup_value::<Rc<dyn IResourceProvider>>(&merging).is_some());
    assert!(from_markup_value::<Option<Rc<dyn IThemeVariantProvider>>>(&merging).unwrap().is_some());

    // A style include is a style and a resource provider.
    let markup = <StyleInclude as MarkupTyped>::MARKUP;
    let include = (markup.constructors[0].invoke)(&[None]).unwrap();
    (markup.find_property("Source").unwrap().set.unwrap())(&[
        include.clone(),
        into_markup_value(uri("ferres://includes.styles/Style.xaml")),
    ])
    .unwrap();
    let styles = Styles::new();
    styles.add(from_markup_value::<Rc<dyn IStyle>>(&include).unwrap());
    assert_eq!(styles.count(), 1);
    assert!(from_markup_value::<Rc<dyn IResourceProvider>>(&include).is_some());
    let loaded = (markup.find_property("Loaded").unwrap().get.unwrap())(&[include.clone()]).unwrap();
    assert!(from_markup_value::<Rc<dyn IStyle>>(&loaded).unwrap().as_object().unwrap().is::<Style>());
    assert_eq!((markup.find_property("Owner").unwrap().get.unwrap())(&[include]), Ok(None));
    scope.dispose();
}

/// A run-time loader that populates borders and keeps the default for anything else.
struct PopulatingRuntimeLoader;

impl IRuntimeXamlLoader for PopulatingRuntimeLoader {
    fn load(
        &self,
        _document: RuntimeXamlLoaderDocument,
        _configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<BoxedValue, XamlLoadException> {
        Err(XamlLoadException::with_message("not used"))
    }

    fn load_object(
        &self,
        instance: &BoxedValue,
        service_provider: Option<&Rc<dyn ferroui_base::metadata::IServiceProvider>>,
    ) -> Result<(), XamlLoadException> {
        match from_markup_value::<Ref<Border>>(&Some(instance.clone())) {
            Some(border) => {
                border.set_tag(Some(boxed(format!("populated, provider: {}", service_provider.is_some()))));
                Ok(())
            }
            None => DefaultRuntimeLoader.load_object(instance, service_provider),
        }
    }
}

/// A run-time loader that keeps the default `load_object`.
struct DefaultRuntimeLoader;

impl IRuntimeXamlLoader for DefaultRuntimeLoader {
    fn load(
        &self,
        _document: RuntimeXamlLoaderDocument,
        _configuration: RuntimeXamlLoaderConfiguration,
    ) -> Result<BoxedValue, XamlLoadException> {
        Err(XamlLoadException::with_message("not used"))
    }
}

#[test]
fn an_object_without_compiled_markup_is_populated_by_the_runtime_loader_service() {
    register_types();
    let scope = TestAssetLoader::new().install();
    let service: Rc<dyn IRuntimeXamlLoader> = Rc::new(PopulatingRuntimeLoader);
    FerroLocator::current_mutable().bind::<dyn IRuntimeXamlLoader>().to_constant(service);

    let border = Border::new();
    FerroXamlLoader::load_object(&boxed(border.clone())).unwrap();
    assert_eq!(text(Some(border.tag())), "populated, provider: false");
    let sp = TestServiceProvider::new().sp();
    FerroXamlLoader::load_object_with_service_provider(Some(&sp), &boxed(border.clone())).unwrap();
    assert_eq!(text(Some(border.tag())), "populated, provider: true");

    // A loader without markup for the class reports the object as not compiled.
    let error = FerroXamlLoader::load_object(&boxed(ResourceDictionary::new())).unwrap_err();
    assert!(error.message().starts_with("No precompiled XAML found for FerroUI.Controls.ResourceDictionary,"));
    scope.dispose();
}

