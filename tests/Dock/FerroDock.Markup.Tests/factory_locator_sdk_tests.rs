//! Real application consumers of Core factory metadata, delegates and original locator maps.
use crate::{locator_factory::LocatorFactory, markup_extension_tests::run};
use ferrodock_model::core::{IDockable, IFactory, IFactoryEvents};
use ferrodock_model_ferro::{
    ContextFactoryReference, ContextLocatorMapReference, DockableFactoryReference,
    DockableLocatorMapReference, FactoryBaseReference, IFactoryMetadata,
};
use ferroui_base::{
    metadata::{MarkupDelegate, MarkupTyped},
    threading::Dispatcher,
    BoxedValue, Ref,
};
use ferroui_controls::{CheckBox, StackPanel, TextBlock};
use std::rc::Rc;
fn load(name: &str) -> BoxedValue {
    crate::compiled_xaml::try_load(
        None,
        &format!("ferres://ferrodock_markup_tests/{name}.xaml"),
    )
    .unwrap()
    .unwrap()
}
#[test]
fn original_locator_declarations_compile_and_keep_the_same_source_targets_objects_and_deferred_calls(
) {
    run(|| {
        crate::locator_factory::register_types();
        let value = load("FactoryLocatorConsumer");
        let factory = value.downcast_ref::<Rc<LocatorFactory>>().unwrap().clone();
        assert_eq!(factory.calls(), (0, 0, 0));
        let first = factory.get_context("missing".into()).unwrap();
        let second = factory.get_context("missing".into()).unwrap();
        assert!(Rc::ptr_eq(&first, &second));
        assert_eq!(first.downcast_ref::<String>().unwrap(), "Located context");
        assert!(Rc::ptr_eq(
            &factory.get_host_window("missing".into()).unwrap(),
            &factory.host()
        ));
        assert_eq!(factory.calls(), (2, 1, 0));
        let map = DockableLocatorMapReference::create();
        let method = LocatorFactory::MARKUP
            .find_methods("CreateDockable")
            .next()
            .unwrap();
        let delegate =
            MarkupDelegate::for_method(Some(value.clone()), LocatorFactory::MARKUP, method);
        let callback = DockableFactoryReference::from_delegate(delegate).unwrap();
        map.add("document".into(), Some(callback)).unwrap();
        map.add("null".into(), None).unwrap();
        factory.set_dockable_locator(Some(map.model()));
        assert_eq!(factory.calls(), (2, 1, 0));
        assert!(Rc::ptr_eq(
            &factory
                .get_dockable::<dyn IDockable>("document".into())
                .unwrap(),
            &factory.document()
        ));
        assert!(factory
            .get_dockable::<dyn IDockable>("null".into())
            .is_none());
        assert_eq!(factory.calls(), (2, 1, 1));
        let weak = Rc::downgrade(&factory);
        map.clear();
        factory.release_callbacks();
        drop(factory);
        drop(value);
        assert!(weak.upgrade().is_none());
    });
}
#[test]
fn compiled_core_interface_bindings_read_original_tracking_maps_and_two_way_policy() {
    run(|| {
        let factory = ferrodock_model_reactiveui::Factory::new();
        let root = factory.create_root_dock();
        let doc = factory.create_document();
        doc.set_title("Tracked model".into());
        factory.add_dockable(Some(root.clone()), Some(doc.clone()));
        factory.on_active_dockable_changed(Some(doc.clone()));
        let map = ContextLocatorMapReference::create();
        map.add("null".into(), None).unwrap();
        map.add(
            "none".into(),
            Some(ContextFactoryReference::new(Rc::new(|| None))),
        )
        .unwrap();
        factory.set_context_locator(Some(map.model()));
        let value = load("CoreFactoryBindings");
        let panel = value.downcast_ref::<Ref<StackPanel>>().unwrap();
        panel.set_data_context(Some(Rc::new(factory.clone() as Rc<dyn IFactory>)));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            panel
                .children()
                .get(0)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("Tracked model")
        );
        assert_eq!(
            panel
                .children()
                .get(2)
                .cast::<TextBlock>()
                .unwrap()
                .text()
                .as_deref(),
            Some("2")
        );
        panel
            .children()
            .get(1)
            .cast::<CheckBox>()
            .unwrap()
            .set_is_checked(Some(true));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(factory.hide_documents_on_close());
        panel.set_data_context(None);
        assert!(std::ptr::eq(
            <dyn IFactoryMetadata as MarkupTyped>::MARKUP,
            <dyn ferrodock_ferro::IFactoryMetadata as MarkupTyped>::MARKUP
        ));
    });
}
#[test]
fn runtime_compiler_uses_the_same_core_contract_owner_and_registered_delegate_casts() {
    run(|| {
        crate::locator_factory::register_types();
        let mut config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        config.local_assembly = Some(&crate::locator_factory::ASSEMBLY);
        let results = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[
                ("Locator.xaml", include_str!("FactoryLocatorConsumer.xaml")),
                ("Bindings.xaml", include_str!("CoreFactoryBindings.xaml")),
                ("Search.xaml", include_str!("FactorySearchConsumer.xaml")),
                ("Api.xaml", include_str!("CoreFactoryApiBindings.xaml")),
            ],
            Some("ferres://ferrodock_markup_tests/"),
            &config,
            &[],
        );
        for result in &results {
            assert!(result.source.is_ok(), "{:?}", result.source);
        }
        let source = results[0].source.as_ref().unwrap();
        assert!(source.contains("__markup_set_DefaultContextLocator"));
        assert!(source.contains("rt::cast"));
        assert!(source.contains("FactoryBaseReference"));
        assert!(results[1]
            .source
            .as_ref()
            .unwrap()
            .contains("CurrentDockable"));
        assert!(FactoryBaseReference::MARKUP
            .find_property("ContextLocator")
            .is_some());
    });
}

