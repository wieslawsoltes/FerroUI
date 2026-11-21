use super::*;
use ferroui_base::controls::{
    ResourceDictionary, ResourceKey, ResourceProviderImpl, ResourceProviderImplExt, ResourceValue,
};
use ferroui_base::{Rect, Size};
use ferroui_controls::{TextBox, Window};
#[repr(C)]
struct RetemplatingResources {
    base: ResourceDictionary,
    on_lookup: RefCell<Option<Rc<dyn Fn()>>>,
}
ferro_class!(RetemplatingResources:ResourceDictionary);
impl FerroObjectImpl for RetemplatingResources {}
impl ResourceProviderImpl for RetemplatingResources {
    fn try_get_resource(
        this: &Self,
        key: &ResourceKey,
        theme: Option<&ferroui_base::styling::ThemeVariant>,
    ) -> Option<ResourceValue> {
        let result = Self::parent_try_get_resource(this, key, theme);
        if key.as_str() == Some("AccentButtonStyle") {
            let callback = this.on_lookup.take();
            if let Some(callback) = callback {
                callback();
            }
        }
        result
    }
}
fn run(test: impl FnOnce() + Send + 'static) {
    use ferroui_headless::FerroHeadlessPlatformExtensions;
    ferroui_headless::HeadlessUnitTestSession::start_new(|| {
        ferroui_controls::AppBuilder::configure::<Application>()
            .use_headless(ferroui_headless::FerroHeadlessPlatformOptions::default())
    })
    .dispatch(
        move || {
            crate::register_types();
            Application::current()
                .unwrap()
                .styles()
                .add(ferroui_themes_simple::SimpleTheme::new().as_style());
            test();
        },
        ferroui_base::threading::CancellationToken::none(),
    )
    .wait()
    .unwrap();
}

#[test]
fn color_dialog_exposes_the_title_and_honors_an_explicit_accessible_name() {
    run(|| {
        let owner = Window::new();
        owner.show();
        let host = RibbonColorDialogHost::new();
        host.set_title(Some(Rc::new("More Colors".to_owned())));
        host.set_primary_button_text(Some("OK".into()));
        AutomationProperties::set_automation_id(&host, Some("MoreColorsDialog"));
        let _result = host.show_dialog(owner.clone().upcast()).unwrap();

        let peer = ControlAutomationPeer::create_peer_for_element(&host);
        assert!(peer.is_dialog() && peer.is_control_element() && peer.is_content_element());
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Window
        );
        assert_eq!(
            peer.get_automation_id().as_deref(),
            Some("MoreColorsDialog")
        );
        assert_eq!(peer.get_name(), "More Colors");

        host.set_title(Some(Rc::new("Custom color".to_owned())));
        assert_eq!(peer.get_name(), "Custom color");
        AutomationProperties::set_name(&host, Some("Edit custom color"));
        assert_eq!(peer.get_name(), "Edit custom color");

        AutomationProperties::set_name(&host, None);
        host.set_title(None);
        let description = TextBlock::new();
        description.set_text(Some("Select a color".into()));
        host.set_content(Some(Control::boxed(description)));
        assert_eq!(peer.get_name(), "Select a color");
        assert!(host.hide());
        owner.close();
    });
}

#[test]
fn automation_close_retires_deferrals_before_the_dialog_reopens() {
    use crate::controls::RibbonDialogOutcome;

    run(|| {
        let owner = Window::new();
        owner.show();
        let root = owner.clone().upcast::<TopLevel>();
        let host = RibbonColorDialogHost::new();
        let _first = host.show_dialog(root.clone()).unwrap();
        let peer = ControlAutomationPeer::create_peer_for_element(&host);
        let provider = peer.get_provider::<dyn IWindowProvider>().unwrap();
        assert_eq!(
            provider.interaction_state(),
            Ok(WindowInteractionState::Running)
        );

        let pending_button = Rc::new(RefCell::new(None));
        let button = pending_button.clone();
        let _button = host.button_click(ContentDialogButton::Primary, move |args| {
            button.replace(Some(args.get_deferral()));
        });
        let pending_close = Rc::new(RefCell::new(None));
        let close = pending_close.clone();
        let _closing = host.dialog_closing(move |args| {
            close.replace(Some(args.get_deferral()));
        });
        let outcomes = Rc::new(RefCell::new(Vec::new()));
        let seen = outcomes.clone();
        let _closed = host.dialog_closed(move |args| seen.borrow_mut().push(args.result()));

        host.process_button(ContentDialogButton::Primary);
        let old_button = pending_button.take().unwrap();
        assert!(!host.hide());
        let old_close = pending_close.take().unwrap();

        host.set_is_enabled(false);
        assert_eq!(
            provider.close(),
            Err(AutomationError::element_not_enabled())
        );
        assert!(host.is_dialog_showing());
        host.set_is_enabled(true);
        provider.close().unwrap();
        assert!(!host.is_dialog_showing());
        assert!(pending_close.borrow().is_none());
        assert_eq!(*outcomes.borrow(), [RibbonDialogOutcome::Dismissed]);
        assert!(owner.is_visible());

        let _second = host.show_dialog(root).unwrap();
        assert!(!host.hide());
        let next_close = pending_close.take().unwrap();
        old_button.complete();
        old_close.complete().unwrap();
        assert!(pending_close.borrow().is_none());
        assert!(host.is_dialog_showing() && host.is_dialog_closing());
        assert_eq!(*outcomes.borrow(), [RibbonDialogOutcome::Dismissed]);
        next_close.complete().unwrap();
        assert!(!host.is_dialog_showing());
        assert_eq!(*outcomes.borrow(), [RibbonDialogOutcome::Dismissed; 2]);
        owner.close();
    });
}

