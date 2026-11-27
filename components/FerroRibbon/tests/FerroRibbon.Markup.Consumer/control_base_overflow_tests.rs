//! Independent subclass inherits default overflow and overrides the primary action.
use ferroribbon::compat::MenuFlyoutItem;
use ferroui_base::{
    Ref,
    data::core::{ValueType, ValueTypes},
};
use std::rc::Rc;

fn command_value(runtime: bool) -> ferroui_base::BoxedValue {
    if runtime {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("Command.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap()
    } else {
        crate::compiled_xaml::try_load(None, "ferres://ferroribbon_markup_consumer/Command.xaml")
            .unwrap()
            .unwrap()
    }
}

#[test]
fn downstream_base_key_tip_focus_matches_source_predicate_in_live_and_detached_trees() {
    use ferroribbon::controls::{IRibbonItem, RibbonKeyTipResult};
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::{Control, StackPanel, Window};

    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/control-base-focus.json")).unwrap();
        for runtime in [false, true] {
            for case in reference["cases"].as_array().unwrap() {
                let value = command_value(runtime);
                let source = value
                    .downcast_ref::<Ref<crate::command_consumer::CommandConsumer>>()
                    .unwrap();
                let item =
                    ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRibbonItem>>()).unwrap();
                let item = item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap();
                source.set_width(120.0);
                source.set_height(40.0);
                source.set_is_visible(case["visible"].as_bool().unwrap());
                source.set_is_enabled(case["enabled"].as_bool().unwrap());
                source.set_is_tab_stop(case["tabStop"].as_bool().unwrap());
                let parent = StackPanel::new();
                parent.set_is_visible(case["parentsVisible"].as_bool().unwrap());
                if case["active"].as_bool().unwrap() {
                    parent.children().add(source.clone());
                }
                let window = Window::new();
                window.set_content(Some(Control::boxed(&parent)));
                window.show();
                Dispatcher::ui_thread().run_jobs(None);
                let result = item.on_key_tip();
                let focused = window
                    .focus_manager()
                    .get_focused_element()
                    .is_some_and(|focused| focused.ptr_eq(source));
                window.close();
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(
                    focused,
                    case["canFocus"].as_bool().unwrap(),
                    "runtime={runtime}, {case}"
                );
                assert_eq!(
                    Rc::ptr_eq(&result, &RibbonKeyTipResult::close()),
                    case["close"]
                );
                assert_eq!(result.next_scope().is_none(), case["emptyScope"]);
                assert_eq!(source.invoke_calls(), 0);
            }
        }
    });
}

#[test]
fn downstream_base_focus_coercion_retains_style_local_and_binding_inputs() {
    use ferroui_base::data::BindingPriority;
    use ferroui_base::input::InputElement;
    use ferroui_base::reactive::{IObserver, LightweightSubject};

    crate::tests::run(|| {
        let source = crate::command_consumer::CommandConsumer::new();
        let property = InputElement::focusable_property();
        assert!(source.focusable());
        assert_eq!(source.get_local_value(property), None);
        let values = Rc::new(LightweightSubject::new());
        let binding = source.bind(property, values.clone(), BindingPriority::LocalValue);
        values.on_next(true);
        source.set_is_tab_stop(false);
        assert!(!source.focusable());
        values.on_next(false);
        source.set_is_tab_stop(true);
        assert!(!source.focusable());
        values.on_next(true);
        assert!(source.focusable());
        source.set_is_tab_stop(false);
        values.on_next(true);
        binding.dispose();
        assert!(!source.focusable());
        source.set_is_tab_stop(true);
        assert!(source.focusable());
        let style = source
            .set_value_with_priority(property, false, BindingPriority::Style)
            .unwrap();
        assert!(!source.focusable());
        source.set_is_tab_stop(false);
        source.set_is_tab_stop(true);
        assert!(!source.focusable());
        source.set_is_tab_stop(false);
        style.dispose();
        assert!(!source.focusable());
        source.set_is_tab_stop(true);
        assert!(source.focusable());
        source.set_focusable(true);
        source.set_is_tab_stop(false);
        assert!(!source.focusable());
        assert_eq!(source.get_local_value(property), Some(true));
        source.set_is_tab_stop(true);
        assert!(source.focusable());
        source.set_focusable(false);
        source.set_is_tab_stop(false);
        source.set_is_tab_stop(true);
        assert!(!source.focusable());
        source.set_is_tab_stop(false);
        source.clear_value(property);
        assert!(!source.focusable());
        source.set_is_tab_stop(true);
        assert!(source.focusable());
        assert_eq!(source.get_local_value(property), None);
    });
}

