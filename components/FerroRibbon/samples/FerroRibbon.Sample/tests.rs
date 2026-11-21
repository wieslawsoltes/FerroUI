//! Independent compiled application exercises the actual public Ribbon controls.
use super::*;
use ferroui_base::{Rect, Size, threading::CancellationToken};
use ferroui_controls::{AppBuilder, Application, Control, TextBox};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use view_model::{InMemoryDocument, SampleViewModel};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "FerroRibbon.Sample.Tests",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn actual_sample_switches_layout_and_reaches_linked_control_overflow() {
    const NAME: &str = "tests::actual_sample_switches_layout_and_reaches_linked_control_overflow";
    if std::env::var_os("FERRORIBBON_SAMPLE_OVERFLOW_ISOLATED").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_SAMPLE_OVERFLOW_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    run(|| {
        use ferroribbon::controls::Ribbon;
        use ferroribbon_core::RibbonDisplayMode;
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.measure(Size::new(360., 700.));
        w.arrange(Rect::new(0., 0., 360., 700.));
        let ribbon = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        model.set_simplified(Some(true));
        assert_eq!(ribbon.display_mode(), RibbonDisplayMode::Simplified);
        w.measure(Size::new(360., 700.));
        w.arrange(Rect::new(0., 0., 360., 700.));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let tab = ribbon.selected_tab().unwrap();
        assert!(tab.groups_panel().has_overflow());
        tab.show_overflow();
        model.set_simplified(Some(false));
        assert_eq!(ribbon.display_mode(), RibbonDisplayMode::Classic);
        w.close();
        w.set_content(None);
    });
}
#[test]
fn actual_workspace_segments_execute_reactive_actions_for_user_input_only() {
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonSegmentedControl};
        use ferroui_base::input::{InputElement, Key, KeyEventArgs};
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let window = create_window(model.clone());
        window.show();
        window.measure(Size::new(1140., 700.));
        window.arrange(Rect::new(0., 0., 1140., 700.));
        let ribbon = window
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        ribbon.set_selected_tab(Some(ribbon.tabs().get(2).unwrap()));
        let control = ribbon
            .find_item(Some("workspace-mode".into()))
            .unwrap()
            .cast::<RibbonSegmentedControl>()
            .unwrap();
        assert_eq!(control.segments().count(), 3);
        control.set_selected_index(1);
        assert_eq!(model.status(), "Ready");
        let key = |key| {
            let mut e = KeyEventArgs::new();
            e.key = key;
            e.set_routed_event(Some(InputElement::key_down_event()));
            control.raise_event(&e);
            assert!(e.handled());
        };
        key(Key::Enter);
        key(Key::Enter);
        assert_eq!(
            model
                .document()
                .matches("Review workspace applied to the sample document.")
                .count(),
            2
        );
        key(Key::Right);
        assert_eq!(
            model.status(),
            "Draw workspace applied to the sample document."
        );
        let replacement = SampleViewModel::new(Rc::new(InMemoryDocument));
        window.set_data_context(Some(Rc::new(replacement.clone())));
        key(Key::Home);
        assert_eq!(
            replacement.status(),
            "Write workspace applied to the sample document."
        );
        assert_eq!(
            model
                .document()
                .matches("Write workspace applied to the sample document.")
                .count(),
            0
        );
        window.close();
        window.set_content(None);
    });
}
#[test]
fn actual_paste_split_tracks_menu_choice_and_repeats_its_reactive_action_once() {
    run(|| {
        use ferroribbon::{
            compat::MenuFlyoutItem,
            controls::{Ribbon, RibbonSplitButton},
        };
        use ferroui_controls::MenuFlyout;
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        let ribbon = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        let paste = ribbon
            .find_item(Some("paste".into()))
            .unwrap()
            .cast::<RibbonSplitButton>()
            .unwrap();
        paste.open_drop_down();
        let menu = paste.flyout().unwrap().cast::<MenuFlyout>().unwrap();
        let choice = Control::from_boxed(&menu.items().view().get_at(1).unwrap())
            .unwrap()
            .cast::<MenuFlyoutItem>()
            .unwrap();
        choice.invoke_menu_core();
        assert!(paste.last_choice().unwrap().ptr_eq(&choice));
        paste.close_drop_down();
        assert_eq!(
            model
                .document()
                .matches("Paste text only applied to the sample document.")
                .count(),
            1
        );
        paste.invoke_primary();
        assert_eq!(
            model
                .document()
                .matches("Paste text only applied to the sample document.")
                .count(),
            2
        );
        assert_eq!(paste.label().as_deref(), Some("Paste"));
        let replacement = SampleViewModel::new(Rc::new(InMemoryDocument));
        w.set_data_context(Some(Rc::new(replacement.clone())));
        paste.invoke_primary();
        assert_eq!(
            replacement
                .document()
                .matches("Paste text only applied to the sample document.")
                .count(),
            1
        );
        assert_eq!(
            model
                .document()
                .matches("Paste text only applied to the sample document.")
                .count(),
            2
        );
        w.close();
    });
}
#[test]
fn source_menu_in_actual_ribbon_flyout_uses_live_reactive_view_model() {
    run(|| {
        use ferroribbon::{compat::MenuFlyoutItem, controls::Ribbon};
        use ferroui_controls::MenuFlyout;
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let ribbon = w
            .get_visual_descendants()
            .find_map(|v| v.cast::<Ribbon>())
            .unwrap();
        let control = ribbon.find_item(Some("editing-menu".into())).unwrap();
        let button = control
            .cast::<ferroribbon::controls::RibbonDropDownButton>()
            .unwrap();
        let flyout = button.flyout().unwrap().cast::<MenuFlyout>().unwrap();
        flyout.show_at(&button);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let value = flyout.items().view().get_at(0).unwrap();
        let item = Control::from_boxed(&value)
            .unwrap()
            .cast::<MenuFlyoutItem>()
            .unwrap();
        assert!(item.command().is_some());
        item.invoke_menu_core();
        assert_eq!(model.status(), "Select all applied to the sample document.");
        assert_eq!(
            model
                .document()
                .matches("Select all applied to the sample document.")
                .count(),
            1
        );
        flyout.hide();
        w.close();
    });
}
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                register_types();
                let app = Application::current().unwrap();
                let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
                app.styles().add(theme.clone());
                test();
                app.styles().remove(theme);
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}

