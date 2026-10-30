//! Source markup extension tests through a separate compiled library consumer.
use ferrodock_markup_extension::{LoadExtension, ReferenceExtension};
use ferrodock_model::core::IDockable;
use ferroui_base::{
    controls::{INameScope, NameScope},
    data::core::ValueTypes,
    metadata::{service, EmptyServiceProvider, IServiceProvider, MarkupTyped},
    utilities::{Uri, UriKind},
    BoxedValue, Ref,
};
use ferroui_controls::{Button, ContentControl, StackPanel, TextBlock};
use ferroui_markup_xaml::{IUriContext, MarkupExtension};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(super) struct UriContext {
    uri: RefCell<Option<Uri>>,
    reads: Cell<usize>,
    on_read: RefCell<Option<Rc<dyn Fn()>>>,
}
impl IUriContext for UriContext {
    fn base_uri(&self) -> Option<Uri> {
        self.reads.set(self.reads.get() + 1);
        let callback = self.on_read.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
        self.uri.borrow().clone()
    }
    fn set_base_uri(&self, value: Option<Uri>) {
        self.uri.replace(value);
    }
}
struct Provider {
    context: Option<Rc<dyn IUriContext>>,
    scope: Option<Rc<dyn INameScope>>,
}
impl IServiceProvider for Provider {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        self.context
            .as_ref()
            .and_then(|context| service(service_type, || context.clone()))
            .or_else(|| {
                self.scope
                    .as_ref()
                    .and_then(|scope| service(service_type, || scope.clone()))
            })
    }
}
fn uri(text: &str) -> Uri {
    Uri::new(text, UriKind::RelativeOrAbsolute).unwrap()
}
pub(super) fn context(base: Option<&str>) -> Rc<UriContext> {
    Rc::new(UriContext {
        uri: RefCell::new(base.map(uri)),
        reads: Cell::new(0),
        on_read: RefCell::new(None),
    })
}
pub(super) fn provider(
    context: Option<Rc<dyn IUriContext>>,
    scope: Option<Rc<dyn INameScope>>,
) -> Rc<dyn IServiceProvider> {
    Rc::new(Provider { context, scope })
}
pub(super) fn run(test: impl FnOnce() + Send + 'static) {
    ferroui_headless::HeadlessUnitTestSession::get_or_start_for_assembly(Some(
        &crate::tests::ASSEMBLY,
    ))
    .dispatch(
        move || {
            ferrodock_ferro::register_types();
            ferrodock_model_reactiveui_ferro::register_types();
            ferrodock_markup_extension::register_types();
            crate::compiled_markup::register();
            test();
        },
        ferroui_base::threading::CancellationToken::none(),
    )
    .wait()
    .unwrap();
}
#[test]
fn load_constructor_and_nullable_source_preserve_the_uri() {
    let source = uri("ferres://ferrodock_markup_tests/LoadedExtensionView.xaml");
    let extension = LoadExtension::with_source(source.clone());
    assert_eq!(extension.source(), Some(source));
    extension.set_source(None);
    assert!(extension.source().is_none());
}
#[test]
fn load_without_context_returns_null_even_for_a_valid_source() {
    run(|| {
        let extension = LoadExtension::with_source(uri(
            "ferres://ferrodock_markup_tests/LoadedExtensionView.xaml",
        ));
        assert!(extension
            .provide_value(&provider(None, None))
            .unwrap()
            .is_none());
    });
}
#[test]
fn load_without_source_returns_null_without_reading_base_uri() {
    let context = context(Some("ferres://ferrodock_markup_tests/"));
    assert!(LoadExtension::new()
        .provide_value(&provider(Some(context.clone()), None))
        .unwrap()
        .is_none());
    assert_eq!(context.reads.get(), 0);
}
#[test]
fn load_absolute_uri_uses_real_compiled_document_and_returns_fresh_objects() {
    run(|| {
        let extension = LoadExtension::with_source(uri(
            "ferres://ferrodock_markup_tests/LoadedExtensionView.xaml",
        ));
        let provider = provider(
            Some(context(Some("ferres://ferrodock_markup_tests/"))),
            None,
        );
        let first = MarkupExtension::provide_value(&*extension, &provider)
            .unwrap()
            .unwrap();
        let second = extension.provide_value(&provider).unwrap().unwrap();
        let first = first.downcast_ref::<Ref<TextBlock>>().unwrap();
        let second = second.downcast_ref::<Ref<TextBlock>>().unwrap();
        assert_eq!(first.text().as_deref(), Some("Hello"));
        assert_ne!(first, second);
    });
}
#[test]
fn load_relative_uri_reads_the_current_context_base_and_keeps_native_errors() {
    run(|| {
        let context = context(Some("ferres://ferrodock_markup_tests/Views/Host.xaml"));
        let provider = provider(Some(context.clone()), None);
        let extension = LoadExtension::with_source(uri("../LoadedExtensionView.xaml"));
        assert_eq!(
            extension
                .provide_value(&provider)
                .unwrap()
                .unwrap()
                .downcast_ref::<Ref<TextBlock>>()
                .unwrap()
                .text()
                .as_deref(),
            Some("Hello")
        );
        assert_eq!(context.reads.get(), 1);
        context.set_base_uri(Some(uri("ferres://ferrodock_markup_tests/Other/Host.xaml")));
        extension.set_source(Some(uri("Missing.xaml")));
        assert!(extension.provide_value(&provider).is_err());
        assert_eq!(context.reads.get(), 2);
    });
}
#[test]
fn absolute_load_still_succeeds_when_present_uri_context_has_null_base() {
    run(|| {
        let extension = LoadExtension::with_source(uri(
            "ferres://ferrodock_markup_tests/LoadedExtensionView.xaml",
        ));
        assert!(extension
            .provide_value(&provider(Some(context(None)), None))
            .unwrap()
            .is_some());
    });
}
#[test]
fn reference_constructor_and_nullable_name_keep_original_value() {
    let extension = ReferenceExtension::with_name("myElement".into());
    assert_eq!(extension.name().as_deref(), Some("myElement"));
    extension.set_name(None);
    assert!(extension.name().is_none());
}
#[test]
fn reference_returns_the_same_concrete_button_in_the_supplied_scope() {
    run(|| {
        let button = Button::new();
        let scope = Rc::new(NameScope::new());
        scope.register("btn", button.clone().upcast());
        let value = ReferenceExtension::with_name("btn".into())
            .provide_value(&provider(None, Some(scope)))
            .unwrap();
        assert_eq!(value.downcast_ref::<Ref<Button>>().unwrap(), &button);
    });
}
#[test]
fn reference_without_scope_name_or_match_returns_null() {
    run(|| {
        let extension = ReferenceExtension::with_name("missing".into());
        assert!(extension.provide_value(&provider(None, None)).is_none());
        let scope = Rc::new(NameScope::new());
        let provider = provider(None, Some(scope));
        assert!(extension.provide_value(&provider).is_none());
        extension.set_name(None);
        assert!(extension.provide_value(&provider).is_none());
    });
}
#[test]
fn reference_uses_immediate_find_and_reads_its_current_name_and_provider_each_time() {
    run(|| {
        let first = Button::new();
        let second = Button::new();
        let scope = Rc::new(NameScope::new());
        let other = Rc::new(NameScope::new());
        other.register("later", second.clone().upcast());
        let extension = ReferenceExtension::with_name("later".into());
        let first_provider = provider(None, Some(scope.clone()));
        assert!(extension.provide_value(&first_provider).is_none());
        scope.register("later", first.clone().upcast());
        assert_eq!(
            ValueTypes::as_object(&*extension.provide_value(&first_provider).unwrap()).unwrap(),
            first.upcast::<ferroui_base::FerroObject>()
        );
        assert_eq!(
            extension
                .provide_value(&provider(None, Some(other)))
                .unwrap()
                .downcast_ref::<Ref<Button>>()
                .unwrap(),
            &second
        );
        extension.set_name(Some("different".into()));
        assert!(extension.provide_value(&first_provider).is_none());
    });
}
#[test]
fn exported_constructor_properties_and_provide_value_methods_keep_source_names() {
    ferrodock_markup_extension::register_types();
    assert_eq!(
        ferrodock_markup_extension::ASSEMBLY.name,
        "FerroDock.MarkupExtension"
    );
    assert_eq!(
        ferrodock_markup_extension::ASSEMBLY.xmlns_definitions[0].namespace,
        "FerroUI.MarkupExtension"
    );
    for metadata in [LoadExtension::MARKUP, ReferenceExtension::MARKUP] {
        assert_eq!(metadata.constructors.len(), 2);
        assert!(metadata
            .methods
            .iter()
            .any(|method| method.name == "ProvideValue"));
    }
    let provider: Rc<dyn IServiceProvider> = Rc::new(EmptyServiceProvider);
    assert!(
        MarkupExtension::provide_value(&*ReferenceExtension::new(), &provider)
            .unwrap()
            .is_none()
    );
}
#[test]
fn independent_compiled_consumer_resolves_both_xml_namespaces_constructors_and_properties() {
    run(|| {
        let value = crate::compiled_xaml::try_load(
            None,
            "ferres://ferrodock_markup_tests/MarkupExtensionConsumer.xaml",
        )
        .unwrap()
        .unwrap();
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        assert_eq!(panel.children().count(), 8);
        let named = panel.children().get(0).cast::<TextBlock>().unwrap();
        for index in [1, 2] {
            let border = panel
                .children()
                .get(index)
                .cast::<ferroui_controls::Border>()
                .unwrap();
            assert_eq!(
                ValueTypes::as_object(&*border.tag().unwrap()).unwrap(),
                named.clone().upcast::<ferroui_base::FerroObject>()
            );
        }
        let mut loaded: Vec<BoxedValue> = Vec::new();
        for index in [3, 4] {
            let host = panel
                .children()
                .get(index)
                .cast::<ContentControl>()
                .unwrap();
            let value = host.content().unwrap();
            assert_eq!(
                ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("Hello")
            );
            loaded.push(value);
        }
        assert_ne!(
            ValueTypes::as_object(&*loaded[0]),
            ValueTypes::as_object(&*loaded[1])
        );
        assert!(panel
            .children()
            .get(5)
            .cast::<ferroui_controls::Border>()
            .unwrap()
            .tag()
            .is_none());
        assert!(panel
            .children()
            .get(6)
            .cast::<ContentControl>()
            .unwrap()
            .content()
            .is_none());
        assert!(panel
            .children()
            .get(7)
            .cast::<ferroui_controls::Border>()
            .unwrap()
            .tag()
            .is_none());
    });
}

