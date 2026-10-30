//! Native application callbacks and original document creation through both compiler hosts.
use crate::{locator_factory::LocatorFactory, markup_extension_tests::run};
use ferrodock_model::{controls::IDocumentDockFactory, core::*};
use ferrodock_model_ferro::{DocumentFactoryReference, IDocumentDockFactoryMetadata};
use ferrodock_model_reactiveui_ferro::reactive_metadata::DocumentDockReference;
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{from_markup_value, MarkupTyped},
    threading::Dispatcher,
    BoxedValue, Ref,
};
use ferroui_controls::{Button, ContentControl, StackPanel};
use std::rc::Rc;
fn load(name: &str) -> BoxedValue {
    crate::compiled_xaml::try_load(
        None,
        &format!("ferres://ferrodock_markup_tests/{name}.xaml"),
    )
    .unwrap()
    .unwrap()
}
fn assert_factory(
    producer: &Rc<LocatorFactory>,
    dock: &DocumentDockReference,
    panel: &Ref<StackPanel>,
) {
    assert_eq!(producer.calls(), (0, 0, 0));
    let native = producer.document_factory().unwrap();
    let original = native.model();
    let receiver: BoxedValue = Rc::new(dock.clone());
    let metadata = DocumentDockReference::MARKUP;
    let property = metadata.find_property("DocumentFactory").unwrap();
    (property.set.unwrap())(&[Some(receiver.clone()), Some(Rc::new(native.clone()))]).unwrap();
    assert!(Rc::ptr_eq(
        &dock.factory_contract().document_factory().unwrap(),
        &original
    ));
    assert_eq!(producer.calls(), (0, 0, 0));
    let exposed = from_markup_value::<DocumentFactoryReference>(
        &(property.get.unwrap())(&[Some(receiver.clone())]).unwrap(),
    )
    .unwrap();
    assert!(Rc::ptr_eq(&exposed.model(), &original));
    let contract =
        ValueTypes::try_cast(&receiver, ValueType::of::<Rc<dyn IDocumentDockFactory>>()).unwrap();
    assert!(ValueTypes::reference_equals(&receiver, &contract));
    panel.set_data_context(Some(contract));
    Dispatcher::ui_thread().run_jobs(None);
    let content = panel
        .children()
        .get(0)
        .cast::<ContentControl>()
        .unwrap()
        .content();
    let bound = from_markup_value::<DocumentFactoryReference>(&content).unwrap();
    assert!(Rc::ptr_eq(&bound.model(), &original));
    let button = panel.children().get(1).cast::<Button>().unwrap();
    button.command().unwrap().execute(None);
    assert_eq!(producer.calls(), (0, 0, 1));
    let f = ferrodock_model_reactiveui::Factory::new();
    let source = dock.model();
    source.set_factory(Some(f.clone()));
    source.set_visible_dockables(Some(f.create_list(Vec::new())));
    source.create_document().unwrap().execute(None);
    assert_eq!(producer.calls(), (0, 0, 2));
    let doc = producer.document();
    assert!(Rc::ptr_eq(
        (source.visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &doc
    ));
    assert!(Rc::ptr_eq(&source.active_dockable().unwrap(), &doc));
    let owner: Rc<dyn IDockable> = source;
    assert!(Rc::ptr_eq(&doc.owner().unwrap(), &owner));
    panel.set_data_context(None);
    (property.set.unwrap())(&[Some(receiver.clone()), None]).unwrap();
    assert!(dock.factory_contract().document_factory().is_none());
    assert!((property.get.unwrap())(&[Some(receiver)])
        .unwrap()
        .is_none());
    producer.release_callbacks();
}
#[test]
fn compiled_native_method_assignment_and_core_func_binding_drive_original_document_creation() {
    run(|| {
        crate::locator_factory::register_types();
        let producer = load("NativeDocumentFactoryProducer")
            .downcast_ref::<Rc<LocatorFactory>>()
            .unwrap()
            .clone();
        let dock =
            from_markup_value::<DocumentDockReference>(&Some(load("ReactiveDocumentFactoryRoot")))
                .unwrap();
        let panel = load("CoreDocumentFactoryBindings")
            .downcast_ref::<Ref<StackPanel>>()
            .unwrap()
            .clone();
        assert_factory(&producer, &dock, &panel);
    });
}
#[test]
fn runtime_emission_and_interpretation_preserve_same_original_callback_class_and_graph() {
    run(|| {
        crate::locator_factory::register_types();
        let mut config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        config.local_assembly = Some(&crate::locator_factory::ASSEMBLY);
        config.use_compiled_bindings_by_default = true;
        let documents = [
            (
                "Producer.xaml",
                include_str!("NativeDocumentFactoryProducer.xaml"),
            ),
            (
                "Dock.xaml",
                include_str!("ReactiveDocumentFactoryRoot.xaml"),
            ),
            (
                "Bindings.xaml",
                include_str!("CoreDocumentFactoryBindings.xaml"),
            ),
        ];
        let emitted = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &documents,
            Some("ferres://ferrodock_markup_tests/"),
            &config,
            &[],
        );
        assert_eq!(emitted.len(), 3);
        for doc in emitted {
            assert!(doc.source.is_ok(), "{:?}", doc.source);
        }
        let values: Vec<_> = documents
            .iter()
            .map(|(_, source)| {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
                    ferroui_markup_xaml::RuntimeXamlLoaderDocument::new(source),
                    Some(config.clone()),
                )
                .unwrap()
            })
            .collect();
        let producer = from_markup_value::<Rc<LocatorFactory>>(&Some(values[0].clone()))
            .expect("runtime producer retains its registered source class handle");
        let dock = from_markup_value::<DocumentDockReference>(&Some(values[1].clone())).unwrap();
        let panel = values[2].downcast_ref::<Ref<StackPanel>>().unwrap();
        assert_factory(&producer, &dock, panel);
    });
}
#[test]
fn core_qualified_factory_interface_and_concrete_class_share_the_same_source_callback() {
    run(|| {
        crate::locator_factory::register_types();
        let dock = DocumentDockReference::create();
        let source = dock.factory_contract();
        let boxed: BoxedValue = Rc::new(source.clone());
        let p = <dyn IDocumentDockFactoryMetadata>::MARKUP
            .find_property("DocumentFactory")
            .unwrap();
        let callback = DocumentFactoryReference::new(Rc::new(move || {
            Some(ferrodock_model_reactiveui::controls::Document::new())
        }));
        (p.set.unwrap())(&[Some(boxed.clone()), Some(Rc::new(callback.clone()))]).unwrap();
        assert!(Rc::ptr_eq(
            &source.document_factory().unwrap(),
            &callback.model()
        ));
        let class_value: BoxedValue = Rc::new(dock);
        let class_get = DocumentDockReference::MARKUP
            .find_property("DocumentFactory")
            .unwrap()
            .get
            .unwrap();
        let actual = from_markup_value::<DocumentFactoryReference>(
            &class_get(&[Some(class_value)]).unwrap(),
        )
        .unwrap();
        assert!(Rc::ptr_eq(&actual.model(), &callback.model()));
        (p.set.unwrap())(&[Some(boxed), None]).unwrap();
    });
}