#[test]
fn actual_zoom_uses_native_layout_transform_and_reactive_command_for_user_changes() {
    run(|| {
        use ferroribbon::{compat::RibbonInputSlider, controls::RibbonZoomControl};
        use ferroui_base::{
            controls::NameScopeExtensions,
            input::{InputElement, Key, KeyEventArgs},
            interactivity::RoutedEventArgs,
        };
        use ferroui_controls::{Button, LayoutTransformControl};
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(vm.clone());
        w.show();
        w.update_layout();
        let zoom =
            NameScopeExtensions::find::<RibbonZoomControl>(&w, "DocumentZoomControl").unwrap();
        let content =
            NameScopeExtensions::find::<LayoutTransformControl>(&w, "DocumentZoom").unwrap();
        assert_eq!(zoom.value(), 100.);
        vm.set_zoom_percent(150.);
        w.update_layout();
        assert_eq!(zoom.value(), 150.);
        assert_eq!(content.layout_transform().unwrap().value().m11, 1.5);
        assert_eq!(
            content
                .transform_root()
                .unwrap()
                .render_transform()
                .unwrap()
                .value()
                .m11,
            1.5
        );
        assert_eq!(vm.status(), "Ready");
        let plus = zoom
            .get_visual_descendants()
            .find_map(|c| {
                c.cast::<Button>()
                    .filter(|c| c.name().as_deref() == Some("PART_ZoomIn"))
            })
            .unwrap();
        plus.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(vm.zoom_percent(), 160.);
        assert_eq!(vm.status(), "Zoom: 160%");
        let slider = zoom
            .get_visual_descendants()
            .find_map(|c| c.cast::<RibbonInputSlider>())
            .unwrap();
        let mut key = KeyEventArgs::new();
        key.key = Key::Left;
        key.set_routed_event(Some(InputElement::key_down_event()));
        slider.raise_event(&key);
        assert_eq!(vm.zoom_percent(), 159.);
        assert_eq!(vm.status(), "Zoom: 159%");
        let replacement = SampleViewModel::new(Rc::new(InMemoryDocument));
        w.set_data_context(Some(replacement.clone()));
        assert_eq!(zoom.value(), 100.);
        vm.set_zoom_percent(200.);
        assert_eq!(zoom.value(), 100.);
        plus.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert_eq!(replacement.zoom_percent(), 110.);
        assert_eq!(replacement.status(), "Zoom: 110%");
        w.close();
    });
}
#[test]
fn compiled_native_ribbon_tabs_and_reactive_commands_execute_once() {
    run(|| {
        use ferroribbon::controls::{IRibbonItem, Ribbon, RibbonSplitButton};
        use ferroui_base::data::core::{ValueType, ValueTypes};
        use ferroui_controls::automation::{AutomationProperties, peers::ControlAutomationPeer};
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let ribbon = w
            .get_visual_descendants()
            .find_map(|v| v.cast::<Ribbon>())
            .unwrap();
        assert_eq!(ribbon.selected_tab_id().as_deref(), Some("home"));
        for (index, expected) in ["RibbonTab_home", "RibbonTab_insert", "RibbonTab_view"]
            .into_iter()
            .enumerate()
        {
            let header = ribbon.tabs().get(index).unwrap().header_element().unwrap();
            let peer = ControlAutomationPeer::create_peer_for_element(&header);
            assert_eq!(peer.get_automation_id().as_deref(), Some(expected));
        }
        let view = ribbon.tabs().get(2).unwrap();
        let header = view.header_element().unwrap();
        let peer = ControlAutomationPeer::create_peer_for_element(&header);
        view.set_id(Some("view-renamed".into()));
        assert_eq!(
            peer.get_automation_id().as_deref(),
            Some("RibbonTab_view-renamed")
        );
        AutomationProperties::set_automation_id(&header, Some("custom-view"));
        view.set_id(Some("view".into()));
        assert_eq!(peer.get_automation_id().as_deref(), Some("custom-view"));
        let paste = ribbon
            .tabs()
            .get(0)
            .unwrap()
            .groups()
            .get(0)
            .unwrap()
            .items()
            .get(0)
            .unwrap()
            .cast::<RibbonSplitButton>()
            .unwrap();
        let value = Control::boxed(paste);
        let item = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRibbonItem>>()).unwrap();
        assert!(item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap().invoke());
        assert_eq!(model.status(), "Paste applied to the sample document.");
        assert_eq!(
            model
                .document()
                .matches("Paste applied to the sample document.")
                .count(),
            1
        );
        let text = w
            .get_visual_descendants()
            .find_map(|v| {
                v.cast::<TextBox>()
                    .filter(|v| v.name().as_deref() == Some("Document"))
            })
            .unwrap();
        assert_eq!(text.text().as_deref(), Some(model.document().as_str()));
        let group = ribbon.tabs().get(0).unwrap().groups().get(0).unwrap();
        let expanded = group.slide_out_items().get(0).unwrap();
        assert!(group.slide_out_button().unwrap().is_visible());
        group.open_slide_out();
        assert!(group.is_slide_out_open());
        let command = ValueTypes::try_cast(
            &Control::boxed(expanded),
            ValueType::of::<Rc<dyn IRibbonItem>>(),
        )
        .unwrap();
        assert!(
            command
                .downcast_ref::<Rc<dyn IRibbonItem>>()
                .unwrap()
                .invoke()
        );
        assert_eq!(
            model.status(),
            "Paste special applied to the sample document."
        );
        assert_eq!(
            model
                .document()
                .matches("Paste special applied to the sample document.")
                .count(),
            1
        );
        // The same reparented control follows a live inherited context change.
        let replacement = SampleViewModel::new(Rc::new(InMemoryDocument));
        w.set_data_context(Some(replacement.clone()));
        assert!(
            command
                .downcast_ref::<Rc<dyn IRibbonItem>>()
                .unwrap()
                .invoke()
        );
        assert_eq!(
            replacement.status(),
            "Paste special applied to the sample document."
        );
        assert_eq!(
            model
                .document()
                .matches("Paste special applied to the sample document.")
                .count(),
            1
        );
        group.set_is_slide_out_pinned(true);
        assert!(ribbon.select_tab_id("insert"));
        assert!(!group.is_slide_out_open());
        assert_eq!(
            ribbon.selected_tab().unwrap().header().as_deref(),
            Some("Insert")
        );
        assert!(ribbon.select_tab_id("home"));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(group.is_slide_out_open());
        group.set_is_slide_out_pinned(false);
        group.close_slide_out();
        w.close();
        w.set_content(None);
    });
}
#[test]
fn sample_command_bridge_reuses_identity_and_releases_unretained_adapters() {
    run(|| {
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let first = command_adapter::adapt(vm.command());
        let second = command_adapter::adapt(vm.command());
        assert!(Rc::ptr_eq(&first, &second));
        let weak = Rc::downgrade(&first);
        drop(first);
        drop(second);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn sample_checked_controls_follow_reactive_view_model_in_both_directions() {
    run(|| {
        use ferroribbon::controls::{RibbonCheckBox, RibbonToggleButton};
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(vm.clone());
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        let toggle = w
            .get_visual_descendants()
            .find_map(|v| v.cast::<RibbonToggleButton>())
            .unwrap();
        vm.set_bold(Some(true));
        assert_eq!(toggle.is_checked(), Some(true));
        toggle.set_is_checked(None);
        assert_eq!(vm.bold(), None);
        let ribbon = w
            .get_visual_descendants()
            .find_map(|v| v.cast::<ferroribbon::controls::Ribbon>())
            .unwrap();
        assert!(ribbon.select_tab_id("view"));
        let check = w
            .get_visual_descendants()
            .find_map(|v| {
                v.cast::<RibbonCheckBox>()
                    .filter(|c| c.id().as_deref() == Some("guides"))
            })
            .unwrap();
        check.set_is_checked(Some(true));
        assert_eq!(vm.show_guides(), Some(true));
        vm.set_show_guides(Some(false));
        assert_eq!(check.is_checked(), Some(false));
        w.close();
        w.set_content(None);
    });
}

#[test]
fn actual_sample_minimizes_reparents_invokes_and_syncs_reactive_mode() {
    const NAME: &str = "tests::actual_sample_minimizes_reparents_invokes_and_syncs_reactive_mode";
    if std::env::var_os("FERRORIBBON_SAMPLE_MINIMIZE_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_SAMPLE_MINIMIZE_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        return;
    }
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonButton};
        use ferroribbon_core::RibbonVisibilityMode;
        use ferroui_controls::automation::{
            peers::ControlAutomationPeer, provider::IExpandCollapseProvider,
        };
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        let root = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        assert!(root.is_minimize_button_visible());
        model.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        assert!(root.is_minimized());
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        let cut = root
            .get_all_items()
            .iter()
            .find_map(|c| {
                c.cast::<RibbonButton>()
                    .filter(|b| b.id().as_deref() == Some("cut"))
            })
            .unwrap();
        root.open_minimized_popup();
        assert!(root.is_minimized_popup_open());
        assert!(cut.invoke_core());
        assert_eq!(model.status(), "Cut applied to the sample document.");
        assert_eq!(model.document().matches("Cut applied").count(), 1);
        assert!(!root.is_minimized_popup_open());
        root.set_visibility_mode(RibbonVisibilityMode::AlwaysShow);
        assert_eq!(model.visibility_mode(), RibbonVisibilityMode::AlwaysShow);
        let peer = ControlAutomationPeer::create_peer_for_element(&root);
        peer.get_provider::<dyn IExpandCollapseProvider>()
            .unwrap()
            .collapse()
            .unwrap();
        assert_eq!(model.visibility_mode(), RibbonVisibilityMode::TabsOnly);
        w.close();
        w.set_content(None);
    });
}