#[test]
fn original_predicate_declarations_compile_and_invoke_lazy_search_on_the_same_engine() {
    run(|| {
        crate::locator_factory::register_types();
        let value = load("FactorySearchConsumer");
        let factory = value.downcast_ref::<Rc<LocatorFactory>>().unwrap().clone();
        assert_eq!(factory.predicate_calls(), 0);
        let root = factory.create_root_dock();
        root.set_id("root".into());
        let document = factory.document();
        document.set_id("item".into());
        factory.add_dockable(Some(root.clone()), Some(document.clone()));
        let metadata = FactoryBaseReference::MARKUP;
        let method = metadata
            .find_methods("Find")
            .find(|method| method.parameters.len() == 2)
            .unwrap();
        let result = (method.invoke)(&[
            Some(Rc::new(FactoryBaseReference::new(factory.clone()))),
            Some(Rc::new(root.clone() as Rc<dyn ferrodock_model::core::IDock>)),
            Some(Rc::new(factory.search_predicate().unwrap())),
        ])
        .unwrap();
        let query = ferroui_base::metadata::from_markup_value::<
            ferrodock_model_ferro::DockableQueryReference,
        >(&result)
        .unwrap();
        assert_eq!(factory.predicate_calls(), 0);
        let cursor = query.get_enumerator();
        assert_eq!(factory.predicate_calls(), 0);
        assert!(cursor.move_next());
        assert!(Rc::ptr_eq(&cursor.current().unwrap(), &document));
        assert!(!cursor.move_next());
        assert_eq!(factory.predicate_calls(), 2);
        let method = metadata
            .find_methods("FindRoot")
            .find(|method| method.parameters.len() == 2)
            .unwrap();
        let found = (method.invoke)(&[
            Some(Rc::new(FactoryBaseReference::new(factory.clone()))),
            Some(Rc::new(document)),
            Some(Rc::new(factory.root_predicate())),
        ])
        .unwrap();
        let found = ferroui_base::metadata::from_markup_value::<
            Rc<dyn ferrodock_model::controls::IRootDock>,
        >(&found)
        .unwrap();
        assert!(Rc::ptr_eq(&found, &root));
        factory.release_callbacks();
    });
}

#[test]
fn core_factory_method_command_binding_activates_the_original_tracked_window() {
    run(|| {
        let factory = ferrodock_model_reactiveui::Factory::new();
        let root = factory.create_root_dock();
        let document = factory.create_document();
        factory.add_dockable(Some(root.clone()), Some(document.clone()));
        factory.init_layout(root.clone());
        let window = factory.create_dock_window();
        window.set_layout(Some(root.clone()));
        root.set_window(Some(window.clone()));
        factory.init_dock_window(Some(window.clone()), Some(root.clone()));
        let value = load("CoreFactoryApiBindings");
        let button = value
            .downcast_ref::<Ref<ferroui_controls::Button>>()
            .unwrap();
        button.set_data_context(Some(Rc::new(factory.clone() as Rc<dyn IFactory>)));
        Dispatcher::ui_thread().run_jobs(None);
        let parameter: BoxedValue = Rc::new(document as Rc<dyn IDockable>);
        button.command().unwrap().execute(Some(&parameter));
        assert!(Rc::ptr_eq(&factory.current_dock_window().unwrap(), &window));
        button.set_data_context(None);
    });
}