#[test]
fn overlapping_input_pane_moves_dialog_enables_scroll_and_restores_template_values() {
    use ferroui_base::{Vector, threading::Dispatcher};
    use ferroui_controls::{
        Border, Control, WindowResizeReason,
        platform::{
            IInputPane, IWindowImpl, InputPaneBase, InputPaneState, InputPaneStateEventArgs,
        },
        testing::{CompositorTestServices, MockWindowImpl},
    };

    run(|| {
        let platform = MockWindowImpl::window(800., 600.);
        platform.setup_compositor(Some(CompositorTestServices::create_dummy_compositor(None)));
        let pane = Rc::new(InputPaneBase::new());
        platform.setup_feature::<dyn IInputPane>(pane.clone());
        let owner = Window::with_impl(platform.clone());
        owner.show();
        owner.update_layout();

        let host = RibbonColorDialogHost::new();
        let editor = Border::new();
        editor.set_height(440.);
        host.set_title(Some(Rc::new("More Colors".to_owned())));
        host.set_primary_button_text(Some("OK".into()));
        host.set_close_button_text(Some("Cancel".into()));
        host.set_content(Some(Control::boxed(editor)));
        let _future = host.show_dialog(owner.clone().upcast()).unwrap();
        let parts = host.parts.borrow().input_pane.clone().unwrap();
        let normal_margin = Thickness::uniform(4.);
        parts.background.set_margin(normal_margin);
        owner.update_layout();

        let root_bounds = |control: &Layoutable| {
            Rect::from_size(control.bounds().size())
                .transform_to_aabb(control.transform_to_visual(&owner).unwrap())
        };
        let normal_bottom = root_bounds(&parts.background).bottom();
        let notify = |rect: Rect| {
            let old = pane.occluded_rect();
            pane.set_occluded_rect(rect);
            let state = if rect.height > 0. {
                InputPaneState::Open
            } else {
                InputPaneState::Closed
            };
            pane.set_state(state);
            pane.on_state_changed(InputPaneStateEventArgs::new(state, Some(old), rect));
            Dispatcher::ui_thread().run_jobs(None);
            owner.update_layout();
        };
        let ordinary = || {
            assert_eq!(parts.background.margin(), normal_margin);
            assert_eq!(
                parts.background.vertical_alignment(),
                VerticalAlignment::Center
            );
            assert_eq!(
                parts.content_scroll.vertical_scroll_bar_visibility(),
                ScrollBarVisibility::Disabled
            );
        };
        ordinary();
        notify(Rect::new(0., normal_bottom + 12., 800., 30.));
        ordinary();

        notify(Rect::new(0., 320., 800., 280.));
        // The original storyboard reserves 600 - 320 + 12 DIPs.
        assert_eq!(parts.background.margin(), Thickness::new(0., 0., 0., 292.));
        assert_eq!(
            parts.background.vertical_alignment(),
            VerticalAlignment::Bottom
        );
        assert_eq!(
            parts.content_scroll.vertical_scroll_bar_visibility(),
            ScrollBarVisibility::Auto
        );
        let commands = host.parts.borrow().commands.clone().unwrap();
        assert!(root_bounds(&commands).bottom() <= 308.);
        assert!(parts.content_scroll.extent().height > parts.content_scroll.viewport().height);
        parts.content_scroll.set_offset(Vector::new(0., 70.));
        owner.update_layout();
        assert_eq!(parts.content_scroll.offset().y, 70.);

        notify(Rect::new(0., 140., 800., 460.));
        assert_eq!(parts.content_scroll.offset().y, 70.);
        // A taller pane still leaves the source's 184-DIP dialog minimum.
        assert_eq!(parts.background.margin(), Thickness::new(0., 0., 0., 428.));

        platform.resize(Size::new(800., 650.), WindowResizeReason::Unspecified);
        Dispatcher::ui_thread().run_jobs(None);
        owner.update_layout();
        assert_eq!(parts.background.margin(), Thickness::new(0., 0., 0., 478.));

        notify(Rect::default());
        ordinary();
        notify(Rect::new(0., 320., 800., 330.));
        assert_eq!(
            parts.content_scroll.vertical_scroll_bar_visibility(),
            ScrollBarVisibility::Auto
        );
        assert!(host.hide());
        Dispatcher::ui_thread().run_jobs(None);
        ordinary();

        let _second = host.show_dialog(owner.clone().upcast()).unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        let reopened = host.parts.borrow().input_pane.clone().unwrap();
        assert_eq!(
            reopened.content_scroll.vertical_scroll_bar_visibility(),
            ScrollBarVisibility::Auto
        );

        let stock_template = host.template();
        assert!(stock_template.is_some());
        host.set_template(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            reopened.content_scroll.vertical_scroll_bar_visibility(),
            ScrollBarVisibility::Disabled
        );
        host.set_template(stock_template);
        Dispatcher::ui_thread().run_jobs(None);
        let replacement = host.parts.borrow().input_pane.clone().unwrap();
        assert_eq!(
            replacement.content_scroll.vertical_scroll_bar_visibility(),
            ScrollBarVisibility::Auto
        );
        assert!(host.hide());
        owner.close();
    });
}