#[test]
fn actual_sample_shortcuts_reach_hidden_tab_and_retained_minimized_popup() {
    const NAME: &str =
        "tests::actual_sample_shortcuts_reach_hidden_tab_and_retained_minimized_popup";
    if std::env::var_os("FERRORIBBON_SAMPLE_SHORTCUT_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_SAMPLE_SHORTCUT_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        return;
    }
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonButton};
        use ferroribbon_core::RibbonVisibilityMode;
        use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        let root = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(root.is_loaded());
        assert!(root.is_shortcut_routing_enabled());
        assert!(root.select_tab_id("insert"));
        let send = |control: &Control, key| {
            let mut e = KeyEventArgs::new();
            e.key = key;
            e.key_modifiers = KeyModifiers::CONTROL;
            e.set_routed_event(Some(InputElement::key_down_event()));
            control.raise_event(&e);
            e.handled()
        };
        assert!(send(&root, Key::X));
        assert_eq!(model.document().matches("Cut applied").count(), 1);
        assert!(root.select_tab_id("home"));
        root.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        root.open_minimized_popup();
        assert!(root.is_minimized_popup_open());
        let copy = root
            .get_all_items()
            .iter()
            .find_map(|c| {
                c.cast::<RibbonButton>()
                    .filter(|b| b.id().as_deref() == Some("copy"))
            })
            .unwrap();
        assert!(send(&copy, Key::C));
        assert_eq!(model.document().matches("Copy applied").count(), 1);
        assert!(!root.is_minimized_popup_open());
        w.close();
        w.set_content(None);
    });
}
#[test]
fn actual_sample_keytips_toggle_command_and_navigate_minimized_popup_and_menu() {
    const NAME: &str =
        "tests::actual_sample_keytips_toggle_command_and_navigate_minimized_popup_and_menu";
    if std::env::var_os("FERRORIBBON_SAMPLE_KEYBOARD_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_SAMPLE_KEYBOARD_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        return;
    }
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonButton};
        use ferroribbon_core::RibbonVisibilityMode;
        use ferroui_base::input::{InputElement, Key, KeyEventArgs};
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        let settle = || ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        settle();
        let r = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        let send = |control: &Control, key| {
            let mut e = KeyEventArgs::new();
            e.key = key;
            e.set_routed_event(Some(InputElement::key_down_event()));
            control.raise_event(&e);
            assert!(e.handled());
        };
        send(&r, Key::F10);
        settle();
        assert!(r.is_key_tip_mode());
        assert_eq!(r.current_key_tips().count(), 5);
        let file_tip = r
            .current_key_tips()
            .iter()
            .find(|entry| {
                entry
                    .tip
                    .as_ref()
                    .is_some_and(|tip| tip.to_utf8().unwrap() == "F")
            })
            .unwrap();
        assert!(
            file_tip
                .target
                .unwrap()
                .ptr_eq(&r.application_button().unwrap())
        );
        assert!(r.active_key_tips().count() > 0);
        send(&r, Key::H);
        settle();
        send(&r, Key::B);
        settle();
        assert!(!r.is_key_tip_mode());
        assert_eq!(model.bold(), Some(true)); // Pinned Uno toggle peer invokes its command, then source InvokeCore executes it again.
        assert_eq!(model.document().matches("Bold applied").count(), 2);
        model.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        settle();
        send(&r, Key::F10);
        send(&r, Key::H);
        settle();
        assert!(r.is_minimized_popup_open());
        let copy = r
            .get_all_items()
            .iter()
            .find_map(|c| {
                c.cast::<RibbonButton>()
                    .filter(|b| b.id().as_deref() == Some("copy"))
            })
            .unwrap();
        send(&copy, Key::C);
        settle();
        assert_eq!(model.document().matches("Copy applied").count(), 1);
        assert!(!r.is_key_tip_mode());
        assert!(!r.is_minimized_popup_open());
        model.set_visibility_mode(RibbonVisibilityMode::AlwaysShow);
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        settle();
        send(&r, Key::F10);
        send(&r, Key::H);
        settle();
        send(&r, Key::M);
        settle();
        assert!(
            r.current_key_tips()
                .iter()
                .any(|e| e.tip == Some("A".into()))
        );
        // Menu host receives the key, then the original command updates the VM once.
        let target = r
            .current_key_tips()
            .iter()
            .find(|e| e.tip == Some("A".into()))
            .unwrap()
            .target
            .unwrap();
        send(&target, Key::A);
        settle();
        assert_eq!(model.document().matches("Select all applied").count(), 1);
        assert!(!r.is_key_tip_mode());
        w.close();
        w.set_content(None);
        settle();
    });
}
#[test]
fn actual_qat_save_linked_copy_moves_and_teardown_use_public_library_controls() {
    const NAME: &str =
        "tests::actual_qat_save_linked_copy_moves_and_teardown_use_public_library_controls";
    if std::env::var_os("FERRORIBBON_SAMPLE_QAT_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_SAMPLE_QAT_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        return;
    }
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonButton};
        use ferroribbon_core::RibbonQuickAccessPosition;
        let weak = {
            let model = SampleViewModel::new(Rc::new(InMemoryDocument));
            let w = create_window(model.clone());
            w.show();
            w.measure(Size::new(1140., 700.));
            w.arrange(Rect::new(0., 0., 1140., 700.));
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            let r = w
                .get_visual_descendants()
                .find_map(|c| c.cast::<Ribbon>())
                .unwrap();
            let weak = r.downgrade();
            let q = r.quick_access_tool_bar().unwrap();
            let save = q.items().get(0).unwrap().cast::<RibbonButton>().unwrap();
            assert_eq!(save.id().as_deref(), Some("save"));
            save.perform_click();
            assert_eq!(model.document().matches("Save applied").count(), 1);
            assert!(r.add_to_quick_access_id(Some("cut".into())));
            let source = r
                .find_item(Some("cut".into()))
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            let copy = r
                .find_quick_access_copy(Some(source.clone().upcast()))
                .unwrap()
                .cast::<RibbonButton>()
                .unwrap();
            copy.perform_click();
            assert_eq!(model.document().matches("Cut applied").count(), 1);
            source.set_label(Some("Cut selection".into()));
            assert_eq!(copy.label().as_deref(), Some("Cut selection"));
            r.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert!(q.items().get(1).unwrap().ptr_eq(&copy));
            assert!(q.get_visual_root().is_some());
            assert!(r.remove_from_quick_access(&source));
            source.set_label(Some("Changed after removal".into()));
            assert_eq!(copy.label().as_deref(), Some("Cut selection"));
            w.close();
            w.set_content(None);
            weak
        };
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(
            weak.upgrade().is_none(),
            "unretained actual QAT/root/template/VM must release"
        );
    });
}
#[test]
fn actual_sample_floating_clipboard_uses_same_commands_slide_out_and_public_lifecycle() {
    const NAME: &str =
        "tests::actual_sample_floating_clipboard_uses_same_commands_slide_out_and_public_lifecycle";
    if std::env::var_os("FERRORIBBON_SAMPLE_FLOAT_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_SAMPLE_FLOAT_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        return;
    }
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonButton};
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let r = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        assert!(r.can_float_groups());
        let group = r.tabs().get(0).unwrap().groups().get(0).unwrap();
        let panel = group.items_panel();
        let cut = r
            .find_item(Some("cut".into()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        group.float(Some(ferroui_base::Point::new(80., 300.)));
        assert!(group.is_floating_panel_open());
        assert!(panel.ptr_eq(&group.items_panel()));
        assert!(r.select_tab_id("insert"));
        assert!(group.is_floating_panel_open());
        cut.perform_click();
        assert_eq!(model.document().matches("Cut applied").count(), 1);
        let history = group
            .slide_out_items()
            .get(1)
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        assert!(history.get_visual_root().is_some());
        history.perform_click();
        assert_eq!(
            model
                .document()
                .matches("Clipboard history applied")
                .count(),
            1
        );
        r.suspend_popups();
        assert!(group.is_floating());
        assert!(!group.is_floating_panel_open());
        r.resume_popups();
        assert!(group.is_floating_panel_open());
        r.return_all_panels_to_ribbon();
        assert!(!group.is_floating());
        assert!(group.items_panel().ptr_eq(&panel));
        w.close();
        w.set_content(None);
    });
}
#[test]
fn actual_sample_ribbon_state_actions_restore_layout_through_public_api() {
    const NAME: &str =
        "tests::actual_sample_ribbon_state_actions_restore_layout_through_public_api";
    if std::env::var_os("FERRORIBBON_SAMPLE_STATE_ISOLATED").is_none() {
        let o = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_SAMPLE_STATE_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            o.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        return;
    }
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonButton};
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let ribbon = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        let click = |label: &str| {
            let item = ribbon
                .tabs()
                .iter()
                .flatten()
                .flat_map(|t| t.groups().iter().flatten())
                .flat_map(|g| g.items().iter().flatten())
                .find_map(|c| {
                    c.cast::<RibbonButton>()
                        .filter(|b| b.label().as_deref() == Some(label))
                })
                .unwrap();
            item.perform_click();
        };
        click("Restore Ribbon");
        assert!(model.status().contains("Save a ribbon layout first"));
        model.set_simplified(Some(true));
        ribbon.select_tab_id("view");
        ribbon.add_to_quick_access_id(Some("cut".into()));
        click("Save Ribbon");
        assert!(model.status().contains("saved"));
        model.set_simplified(Some(false));
        ribbon.select_tab_id("home");
        click("Reset Quick Access");
        assert_eq!(ribbon.get_quick_access_item_ids().count(), 1);
        let document = model.document();
        click("Restore Ribbon");
        assert_eq!(ribbon.selected_tab().unwrap().id().as_deref(), Some("view"));
        assert_eq!(model.simplified(), Some(true));
        assert_eq!(ribbon.get_quick_access_item_ids().count(), 2);
        assert_eq!(model.document(), document);
        let c = ferroribbon_core::state::RibbonCustomization::new();
        c.labels()
            .unwrap()
            .insert("home".into(), Some("Start".into()));
        ribbon.apply_customization(Some(c)).unwrap();
        click("Reset Customization");
        assert_eq!(
            ribbon.find_tab("home").unwrap().header().as_deref(),
            Some("Home")
        );
        w.close();
        w.set_content(None);
    });
}
#[test]
fn actual_text_input_shortcut_focus_two_way_text_and_submit_use_reactive_search() {
    const NAME: &str =
        "tests::actual_text_input_shortcut_focus_two_way_text_and_submit_use_reactive_search";
    if std::env::var_os("FERRORIBBON_SAMPLE_TEXT_ISOLATED").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_SAMPLE_TEXT_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonTextBox};
        use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.measure(Size::new(1400., 700.));
        w.arrange(Rect::new(0., 0., 1400., 700.));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let r = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        let input = r
            .find_item(Some("find-text".into()))
            .unwrap()
            .cast::<RibbonTextBox>()
            .unwrap();
        let inner = input
            .get_visual_descendants()
            .find_map(|c| c.cast::<TextBox>())
            .unwrap();
        let mut shortcut = KeyEventArgs::new();
        shortcut.key = Key::F;
        shortcut.key_modifiers = KeyModifiers::CONTROL;
        shortcut.set_routed_event(Some(InputElement::key_down_event()));
        r.raise_event(&shortcut);
        assert!(shortcut.handled());
        assert!(inner.is_focused());
        let document = model.document();
        inner.set_text(Some("FerroRibbon"));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(model.search_text(), "FerroRibbon");
        let mut enter = KeyEventArgs::new();
        enter.key = Key::Enter;
        enter.set_routed_event(Some(InputElement::key_down_event()));
        inner.raise_event(&enter);
        assert!(enter.handled());
        assert_eq!(model.status(), "Find “FerroRibbon”: 1 matches.");
        assert_eq!(model.document(), document);
        model.set_search_text("absent query".into());
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(inner.text().as_deref(), Some("absent query"));
        w.close();
        w.set_content(None);
    });
}

