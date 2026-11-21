use crate::context_menu_consumer::MenuEventsView;
use ferroribbon::controls::{Ribbon, RibbonCustomizePage};
use ferroui_base::{Point, TypeInfo, data::core::ValueTypes};
use std::{cell::Cell, rc::Rc};
#[test]
fn independent_compiled_runtime_menu_properties_enum_and_cancel_events_preserve_identity() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::context_menu_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[MenuEventsView::TYPE]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("MenuEvents.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/MenuEvents.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let view = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<MenuEventsView>()
                .unwrap();
            let root = view.children().get(0).cast::<Ribbon>().unwrap();
            assert!(root.is_context_menu_enabled());
            assert!(!root.can_customize());
            assert!(root.is_visibility_menu_enabled());
            let source = root.find_item(Some("save".into())).unwrap();
            let seen = Rc::new(Cell::new(false));
            let observed = seen.clone();
            let weak = view.downgrade();
            let subscription = root.context_menu_opening(move |args| {
                let recorded = weak.upgrade().unwrap().context().unwrap();
                assert!(std::ptr::eq(args, &*recorded));
                assert!(args.handled.get());
                observed.set(true);
            });
            assert!(
                root.show_item_context_menu(source.clone(), Point::new(3.25, 4.75))
                    .unwrap()
            );
            assert!(seen.get());
            assert_eq!(view.context_calls(), 1);
            assert!(view.context().unwrap().element.ptr_eq(&source));
            assert!(
                root.show_customize_dialog(RibbonCustomizePage::QuickAccessToolbar)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(view.customize_calls(), 1);
            assert_eq!(
                view.customization().unwrap().page,
                RibbonCustomizePage::QuickAccessToolbar
            );
            assert!(view.customization().unwrap().handled.get());
            let tag = view
                .children()
                .get(1)
                .cast::<ferroui_controls::TextBlock>()
                .unwrap()
                .tag()
                .unwrap();
            assert_eq!(
                *tag.downcast_ref::<RibbonCustomizePage>().unwrap(),
                RibbonCustomizePage::QuickAccessToolbar
            );
            subscription.dispose();
            root.set_is_context_menu_enabled(false);
            assert!(
                !root
                    .show_item_context_menu(source, Point::default())
                    .unwrap()
            );
            assert_eq!(view.context_calls(), 1);
        }
    });
}

#[test]
fn independent_menu_events_use_native_weak_receiver_lifetime_in_both_loaders() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::context_menu_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[MenuEventsView::TYPE]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("MenuEvents.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/MenuEvents.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let view = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<MenuEventsView>()
                .unwrap();
            let root = view.children().get(0).cast::<Ribbon>().unwrap();
            let source = root.find_item(Some("save".into())).unwrap();
            root.show_item_context_menu(source.clone(), Point::default())
                .unwrap();
            assert_eq!(view.context_calls(), 1);
            assert!(
                root.show_customize_dialog(RibbonCustomizePage::Ribbon)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(view.customize_calls(), 1);
            let weak_view = view.downgrade();
            drop(value);
            drop(view);
            assert!(
                weak_view.upgrade().is_none(),
                "a retained child ribbon must not keep the XAML receiver alive ({runtime})"
            );
            let calls = Rc::new(Cell::new(0));
            let observed = calls.clone();
            let token = root.context_menu_opening(move |args| {
                assert!(
                    !args.handled.get(),
                    "expired named receiver must not be invoked"
                );
                args.handled.set(true);
                observed.set(observed.get() + 1);
            });
            root.show_item_context_menu(source.clone(), Point::default())
                .unwrap();
            assert_eq!(calls.get(), 1);
            token.dispose();
            let weak_root = root.downgrade();
            drop(root);
            assert!(
                weak_root.upgrade().is_none(),
                "a retained command presentation must not keep its ribbon owner alive ({runtime})"
            );
            drop(source);
        }
    });
}

#[test]
fn independent_compiled_runtime_dialog_outcome_enum_is_reusable() {
    crate::tests::run_standalone(|| {
        use ferroribbon::controls::RibbonDialogOutcome;
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("DialogOutcomes.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/DialogOutcomes.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let panel = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ferroui_controls::StackPanel>()
                .unwrap();
            for (index, expected) in [
                RibbonDialogOutcome::Dismissed,
                RibbonDialogOutcome::Primary,
                RibbonDialogOutcome::Secondary,
            ]
            .into_iter()
            .enumerate()
            {
                let tag = panel
                    .children()
                    .get(index)
                    .cast::<ferroui_controls::TextBlock>()
                    .unwrap()
                    .tag()
                    .unwrap();
                assert_eq!(
                    *tag.downcast_ref::<RibbonDialogOutcome>().unwrap(),
                    expected
                );
            }
        }
    });
}

#[test]
fn independent_compiled_runtime_native_dialog_buttons_cancel_with_typed_args() {
    crate::tests::run_standalone(|| {
        use ferroribbon::controls::RibbonColorDialogHost;
        use ferroui_base::interactivity::RoutedEventArgs;
        use ferroui_controls::{Button, Control, Window};
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::context_menu_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[MenuEventsView::TYPE]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("DialogButtons.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/DialogButtons.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let view = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<MenuEventsView>()
                .unwrap();
            let host = view
                .children()
                .get(0)
                .cast::<RibbonColorDialogHost>()
                .unwrap();
            let window = Window::new();
            window.set_content(Some(Control::boxed(view.clone())));
            window.show();
            window.update_layout();
            let context = view.data_context().unwrap();
            let value = ValueTypes::try_convert(
                Some(&context),
                ferroui_base::data::core::ValueType::of::<
                    Rc<crate::command_consumer::CommandViewModel>,
                >(),
            )
            .flatten()
            .unwrap();
            let vm = value
                .downcast_ref::<Rc<crate::command_consumer::CommandViewModel>>()
                .unwrap()
                .clone();
            assert!(host.primary_button_command().is_some());
            assert!(host.secondary_button_command().is_some());
            assert!(host.close_button_command().is_some());
            let cancelled = Rc::new(Cell::new(0));
            let mut subscriptions = Vec::new();
            for (button, part, parameter) in [
                (
                    ferroribbon::compat::ContentDialogButton::Primary,
                    host.primary_button().unwrap(),
                    host.primary_button_command_parameter(),
                ),
                (
                    ferroribbon::compat::ContentDialogButton::Secondary,
                    host.secondary_button().unwrap(),
                    host.secondary_button_command_parameter(),
                ),
                (
                    ferroribbon::compat::ContentDialogButton::Close,
                    host.close_button().unwrap(),
                    host.close_button_command_parameter(),
                ),
            ] {
                assert!(parameter.is_some());
                let seen = cancelled.clone();
                subscriptions.push(host.button_click(button, move |args| {
                    assert!(args.cancel());
                    seen.set(seen.get() + 1);
                }));
                part.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            }
            assert_eq!(view.dialog_calls(), 3);
            assert_eq!(cancelled.get(), 3);
            assert_eq!(
                vm.executed(),
                0,
                "cancelled source events prevent command invocation"
            );
            view.set_dialog_cancel(false);
            for token in &subscriptions {
                token.dispose();
            }
            for part in [
                host.primary_button().unwrap(),
                host.secondary_button().unwrap(),
                host.close_button().unwrap(),
            ] {
                part.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            }
            assert_eq!(vm.executed(), 3);
            assert_eq!(view.dialog_calls(), 6);
            for token in subscriptions {
                token.dispose();
            }
            window.close();
        }
    });
}
