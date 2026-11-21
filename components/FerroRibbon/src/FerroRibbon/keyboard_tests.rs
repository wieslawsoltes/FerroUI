//! Native source keyboard/scopes/badges/popup/focus integration.
use super::*;
use crate::controls::{RibbonDropDownButton, RibbonToggleButton};
use ferroui_base::metadata::MarkupTyped;
use ferroui_controls::{Panel, TextBox, Window};
fn isolated(name: &str) -> bool {
    let name = name
        .strip_prefix(concat!(env!("CARGO_CRATE_NAME"), "::"))
        .unwrap_or(name);
    if std::env::var("FERRORIBBON_KEYBOARD_ISOLATED").as_deref() == Ok(name) {
        return false;
    }
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_KEYBOARD_ISOLATED", name)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&o.stdout).contains("running 1 test\n") && o.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    true
}
fn settle() {
    Dispatcher::ui_thread().run_jobs(None);
}
fn layout(w: &Window, width: f64) {
    w.set_width(width);
    w.measure(Size::new(width, 400.));
    w.arrange(Rect::new(0., 0., width, 400.));
    settle();
}
fn setup() -> (
    Ref<Ribbon>,
    Ref<Window>,
    Ref<RibbonGroup>,
    Ref<RibbonToggleButton>,
) {
    let r = Ribbon::new();
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    let tab = RibbonTab::new();
    tab.set_id(Some("home".into()));
    tab.set_header(Some("Home".into()));
    tab.set_key_tip(Some("H".into()));
    let group = RibbonGroup::new();
    group.set_header(Some("Tools".into()));
    group.set_key_tip(Some("G".into()));
    let b = RibbonToggleButton::new();
    b.set_id(Some("bold".into()));
    b.set_label(Some("Bold".into()));
    b.set_key_tip(Some("1".into()));
    group.items().add(Some(b.clone().upcast()));
    tab.groups().add(Some(group.clone()));
    r.tabs().add(Some(tab));
    let second = RibbonTab::new();
    second.set_header(Some("Insert".into()));
    second.set_key_tip(Some("N".into()));
    r.tabs().add(Some(second));
    let w = crate::ribbon_tests::mount(&r);
    // Match the independent sample's explicit control-theme imports. Ribbon.xaml
    // currently includes push buttons; checked controls have their own dictionary.
    let value = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Checked.xaml")
        .unwrap()
        .unwrap();
    let dictionary = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ferroui_base::controls::ResourceDictionary>()
        .unwrap();
    r.resources().add_merged_dictionary(dictionary);
    layout(&w, 1400.);
    (r, w, group, b)
}
fn send(control: &Control, key: Key, up: bool, mods: KeyModifiers, handled: bool) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.key_modifiers = mods;
    e.set_routed_event(Some(if up {
        InputElement::key_up_event()
    } else {
        InputElement::key_down_event()
    }));
    e.set_handled(handled);
    control.raise_event(&e);
    e
}

