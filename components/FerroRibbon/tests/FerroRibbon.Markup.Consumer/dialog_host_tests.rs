use ferroui_base::{Rect, Size, TypeInfo, data::core::ValueTypes};
use ferroui_controls::{Control, TextBox, Window};
fn load_lazy_state_host(
    runtime: bool,
) -> ferroui_base::Ref<crate::dialog_consumer::ConsumerDialogHost> {
    TypeInfo::register_namespaces(&[(
        "ferroribbon_markup_consumer::dialog_consumer",
        "FerroRibbon.Consumer",
    )]);
    TypeInfo::register_all(&[crate::dialog_consumer::ConsumerDialogHost::TYPE]);
    let value = if runtime {
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("LazyDialogStates.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap()
    } else {
        crate::compiled_xaml::try_load(
            None,
            "ferres://ferroribbon_markup_consumer/LazyDialogStates.xaml",
        )
        .unwrap()
        .unwrap()
    };
    ValueTypes::as_object(&*value).unwrap().cast().unwrap()
}
#[test]
fn independent_compiled_runtime_lazy_static_state_capture_matches_source_corpus() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        use ferroribbon::compat::ContentDialogButton;
        use ferroui_base::styling::ControlTheme;
        use std::rc::Rc;
        for runtime in [false, true] {
            let host = load_lazy_state_host(runtime);
            let window = Window::new();
            window.set_content(Some(Control::boxed(host.clone())));
            window.show();
            window.update_layout();
            let accent = host
                .resources()
                .try_get_resource(&"AccentButtonStyle".into(), None)
                .flatten()
                .unwrap();
            let accent = ValueTypes::as_object(&*accent)
                .unwrap()
                .cast::<ControlTheme>()
                .unwrap();
            let mut themes = std::collections::HashMap::from([
                ("A", accent),
                ("base", host.primary_button_style().unwrap()),
            ]);
            for id in ["B", "C", "new_base"] {
                let theme = ControlTheme::new();
                theme.set_target_type(Some(ferroui_controls::Button::TYPE));
                themes.insert(id, theme);
            }
            let template = host.template().unwrap();
            let data: serde_json::Value =
                serde_json::from_str(include_str!("../fixtures/dialog-state-resources.json"))
                    .unwrap();
            assert_eq!(data["corpus"].as_array().unwrap().len(), 10);
            for case in data["corpus"].as_array().unwrap() {
                match case["name"].as_str().unwrap() {
                    "primary_after_replace" | "secondary_keeps_B" => {
                        let id = if case["name"] == "primary_after_replace" {
                            "B"
                        } else {
                            "C"
                        };
                        host.resources()
                            .set("AccentButtonStyle", Some(Rc::new(themes[id].clone())));
                    }
                    "none_restores_base" => {
                        host.set_primary_button_style(Some(themes["new_base"].clone()));
                        host.set_secondary_button_style(Some(themes["new_base"].clone()));
                        host.set_close_button_style(Some(themes["new_base"].clone()));
                    }
                    "retemplate_primary_C" => {
                        host.set_template(None);
                        host.apply_template();
                        host.set_template(Some(template.clone()));
                        host.apply_template();
                    }
                    _ => {}
                }
                let choice = ContentDialogButton::from_raw(case["choice"].as_i64().unwrap() as i32);
                host.set_default_button(choice);
                window.update_layout();
                let selected = match choice {
                    ContentDialogButton::Secondary => host.secondary_button(),
                    ContentDialogButton::Close => host.close_button(),
                    _ => host.primary_button(),
                }
                .unwrap();
                assert_eq!(
                    selected.theme(),
                    Some(themes[case["effective"].as_str().unwrap()].clone()),
                    "runtime={runtime}: {case}"
                );
            }
            window.close();
            window.set_content(None);
        }
    });
}
#[test]
fn independent_compiled_runtime_focus_callback_can_remove_subclass_template() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        use ferroui_base::input::InputElement;
        use std::{cell::Cell, rc::Rc};
        for runtime in [false, true] {
            let host = load_lazy_state_host(runtime);
            let window = Window::new();
            window.set_content(Some(Control::boxed(host.clone())));
            window.show();
            window.update_layout();
            host.set_default_button(ferroribbon::compat::ContentDialogButton::Primary);
            assert!(host.secondary_button().unwrap().focus());
            let called = Rc::new(Cell::new(false));
            let seen = called.clone();
            let weak = host.downgrade();
            host.primary_button().unwrap().add_handler(
                InputElement::got_focus_event(),
                move |_, _| {
                    seen.set(true);
                    weak.upgrade().unwrap().set_template(None);
                },
            );
            host.focus_dialog();
            assert!(called.get());
            assert!(host.primary_button().is_none());
            window.close();
            window.set_content(None);
        }
    });
}
#[test]
fn independent_compiled_runtime_subclass_content_properties_resources_and_retemplate() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::dialog_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[crate::dialog_consumer::ConsumerDialogHost::TYPE]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ColorDialogHost.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ColorDialogHost.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<crate::dialog_consumer::ConsumerDialogHost>()
                .unwrap();
            assert_eq!(
                host.title().unwrap().downcast_ref::<String>().unwrap(),
                "Consumer dialog"
            );
            assert_eq!(host.primary_button_text().as_deref(), Some("Apply"));
            let editor = Control::from_boxed(&host.content().unwrap())
                .unwrap()
                .cast::<TextBox>()
                .unwrap();
            editor.set_text(Some("edited from consumer"));
            let window = Window::new();
            window.set_content(Some(Control::boxed(host.clone())));
            window.show();
            window.measure(Size::new(700., 700.));
            window.arrange(Rect::new(0., 0., 700., 700.));
            window.update_layout();
            assert!(host.focus_dialog());
            assert!(editor.is_focused());
            assert!(!host.secondary_button().unwrap().is_enabled());
            assert_eq!(host.primary_button().unwrap().min_height(), 41.5);
            assert!(
                host.primary_button()
                    .unwrap()
                    .theme()
                    .unwrap()
                    .ptr_eq(&host.primary_button_style().unwrap())
            );
            use ferroribbon::compat::ContentDialogButton;
            let primary = host.primary_button().unwrap();
            host.set_default_button(ContentDialogButton::Primary);
            assert_ne!(primary.theme(), host.primary_button_style());
            let custom = ferroui_base::styling::ControlTheme::new();
            custom.set_target_type(Some(ferroui_controls::Button::TYPE));
            host.set_primary_button_style(Some(custom.clone()));
            assert_ne!(primary.theme(), Some(custom.clone()));
            host.set_default_button(ContentDialogButton::None);
            assert_eq!(primary.theme(), Some(custom));
            let old = host.content_presenter().unwrap();
            let resource = host
                .resources()
                .try_get_resource(&"Replacement".into(), None)
                .flatten()
                .unwrap();
            let template = ValueTypes::try_cast(
                &resource,
                ferroui_base::data::core::ValueType::of::<
                    std::rc::Rc<dyn ferroui_controls::templates::IControlTemplate>,
                >(),
            )
            .unwrap()
            .downcast_ref::<std::rc::Rc<dyn ferroui_controls::templates::IControlTemplate>>()
            .unwrap()
            .clone();
            host.set_template(Some(template));
            host.apply_template();
            window.update_layout();
            assert!(old.content().is_none());
            assert!(
                Control::from_boxed(&host.content_presenter().unwrap().content().unwrap())
                    .unwrap()
                    .ptr_eq(&editor)
            );
            assert_eq!(editor.text().as_deref(), Some("edited from consumer"));
            host.set_primary_button_text(Some("Changed".into()));
            assert_eq!(
                host.primary_button()
                    .unwrap()
                    .content()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap(),
                "Changed"
            );
            let columns = host.command_space().unwrap().column_definitions();
            assert_eq!(
                columns.get(1).width(),
                ferroui_controls::GridLength::new(7.25, ferroui_controls::GridUnitType::Pixel)
            );
            assert_eq!(
                columns.get(3).width(),
                ferroui_controls::GridLength::new(8., ferroui_controls::GridUnitType::Pixel),
                "state targets named FirstSpacer, not its default position"
            );
            host.resources().set(
                "ContentDialogButtonSpacing",
                Some(std::rc::Rc::new(ferroui_controls::GridLength::new(
                    12.25,
                    ferroui_controls::GridUnitType::Pixel,
                ))),
            );
            assert_eq!(
                columns.get(3).width(),
                ferroui_controls::GridLength::new(12.25, ferroui_controls::GridUnitType::Pixel)
            );
            assert_eq!(
                columns.get(1).width(),
                ferroui_controls::GridLength::new(7.25, ferroui_controls::GridUnitType::Pixel)
            );
            host.set_is_secondary_button_enabled(true);
            assert!(host.secondary_button().unwrap().is_enabled());
            window.close();
            window.set_content(None);
        }
    });
}