#[test]
fn compiled_dialog_template_has_original_parts_and_retains_native_content() {
    run(|| {
        let host = RibbonColorDialogHost::new();
        let named = Rc::new(RefCell::new(false));
        let seen = named.clone();
        host.template_applied(move |_, e| {
            let scope = e.name_scope();
            let mapping: serde_json::Value = serde_json::from_str(include_str!(
                "../../tests/fixtures/color-dialog-theme-mapping.json"
            ))
            .unwrap();
            for name in mapping["parts"].as_array().unwrap() {
                assert!(
                    scope.find(name.as_str().unwrap()).is_some(),
                    "missing original part {name}"
                );
            }
            let commands = scope.find("CommandSpace").unwrap().cast::<Grid>().unwrap();
            for (index, name) in [
                "PrimaryColumn",
                "FirstSpacer",
                "SecondaryColumn",
                "SecondSpacer",
                "CloseColumn",
            ]
            .iter()
            .enumerate()
            {
                assert!(
                    scope
                        .find(name)
                        .unwrap()
                        .ptr_eq(&commands.column_definitions().get(index))
                );
            }
            let scale = scope
                .find("ScaleTransform")
                .unwrap()
                .cast::<ferroui_base::media::ScaleTransform>()
                .unwrap();
            let surface = scope
                .find("BackgroundElement")
                .unwrap()
                .cast::<ferroui_controls::Border>()
                .unwrap();
            assert!(
                surface
                    .render_transform()
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .to_ref()
                    .ptr_eq(&scale)
            );
            *seen.borrow_mut() = true;
        });
        host.set_title(Some(Rc::new("More Colors".to_owned())));
        host.set_primary_button_text(Some("OK".into()));
        host.set_close_button_text(Some("Cancel".into()));
        let content = TextBox::new();
        content.set_text(Some("edited"));
        host.set_content(Some(ferroui_controls::Control::boxed(content.clone())));
        let w = Window::new();
        w.set_content(Some(ferroui_controls::Control::boxed(host.clone())));
        w.show();
        w.measure(Size::new(700., 700.));
        w.arrange(Rect::new(0., 0., 700., 700.));
        w.update_layout();
        assert!(*named.borrow());
        assert!(host.content_presenter().is_some());
        assert!(host.primary_button().is_some());
        assert!(host.close_button().is_some());
        assert!(host.command_space().is_some());
        let p = host.content_presenter().unwrap();
        assert!(
            ferroui_controls::Control::from_boxed(&p.content().unwrap())
                .unwrap()
                .ptr_eq(&content)
        );
        assert!(host.focus_dialog());
        assert!(content.is_focused());
        assert_eq!(content.text().as_deref(), Some("edited"));
        host.set_primary_button_text(None);
        assert!(!host.primary_button().unwrap().is_visible());
        host.set_close_button_text(None);
        assert!(!host.command_space().unwrap().is_visible());
        assert!(host.primary_button().unwrap().is_visible());
        w.close();
    });
}