#[test]
fn native_focus_root_keys_activate_without_a_focused_content_element() {
    if isolated(concat!(
        module_path!(),
        "::native_focus_root_keys_activate_without_a_focused_content_element"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        use ferroui_base::input::{KeyboardDevice, PhysicalKey, RawInputModifiers};
        use ferroui_headless::HeadlessWindowExtensions;

        let (ribbon, window, _, _) = setup();
        let keyboard = KeyboardDevice::instance().unwrap();
        keyboard.set_focused_element(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
        assert!(keyboard.focused_element().is_none());
        window.key_press(Key::F10, RawInputModifiers::NONE, PhysicalKey::None, None);
        window.key_release(Key::F10, RawInputModifiers::NONE, PhysicalKey::None, None);
        settle();
        assert!(
            ribbon.is_key_tip_mode(),
            "native focus-root key missed the content hook"
        );
        assert!(keyboard.focused_element().is_some());

        // A content-originating key must still be handled only once.
        window.key_press(Key::F10, RawInputModifiers::NONE, PhysicalKey::None, None);
        window.key_release(Key::F10, RawInputModifiers::NONE, PhysicalKey::None, None);
        settle();
        assert!(!ribbon.is_key_tip_mode());

        keyboard.set_focused_element(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
        window.key_press(
            Key::LeftAlt,
            RawInputModifiers::ALT,
            PhysicalKey::None,
            None,
        );
        window.key_release(
            Key::LeftAlt,
            RawInputModifiers::NONE,
            PhysicalKey::None,
            None,
        );
        settle();
        assert!(
            ribbon.is_key_tip_mode(),
            "native focus-root key-up missed the content hook"
        );
        ribbon.hide_key_tips();
        settle();
        window.close();
    });
}

#[test]
fn native_focus_root_hooks_follow_reparenting_and_leave_the_old_window() {
    if isolated(concat!(
        module_path!(),
        "::native_focus_root_hooks_follow_reparenting_and_leave_the_old_window"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        use ferroui_base::input::{KeyboardDevice, PhysicalKey, RawInputModifiers};
        use ferroui_headless::HeadlessWindowExtensions;

        let (ribbon, first, _, _) = setup();
        first.set_content(None);
        settle();
        let second = Window::new();
        second.set_content(Some(Control::boxed(ribbon.clone())));
        second.show();
        layout(&second, 1400.);
        let keyboard = KeyboardDevice::instance().unwrap();
        keyboard.set_focused_element(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
        first.key_press(Key::F10, RawInputModifiers::NONE, PhysicalKey::None, None);
        first.key_release(Key::F10, RawInputModifiers::NONE, PhysicalKey::None, None);
        settle();
        assert!(
            !ribbon.is_key_tip_mode(),
            "old owner retained its keyboard subscription"
        );

        send(&second, Key::F10, false, KeyModifiers::NONE, true);
        assert!(
            !ribbon.is_key_tip_mode(),
            "already handled owner key must be left alone"
        );
        second.key_press(Key::F10, RawInputModifiers::NONE, PhysicalKey::None, None);
        second.key_release(Key::F10, RawInputModifiers::NONE, PhysicalKey::None, None);
        settle();
        assert!(ribbon.is_key_tip_mode());
        ribbon.hide_key_tips();
        settle();
        second.close();
        first.close();
    });
}
struct ModeAction(Box<dyn Fn(bool)>);
impl PartialEq for ModeAction {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class ModeAction{this:Rc<ModeAction>,handles:[Rc<ModeAction>],methods:[fn Invoke(Option<BoxedValue>,bool)=>|h:&Rc<ModeAction>,_,v|(h.0)(v)]});
fn mode_action(f: impl Fn(bool) + 'static) -> MarkupDelegate {
    ValueTypes::register_reference::<ModeAction>();
    MarkupDelegate::for_method(
        Some(Rc::new(ModeAction(Box::new(f)))),
        ModeAction::MARKUP,
        &ModeAction::MARKUP.methods[0],
    )
}
#[test]
fn newly_selected_tab_badges_wait_for_target_layout() {
    const NAME: &str =
        "controls::ribbon::keyboard::tests::newly_selected_tab_badges_wait_for_target_layout";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (ribbon, window, _, _) = setup();
        let insert = ribbon.tabs().get(1).unwrap();
        let group = RibbonGroup::new();
        group.set_header(Some("Insert".into()));
        let command = RibbonButton::new();
        command.set_label(Some("Picture".into()));
        command.set_key_tip(Some("P".into()));
        group.items().add(Some(command.clone().upcast()));
        insert.groups().add(Some(group));
        ribbon.show_key_tips();
        settle();
        assert!(ribbon.process_key_tip_input(b'N' as u16));
        Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::NORMAL));
        window.get_layout_manager().unwrap().execute_layout_pass();
        settle();
        assert!(ribbon.selected_tab().unwrap().ptr_eq(&insert));
        assert!(command.bounds().width > 0.);
        assert!(
            ribbon
                .current_key_tips()
                .iter()
                .any(|entry| { entry.target.is_some_and(|target| target.ptr_eq(&command)) })
        );
        let badge = ribbon
            .active_key_tips()
            .iter()
            .flatten()
            .find(|badge| badge.target().is_some_and(|target| target.ptr_eq(&command)))
            .expect("a command on the newly selected tab has a visible badge");
        assert!(badge.is_attached_to_visual_tree());
        assert!(badge.bounds().width > 0. && badge.bounds().height > 0.);
        ribbon.cancel_key_tips();
        window.close();
    });
}
#[test]
fn upstream_levels_invoke_pop_and_real_badges_restore_document_focus() {
    const NAME: &str = "controls::ribbon::keyboard::tests::upstream_levels_invoke_pop_and_real_badges_restore_document_focus";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, _, bold) = setup();
        let host = Panel::new();
        let document = TextBox::new();
        w.set_content(None);
        host.children().add(r.clone());
        host.children().add(document.clone());
        w.set_content(Some(Control::boxed(host)));
        layout(&w, 1400.);
        assert!(document.focus());
        let events = Rc::new(RefCell::new(Vec::new()));
        let trace = events.clone();
        r.add_key_tip_mode_changed(Some(mode_action(move |v| trace.borrow_mut().push(v))));
        r.show_key_tips();
        assert!(r.is_key_tip_mode());
        assert_eq!(&*events.borrow(), &[true]);
        settle();
        let live = r.active_key_tips();
        assert!(live.count() >= 2);
        assert!(Rc::ptr_eq(&r.active_key_tips(), &live));
        let snapshot = r.current_key_tips();
        assert_eq!(snapshot.count(), 2);
        for badge in live.iter() {
            let badge = badge.unwrap();
            assert!(badge.target().is_some());
            assert!(
                badge.desired_size().width > 0. && badge.desired_size().height > 0.,
                "source badge style must measure"
            );
            assert!(!badge.is_hit_test_visible());
        }
        let header = r.tabs().get(0).unwrap().header_element().unwrap();
        assert!(
            w.focus_manager()
                .get_focused_element()
                .unwrap()
                .ptr_eq(&header)
        );
        assert!(r.process_key_tip_input(b'H' as u16));
        settle();
        assert!(
            r.current_key_tips()
                .iter()
                .any(|e| e.tip == Some("1".into()))
        );
        assert_eq!(snapshot.count(), 2, "old snapshot retained");
        assert!(r.process_key_tip_input(b'1' as u16));
        assert_eq!(bold.is_checked(), Some(true));
        assert!(!r.is_key_tip_mode());
        assert_eq!(live.count(), 0);
        settle();
        assert!(
            w.focus_manager()
                .get_focused_element()
                .unwrap()
                .ptr_eq(&document)
        );
        assert_eq!(&*events.borrow(), &[true, false]);
        r.show_key_tips();
        settle();
        r.process_key_tip_input(b'H' as u16);
        settle();
        assert!(r.pop_key_tip_level());
        assert!(!r.pop_key_tip_level());
        assert!(!r.is_key_tip_mode());
        settle();
        w.close();
        w.set_content(None);
    });
}
#[test]
fn alt_f10_handled_precedence_prefix_backspace_and_disabled_targets() {
    const NAME: &str = "controls::ribbon::keyboard::tests::alt_f10_handled_precedence_prefix_backspace_and_disabled_targets";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, g, bold) = setup();
        assert!(!send(&r, Key::LeftAlt, false, KeyModifiers::ALT, false).handled());
        assert!(send(&r, Key::LeftAlt, true, KeyModifiers::NONE, false).handled());
        settle();
        assert!(r.is_key_tip_mode());
        // Active KeyTip mode consumes keys even if a focused control marked them.
        assert!(send(&r, Key::H, false, KeyModifiers::NONE, true).handled());
        settle();
        bold.set_key_tip(Some("AB".into()));
        let second = RibbonButton::new();
        second.set_key_tip(Some("AC".into()));
        second.set_label(Some("Other".into()));
        g.items().add(Some(second.clone().upcast()));
        r.cancel_key_tips();
        r.show_key_tips();
        r.process_key_tip_input(b'H' as u16);
        settle();
        assert!(r.process_key_tip_input(b'A' as u16));
        assert_eq!(r.keyboard.navigator.current().unwrap().typed(), "A".into());
        assert!(!r.process_key_tip_input(0xd800));
        assert_eq!(r.keyboard.navigator.current().unwrap().typed(), "A".into());
        send(&r, Key::Back, false, KeyModifiers::NONE, false);
        assert_eq!(r.keyboard.navigator.current().unwrap().typed(), "".into());
        bold.set_is_enabled(false);
        assert!(r.process_key_tip_input(b'A' as u16));
        assert!(r.process_key_tip_input(b'B' as u16));
        assert!(r.is_key_tip_mode());
        assert_ne!(bold.is_checked(), Some(true));
        send(&r, Key::F10, false, KeyModifiers::NONE, false);
        assert!(!r.is_key_tip_mode());
        assert!(send(&r, Key::F10, false, KeyModifiers::NONE, true).handled());
        assert!(!r.is_key_tip_mode());
        assert!(send(&r, Key::F10, false, KeyModifiers::NONE, false).handled());
        settle();
        assert!(r.is_key_tip_mode());
        send(&r, Key::F10, false, KeyModifiers::NONE, false);
        r.set_is_key_tips_enabled(false);
        assert!(!send(&r, Key::F10, false, KeyModifiers::NONE, false).handled());
        assert!(!r.is_key_tip_mode());
        w.close();
        w.set_content(None);
    });
}
#[test]
fn minimized_fullscreen_collapsed_and_expanded_scope_cleanup() {
    const NAME: &str = "controls::ribbon::keyboard::tests::minimized_fullscreen_collapsed_and_expanded_scope_cleanup";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, g, _) = setup();
        r.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        layout(&w, 1400.);
        r.show_key_tips();
        r.process_key_tip_input(b'H' as u16);
        settle();
        assert!(r.is_minimized_popup_open());
        assert!(r.current_key_tips().count() > 0);
        r.hide_key_tips();
        assert!(r.is_minimized_popup_open(), "Hide retains navigation popup");
        r.close_minimized_popup();
        r.show_key_tips();
        r.process_key_tip_input(b'H' as u16);
        settle();
        r.cancel_key_tips();
        assert!(!r.is_minimized_popup_open());
        r.set_visibility_mode(RibbonVisibilityMode::FullScreen);
        r.show_key_tips();
        r.process_key_tip_input(b'H' as u16);
        settle();
        assert!(r.is_full_screen_revealed());
        assert!(r.pop_key_tip_level());
        assert!(!r.is_full_screen_revealed());
        r.cancel_key_tips();
        r.set_visibility_mode(RibbonVisibilityMode::PanelButtons);
        layout(&w, 1400.);
        r.show_key_tips();
        r.process_key_tip_input(b'H' as u16);
        settle();
        assert!(
            r.current_key_tips()
                .iter()
                .any(|e| e.tip == Some("G".into()))
        );
        r.process_key_tip_input(b'G' as u16);
        settle();
        assert!(g.is_popup_open());
        assert!(r.active_key_tips().count() > 0);
        r.cancel_key_tips();
        assert!(!g.is_popup_open());
        r.set_visibility_mode(RibbonVisibilityMode::AlwaysShow);
        layout(&w, 1400.);
        let slide = RibbonButton::new();
        slide.set_label(Some("Advanced".into()));
        slide.set_key_tip(Some("Z".into()));
        g.slide_out_items().add(Some(slide.upcast()));
        let button = g.slide_out_button().unwrap();
        RibbonKeyTip::set_key_tip(&button, Some("X".into()));
        r.show_key_tips();
        r.process_key_tip_input(b'H' as u16);
        settle();
        r.process_key_tip_input(b'X' as u16);
        settle();
        assert!(g.is_slide_out_open());
        g.set_is_slide_out_pinned(true);
        r.cancel_key_tips();
        assert!(g.is_slide_out_open(), "source pinned cleanup retains panel");
        g.set_is_slide_out_pinned(false);
        g.close_slide_out();
        w.close();
        w.set_content(None);
    });
}
#[test]
fn menu_continuation_invoke_once_and_closed_subscription() {
    const NAME: &str =
        "controls::ribbon::keyboard::tests::menu_continuation_invoke_once_and_closed_subscription";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, g, _) = setup();
        let drop = RibbonDropDownButton::new();
        drop.set_label(Some("Menu".into()));
        drop.set_key_tip(Some("M".into()));
        let menu = MenuFlyout::new();
        let entry = MenuFlyoutItem::new();
        entry.set_text(Some("Run".into()));
        RibbonKeyTip::set_key_tip(&entry, Some("R".into()));
        let calls = Rc::new(Cell::new(0));
        let n = calls.clone();
        entry.add_click_action(move |_, _| n.set(n.get() + 1));
        menu.items().add(Some(Control::boxed(entry.clone())));
        drop.set_flyout(Some(menu.clone().upcast()));
        g.items().add(Some(drop.upcast()));
        layout(&w, 1400.);
        r.show_key_tips();
        r.process_key_tip_input(b'H' as u16);
        settle();
        r.process_key_tip_input(b'M' as u16);
        settle();
        assert!(menu.is_open());
        assert!(
            r.current_key_tips()
                .iter()
                .any(|e| e.tip == Some("R".into()))
        );
        assert!(r.process_key_tip_input(b'R' as u16));
        assert_eq!(calls.get(), 1);
        assert!(!r.is_key_tip_mode());
        assert!(!menu.is_open());
        r.show_key_tips();
        r.process_key_tip_input(b'H' as u16);
        settle();
        r.process_key_tip_input(b'M' as u16);
        settle();
        menu.hide();
        assert!(!r.is_key_tip_mode());
        w.close();
        w.set_content(None);
    });
}
fn reference_case(name: &str) -> serde_json::Value {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/keyboard.json")).unwrap();
    reference["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
        .clone()
}
#[test]
fn unchanged_source_fractional_badge_positions() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/keyboard.json")).unwrap();
    for case in reference["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["name"] == "position")
    {
        let spec = case["spec"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_f64().unwrap())
            .collect::<Vec<_>>();
        let p = badge_position(
            Rect::new(spec[0], spec[1], spec[2], spec[3]),
            Size::new(
                case["width"].as_f64().unwrap(),
                case["height"].as_f64().unwrap(),
            ),
            Size::new(800., 400.),
            spec[4] == 1.,
        );
        assert_eq!(
            (p.x.to_bits(), p.y.to_bits()),
            (
                case["x"].as_f64().unwrap().to_bits(),
                case["y"].as_f64().unwrap().to_bits()
            ),
            "{case}"
        );
    }
}
#[test]
fn qat_badges_use_source_bottom_center_rule_in_both_toolbar_positions() {
    const NAME: &str = "controls::ribbon::keyboard::tests::qat_badges_use_source_bottom_center_rule_in_both_toolbar_positions";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        use ferroribbon_core::RibbonQuickAccessPosition;
        let (r, w, _, _) = setup();
        let qat = r.quick_access_tool_bar().unwrap();
        let save = RibbonButton::new();
        save.set_label(Some("Save".into()));
        qat.items().add(Some(save.clone().upcast()));
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/keyboard.json")).unwrap();
        // This is the whole original RenderBadges output, not a port-computed
        // expected classification. Small QAT entries were incorrectly treated
        // like ordinary small controls before the regression was fixed.
        let case = source["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| {
                c["name"] == "qatPosition"
                    && c["inQat"] == true
                    && c["size"] == 2
                    && c["simplified"] == false
            })
            .unwrap();
        assert_eq!(case["x"].as_f64().unwrap(), 76.75);
        assert_eq!(case["y"].as_f64().unwrap(), 41.);
        for position in [
            RibbonQuickAccessPosition::AboveRibbon,
            RibbonQuickAccessPosition::BelowRibbon,
        ] {
            r.set_quick_access_position(position);
            layout(&w, 1400.);
            r.show_key_tips();
            settle();
            let badge = r
                .active_key_tips()
                .iter()
                .flatten()
                .find(|b| b.target().is_some_and(|t| t.ptr_eq(&save)))
                .expect("QAT badge");
            assert_eq!(save.current_size(), RibbonItemSize::Small);
            let bounds = bounds_in_window(&save, &w).unwrap();
            let expected = badge_position(bounds, badge.desired_size(), w.bounds().size(), true);
            assert_eq!(
                (Canvas::get_left(&badge), Canvas::get_top(&badge)),
                (expected.x, expected.y),
                "{position:?}"
            );
            assert!(!badge.is_hit_test_visible());
            assert!(badge.is_attached_to_visual_tree());
            r.cancel_key_tips();
            settle();
        }
        w.close();
        w.set_content(None);
        settle();
    });
}
#[test]
fn deferred_header_after_hide_matches_source_and_does_not_steal_new_focus() {
    const NAME: &str = "controls::ribbon::keyboard::tests::deferred_header_after_hide_matches_source_and_does_not_steal_new_focus";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, _, _) = setup();
        let events = Rc::new(RefCell::new(Vec::new()));
        let trace = events.clone();
        r.add_key_tip_mode_changed(Some(mode_action(move |v| trace.borrow_mut().push(v))));
        r.show_key_tips();
        r.process_key_tip_input(b'H' as u16);
        r.hide_key_tips();
        settle();
        let source = reference_case("queuedAfterHide");
        assert_eq!(r.is_key_tip_mode(), source["active"].as_bool().unwrap());
        assert_eq!(
            r.keyboard.navigator.depth(),
            source["depth"].as_i64().unwrap() as i32
        );
        assert_eq!(
            r.current_key_tips().get(0).tip.unwrap().to_utf8().unwrap(),
            source["tips"][0]["tip"].as_str().unwrap()
        );
        r.cancel_key_tips();
        settle();
        // The reference records its mutable event list through final cleanup.
        assert_eq!(serde_json::json!(*events.borrow()), source["events"]);
        let panel = Panel::new();
        let before = TextBox::new();
        let after = TextBox::new();
        w.set_content(None);
        panel.children().add(r.clone());
        panel.children().add(before.clone());
        panel.children().add(after.clone());
        w.set_content(Some(Control::boxed(panel)));
        layout(&w, 1400.);
        before.focus();
        r.show_key_tips();
        after.focus();
        r.hide_key_tips();
        settle();
        assert!(
            w.focus_manager()
                .get_focused_element()
                .unwrap()
                .ptr_eq(&after)
        );
        assert!(r.process_key_tip_char('😀').is_err());
        assert!(!r.process_key_tip_input(0xd800));
        w.close();
        w.set_content(None);
    });
}
#[test]
fn popup_routes_once_per_owner_isolate_foreign_ribbons_and_remove_hooks() {
    const NAME: &str = "controls::ribbon::keyboard::tests::popup_routes_once_per_owner_isolate_foreign_ribbons_and_remove_hooks";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        use ferroui_base::input::{
            Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType,
        };
        let (r, w, g, b) = setup();
        let foreign = Ribbon::new();
        foreign.set_is_application_button_visible(false);
        foreign.set_is_display_options_button_visible(false);
        let tab = RibbonTab::new();
        tab.set_header(Some("Other".into()));
        foreign.tabs().add(Some(tab));
        let host = Panel::new();
        w.set_content(None);
        host.children().add(r.clone());
        host.children().add(foreign.clone());
        w.set_content(Some(Control::boxed(host.clone())));
        layout(&w, 1400.);
        r.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        layout(&w, 1400.);
        r.open_minimized_popup();
        settle();
        assert!(send(&b, Key::F10, false, KeyModifiers::NONE, false).handled());
        settle();
        assert!(
            r.is_key_tip_mode(),
            "duplicate owner route would cancel F10 again"
        );
        assert!(
            !foreign.is_key_tip_mode(),
            "separate source popup root must not route to foreign window ribbon"
        );
        // A pointer inside the popup never reaches the source window keyboard hook.
        let e = PointerPressedEventArgs::new(
            b.to_ref().upcast::<ferroui_base::FerroObject>(),
            Pointer::new(1, PointerType::Mouse, true),
            &w,
            Point::new(20., 110.),
            0,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
            1,
        );
        b.raise_event(&e);
        assert!(r.is_key_tip_mode());
        assert!(r.is_minimized_popup_open());
        // Reentrant input keeps the source owner/window dispatch boundaries.
        let weak = r.downgrade();
        let calls = Rc::new(Cell::new(0));
        let n = calls.clone();
        b.key_down(move |_, e| {
            if e.key == Key::Q
                && let Some(root) = weak.upgrade()
            {
                n.set(n.get() + 1);
                send(&root, Key::F10, false, KeyModifiers::NONE, false);
            }
        });
        send(&b, Key::Q, false, KeyModifiers::NONE, false);
        settle();
        assert_eq!(calls.get(), 1);
        r.cancel_key_tips();
        foreign.cancel_key_tips();
        r.close_minimized_popup();
        host.children().remove(r.clone());
        settle();
        assert!(r.keyboard.popup_hooks.borrow().is_empty());
        host.children().add(r.clone());
        layout(&w, 1400.);
        r.open_minimized_popup();
        settle();
        send(&b, Key::F10, false, KeyModifiers::NONE, false);
        settle();
        assert!(r.is_key_tip_mode(), "reload must reinstall popup hook");
        assert!(!foreign.is_key_tip_mode());
        r.cancel_key_tips();
        r.close_minimized_popup();
        // Actual collapsed group uses the same retained child and owner route.
        r.set_visibility_mode(RibbonVisibilityMode::PanelButtons);
        layout(&w, 1400.);
        g.open_popup();
        settle();
        send(&b, Key::F10, false, KeyModifiers::NONE, false);
        settle();
        assert!(r.is_key_tip_mode());
        assert!(!foreign.is_key_tip_mode());
        r.cancel_key_tips();
        g.close_popup();
        w.close();
        w.set_content(None);
        settle();
    });
}
#[test]
fn original_key_character_mapping_matches_every_native_virtual_key_projection() {
    let source = reference_case("keyCharacters");
    let mappings: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/shortcuts.json")).unwrap();
    for row in mappings["keyNames"].as_array().unwrap() {
        let key: Key = row["native"].as_str().unwrap().parse().unwrap();
        let expected = source["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["key"] == row["value"])
            .unwrap();
        assert_eq!(
            key_to_char(key).map(i64::from),
            expected["unit"].as_i64(),
            "{row}"
        );
    }
    assert_eq!(key_to_char(Key::FnLeftArrow), None);
}
#[test]
fn source_current_scope_keeps_removed_header_tab_alive() {
    const NAME: &str =
        "controls::ribbon::keyboard::tests::source_current_scope_keeps_removed_header_tab_alive";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, _, _) = setup();
        r.show_key_tips();
        settle();
        let snapshot = r.current_key_tips();
        let tab = r.tabs().get(0).unwrap();
        let weak = tab.downgrade();
        r.tabs().remove(&Some(tab.clone()));
        drop(tab);
        settle();
        assert!(snapshot.get(0).target.is_some());
        assert!(
            weak.upgrade().is_some(),
            "A retained current-scope header must keep its actual Tab alive"
        );
        let retained = snapshot
            .get(0)
            .target
            .unwrap()
            .cast::<RibbonTabHeader>()
            .unwrap();
        assert_eq!(retained.tab().header().as_deref(), Some("Home"));
        drop(retained);
        r.cancel_key_tips();
        w.close();
        w.set_content(None);
        settle();
        drop(r);
        drop(w);
        settle();
        assert!(
            weak.upgrade().is_some(),
            "The external snapshot owns its target after root disposal"
        );
        drop(snapshot);
        settle();
        assert!(
            weak.upgrade().is_none(),
            "Dropping the last snapshot releases its removed Tab"
        );
    });
}
struct SearchAction(Box<dyn Fn()>);
impl PartialEq for SearchAction {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class SearchAction{this:Rc<SearchAction>,handles:[Rc<SearchAction>],methods:[fn Invoke(Option<BoxedValue>,ferroui_base::utilities::EventArgs)=>|v:&Rc<SearchAction>,_,_|(v.0)()]});
#[test]
fn source_split_menu_contextual_prefix_and_search_request_precedence() {
    const NAME: &str = "controls::ribbon::keyboard::tests::source_split_menu_contextual_prefix_and_search_request_precedence";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, g, _) = setup();
        let value =
            crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Split.xaml")
                .unwrap()
                .unwrap();
        r.resources().add_merged_dictionary(
            ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ferroui_base::controls::ResourceDictionary>()
                .unwrap(),
        );
        let split = RibbonSplitButton::new();
        split.set_id(Some("paste".into()));
        split.set_label(Some("Paste".into()));
        split.set_key_tip(Some("V".into()));
        let menu = MenuFlyout::new();
        let item = MenuFlyoutItem::new();
        item.set_text(Some("Keep Source".into()));
        menu.items().add(Some(Control::boxed(item)));
        split.set_flyout(Some(menu.upcast()));
        g.items().add(Some(split.clone().upcast()));
        layout(&w, 1400.);
        r.show_key_tips();
        r.process_key_tip_input(b'H' as u16);
        settle();
        r.process_key_tip_input(b'V' as u16);
        settle();
        assert!(r.is_key_tip_mode());
        assert!(split.is_drop_down_open());
        assert!(
            r.current_key_tips()
                .iter()
                .any(|e| e.target.is_some_and(|t| t.is::<MenuFlyoutItem>()))
        );
        r.cancel_key_tips();
        settle();
        assert!(!split.is_drop_down_open());
        let tools = RibbonContextualTabGroup::new();
        tools.set_id(Some("tools".into()));
        tools.set_key_tip(Some("J".into()));
        tools.set_is_visible(true);
        r.contextual_groups().add(Some(tools.clone()));
        let tab = RibbonTab::new();
        tab.set_id(Some("tableDesign".into()));
        tab.set_header(Some("Table Design".into()));
        tab.set_contextual_group_id(Some("tools".into()));
        r.tabs().add(Some(tab.clone()));
        layout(&w, 1400.);
        r.show_key_tips();
        settle();
        assert!(
            r.current_key_tips()
                .iter()
                .any(|e| e.tip == Some("JT".into()))
        );
        r.cancel_key_tips();
        tools.set_is_enabled(false);
        settle();
        assert!(!tab.header_element().unwrap().is_enabled());
        assert!(!send(&r, Key::Q, false, KeyModifiers::ALT, false).handled());
        ValueTypes::register_reference::<SearchAction>();
        let calls = Rc::new(Cell::new(0));
        let n = calls.clone();
        let h = MarkupDelegate::for_method(
            Some(Rc::new(SearchAction(Box::new(move || n.set(n.get() + 1))))),
            SearchAction::MARKUP,
            &SearchAction::MARKUP.methods[0],
        );
        r.add_search_requested(Some(h.clone()));
        assert!(send(&r, Key::Q, false, KeyModifiers::ALT, false).handled());
        assert_eq!(calls.get(), 1);
        r.show_key_tips();
        send(&r, Key::Q, false, KeyModifiers::ALT, false);
        assert_eq!(calls.get(), 1, "KeyTip mode precedes search");
        r.cancel_key_tips();
        r.remove_search_requested(Some(h));
        assert!(!send(&r, Key::Q, false, KeyModifiers::ALT, false).handled());
        w.close();
        w.set_content(None);
        settle();
    });
}