#[test]
fn actual_font_combo_updates_reactive_font_and_document_through_public_control() {
    run(|| {
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(vm.clone());
        w.show();
        w.measure(Size::new(1400., 700.));
        w.arrange(Rect::new(0., 0., 1400., 700.));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let combo = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<ferroribbon::controls::RibbonFontComboBox>())
            .unwrap();
        assert_eq!(combo.items().count(), 30);
        combo.commit_text(Some("Consolas".into()));
        assert_eq!(vm.font_family(), "Consolas");
        assert_eq!(vm.status(), "Font: Consolas");
        let editor = w
            .get_visual_descendants()
            .find_map(|c| {
                c.cast::<TextBox>()
                    .filter(|c| c.name().as_deref() == Some("Document"))
            })
            .unwrap();
        assert_eq!(
            editor.font_family(),
            ferroui_base::media::FontFamily::new("Consolas")
        );
        vm.set_font_family("Arial".into());
        assert_eq!(combo.text().as_deref(), Some("Arial"));
        w.close();
    });
}

#[test]
fn actual_spinner_commits_reactive_font_size_and_preserves_programmatic_updates() {
    run(|| {
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(vm.clone());
        w.show();
        w.measure(Size::new(1400., 700.));
        w.arrange(Rect::new(0., 0., 1400., 700.));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let spinner = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<ferroribbon::controls::RibbonSpinner>())
            .unwrap();
        let input = spinner
            .get_visual_descendants()
            .find_map(|c| c.cast::<TextBox>())
            .unwrap();
        let editor = w
            .get_visual_descendants()
            .find_map(|c| {
                c.cast::<TextBox>()
                    .filter(|c| c.name().as_deref() == Some("Document"))
            })
            .unwrap();
        assert_eq!(input.text().as_deref(), Some("17 pt"));
        input.set_text(Some("24 pt"));
        spinner.commit_text();
        assert_eq!(vm.font_size(), 24.);
        assert_eq!(vm.status(), "Font size: 24");
        assert_eq!(editor.font_size(), 24.);
        vm.set_font_size(17.);
        assert_eq!(spinner.value(), 17.);
        assert_eq!(editor.font_size(), 17.);
        assert_eq!(vm.status(), "Font size: 24");
        w.close();
    });
}