fn mount(host: &Ref<RibbonColorDialogHost>) -> Ref<Window> {
    let w = Window::new();
    w.set_content(Some(ferroui_controls::Control::boxed(host.clone())));
    w.show();
    w.measure(Size::new(700., 700.));
    w.arrange(Rect::new(0., 0., 700., 700.));
    w.update_layout();
    w
}
#[test]
fn focus_callbacks_can_replace_template_without_borrowing_outgoing_parts() {
    run(|| {
        for content_focus in [true, false] {
            let host = RibbonColorDialogHost::new();
            host.set_primary_button_text(Some("OK".into()));
            host.set_secondary_button_text(Some("Other".into()));
            host.set_default_button(ContentDialogButton::Primary);
            let editor = TextBox::new();
            let other = Button::new();
            if content_focus {
                let panel = ferroui_controls::StackPanel::new();
                panel
                    .children()
                    .add(editor.clone().upcast::<ferroui_controls::Control>());
                panel
                    .children()
                    .add(other.clone().upcast::<ferroui_controls::Control>());
                host.set_content(Some(ferroui_controls::Control::boxed(panel)));
            }
            let w = mount(&host);
            let target = if content_focus {
                editor.clone().upcast::<InputElement>()
            } else {
                host.primary_button().unwrap().upcast::<InputElement>()
            };
            let previous = if content_focus {
                other.clone().upcast::<InputElement>()
            } else {
                host.secondary_button().unwrap().upcast::<InputElement>()
            };
            assert!(previous.focus());
            assert!(!target.is_focused());
            let called = Rc::new(Cell::new(false));
            let seen = called.clone();
            let weak = host.downgrade();
            target.add_handler(InputElement::got_focus_event(), move |_, _| {
                if !seen.replace(true)
                    && let Some(host) = weak.upgrade()
                {
                    host.set_template(None);
                }
            });
            host.focus_dialog();
            assert!(called.get());
            assert!(host.template().is_none());
            assert!(host.content_presenter().is_none());
            w.close();
        }
    });
}
#[test]
fn visual_state_property_notification_can_replace_template_synchronously() {
    run(|| {
        let host = RibbonColorDialogHost::new();
        host.set_primary_button_text(Some("OK".into()));
        host.set_secondary_button_text(Some("Other".into()));
        let w = mount(&host);
        let primary = host.primary_button().unwrap();
        let called = Rc::new(Cell::new(false));
        let seen = called.clone();
        let weak = host.downgrade();
        let subscription = primary.property_changed(move |e| {
            if e.property() == ferroui_base::Visual::is_visible_property().as_property()
                && !seen.replace(true)
                && let Some(host) = weak.upgrade()
            {
                host.set_template(None);
            }
        });
        host.set_primary_button_text(None);
        assert!(called.get());
        assert!(host.primary_button().is_none());
        subscription.dispose();
        w.close();
    });
}
#[test]
fn original_lazy_static_state_style_identity_survives_resource_replacement_until_retemplate() {
    run(|| {
        use ferroui_base::styling::ControlTheme;
        let themes: std::collections::HashMap<_, _> = ["A", "B", "C", "base", "new_base"]
            .into_iter()
            .map(|id| {
                let theme = ControlTheme::new();
                theme.set_target_type(Some(Button::TYPE));
                (id, theme)
            })
            .collect();
        let app = Application::current().unwrap();
        app.resources()
            .set("AccentButtonStyle", Some(Rc::new(themes["A"].clone())));
        let host = RibbonColorDialogHost::new();
        host.set_primary_button_text(Some("OK".into()));
        host.set_secondary_button_text(Some("Other".into()));
        host.set_close_button_text(Some("Cancel".into()));
        host.set_primary_button_style(Some(themes["base"].clone()));
        host.set_secondary_button_style(Some(themes["base"].clone()));
        host.set_close_button_style(Some(themes["base"].clone()));
        let w = mount(&host);
        let template = host.template().unwrap();
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/dialog-state-resources.json"
        ))
        .unwrap();
        assert_eq!(data["corpus"].as_array().unwrap().len(), 10);
        for case in data["corpus"].as_array().unwrap() {
            match case["name"].as_str().unwrap() {
                "primary_after_replace" => {
                    app.resources()
                        .set("AccentButtonStyle", Some(Rc::new(themes["B"].clone())));
                }
                "secondary_keeps_B" => {
                    app.resources()
                        .set("AccentButtonStyle", Some(Rc::new(themes["C"].clone())));
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
            w.update_layout();
            let selected = match choice {
                ContentDialogButton::Secondary => host.secondary_button(),
                ContentDialogButton::Close => host.close_button(),
                _ => host.primary_button(),
            }
            .unwrap();
            assert_eq!(
                selected.theme(),
                Some(themes[case["effective"].as_str().unwrap()].clone()),
                "{case}"
            );
        }
        w.close();
    });
}
#[test]
fn resource_lookup_retemplate_does_not_capture_outgoing_state_in_new_template() {
    run(|| {
        use ferroui_base::styling::ControlTheme;
        let host = RibbonColorDialogHost::new();
        host.set_primary_button_text(Some("OK".into()));
        let w = mount(&host);
        let old_button = host.primary_button().unwrap();
        let template = host.template().unwrap();
        let old_style = ControlTheme::new();
        old_style.set_target_type(Some(Button::TYPE));
        let new_style = ControlTheme::new();
        new_style.set_target_type(Some(Button::TYPE));
        let resources = instantiate(RetemplatingResources {
            base: ResourceDictionary::construct(),
            on_lookup: RefCell::default(),
        });
        resources.set("AccentButtonStyle", Some(Rc::new(old_style.clone())));
        host.resources()
            .merged_dictionaries()
            .add(resources.clone().into());
        let weak_host = host.downgrade();
        let weak_resources = resources.downgrade();
        let replacement = new_style.clone();
        resources.on_lookup.replace(Some(Rc::new(move || {
            let host = weak_host.upgrade().unwrap();
            weak_resources
                .upgrade()
                .unwrap()
                .set("AccentButtonStyle", Some(Rc::new(replacement.clone())));
            host.set_template(None);
            host.apply_template();
            host.set_template(Some(template.clone()));
            host.apply_template();
        })));
        host.set_default_button(ContentDialogButton::Primary);
        w.update_layout();
        let new_button = host.primary_button().unwrap();
        assert!(!new_button.ptr_eq(&old_button));
        assert_eq!(new_button.theme(), Some(new_style));
        assert_ne!(old_button.theme(), Some(old_style));
        w.close();
    });
}
#[test]
fn lazy_state_retains_style_until_template_detaches_and_releases_without_host_cycle() {
    run(|| {
        use ferroui_base::styling::ControlTheme;
        let host = RibbonColorDialogHost::new();
        host.set_primary_button_text(Some("OK".into()));
        let w = mount(&host);
        let captured = ControlTheme::new();
        captured.set_target_type(Some(Button::TYPE));
        let weak_style = captured.downgrade();
        let app = Application::current().unwrap();
        app.resources()
            .set("AccentButtonStyle", Some(Rc::new(captured.clone())));
        host.set_default_button(ContentDialogButton::Primary);
        host.set_default_button(ContentDialogButton::None);
        app.resources().set("AccentButtonStyle", None);
        drop(captured);
        assert!(
            weak_style.upgrade().is_some(),
            "lazy setter retains inactive style"
        );
        host.set_template(None);
        host.apply_template();
        w.update_layout();
        assert!(
            weak_style.upgrade().is_none(),
            "detached states release captured style"
        );
        let weak_host = host.downgrade();
        w.set_content(None);
        w.update_layout();
        drop(host);
        assert!(weak_host.upgrade().is_none());
        w.close();
    });
}
#[test]
fn source_default_button_states_focus_and_style_override_restore_binding_identity() {
    run(|| {
        let host = RibbonColorDialogHost::new();
        assert_eq!(host.default_button(), ContentDialogButton::None);
        host.set_primary_button_text(Some("OK".into()));
        host.set_secondary_button_text(Some("Other".into()));
        host.set_close_button_text(Some("Cancel".into()));
        let custom = ferroui_base::styling::ControlTheme::new();
        custom.set_target_type(Some(Button::TYPE));
        host.set_primary_button_style(Some(custom.clone()));
        let w = mount(&host);
        let buttons = [
            host.primary_button().unwrap(),
            host.secondary_button().unwrap(),
            host.close_button().unwrap(),
        ];
        let defaults: Vec<_> = buttons.iter().map(|b| b.theme()).collect();
        assert_eq!(defaults[0], Some(custom));
        let accent = host
            .try_find_resource(
                &"AccentButtonStyle".into(),
                host.actual_theme_variant().as_ref(),
            )
            .flatten()
            .and_then(|v| ferroui_base::data::core::ValueTypes::as_object(&*v))
            .unwrap()
            .cast::<ferroui_base::styling::ControlTheme>()
            .unwrap();
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/content-dialog-visual.json"
        ))
        .unwrap();
        let cases = data["default_button_corpus"].as_array().unwrap();
        assert_eq!(cases.len(), 5);
        for case in cases.iter().chain(cases.iter().rev()) {
            let choice = ContentDialogButton::from_raw(case["choice"].as_i64().unwrap() as i32);
            host.set_default_button(choice);
            w.update_layout();
            for (i, (name, button)) in ["Primary", "Secondary", "Close"]
                .iter()
                .zip(&buttons)
                .enumerate()
            {
                let selected = case["properties"]
                    .get(format!("{name}Button.Style"))
                    .is_some();
                assert_eq!(
                    button.theme(),
                    if selected {
                        Some(accent.clone())
                    } else {
                        defaults[i].clone()
                    },
                    "{case}"
                );
                assert!(
                    !button.is_default(),
                    "source state must not install a native root key handler"
                );
                assert!(!button.is_cancel());
            }
            assert!(host.focus_dialog());
            let focused = buttons.iter().position(|b| b.is_focused()).unwrap();
            assert_eq!(
                ["primary", "secondary", "close"][focused],
                case["focused"].as_str().unwrap(),
                "{case}"
            );
        }
        // A source visual-state style setter overrides the current base style;
        // removing it must expose the SAME live underlying TemplateBinding.
        host.set_default_button(ContentDialogButton::Primary);
        let replacement = ferroui_base::styling::ControlTheme::new();
        replacement.set_target_type(Some(Button::TYPE));
        host.set_primary_button_style(Some(replacement.clone()));
        assert_eq!(buttons[0].theme(), Some(accent));
        host.set_default_button(ContentDialogButton::None);
        assert_eq!(buttons[0].theme(), Some(replacement));
        w.close();
    });
}