#[test]
fn dismissing_a_keytip_flyout_restores_pre_keytip_focus() {
    use crate::controls::RibbonGridPicker;
    use ferroui_controls::Flyout;

    if isolated(concat!(
        module_path!(),
        "::dismissing_a_keytip_flyout_restores_pre_keytip_focus"
    )) {
        return;
    }

    crate::group_tests::run(|| {
        let (ribbon, window, _, _) = setup();
        window.set_content(None);
        let host = Panel::new();
        let document = TextBox::new();
        let elsewhere = TextBox::new();
        host.children().add(ribbon.clone());
        host.children().add(document.clone());
        host.children().add(elsewhere.clone());
        window.set_content(Some(Control::boxed(host)));

        let button = RibbonDropDownButton::new();
        button.set_label(Some("Table".into()));
        button.set_key_tip(Some("T".into()));
        let picker = RibbonGridPicker::new();
        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(picker.clone())));
        button.set_flyout(Some(flyout.clone().upcast()));
        let insert = ribbon.tabs().get(1).unwrap();
        let group = RibbonGroup::new();
        group.set_header(Some("Tables".into()));
        group.items().add(Some(button.clone().upcast()));
        insert.groups().add(Some(group));
        layout(&window, 1400.);

        let focus_is = |control: &Control| {
            window
                .focus_manager()
                .get_focused_element()
                .is_some_and(|focused| focused.ptr_eq(&control.to_ref()))
        };
        let open_with_key_tips = || {
            ribbon.show_key_tips();
            assert!(ribbon.process_key_tip_input(b'N' as u16));
            settle();
            assert!(ribbon.process_key_tip_input(b'T' as u16));
            settle();
            assert!(flyout.is_open());
            assert!(focus_is(&picker));
            assert!(!ribbon.is_key_tip_mode());
        };

        assert!(document.focus());
        open_with_key_tips();
        assert!(send(&picker, Key::Escape, false, KeyModifiers::NONE, false).handled());
        settle();
        assert!(!flyout.is_open());
        assert!(focus_is(&document));
        assert!(ribbon.selected_tab().unwrap().ptr_eq(&insert));

        assert!(button.focus());
        flyout.show_at(&button);
        settle();
        flyout.hide();
        settle();
        assert!(
            focus_is(&button),
            "A regular opening keeps its own return focus"
        );

        ribbon.select_tab(ribbon.tabs().get(0).unwrap());
        assert!(document.focus());
        open_with_key_tips();
        let _closed = flyout.closed({
            let elsewhere = elsewhere.clone();
            move || {
                elsewhere.focus();
            }
        });
        flyout.hide();
        settle();
        assert!(
            focus_is(&elsewhere),
            "The close handler can choose where focus goes"
        );

        window.close();
        window.set_content(None);
    });
}

