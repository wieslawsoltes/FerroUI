//! Existing native Click metadata, compiled bindings and base-class subscriptions.
use crate::{
    button_consumer::ClickDeliveryConsumer,
    tests::{NativeThemeScope, run},
};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    interactivity::Interactive,
};
use ferroui_controls::{Button, Control, Window, automation::peers::ButtonAutomationPeer};
use std::{cell::Cell, rc::Rc};

#[test]
fn independent_command_enabled_property_compiled_runtime_and_downstream_inheritance() {
    run(|| {
        let _theme = NativeThemeScope::new();
        let compiled = crate::compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/ClickDelivery.xaml",
        )
        .unwrap()
        .unwrap();
        let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
            include_str!("ClickDelivery.xaml"),
            None,
            None,
            None,
            false,
        )
        .unwrap();
        for value in [compiled, runtime] {
            let root = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ClickDeliveryConsumer>()
                .unwrap();
            let window = Window::new();
            window.set_content(Some(Control::boxed(root.clone())));
            window.show();
            for index in 0..5 {
                let button = root.children().get(index).cast::<Button>().unwrap();
                let command = button.command().unwrap();
                assert!(button.is_enabled());
                button.set_command_parameter(Some(Rc::new(-1_i32)));
                assert!(
                    !button.is_enabled(),
                    "command denial must reach the public property"
                );
                assert!(!button.is_effectively_enabled());
                button.set_is_enabled(true);
                assert!(
                    !button.is_enabled(),
                    "a local request cannot override command denial"
                );
                button.set_command_parameter(Some(Rc::new(7_i32)));
                assert!(button.is_enabled());
                assert!(button.is_effectively_enabled());
                assert!(Rc::ptr_eq(&command, &button.command().unwrap()));
            }
            root.set_is_enabled(false);
            for child in root.children().snapshot().iter() {
                assert!(!child.cast::<Button>().unwrap().is_enabled());
            }
            root.set_is_enabled(true);
            for child in root.children().snapshot().iter() {
                assert!(child.cast::<Button>().unwrap().is_enabled());
            }
            window.close();
        }
    });
}

#[test]
fn independent_click_delivery_compiled_runtime_xaml_and_downstream_inheritance() {
    run(|| {
        let _theme = NativeThemeScope::new();
        let compiled = crate::compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/ClickDelivery.xaml",
        )
        .unwrap()
        .unwrap();
        let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load(
            include_str!("ClickDelivery.xaml"),
            None,
            None,
            None,
            false,
        )
        .unwrap();
        for (phase, value) in [("compiled", compiled), ("runtime", runtime)] {
            let root = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ClickDeliveryConsumer>()
                .unwrap();
            let vm = ValueTypes::try_convert(
                root.data_context().as_ref(),
                ValueType::of::<Rc<crate::command_consumer::CommandViewModel>>(),
            )
            .flatten()
            .unwrap();
            let vm = vm
                .downcast_ref::<Rc<crate::command_consumer::CommandViewModel>>()
                .unwrap();
            let bubbled = Rc::new(Cell::new(0));
            let b = bubbled.clone();
            root.add_handler_with(
                Button::click_event(),
                move |_, _| b.set(b.get() + 1),
                Interactive::DEFAULT_ROUTES,
                true,
            );
            let window = Window::new();
            window.set_content(Some(Control::boxed(root.clone())));
            window.show();
            assert_eq!(root.children().count(), 5);
            for index in 0..root.children().count() {
                let button = root.children().get(index).cast::<Button>().unwrap();
                let calls = Rc::new(Cell::new(0));
                let c = calls.clone();
                let handled = Rc::new(Cell::new(None));
                let h = handled.clone();
                let token = button.click(move |sender, e| {
                    h.set(Some(e.handled()));
                    assert!(e.is_source(sender));
                    c.set(c.get() + 1);
                });
                ButtonAutomationPeer::new(&button).invoke().unwrap();
                assert_eq!(
                    handled.get(),
                    Some(true),
                    "{phase} item {index}: markup_calls={}, last_sender={}",
                    root.clicks(),
                    root.last_sender().is_some()
                );
                assert_eq!(
                    calls.get(),
                    1,
                    "base Button subscription after handled markup callback"
                );
                assert_eq!(root.clicks(), index + 1);
                assert!(root.last_sender().unwrap().ptr_eq(&button));
                assert_eq!(vm.executed(), (index + 1) as i32);
                assert!(button.remove_handler(Button::click_event(), token));
            }
            assert_eq!(
                bubbled.get(),
                0,
                "handled-events-too ancestor must not receive local Click"
            );
            window.close();
        }
    });
}

#[test]
fn independent_click_delivery_model_factory_linked_copies_and_checked_feedback() {
    run(|| {
        use ferroribbon::{
            controls::IRibbonItem,
            mvvm::{RibbonElementFactory, RibbonItemModelReference, RibbonModelBindings},
        };
        use ferroribbon_core::model::*;
        let _theme = NativeThemeScope::new();
        let factory = RibbonElementFactory::new();
        let push = RibbonButtonModel::new(Some("push".into()), Some("Push".into()));
        let toggle = RibbonToggleButtonModel::new(Some("toggle".into()), Some("Toggle".into()));
        let check = RibbonCheckBoxModel::new(Some("check".into()), Some("Check".into()));
        let models: [RibbonItemModelReference; 3] = [push, toggle.clone(), check.clone()];
        for (index, model) in models.into_iter().enumerate() {
            let commands = Rc::new(Cell::new(0));
            let c = commands.clone();
            model.set_command(Some(ferroribbon_core::commands::RibbonRelayCommand::new(
                move || c.set(c.get() + 1),
            )));
            let scope = RibbonModelBindings::new();
            let source = factory
                .create_item(Some(model.clone()), scope.clone())
                .unwrap()
                .cast::<Button>()
                .unwrap();
            let item = ValueTypes::try_cast(
                &Control::boxed(source.clone()),
                ValueType::of::<Rc<dyn IRibbonItem>>(),
            )
            .unwrap();
            let item = item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap();
            let copy = item.create_linked_copy().unwrap().cast::<Button>().unwrap();
            let parent = ferroui_controls::StackPanel::new();
            parent.children().add(source.clone().upcast::<Control>());
            parent.children().add(copy.clone().upcast::<Control>());
            let window = Window::new();
            window.set_content(Some(Control::boxed(parent)));
            window.show();
            for button in [&source, &copy] {
                let calls = Rc::new(Cell::new(0));
                let c = calls.clone();
                button.click(|_, e| e.set_handled(true));
                button.click(move |_, e| {
                    assert!(e.handled());
                    c.set(c.get() + 1);
                });
                ButtonAutomationPeer::new(button).invoke().unwrap();
                assert_eq!(calls.get(), 1);
            }
            assert_eq!(
                commands.get(),
                2,
                "one command per source/copy presentation invocation"
            );
            if index == 1 {
                assert!(!toggle.is_checked());
            }
            if index == 2 {
                assert_eq!(check.is_checked(), Some(false));
            }
            scope.dispose();
            assert!(scope.is_disposed());
            window.close();
        }
    });
}