#[test]
fn default_style_reentrancy_retemplate_cleanup_and_host_lifetime() {
    run(|| {
        let host = RibbonColorDialogHost::new();
        host.set_primary_button_text(Some("OK".into()));
        host.set_secondary_button_text(Some("Other".into()));
        host.set_close_button_text(Some("Cancel".into()));
        let w = mount(&host);
        let original_template = host.template().unwrap();
        let old = host.primary_button().unwrap();
        // A retained part's TemplateBinding is disposed when it leaves the
        // template. A local value remains and must survive our visual override.
        old.set_theme(old.theme());
        let base = old.theme();
        let weak = host.downgrade();
        let changed = Rc::new(Cell::new(false));
        let fired = changed.clone();
        let subscription = old.property_changed(move |e| {
            if e.property() == ferroui_base::StyledElement::theme_property().as_property()
                && !fired.replace(true)
                && let Some(host) = weak.upgrade()
            {
                host.set_default_button(ContentDialogButton::Secondary);
            }
        });
        host.set_default_button(ContentDialogButton::Primary);
        assert!(changed.get());
        assert_eq!(host.default_button(), ContentDialogButton::Secondary);
        assert_eq!(old.theme(), base);
        assert!(
            host.default_visual
                .borrow()
                .as_ref()
                .unwrap()
                .button
                .ptr_eq(&host.secondary_button().unwrap())
        );
        subscription.dispose();
        host.set_default_button(ContentDialogButton::Primary);
        host.set_template(None);
        host.apply_template();
        assert_eq!(old.theme(), base);
        host.set_template(Some(original_template));
        host.apply_template();
        w.update_layout();
        assert!(!host.primary_button().unwrap().ptr_eq(&old));
        assert!(
            host.default_visual
                .borrow()
                .as_ref()
                .unwrap()
                .button
                .ptr_eq(&host.primary_button().unwrap())
        );
        // Retaining old native parts and resource dictionaries must not root host.
        w.set_content(None);
        w.update_layout();
        let weak = host.downgrade();
        drop(host);
        assert!(weak.upgrade().is_none());
        assert_eq!(old.theme(), base);
        w.close();
    });
}