#[test]
fn keytip_focus_restores_attached_input_elements_without_requiring_control_inheritance() {
    const NAME: &str = concat!(
        module_path!(),
        "::keytip_focus_restores_attached_input_elements_without_requiring_control_inheritance"
    );
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, _, _) = setup();
        w.set_content(None);
        let host = Panel::new();
        host.children().add(r.clone());
        let document = TextBox::new();
        host.children().add(document.clone());
        let input = InputElement::new();
        input.set_focusable(true);
        host.visual_children().add(input.clone().upcast());
        w.set_content(Some(Control::boxed(host.clone())));
        layout(&w, 1400.);
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/keyboard.json")).unwrap();
        for kind in ["element", "detached", "moved"] {
            if input.visual_parent().is_none() {
                host.visual_children().add(input.clone().upcast());
            }
            assert!(input.visual_root().is_some());
            assert!(input.to_ref().cast::<Control>().is_none());
            assert!(input.focus());
            r.show_key_tips();
            settle();
            let header = r.tabs().get(0).unwrap().header_element().unwrap();
            let header_focused = w
                .focus_manager()
                .get_focused_element()
                .is_some_and(|f| f.ptr_eq(&header));
            if kind == "detached" {
                host.visual_children().remove(&input.clone().upcast());
            }
            if kind == "moved" {
                assert!(document.focus());
            }
            r.hide_key_tips();
            settle();
            let focused = w.focus_manager().get_focused_element();
            let actual = serde_json::json!({
                "name":"focusRestoreKind", "kind":kind, "headerFocused":header_focused,
                "restored":focused.as_ref().is_some_and(|f| f.ptr_eq(&input)),
                "keptHeader":focused.as_ref().is_some_and(|f| f.ptr_eq(&header)),
                "keptMoved":focused.as_ref().is_some_and(|f| f.ptr_eq(&document))
            });
            let expected = source["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == "focusRestoreKind" && c["kind"] == kind)
                .unwrap();
            assert_eq!(&actual, expected, "{kind}");
        }
        w.close();
        w.set_content(None);
        host.visual_children().remove(&input.upcast());
    });
}

