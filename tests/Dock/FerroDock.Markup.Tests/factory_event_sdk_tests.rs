//! Compiled application boundary for original factory events and argument roots.
use crate::{factory_event_consumer::FactoryEventConsumer, markup_extension_tests::run};
use ferrodock_model::core::{events::DockableClosingEventArgs, GlobalDockTrackingState};
use ferrodock_model_ferro::{
    event_metadata::DockableClosingEventArgsMetadata, FactoryBaseReference,
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupDelegate, MarkupType, MarkupTyped},
    BoxedValue,
};
use std::{cell::Cell, rc::Rc};
fn load(name: &str) -> BoxedValue {
    crate::compiled_xaml::try_load(
        None,
        &format!("ferres://ferrodock_markup_tests/{name}.xaml"),
    )
    .unwrap()
    .unwrap()
}
#[test]
fn compiled_constructor_arguments_return_the_original_argument_and_weak_snapshot_types() {
    run(|| {
        crate::factory_event_consumer::register_types();
        let value = load("FactoryEventArgumentRoot");
        let args = value.downcast_ref::<DockableClosingEventArgs>().unwrap();
        assert!(args.cancel());
        assert_eq!(args.dockable().unwrap().title(), "Event document");
        let clone: BoxedValue = Rc::new(args.clone());
        assert!(ValueTypes::reference_equals(&value, &clone));
        assert!(!ValueTypes::reference_equals(
            &value,
            &load("FactoryEventArgumentRoot")
        ));
        let snapshot = load("FactoryTrackingStateRoot");
        let snapshot = snapshot.downcast_ref::<GlobalDockTrackingState>().unwrap();
        assert!(
            snapshot.dockable().is_none()
                && snapshot.root_dock().is_none()
                && snapshot.window().is_none()
        );
    });
}
#[test]
fn compiled_event_handler_receives_and_cancels_the_original_factory_event() {
    run(|| {
        crate::factory_event_consumer::register_types();
        let value = load("FactoryEventConsumer");
        let consumer = value.downcast_ref::<Rc<FactoryEventConsumer>>().unwrap();
        let factory = consumer.factory().unwrap().model();
        let document = factory.create_document();
        assert!(!factory.on_dockable_closing(Some(document.clone())));
        assert_eq!(consumer.calls(), 1);
        let sender = consumer.sender().unwrap();
        assert!(Rc::ptr_eq(
            sender
                .downcast_ref::<Rc<dyn ferrodock_model::core::IFactory>>()
                .unwrap(),
            &factory
        ));
        // The interface describes exactly the same source events as the abstract class.
        let interface = <dyn ferrodock_ferro::IFactoryMetadata as MarkupTyped>::MARKUP;
        assert_eq!(
            interface.events.iter().map(|e| e.name).collect::<Vec<_>>(),
            FactoryBaseReference::MARKUP
                .events
                .iter()
                .map(|e| e.name)
                .collect::<Vec<_>>()
        );
        let called = Rc::new(Cell::new(0));
        let count = called.clone();
        let handler = MarkupDelegate::new(move |values| {
            let args = values[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<DockableClosingEventArgs>()
                .unwrap();
            assert!(args.cancel());
            count.set(count.get() + 1);
            None
        });
        (interface.find_event("DockableClosing").unwrap().add)(&[
            Some(Rc::new(factory.clone())),
            Some(Rc::new(handler)),
        ])
        .unwrap();
        assert!(!factory.on_dockable_closing(Some(document)));
        assert_eq!(called.get(), 1);
        let weak_owner = Rc::downgrade(consumer);
        let weak_factory = Rc::downgrade(&factory);
        consumer.release_references();
        assert_eq!(
            <dyn DockableClosingEventArgsMetadata as MarkupTyped>::MARKUP.namespace(),
            "FerroDock.Model.Core.Events"
        );
        drop(sender);
        drop(factory);
        drop(value);
        assert!(weak_owner.upgrade().is_none());
        assert!(weak_factory.upgrade().is_none());
    });
}
#[test]
fn both_compiler_hosts_use_public_original_event_argument_paths() {
    run(|| {
        crate::factory_event_consumer::register_types();
        let mut configuration = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::default();
        configuration.local_assembly = Some(&crate::factory_event_consumer::ASSEMBLY);
        let emitted = ferroui_markup_xaml_loader::rust_emitter::compile_documents(
            &[
                (
                    "Argument.xaml",
                    include_str!("FactoryEventArgumentRoot.xaml"),
                ),
                (
                    "Tracking.xaml",
                    include_str!("FactoryTrackingStateRoot.xaml"),
                ),
                ("Consumer.xaml", include_str!("FactoryEventConsumer.xaml")),
            ],
            Some("ferres://ferrodock_markup_tests/"),
            &configuration,
            &[],
        );
        let source = emitted
            .iter()
            .map(|doc| doc.source.as_ref().unwrap())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        assert!(source.contains("ferrodock_model::core::events::DockableClosingEventArgs"));
        assert!(source.contains("ferrodock_model::core::GlobalDockTrackingState"));
        assert!(
            MarkupType::find("FerroDock.Model.Core.Events", "DockableClosingEventArgs").is_some()
        );
    });
}

#[test]
fn compiled_tracking_reason_binding_uses_the_original_core_enum_and_source_event_argument() {
    run(|| {
        use ferrodock_model::core::{
            events::GlobalDockTrackingChangedEventArgs, DockTrackingChangeReason as Reason,
        };
        use ferroui_base::{threading::Dispatcher, Ref};
        use ferroui_controls::TextBlock;
        let value = load("FactoryTrackingEventBindings");
        let text = value.downcast_ref::<Ref<TextBlock>>().unwrap();
        for reason in [
            Reason::Unknown,
            Reason::ActiveDockableChanged,
            Reason::FocusedDockableChanged,
            Reason::WindowActivated,
            Reason::WindowDeactivated,
            Reason::DockableActivated,
            Reason::DockableDeactivated,
            Reason::WindowClosed,
            Reason::WindowRemoved,
            Reason::LayoutInitialized,
        ] {
            text.set_data_context(Some(Rc::new(GlobalDockTrackingChangedEventArgs::new(
                GlobalDockTrackingState::empty(),
                GlobalDockTrackingState::empty(),
                reason,
            ))));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(text.text().as_deref(), Some(format!("{reason:?}").as_str()));
        }
    });
}