#[test]
fn reentrant_default_style_notification_replacing_template_disposes_in_flight_override() {
    run(|| {
        let host = RibbonColorDialogHost::new();
        host.set_primary_button_text(Some("OK".into()));
        let w = mount(&host);
        let old = host.primary_button().unwrap();
        old.set_theme(old.theme());
        let base = old.theme();
        let weak = host.downgrade();
        let once = Rc::new(Cell::new(false));
        let changed = once.clone();
        let subscription = old.property_changed(move |e| {
            if e.property() == ferroui_base::StyledElement::theme_property().as_property()
                && !changed.replace(true)
                && let Some(host) = weak.upgrade()
            {
                host.set_template(None);
            }
        });
        host.set_default_button(ContentDialogButton::Primary);
        assert!(once.get());
        assert!(host.primary_button().is_none());
        assert!(host.default_visual.borrow().is_none());
        assert_eq!(old.theme(), base);
        subscription.dispose();
        w.close();
    });
}
#[test]
fn fluent_v2_source_states_and_repeated_transitions_match_original_methods_and_xaml() {
    run(|| {
        let host = RibbonColorDialogHost::new();
        let w = mount(&host);
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/content-dialog-visual.json"
        ))
        .unwrap();
        let cases = data["fluent_corpus"].as_array().unwrap();
        assert_eq!(cases.len(), 24);
        for case in cases.iter().chain(cases.iter().rev()) {
            host.set_primary_button_text(case["primary"].as_bool().unwrap().then(|| "OK".into()));
            host.set_secondary_button_text(
                case["secondary"].as_bool().unwrap().then(|| "Other".into()),
            );
            host.set_close_button_text(case["close"].as_bool().unwrap().then(|| "Cancel".into()));
            host.set_title(
                case["title"]
                    .as_str()
                    .map(|s| Rc::new(s.to_owned()) as BoxedValue),
            );
            w.update_layout();
            let p = host.parts.borrow();
            let expected = &case["properties"];
            assert_eq!(
                p.title.as_ref().unwrap().is_visible(),
                case["titleVisibility"] == "Visible",
                "{case}"
            );
            assert_eq!(
                p.commands.as_ref().unwrap().is_visible(),
                expected["CommandSpace.Visibility"] == "Visible",
                "{case}"
            );
            // Padding wrapper must collapse with its named original grid.
            assert_eq!(
                p.commands
                    .as_ref()
                    .unwrap()
                    .get_visual_parent()
                    .unwrap()
                    .is_visible(),
                expected["CommandSpace.Visibility"] == "Visible",
                "{case}"
            );
            for (name, button) in [
                ("PrimaryButton", &p.primary),
                ("SecondaryButton", &p.secondary),
                ("CloseButton", &p.close),
            ] {
                let button = button.as_ref().unwrap();
                assert_eq!(
                    button.is_visible(),
                    expected[format!("{name}.Visibility")] == "Visible",
                    "{case}"
                );
                assert_eq!(
                    Grid::get_column(button),
                    expected[format!("{name}.(Grid.Column)")]
                        .as_str()
                        .unwrap()
                        .parse::<i32>()
                        .unwrap(),
                    "{case}"
                );
                assert_eq!(Grid::get_column_span(button), 1);
                assert!(button.is_tab_stop());
            }
            let columns = p.commands.as_ref().unwrap().column_definitions();
            assert_eq!(columns.count(), 5);
            assert_eq!(
                columns.get(1).width(),
                if expected["FirstSpacer.Width"] == "0" {
                    GridLength::new(0., ferroui_controls::GridUnitType::Pixel)
                } else {
                    columns.get(3).width()
                },
                "{case}"
            );
            assert_eq!(
                columns.get(2).width(),
                if expected["SecondaryColumn.Width"] == "*" {
                    GridLength::STAR
                } else {
                    GridLength::new(0., ferroui_controls::GridUnitType::Pixel)
                },
                "{case}"
            );
        }
        // Source state resource references remain responsive after a transition.
        host.set_primary_button_text(Some("OK".into()));
        host.set_secondary_button_text(Some("Other".into()));
        host.set_close_button_text(Some("Cancel".into()));
        let columns = host.command_space().unwrap().column_definitions();
        let spacing = columns.get(1).width();
        columns
            .get(3)
            .set_width(GridLength::new(13.5, ferroui_controls::GridUnitType::Pixel));
        assert_eq!(
            columns.get(1).width(),
            spacing,
            "source state uses its theme resource, not another column's local width"
        );
        host.resources().set(
            "ContentDialogButtonSpacing",
            Some(Rc::new(GridLength::new(
                11.75,
                ferroui_controls::GridUnitType::Pixel,
            ))),
        );
        assert_eq!(
            columns.get(1).width(),
            GridLength::new(11.75, ferroui_controls::GridUnitType::Pixel)
        );
        assert_eq!(
            columns.get(3).width(),
            GridLength::new(13.5, ferroui_controls::GridUnitType::Pixel)
        );
        w.close();
    });
}
#[test]
fn retemplate_retains_content_and_detaches_outgoing_actions_and_subscriptions() {
    run(|| {
        let host = RibbonColorDialogHost::new();
        host.set_primary_button_text(Some("OK".into()));
        host.set_close_button_text(Some("Cancel".into()));
        let editor = TextBox::new();
        editor.set_text(Some("same editor, edited value"));
        host.set_content(Some(ferroui_controls::Control::boxed(editor.clone())));
        let calls = Rc::new(RefCell::new(Vec::new()));
        let captured = calls.clone();
        host.on_action(Some(Rc::new(move |primary| {
            captured.borrow_mut().push(primary)
        })));
        let w = mount(&host);
        let original_template = host.template().unwrap();
        let old_primary = host.primary_button().unwrap();
        let old_content = host.content_presenter().unwrap();
        let old_columns = host.command_space().unwrap().column_definitions();
        old_primary.on_click();
        assert_eq!(*calls.borrow(), vec![true]);
        host.set_template(None);
        host.apply_template();
        host.set_template(Some(original_template));
        host.apply_template();
        w.update_layout();
        assert!(!host.primary_button().unwrap().ptr_eq(&old_primary));
        assert!(old_content.content().is_none());
        assert!(
            ferroui_controls::Control::from_boxed(
                &host.content_presenter().unwrap().content().unwrap()
            )
            .unwrap()
            .ptr_eq(&editor)
        );
        assert_eq!(editor.text().as_deref(), Some("same editor, edited value"));
        old_primary.on_click();
        assert_eq!(*calls.borrow(), vec![true]);
        host.primary_button().unwrap().on_click();
        host.close_button().unwrap().on_click();
        assert_eq!(*calls.borrow(), vec![true, true, false]);
        assert!(host.focus_dialog());
        assert!(editor.is_focused());
        let new_columns = host.command_space().unwrap().column_definitions();
        let width = new_columns.get(1).width();
        old_columns
            .get(3)
            .set_width(GridLength::new(99., ferroui_controls::GridUnitType::Pixel));
        assert_eq!(new_columns.get(1).width(), width);
        host.on_action(None);
        host.primary_button().unwrap().on_click();
        assert_eq!(*calls.borrow(), vec![true, true, false]);
        w.close();
    });
}