#[test]
fn actual_clipboard_expansion_has_visible_same_controls_in_popup_root() {
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonButton};
        use ferroui_base::threading::Dispatcher;
        use ferroui_controls::{TopLevel, primitives::PopupRoot};
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let ribbon = w
            .get_visual_descendants()
            .find_map(|v| v.cast::<Ribbon>())
            .unwrap();
        let group = ribbon.tabs().get(0).unwrap().groups().get(0).unwrap();
        let extra = group
            .slide_out_items()
            .get(0)
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        let identity = extra.clone();
        assert!(!extra.is_attached_to_visual_tree());
        group.open_slide_out();
        Dispatcher::ui_thread().run_jobs(None);
        let root =
            TopLevel::get_top_level(Some(&extra)).expect("expanded command must be attached");
        root.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        eprintln!(
            "expanded root={:?}, child={:?}, origin={:?}, separate={}",
            root.bounds(),
            extra.bounds(),
            extra.translate_point(ferroui_base::Point::default(), &root),
            root.cast::<PopupRoot>().is_some()
        );
        assert!(group.is_slide_out_open());
        assert!(extra.is_effectively_visible());
        assert!(extra.bounds().width > 0. && extra.bounds().height > 0.);
        assert!(root.bounds().width > 0. && root.bounds().height > 0.);
        assert!(extra.ptr_eq(&identity));
        extra.perform_click();
        assert_eq!(
            model.status(),
            "Paste special applied to the sample document."
        );
        group.close_slide_out();
        assert!(!extra.is_attached_to_visual_tree());
        assert!(group.slide_out_items().get(0).unwrap().ptr_eq(&identity));
        w.close();
    });
}