#[test]
fn load_re_reads_source_after_external_base_uri_getter_including_null_failure() {
    run(|| {
        let extension = LoadExtension::with_source(uri("Missing.xaml"));
        let context = context(Some("ferres://ferrodock_markup_tests/"));
        let provider = provider(Some(context.clone()), None);
        let target = Rc::downgrade(&extension);
        context.on_read.replace(Some(Rc::new(move || {
            target
                .upgrade()
                .unwrap()
                .set_source(Some(uri("LoadedExtensionView.xaml")))
        })));
        let loaded = extension.provide_value(&provider).unwrap().unwrap();
        assert_eq!(
            loaded
                .downcast_ref::<Ref<TextBlock>>()
                .unwrap()
                .text()
                .as_deref(),
            Some("Hello")
        );
        let target = Rc::downgrade(&extension);
        context.on_read.replace(Some(Rc::new(move || {
            target.upgrade().unwrap().set_source(None)
        })));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            extension.provide_value(&provider)
        }));
        assert!(result.is_err());
        assert!(extension.source().is_none());
        let reads = context.reads.get();
        assert!(extension.provide_value(&provider).unwrap().is_none());
        assert_eq!(context.reads.get(), reads);
    });
}

#[test]
fn load_plain_model_root_uses_the_native_compiled_asset_loader_and_returns_fresh_engine_objects() {
    run(|| {
        ValueTypes::register_reference::<crate::named_document::NamedDocument>();
        let extension = LoadExtension::with_source(uri("NamedModelRoot.xaml"));
        let provider = provider(
            Some(context(Some("ferres://ferrodock_markup_tests/"))),
            None,
        );
        let first = extension.provide_value(&provider).unwrap().unwrap();
        let second = extension.provide_value(&provider).unwrap().unwrap();
        let first = first
            .downcast_ref::<Rc<crate::named_document::NamedDocument>>()
            .unwrap();
        let second = second
            .downcast_ref::<Rc<crate::named_document::NamedDocument>>()
            .unwrap();
        assert_eq!(first.model().title(), "Plain root");
        assert!(!Rc::ptr_eq(first, second));
        assert!(!Rc::ptr_eq(&first.model(), &second.model()));
    });
}