#[test]
fn default_resource_ownership_theme_changes_and_application_overrides() {
    run(|| {
        use ferroui_base::{Thickness, styling::ThemeVariant};
        use ferroui_controls::Border;
        let app = Application::current().unwrap();
        let before = app.resources().merged_dictionaries().count();
        let host = RibbonColorDialogHost::new();
        let other = RibbonColorDialogHost::new();
        assert_eq!(
            app.resources().merged_dictionaries().count(),
            before + 1,
            "one dictionary per application, not per host"
        );
        drop(other);
        host.set_primary_button_text(Some("OK".into()));
        let w = mount(&host);
        let background = host
            .get_visual_descendants()
            .find_map(|v| {
                v.cast::<Border>()
                    .filter(|v| v.name().as_deref() == Some("BackgroundElement"))
            })
            .unwrap();
        assert_eq!(background.min_width(), 320.);
        assert_eq!(background.max_width(), 548.);
        assert_eq!(background.min_height(), 184.);
        assert_eq!(background.max_height(), 756.);
        for (variant, width, color) in [
            (
                ThemeVariant::light(),
                1.,
                ferroui_base::media::Color::from_rgb(243, 243, 243),
            ),
            (
                ThemeVariant::dark(),
                1.,
                ferroui_base::media::Color::from_rgb(32, 32, 32),
            ),
            (
                crate::theming::RibbonThemeResources::high_contrast_variant(),
                2.,
                ferroui_base::media::Color::from_rgb(0, 0, 0),
            ),
        ] {
            host.set_value(
                ThemeVariant::requested_theme_variant_property(),
                Some(variant),
            );
            w.update_layout();
            assert_eq!(background.border_thickness(), Thickness::uniform(width));
            assert_eq!(
                background
                    .background()
                    .unwrap()
                    .as_solid_color_brush()
                    .unwrap()
                    .color(),
                color
            );
        }
        host.set_value(
            ThemeVariant::requested_theme_variant_property(),
            Some(ThemeVariant::light()),
        );
        app.resources()
            .set("ContentDialogMinWidth", Some(Rc::new(411.5_f64)));
        w.update_layout();
        assert_eq!(
            background.min_width(),
            411.5,
            "application overrides default merged resources dynamically"
        );
        let columns = host.command_space().unwrap().column_definitions();
        host.set_secondary_button_text(Some("Other".into()));
        host.set_close_button_text(Some("Cancel".into()));
        app.resources().set(
            "ContentDialogButtonSpacing",
            Some(Rc::new(GridLength::new(
                12.25,
                ferroui_controls::GridUnitType::Pixel,
            ))),
        );
        w.update_layout();
        assert_eq!(
            columns.get(3).width(),
            GridLength::new(12.25, ferroui_controls::GridUnitType::Pixel)
        );
        assert_eq!(columns.get(1).width(), columns.get(3).width());
        w.set_content(None);
        w.update_layout();
        let weak = host.downgrade();
        drop(background);
        drop(columns);
        drop(host);
        assert!(
            weak.upgrade().is_none(),
            "default resources and state callbacks must not retain the host"
        );
        w.close();
    });
}

fn brush_snapshot(brush: Rc<dyn ferroui_base::media::IBrush>) -> serde_json::Value {
    use serde_json::json;
    let hex =
        |c: ferroui_base::media::Color| format!("#{:02X}{:02X}{:02X}{:02X}", c.a, c.r, c.g, c.b);
    if let Some(solid) = brush.as_solid_color_brush() {
        json!({"kind":"solid","color":hex(solid.color()),"opacity":brush.opacity()})
    } else {
        let linear = brush
            .as_linear_gradient_brush()
            .expect("source linear brush");
        let start = linear.start_point();
        let end = linear.end_point();
        assert_eq!(start.unit, end.unit);
        let flip = if let Some(transform) = brush.relative_transform() {
            assert_eq!(
                transform.value(),
                ferroui_base::Matrix::parse("1,0,0,-1,0,1").unwrap()
            );
            true
        } else {
            false
        };
        json!({"kind":"linear", "opacity":brush.opacity(),
            "start":[start.point.x,start.point.y],"end":[end.point.x,end.point.y],
            "mapping":if start.unit == ferroui_base::RelativeUnit::Absolute {"Absolute"} else {"RelativeToBoundingBox"},
            "flip_y":flip,
            "stops":linear.gradient_stops().iter().map(|s|json!({"offset":s.offset(),"color":hex(s.color())})).collect::<Vec<_>>()})
    }
}