#[test]
fn actual_slider_changes_document_opacity_through_reactive_binding_and_raw_input() {
    run(|| {
        use ferroribbon::{
            compat::RibbonInputSlider,
            controls::{Ribbon, RibbonSlider},
        };
        use ferroui_base::{
            Point, Vector,
            input::{MouseButton, RawInputModifiers},
            threading::Dispatcher,
        };
        use ferroui_controls::Border;
        use ferroui_headless::HeadlessWindowExtensions;
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(vm.clone());
        w.show();
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let ribbon = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        ribbon.set_selected_tab_id(Some("view".into()));
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let slider = ribbon
            .find_item(Some("opacity".into()))
            .unwrap()
            .cast::<RibbonSlider>()
            .unwrap();
        let page = w
            .get_visual_descendants()
            .find_map(|c| {
                c.cast::<Border>()
                    .filter(|b| b.name().as_deref() == Some("DocumentPage"))
            })
            .unwrap();
        assert_eq!(slider.value(), 100.);
        assert_eq!(page.opacity(), 1.);
        vm.set_opacity_percent(60.);
        assert_eq!(slider.value(), 60.);
        assert_eq!(page.opacity(), 0.6);
        assert_eq!(vm.status(), "Ready");
        let p = slider
            .get_visual_descendants()
            .find_map(|c| c.cast::<RibbonInputSlider>())
            .unwrap();
        let track = p.track().unwrap();
        w.update_layout();
        let thumb = track.thumb().unwrap();
        let point = track
            .translate_point(
                Point::new(
                    thumb.bounds().width / 2. + 0.4 * (track.bounds().width - thumb.bounds().width),
                    track.bounds().height / 2.,
                ),
                &w,
            )
            .unwrap();
        w.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        w.mouse_up(point, MouseButton::Left, RawInputModifiers::NONE);
        assert_eq!(vm.opacity_percent(), 40.);
        assert_eq!(page.opacity(), 0.4);
        assert_eq!(vm.status(), "Opacity: 40%");
        let replacement = SampleViewModel::new(Rc::new(InMemoryDocument));
        w.set_data_context(Some(replacement.clone()));
        assert_eq!(slider.value(), 100.);
        assert_eq!(page.opacity(), 1.);
        p.set_numeric_value(25.);
        assert_eq!(replacement.opacity_percent(), 25.);
        assert_eq!(replacement.status(), "Opacity: 25%");
        assert_eq!(page.opacity(), 0.25);
        assert_eq!(vm.opacity_percent(), 40.);
        w.mouse_move(point + Vector::new(1., 0.), RawInputModifiers::NONE);
        assert_eq!(replacement.opacity_percent(), 25.);
        w.close();
    });
}

#[test]
fn actual_file_backstage_keyboard_page_identity_and_reactive_save_action() {
    run(|| {
        use ferroribbon::controls::Ribbon;
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(model.clone());
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let ribbon = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        let backstage = ribbon.backstage().unwrap();
        let page = backstage.items().get(0).unwrap();
        let content = page.content().unwrap();
        ribbon.application_button().unwrap().on_click();
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(ribbon.is_backstage_open());
        assert_eq!(backstage.selected_item(), Some(page.clone()));
        assert!(
            ferroui_base::data::core::ValueTypes::as_object(&*content)
                .unwrap()
                .cast::<Control>()
                .unwrap()
                .is_loaded()
        );
        let navigation = page.navigation_button().unwrap();
        for key in [ferroui_base::input::Key::F10, ferroui_base::input::Key::S] {
            let mut e = ferroui_base::input::KeyEventArgs::new();
            e.key = key;
            e.set_routed_event(Some(ferroui_base::input::InputElement::key_down_event()));
            navigation.raise_event(&e);
            assert!(e.handled());
        }
        assert!(!ribbon.is_backstage_open());
        assert_eq!(
            model
                .document()
                .matches("Save applied to the sample document.")
                .count(),
            1
        );
        ribbon.invoke_application_button();
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(Rc::ptr_eq(&page.content().unwrap(), &content));
        assert_eq!(backstage.selected_item(), Some(page));
        ribbon.suspend_popups();
        assert!(!ribbon.is_backstage_open());
        w.close();
        w.set_content(None);
    });
}