fn assert_cross_assignment(panel: &Ref<StackPanel>) {
    use ferrodock_model_ferro::{DockableFactoryReference, DockableLocatorMapReference};
    let source = DocumentDockReference::create();
    let contract = source.factory_contract();
    let boxed: BoxedValue = Rc::new(contract.clone());
    panel.set_data_context(Some(boxed.clone()));
    Dispatcher::ui_thread().run_jobs(None);
    let calls = Rc::new(std::cell::Cell::new(0));
    let seen = calls.clone();
    let document: Rc<dyn IDockable> = ferrodock_model_reactiveui::controls::Document::new();
    let returned = document.clone();
    let callback: ferrodock_model::core::DockableFactory = Rc::new(move || {
        seen.set(seen.get() + 1);
        Some(returned.clone())
    });
    let original: BoxedValue = Rc::new(DockableFactoryReference::new(callback.clone()));
    let host = panel.children().get(0).cast::<ContentControl>().unwrap();
    host.set_content(Some(original.clone()));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(calls.get(), 0);
    let property = <dyn IDocumentDockFactoryMetadata>::MARKUP
        .find_property("DocumentFactory")
        .unwrap();
    let native = (property.get.unwrap())(&[Some(boxed.clone())])
        .unwrap()
        .unwrap();
    assert!(ValueTypes::reference_equals(&original, &native));
    let map = DockableLocatorMapReference::create();
    let metadata = DockableLocatorMapReference::MARKUP;
    (metadata.find_indexer(1).unwrap().set.unwrap())(&[
        Some(Rc::new(map.clone())),
        Some(Rc::new("callback".to_owned())),
        Some(native),
    ])
    .unwrap();
    assert!(Rc::ptr_eq(
        &map.get("callback".into()).unwrap().unwrap().model(),
        &callback
    ));
    let f = ferrodock_model_reactiveui::Factory::new();
    source.model().set_factory(Some(f.clone()));
    source
        .model()
        .set_visible_dockables(Some(f.create_list(Vec::new())));
    source.model().create_document().unwrap().execute(None);
    assert_eq!(calls.get(), 1);
    assert!(Rc::ptr_eq(
        (source.model().visible_dockables().unwrap().get(0))
            .as_ref()
            .expect("nonnull identity fixture"),
        &document
    ));
    panel.set_data_context(None);
    panel.set_data_context(Some(boxed));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(ValueTypes::reference_equals(
        host.content().as_ref().unwrap(),
        &original
    ));
    panel.set_data_context(None);
    contract.set_document_factory(None);
    map.clear();
    host.set_content(None);
}
#[test]
fn compiled_two_way_object_assignment_preserves_callback_identity_through_original_factory_and_locator_map(
) {
    run(|| {
        let panel = load("CoreFactoryAssignmentBindings")
            .downcast_ref::<Ref<StackPanel>>()
            .unwrap()
            .clone();
        assert_cross_assignment(&panel);
    });
}
#[test]
fn runtime_two_way_object_assignment_uses_the_same_source_delegate_bridge() {
    run(|| {
        let mut config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        config.local_assembly = Some(&crate::locator_factory::ASSEMBLY);
        config.use_compiled_bindings_by_default = true;
        let source = include_str!("CoreFactoryAssignmentBindings.xaml");
        let emitted = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[("Assignment.xaml", source)],
            Some("ferres://ferrodock_markup_tests/"),
            &config,
            &[],
        );
        assert_eq!(emitted.len(), 1);
        assert!(emitted[0].source.is_ok(), "{:?}", emitted[0].source);
        let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            ferroui_markup_xaml::RuntimeXamlLoaderDocument::new(source),
            Some(config),
        )
        .unwrap();
        assert_cross_assignment(value.downcast_ref::<Ref<StackPanel>>().unwrap());
    });
}