#[test]
fn original_button_styles_theme_resources_state_precedence_gradients_and_label_presenter() {
    run(|| {
        use ferroui_base::{Thickness, styling::ThemeVariant};
        let host = RibbonColorDialogHost::new();
        host.set_primary_button_text(Some("_A e\u{301} 😀 אבג".into()));
        host.set_close_button_text(Some("Cancel".into()));
        let w = mount(&host);
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/content-dialog-buttons.json"
        ))
        .unwrap();
        let cases = data["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 24);
        let mut seen: Vec<WeakRef<ContentPresenter>> = Vec::new();
        for case in cases {
            host.set_value(
                ThemeVariant::requested_theme_variant_property(),
                Some(match case["theme"].as_str().unwrap() {
                    "Default" => ThemeVariant::dark(),
                    "Light" => ThemeVariant::light(),
                    "HighContrast" => crate::theming::RibbonThemeResources::high_contrast_variant(),
                    _ => unreachable!(),
                }),
            );
            host.set_default_button(if case["style"] == "AccentButtonStyle" {
                ContentDialogButton::Primary
            } else {
                ContentDialogButton::None
            });
            w.update_layout();
            let button = host.primary_button().unwrap();
            button.set_is_enabled(true);
            button.pseudo_classes().set(":pointerover", false);
            button.pseudo_classes().set(":pressed", false);
            button.apply_template();
            let presenter = button
                .get_visual_descendants()
                .find_map(|v| v.cast::<ContentPresenter>())
                .unwrap();
            assert_eq!(presenter.name().as_deref(), Some("ContentPresenter"));
            assert_eq!(
                presenter
                    .content()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap(),
                "_A e\u{301} 😀 אבג"
            );
            assert!(
                !presenter.recognizes_access_key(),
                "original ordinary text does not consume underscores"
            );
            if !seen
                .iter()
                .any(|p| p.upgrade().as_ref() == Some(&presenter))
            {
                let transitions = presenter
                    .transitions()
                    .expect("original 83ms background transition");
                assert_eq!(transitions.count(), 1);
                let t = transitions
                    .get(0)
                    .as_object()
                    .unwrap()
                    .cast::<ferroui_base::animation::BrushTransition>()
                    .unwrap();
                assert_eq!(
                    t.duration().total_milliseconds(),
                    case["background_transition_ms"].as_f64().unwrap()
                );
                // Snapshot settled state values; temporal animation/native paint
                // must be checked separately with the framework clock/platform.
                presenter.set_transitions(None);
                seen.push(presenter.downgrade());
            }
            assert_eq!(
                button.padding(),
                Thickness::parse(case["padding"].as_str().unwrap()).unwrap()
            );
            assert_eq!(button.font_size(), case["font_size"].as_f64().unwrap());
            match case["state"].as_str().unwrap() {
                "Normal" => {}
                "PointerOver" => {
                    button.pseudo_classes().set(":pointerover", true);
                }
                "Pressed" => {
                    button.pseudo_classes().set(":pointerover", true);
                    button.pseudo_classes().set(":pressed", true);
                }
                "Disabled" => {
                    button.pseudo_classes().set(":pointerover", true);
                    button.pseudo_classes().set(":pressed", true);
                    button.set_is_enabled(false);
                }
                _ => unreachable!(),
            }
            for (name, actual) in [
                ("Background", presenter.background()),
                ("Foreground", presenter.foreground()),
                ("BorderBrush", presenter.border_brush()),
            ] {
                assert_eq!(
                    brush_snapshot(actual.unwrap()),
                    case["brushes"][name],
                    "{name} {case}"
                );
            }
        }
        w.close();
    });
}

#[test]
fn compiled_button_theme_sharing_local_resource_scope_and_live_application_updates() {
    run(|| {
        use ferroui_base::media::{Color, SolidColorBrush};
        let host = RibbonColorDialogHost::new();
        let other = RibbonColorDialogHost::new();
        host.set_primary_button_text(Some("First".into()));
        other.set_primary_button_text(Some("Other".into()));
        let w = mount(&host);
        let ow = mount(&other);
        let button = host.primary_button().unwrap();
        let other_button = other.primary_button().unwrap();
        assert_eq!(
            button.theme(),
            other_button.theme(),
            "default resource identity is shared"
        );
        let presenter = button
            .get_visual_descendants()
            .find_map(|v| v.cast::<ContentPresenter>())
            .unwrap();
        let other_presenter = other_button
            .get_visual_descendants()
            .find_map(|v| v.cast::<ContentPresenter>())
            .unwrap();
        let local = SolidColorBrush::new();
        local.set_color(Color::from_rgb(17, 31, 47));
        host.resources()
            .set("ButtonForeground", Some(Rc::new(local.clone())));
        assert_eq!(presenter.foreground(), Some(local.clone().into()));
        assert_ne!(other_presenter.foreground(), Some(local.clone().into()));
        let shared = SolidColorBrush::new();
        shared.set_color(Color::from_rgb(91, 61, 31));
        Application::current()
            .unwrap()
            .resources()
            .set("ButtonForeground", Some(Rc::new(shared.clone())));
        assert_eq!(other_presenter.foreground(), Some(shared.clone().into()));
        assert_eq!(presenter.foreground(), Some(local.into()));
        host.resources().remove(&"ButtonForeground".into());
        assert_eq!(presenter.foreground(), Some(shared.into()));
        w.close();
        ow.close();
    });
}