#[test]
fn actual_sample_insert_table_popup_uses_reactive_command_and_same_picker() {
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonDropDownButton, RibbonGridPicker};
        use ferroribbon_core::model::RibbonGridSize;
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        vm.set_document("Before".to_owned());
        let w = create_window(vm.clone());
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        let r = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        let insert = r.tabs().get(1).unwrap();
        r.set_selected_tab(Some(insert));
        w.update_layout();
        let invoked = Rc::new(std::cell::Cell::new(0));
        let out = invoked.clone();
        r.add_item_invoked(Some(ferroui_base::metadata::MarkupDelegate::new(
            move |_| {
                out.set(out.get() + 1);
                None
            },
        )));
        let button = r
            .find_item(Some("insert-table".into()))
            .unwrap()
            .cast::<RibbonDropDownButton>()
            .unwrap();
        let flyout = button
            .flyout()
            .unwrap()
            .cast::<ferroui_controls::Flyout>()
            .unwrap();
        let picker = Control::from_boxed(&flyout.content().unwrap())
            .unwrap()
            .cast::<RibbonGridPicker>()
            .unwrap();
        flyout.show_at(&button);
        assert!(flyout.is_open());
        picker.highlight(RibbonGridSize::new(1, 2));
        for key in [
            ferroui_base::input::Key::Right,
            ferroui_base::input::Key::Down,
            ferroui_base::input::Key::Enter,
        ] {
            let mut e = ferroui_base::input::KeyEventArgs::new();
            e.key = key;
            e.set_routed_event(Some(ferroui_base::input::InputElement::key_down_event()));
            picker.raise_event(&e);
            assert!(e.handled());
        }
        assert!(!flyout.is_open());
        assert_eq!(
            vm.status(),
            "Insert 3x2 table applied to the sample document."
        );
        assert_eq!(
            vm.document(),
            "Before\n\nInsert 3x2 table applied to the sample document."
        );
        assert_eq!(invoked.get(), 1);
        flyout.show_at(&button);
        let same = Control::from_boxed(&flyout.content().unwrap()).unwrap();
        assert!(same.ptr_eq(&picker));
        flyout.hide();
        w.close();
    });
}
#[test]
fn actual_font_color_palette_updates_passive_document_through_core_reactive_command() {
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonColorPicker};
        use ferroui_base::{media::Color, threading::Dispatcher};
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(vm.clone());
        w.show();
        w.measure(Size::new(1400., 800.));
        w.arrange(Rect::new(0., 0., 1400., 800.));
        let r = w
            .get_visual_descendants()
            .find_map(|v| v.cast::<Ribbon>())
            .unwrap();
        let picker = r
            .find_item(Some("font-color".into()))
            .unwrap()
            .cast::<RibbonColorPicker>()
            .unwrap();
        assert_eq!(picker.selected_color(), Some(Color::from_rgb(32, 37, 48)));
        picker.open_drop_down();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(picker.is_drop_down_open());
        assert!(picker.palette().get_visual_root().is_some());
        picker
            .palette()
            .select(Some(Color::from_argb(128, 17, 34, 51)));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!picker.is_drop_down_open());
        assert_eq!(
            vm.font_color(),
            Some(ferroribbon_core::theming::RibbonColor::new(128, 17, 34, 51))
        );
        assert_eq!(vm.status(), "Font color: #80112233");
        let document = w
            .get_visual_descendants()
            .filter_map(|v| v.cast::<TextBox>())
            .find(|v| v.name().as_deref() == Some("Document"))
            .unwrap();
        assert_eq!(
            document
                .foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Color::from_argb(128, 17, 34, 51)
        );
        picker.palette().select(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(vm.font_color(), None);
        assert_eq!(vm.status(), "Font color: No Color");
        assert_eq!(
            document
                .foreground()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Color::from_rgb(32, 37, 48)
        );
        w.close();
        w.set_content(None);
    });
}
#[test]
fn actual_gallery_uses_compiled_items_source_layout_and_reactive_command() {
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonGallery, RibbonItemLayout};
        use ferroui_base::threading::Dispatcher;
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(vm.clone());
        w.show();
        w.measure(Size::new(1400., 800.));
        w.arrange(Rect::new(0., 0., 1400., 800.));
        let r = w
            .get_visual_descendants()
            .find_map(|v| v.cast::<Ribbon>())
            .unwrap();
        r.set_selected_tab_id(Some("insert".into()));
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let g = r
            .find_item(Some("styles-gallery".into()))
            .unwrap()
            .cast::<RibbonGallery>()
            .unwrap();
        assert_eq!(g.items().count(), 5);
        g.apply_layout(RibbonItemLayout::new(
            ferroribbon_core::RibbonItemSize::Medium,
            g.metrics(),
        ));
        assert_eq!(g.columns(), 2);
        g.open_drop_down();
        assert!(g.is_drop_down_open());
        g.pick(g.effective_items().get(1));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!g.is_drop_down_open());
        assert_eq!(
            vm.status(),
            "Heading 1 style applied to the sample document."
        );
        assert!(
            vm.document()
                .ends_with("Heading 1 style applied to the sample document.")
        );
        w.close();
    });
}

#[test]
fn actual_standalone_toolbars_route_commands_and_menu_actions_switch_retained_contexts() {
    run(|| {
        use ferroribbon::compat::MenuFlyoutItem;
        use ferroribbon::controls::{
            RibbonButton, RibbonContextualToolBar, RibbonMenuBar, RibbonToolBar,
        };
        use ferroui_base::{controls::NameScopeExtensions, threading::Dispatcher};
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(vm.clone());
        w.show();
        w.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        let tools = NameScopeExtensions::find::<RibbonToolBar>(&w, "DocumentToolbar").unwrap();
        assert!(tools.overflow_items().count() > 0);
        let first = tools
            .items()
            .get(0)
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        first.perform_click();
        assert_eq!(vm.status(), "Save applied to the sample document.");
        let contexts =
            NameScopeExtensions::find::<RibbonContextualToolBar>(&w, "ToolOptions").unwrap();
        let text = NameScopeExtensions::find::<RibbonToolBar>(&w, "TextOptions").unwrap();
        let draw = NameScopeExtensions::find::<RibbonToolBar>(&w, "DrawOptions").unwrap();
        assert!(text.is_visible());
        assert!(!draw.is_visible());
        let bar = NameScopeExtensions::find::<RibbonMenuBar>(&w, "CommandMenus").unwrap();
        assert!(bar.open_menu(Some("TOOLS")));
        assert!(bar.is_menu_open());
        let action = NameScopeExtensions::find::<MenuFlyoutItem>(&w, "DrawToolsAction").unwrap();
        action.invoke_menu_core();
        assert_eq!(vm.active_tool(), "draw");
        assert_eq!(contexts.active_context().as_deref(), Some("draw"));
        assert!(!text.is_visible());
        assert!(draw.is_visible());
        assert!(contexts.items().get(0).unwrap().ptr_eq(&text));
        assert!(contexts.items().get(1).unwrap().ptr_eq(&draw));
        w.close();
    });
}