#[test]
fn downstream_base_default_key_tip_takes_keyboard_focus() {
    use ferroribbon::controls::{IRibbonItem, RibbonKeyTipResult};
    use ferroui_base::input::{InputElement, KeyModifiers, NavigationMethod};
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::{Button, Control, StackPanel, Window};
    use std::cell::Cell;

    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Command.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Command.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let source = value
                .downcast_ref::<Ref<crate::command_consumer::CommandConsumer>>()
                .unwrap();
            let item =
                ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRibbonItem>>()).unwrap();
            let item = item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap();
            source.set_width(120.0);
            source.set_height(40.0);
            let navigation = Rc::new(Cell::new(None));
            let observed = navigation.clone();
            source.got_focus(move |_, event| observed.set(Some(event.navigation_method)));
            let sentinel = Button::new();
            sentinel.set_width(120.0);
            sentinel.set_height(40.0);
            let parent = StackPanel::new();
            parent.children().add(sentinel.clone());
            parent.children().add(source.clone());
            let window = Window::new();
            window.set_content(Some(Control::boxed(&parent)));
            window.show();
            Dispatcher::ui_thread().run_jobs(None);
            assert!(source.is_loaded());
            assert!(source.is_tab_stop());
            assert!(sentinel.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE));
            navigation.set(None);
            let result = item.on_key_tip();
            assert!(Rc::ptr_eq(&result, &RibbonKeyTipResult::close()));
            let focused = window.focus_manager().get_focused_element();
            let method = navigation.get();
            window.close();
            Dispatcher::ui_thread().run_jobs(None);
            assert!(
                focused.is_some_and(|focused| focused.ptr_eq(&source.clone().upcast::<InputElement>())),
                "Default RibbonControlBase KeyTip must focus the downstream control (runtime={runtime})"
            );
            assert_eq!(method, Some(NavigationMethod::Tab));
        }
    });
}

#[test]
fn compiled_runtime_base_default_overflow_dispatches_downstream_override_and_live_bound_command() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Command.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Command.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let source = value
                .downcast_ref::<Ref<crate::command_consumer::CommandConsumer>>()
                .unwrap();
            source.set_icon(None);
            let context = source.data_context().unwrap();
            let vm = ValueTypes::try_cast(
                &context,
                ValueType::of::<Rc<crate::command_consumer::CommandViewModel>>(),
            )
            .unwrap();
            let vm = vm
                .downcast_ref::<Rc<crate::command_consumer::CommandViewModel>>()
                .unwrap();
            let sequence = source.create_overflow_menu_items_core();
            assert_eq!(sequence.iter().count(), 1);
            let entry = sequence
                .iter()
                .next()
                .unwrap()
                .unwrap()
                .cast::<MenuFlyoutItem>()
                .unwrap();
            assert_eq!(entry.text().as_deref(), Some("Paste"));
            assert_eq!(entry.keyboard_accelerator_text_override(), "Ctrl+V");
            assert_eq!(source.invoke_calls(), 0);
            assert_eq!(vm.executed(), 0);
            source.set_label(Some("Changed".into()));
            source.set_shortcut(Some("later".into()));
            assert_eq!(entry.text().as_deref(), Some("Paste"));
            entry.invoke_menu_core();
            assert_eq!(source.invoke_calls(), 1);
            assert_eq!(vm.executed(), 1);
            source.set_command_parameter(Some(Rc::new(-1_i32)));
            assert!(!source.is_enabled());
            assert!(entry.is_enabled());
            entry.invoke_menu_core();
            assert_eq!(source.invoke_calls(), 2);
            assert_eq!(vm.executed(), 1);
            source.set_command_parameter(Some(Rc::new(7_i32)));
            entry.invoke_menu_core();
            assert_eq!(source.invoke_calls(), 3);
            assert_eq!(vm.executed(), 2);
        }
    });
}