#[test]
fn source_control_inline_link_and_null_focus_restore_through_native_services() {
    const NAME: &str = concat!(
        module_path!(),
        "::source_control_inline_link_and_null_focus_restore_through_native_services"
    );
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, _, _) = setup();
        w.set_content(None);
        let host = Panel::new();
        host.children().add(r.clone());
        let document = TextBox::new();
        host.children().add(document.clone());
        let text = ferroui_controls::TextBlock::new();
        let link = ferroui_controls::HyperlinkButton::new();
        link.set_content(Some(Rc::new("Help".to_owned())));
        text.inlines().unwrap().add_control(link.clone());
        host.children().add(text.clone());
        w.set_content(Some(Control::boxed(host)));
        layout(&w, 1400.);
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/keyboard.json")).unwrap();
        let header = r.tabs().get(0).unwrap().header_element().unwrap();
        for kind in ["control", "hyperlink", "none"] {
            let before: Option<Ref<InputElement>> = match kind {
                "control" => Some(document.clone().upcast()),
                "hyperlink" => Some(link.clone().upcast()),
                _ => None,
            };
            if let Some(before) = &before {
                assert!(before.visual_root().is_some());
                assert!(before.focus());
            } else {
                ferroui_base::input::KeyboardDevice::instance()
                    .unwrap()
                    .set_focused_element_with(
                        None,
                        NavigationMethod::Unspecified,
                        KeyModifiers::NONE,
                        false,
                    );
                assert!(w.focus_manager().get_focused_element().is_none());
            }
            r.show_key_tips();
            settle();
            let header_focused = w
                .focus_manager()
                .get_focused_element()
                .is_some_and(|f| f.ptr_eq(&header));
            r.hide_key_tips();
            settle();
            let focused = w.focus_manager().get_focused_element();
            let actual = serde_json::json!({
                "name":"focusRestoreKind", "kind":kind, "headerFocused":header_focused,
                "restored":before.as_ref().is_some_and(|before| focused.as_ref().is_some_and(|f| f.ptr_eq(before))),
                "keptHeader":focused.as_ref().is_some_and(|f| f.ptr_eq(&header)),
                "keptMoved":false
            });
            let expected = source["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == "focusRestoreKind" && c["kind"] == kind)
                .unwrap();
            assert_eq!(&actual, expected, "{kind}");
        }
        w.close();
        w.set_content(None);
        text.inlines().unwrap().clear();
    });
}