#[test]
fn actual_command_search_uses_ribbon_index_and_executes_reactive_save_once() {
    const NAME: &str =
        "tests::actual_command_search_uses_ribbon_index_and_executes_reactive_save_once";
    if std::env::var_os("FERRORIBBON_SAMPLE_COMMAND_SEARCH_ISOLATED").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", NAME, "--nocapture"])
            .env("FERRORIBBON_SAMPLE_COMMAND_SEARCH_ISOLATED", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonSearchBox};
        use ferroui_base::{
            input::{InputElement, Key, KeyEventArgs, KeyModifiers},
            threading::Dispatcher,
        };
        let model = SampleViewModel::new(Rc::new(InMemoryDocument));
        let window = create_window(model.clone());
        window.show();
        window.measure(Size::new(1400., 850.));
        window.arrange(Rect::new(0., 0., 1400., 850.));
        Dispatcher::ui_thread().run_jobs(None);
        let ribbon = window
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        let search = window
            .get_visual_descendants()
            .find_map(|c| c.cast::<RibbonSearchBox>())
            .unwrap();
        assert!(search.ribbon().unwrap().ptr_eq(&ribbon));
        let part = search
            .get_visual_descendants()
            .find_map(|c| c.cast::<TextBox>())
            .unwrap();
        let mut activate = KeyEventArgs::new();
        activate.set_routed_event(Some(InputElement::key_down_event()));
        activate.key = Key::Q;
        activate.key_modifiers = KeyModifiers::ALT;
        ribbon.raise_event(&activate);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(activate.handled());
        assert!(part.is_focused());
        assert!(search.is_results_open());
        let trace = Rc::new(std::cell::RefCell::new(Vec::new()));
        let observed = trace.clone();
        let _query_subscription = search.query_submitting(move |view, args| {
            observed.borrow_mut().push(format!(
                "query={:?}, open={}",
                args.query(),
                view.is_results_open()
            ));
        });
        let observed = trace.clone();
        part.got_focus(move |_, _| observed.borrow_mut().push("got-focus".to_string()));
        let observed = trace.clone();
        let weak = part.downgrade();
        part.text_changed(move |_, _| {
            observed
                .borrow_mut()
                .push(format!("text-changed={:?}", weak.upgrade().unwrap().text()))
        });
        let calls = Rc::new(std::cell::Cell::new(0));
        let observed = calls.clone();
        let result_trace = trace.clone();
        let _result_subscription = search.result_executed(move |_, _| {
            observed.set(observed.get() + 1);
            result_trace
                .borrow_mut()
                .push("result-executed".to_string());
        });
        for prefix in ["S", "Sa", "Sav", "Save"] {
            part.set_text(Some(prefix));
            Dispatcher::ui_thread().run_jobs(None);
        }
        let mut enter = KeyEventArgs::new();
        enter.set_routed_event(Some(InputElement::key_down_event()));
        enter.key = Key::Enter;
        part.raise_event(&enter);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(enter.handled());
        assert_eq!(calls.get(), 1);
        assert_eq!(model.status(), "Save applied to the sample document.");
        assert_eq!(
            model
                .document()
                .matches("Save applied to the sample document.")
                .count(),
            1
        );
        assert_eq!(part.text().as_deref(), Some(""));
        assert!(
            !search.is_results_open(),
            "search trace: {:?}",
            trace.borrow()
        );
        window.close();
        window.set_content(None);
    });
}

#[test]
fn actual_title_bar_hosts_original_qat_search_and_app_menu() {
    run(|| {
        use ferroribbon::controls::{Ribbon, RibbonTitleBar};
        use ferroribbon_core::RibbonQuickAccessPosition;
        use ferroui_base::interactivity::RoutedEventArgs;
        use ferroui_controls::Button;
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let w = create_window(vm);
        w.show();
        w.measure(Size::new(1140., 700.));
        w.arrange(Rect::new(0., 0., 1140., 700.));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let title = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<RibbonTitleBar>())
            .unwrap();
        let ribbon = w
            .get_visual_descendants()
            .find_map(|c| c.cast::<Ribbon>())
            .unwrap();
        assert!(title.ribbon().unwrap().ptr_eq(&ribbon));
        assert_eq!(
            title.display_title().as_deref(),
            Some("FerroRibbon document")
        );
        let search = title.search_box().unwrap();
        assert!(search.ribbon().unwrap().ptr_eq(&ribbon));
        assert!(search.is_visible());
        let qat = ribbon.quick_access_tool_bar().unwrap();
        assert!(title.get_visual_descendants().any(|v| v.ptr_eq(&qat)));
        ribbon.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(!title.get_visual_descendants().any(|v| v.ptr_eq(&qat)));
        assert!(ribbon.get_visual_descendants().any(|v| v.ptr_eq(&qat)));
        ribbon.set_quick_access_position(RibbonQuickAccessPosition::AboveRibbon);
        assert!(title.get_visual_descendants().any(|v| v.ptr_eq(&qat)));
        title
            .app_button()
            .unwrap()
            .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        assert!(ribbon.is_backstage_open());
        ribbon.set_is_backstage_open(false);
        w.close();
        w.set_content(None);
    });
}

#[test]
fn actual_sample_customization_actions_use_reactive_commands_and_native_dialog() {
    run(|| {
        use ferroribbon::controls::{
            IRibbonItem, Ribbon, RibbonCustomizeDialog, RibbonCustomizePage, RibbonDialogHost,
        };
        use ferroui_base::data::core::{ValueType, ValueTypes};
        use ferroui_controls::primitives::PopupOverlayLayer;
        let vm = SampleViewModel::new(Rc::new(InMemoryDocument));
        let window = create_window(vm.clone());
        window.show();
        window.update_layout();
        let ribbon = window
            .get_visual_descendants()
            .find_map(|v| v.cast::<Ribbon>())
            .unwrap();
        ribbon.set_selected_tab_id(Some("view".into()));
        let calls = Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen = calls.clone();
        let subscription =
            ribbon.customize_requested(move |args| seen.borrow_mut().push(args.page));
        for (id, page) in [
            ("customize-ribbon", RibbonCustomizePage::Ribbon),
            ("customize-qat", RibbonCustomizePage::QuickAccessToolbar),
        ] {
            let control = ribbon.find_item(Some(id.into())).unwrap();
            let item = ValueTypes::try_cast(
                &Control::boxed(control),
                ValueType::of::<Rc<dyn IRibbonItem>>(),
            )
            .unwrap();
            assert!(item.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap().invoke());
            let dialog = PopupOverlayLayer::get_popup_overlay_layer(&window)
                .unwrap()
                .children()
                .snapshot()
                .iter()
                .find_map(|v| v.cast::<RibbonCustomizeDialog>())
                .unwrap();
            assert_eq!(dialog.page(), page);
            assert!(dialog.is_open());
            assert!(
                dialog
                    .cast::<RibbonDialogHost>()
                    .unwrap()
                    .primary_button()
                    .is_some()
            );
            assert_eq!(vm.status(), "Ribbon customization requested.");
            dialog.close(false).unwrap();
        }
        assert_eq!(
            *calls.borrow(),
            [
                RibbonCustomizePage::Ribbon,
                RibbonCustomizePage::QuickAccessToolbar
            ]
        );
        subscription.dispose();
        window.close();
        window.set_content(None);
    });
}