#[test]
fn independent_compiled_runtime_default_button_named_static_unknown_numeric_and_nullable_metadata()
{
    crate::tests::run_standalone(|| {
        use ferroribbon::compat::ContentDialogButton;
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::dialog_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[crate::dialog_consumer::ConsumerDialogHost::TYPE]);
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("DefaultButtons.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/DefaultButtons.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let stack = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ferroui_controls::StackPanel>()
                .unwrap();
            assert_eq!(stack.children().count(), 6);
            for (i, (value, optional)) in [
                (0, None),
                (1, Some(3)),
                (2, Some(0)),
                (3, Some(99)),
                (99, Some(i32::MIN)),
                (i32::MIN, Some(i32::MAX)),
            ]
            .into_iter()
            .enumerate()
            {
                let host = stack
                    .children()
                    .get(i)
                    .cast::<crate::dialog_consumer::ConsumerDialogHost>()
                    .unwrap();
                assert_eq!(host.default_button(), ContentDialogButton::from_raw(value));
                assert_eq!(
                    host.optional_default_button(),
                    optional.map(ContentDialogButton::from_raw)
                );
                host.set_default_button(ContentDialogButton::from_raw(i32::MAX));
                assert_eq!(host.default_button().raw(), i32::MAX);
                host.set_optional_default_button(None);
                assert_eq!(host.optional_default_button(), None);
            }
        }
    });
}

#[test]
fn independent_runtime_unknown_default_button_and_nullable_integer_boundaries() {
    crate::tests::run_standalone(|| {
        use ferroribbon::compat::ContentDialogButton;
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::dialog_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[crate::dialog_consumer::ConsumerDialogHost::TYPE]);
        let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("../fixtures/markup/UnknownDialogButtons.xaml"),
            Some(&crate::CONSUMER_ASSEMBLY),
        )
        .unwrap();
        let stack = ValueTypes::as_object(&*value)
            .unwrap()
            .cast::<ferroui_controls::StackPanel>()
            .unwrap();
        assert_eq!(stack.children().count(), 6);
        for (i, (value, optional)) in [
            (0, None),
            (1, Some(3)),
            (2, Some(0)),
            (3, Some(99)),
            (99, Some(i32::MIN)),
            (i32::MIN, Some(i32::MAX)),
        ]
        .into_iter()
        .enumerate()
        {
            let host = stack
                .children()
                .get(i)
                .cast::<crate::dialog_consumer::ConsumerDialogHost>()
                .unwrap();
            assert_eq!(host.default_button(), ContentDialogButton::from_raw(value));
            assert_eq!(
                host.optional_default_button(),
                optional.map(ContentDialogButton::from_raw)
            );
        }
        // The same original fixture now also compiles through DefaultButtons.xaml;
    });
}
