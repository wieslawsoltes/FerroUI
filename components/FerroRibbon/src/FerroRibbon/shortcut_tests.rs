//! Source shortcut corpus and native input/cache lifecycle regressions.
use super::*;
use crate::controls::{RibbonButton, RibbonButtonImpl, RibbonToggleButton};
use ferroui_base::{
    FerroObjectImpl, StyledElementImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElement, instantiate,
};
use ferroui_controls::{Panel, Window};
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../tests/fixtures/shortcuts.json")).unwrap()
}
fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_SHORTCUT_ISOLATED").as_deref() == Ok(name) {
        return false;
    }
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_SHORTCUT_ISOLATED", name)
        .output()
        .unwrap();
    assert!(
        o.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    true
}
fn key(value: i64) -> Key {
    let data = fixture();
    let name = data["keyNames"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["value"] == value)
        .unwrap()["native"]
        .as_str()
        .unwrap();
    name.parse().unwrap()
}
fn setup() -> (Ref<Ribbon>, Ref<Window>, Ref<RibbonGroup>) {
    let r = Ribbon::new();
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    let tab = RibbonTab::new();
    tab.set_header(Some("Home".into()));
    let group = RibbonGroup::new();
    tab.groups().add(Some(group.clone()));
    r.tabs().add(Some(tab));
    let w = crate::ribbon_tests::mount(&r);
    (r, w, group)
}
fn event(root: &Control, key: Key, mods: KeyModifiers, handled: bool) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.key_modifiers = mods;
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.set_handled(handled);
    root.raise_event(&e);
    e
}
fn item(name: &str, shortcut: &str) -> (Ref<RibbonButton>, Rc<Cell<usize>>) {
    let b = RibbonButton::new();
    b.set_name(Some(name.into()));
    b.set_shortcut(Some(shortcut.into()));
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    b.click(move |_, _| c.set(c.get() + 1));
    (b, count)
}
#[test]
fn original_gesture_focus_matrix_and_dictionary_order() {
    const NAME: &str =
        "controls::ribbon::shortcuts::tests::original_gesture_focus_matrix_and_dictionary_order";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let data = fixture();
        for c in data["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "matrix")
        {
            let (r, w, g) = setup();
            let (b, n) = item("item", c["shortcut"].as_str().unwrap());
            g.items().add(Some(b.upcast()));
            // Native TextBox focus belongs to the same actual TopLevel.
            if c["focused"].as_bool().unwrap() {
                let host = Panel::new();
                w.set_content(None);
                host.children().add(r.clone());
                let text = TextBox::new();
                host.children().add(text.clone());
                w.set_content(Some(Control::boxed(host)));
                text.focus();
            }
            let result = r.try_route_shortcut(
                key(c["key"].as_i64().unwrap()),
                RibbonModifierKeys::from_raw(c["mods"].as_i64().unwrap() as i32),
            );
            assert_eq!(result, c["result"].as_bool().unwrap(), "{c}");
            assert_eq!(n.get() as u64, c["Invokes"].as_u64().unwrap(), "{c}");
            let cache=r.build_shortcut_cache().into_iter().map(|(key,entries)|serde_json::json!({"key":key.to_utf8().unwrap(),"items":entries.into_iter().map(|(_,i)|i.name().unwrap()).collect::<Vec<_>>()})).collect::<Vec<_>>();
            assert_eq!(serde_json::json!(cache), c["cache"], "{c}");
            w.close();
            w.set_content(None);
        }
    });
}
#[test]
fn source_editing_focus_keeps_function_keys_and_routes_modified_commands() {
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::{AutoSuggestBox, Button, RichEditBox};

    crate::group_tests::run(|| {
        let (ribbon, window, group) = setup();
        let (command, calls) = item("command", "F5, Shift+F6, Ctrl+B");
        group.items().add(Some(command.upcast()));
        ribbon.set_is_shortcut_routing_enabled(true);

        let password = TextBox::new();
        password.set_password_char('*');
        let suggest = AutoSuggestBox::new();
        suggest.set_focusable(true);
        let editors = [
            ("Control", Button::new().upcast::<Control>()),
            ("TextBox", TextBox::new().upcast()),
            ("PasswordBox", password.upcast()),
            ("RichEditBox", RichEditBox::new().upcast()),
            ("AutoSuggestBox", suggest.upcast()),
        ];
        let host = Panel::new();
        window.set_content(None);
        host.children().add(ribbon.clone());
        for (_, editor) in &editors {
            host.children().add(editor.clone());
        }
        window.set_content(Some(Control::boxed(host)));
        Dispatcher::ui_thread().run_jobs(None);

        let data = fixture();
        for case in data["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| case["name"] == "editingFocus")
        {
            let (_, editor) = editors
                .iter()
                .find(|(name, _)| case["editor"] == *name)
                .unwrap();
            assert!(editor.focus(), "{case}");
            assert!(
                window
                    .focus_manager()
                    .get_focused_element()
                    .unwrap()
                    .ptr_eq(editor),
                "{case}"
            );
            let (key, modifiers) = match case["shortcut"].as_str().unwrap() {
                "F5" => (Key::F5, KeyModifiers::NONE),
                "Shift+F6" => (Key::F6, KeyModifiers::SHIFT),
                "Ctrl+B" => (Key::B, KeyModifiers::CONTROL),
                _ => unreachable!("unexpected editor shortcut {case}"),
            };
            let before = calls.get();
            assert_eq!(
                event(&ribbon, key, modifiers, false).handled(),
                case["result"].as_bool().unwrap(),
                "{case}"
            );
            assert_eq!(
                calls.get() - before,
                case["Invokes"].as_u64().unwrap() as usize
            );
        }
        window.close();
    });
}

#[test]
fn source_runtime_shortcut_toggle_and_native_handler_lifecycle() {
    const NAME: &str = "controls::ribbon::shortcuts::tests::source_runtime_shortcut_toggle_and_native_handler_lifecycle";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, g) = setup();
        let b = RibbonToggleButton::new();
        b.set_shortcut(Some("Ctrl+B".into()));
        g.items().add(Some(b.clone().upcast()));
        assert!(!r.is_shortcut_routing_enabled());
        assert!(!event(&r, Key::B, KeyModifiers::CONTROL, false).handled());
        assert_ne!(b.is_checked(), Some(true));
        r.set_is_shortcut_routing_enabled(true);
        assert!(event(&r, Key::B, KeyModifiers::CONTROL, false).handled());
        assert_eq!(b.is_checked(), Some(true));
        event(&r, Key::B, KeyModifiers::CONTROL, true);
        assert_eq!(b.is_checked(), Some(true));
        w.close();
        event(&r, Key::B, KeyModifiers::CONTROL, false);
        assert_eq!(b.is_checked(), Some(true));
        w.set_content(None);
    });
}

#[repr(C)]
struct RejectButton {
    base: RibbonButton,
    calls: Rc<Cell<usize>>,
}
ferro_class!(RejectButton:RibbonButton);
ferro_impl_classes!(RejectButton:FerroObjectImpl,StyledElementImpl,ferroui_base::VisualImpl,ferroui_base::layout::LayoutableImpl,ferroui_base::interactivity::InteractiveImpl,ferroui_base::input::InputElementImpl,ferroui_controls::ControlImpl,ferroui_controls::primitives::TemplatedControlImpl,ferroui_controls::ContentControlImpl,ferroui_controls::ButtonImpl);
ferro_class_info!(RejectButton {});
impl RibbonButtonImpl for RejectButton {
    fn invoke_core(this: &Self) -> bool {
        this.calls.set(this.calls.get() + 1);
        false
    }
}
#[test]
fn source_false_invoke_stops_linked_dedup_and_reentrant_invalidation() {
    const NAME: &str = "controls::ribbon::shortcuts::tests::source_false_invoke_stops_linked_dedup_and_reentrant_invalidation";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w, g) = setup();
        let calls = Rc::new(Cell::new(0));
        let first = instantiate(RejectButton {
            base: RibbonButton::construct(),
            calls: calls.clone(),
        });
        first.set_shortcut(Some("Ctrl+B".into()));
        let (second, count) = item("second", "Ctrl+B");
        g.items().add(Some(first.clone().upcast()));
        g.items().add(Some(second.clone().upcast()));
        assert!(!r.try_route_shortcut(Key::B, RibbonModifierKeys::CONTROL));
        let data = fixture();
        let source = data["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "falseStops")
            .unwrap();
        assert_eq!(
            serde_json::json!([calls.get(), count.get()]),
            source["invokes"]
        );
        g.items().clear();
        let (a, n) = item("a", "Ctrl+B,Ctrl+B,Ctrl+Shift+B,Ctrl+B");
        let (b, _) = item("b", "Ctrl+B");
        RibbonItemHelper::set_source_item(&b, a.clone());
        g.items().add(Some(a.clone().upcast()));
        g.items().add(Some(b.clone().upcast()));
        let cache=r.build_shortcut_cache().into_iter().map(|(key,entries)|serde_json::json!({"key":key.to_utf8().unwrap(),"items":entries.into_iter().map(|(_,i)|i.name().unwrap()).collect::<Vec<_>>()})).collect::<Vec<_>>();
        let source = data["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "linkedDedup")
            .unwrap();
        assert_eq!(serde_json::json!(cache), source["cache"]);
        let weak = r.downgrade();
        a.click(move |_, _| {
            if let Some(r) = weak.upgrade() {
                r.invalidate_shortcuts();
            }
        });
        assert!(r.try_route_shortcut(Key::B, RibbonModifierKeys::CONTROL));
        assert_eq!(n.get(), 1);
        assert!(r.shortcuts.cache.borrow().is_none());
        a.set_shortcut(Some("Ctrl+C".into()));
        b.set_shortcut(Some("Ctrl+C".into()));
        assert!(!r.try_route_shortcut(Key::B, RibbonModifierKeys::CONTROL));
        assert!(r.try_route_shortcut(Key::C, RibbonModifierKeys::CONTROL));
        assert_eq!(n.get(), 2);
        g.items().clear();
        assert!(!r.try_route_shortcut(Key::C, RibbonModifierKeys::CONTROL));
        w.close();
        w.set_content(None);
    });
}

#[test]
fn source_reachability_visual_logical_contextual_and_detached_membership() {
    const NAME: &str = "controls::ribbon::shortcuts::tests::source_reachability_visual_logical_contextual_and_detached_membership";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let corpus = fixture();
        for c in corpus["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "reachability")
        {
            let state = c["state"].as_str().unwrap();
            let (r, w, g) = setup();
            let (first, n) = item("first", "Ctrl+B");
            let (second, m) = item("second", "Ctrl+B");
            g.items().add(Some(first.clone().upcast()));
            g.items().add(Some(second.upcast()));
            let tab = r.tabs().get(0).unwrap();
            let host = Panel::new();
            w.set_content(None);
            host.children().add(r.clone());
            w.set_content(Some(Control::boxed(host.clone())));
            match state {
                "disabled" => first.set_is_enabled(false),
                "hidden" => first.set_is_visible(false),
                "tabUnavailable" => tab.set_is_tab_visible(false),
                "groupHidden" => g.set_is_group_visible(false),
                "unselectedTab" => tab.set_is_visible(false),
                "unmounted" => w.set_content(None),
                "visualHidden" | "logicalHidden" | "parentDisabled" => {
                    if let Some(p) = first.get_logical_parent().and_then(|p| p.cast::<Panel>()) {
                        p.children().remove(first.clone());
                    }
                    let hidden = Panel::new();
                    if state == "parentDisabled" {
                        hidden.set_is_enabled(false);
                    } else {
                        hidden.set_is_visible(false);
                    }
                    if state == "logicalHidden" {
                        first.set_parent(hidden.clone());
                    } else {
                        hidden.children().add(first.clone());
                    }
                    host.children().add(hidden);
                }
                "strip" => {
                    g.items().remove(&Some(first.clone().upcast()));
                    r.tab_strip_items().add(Some(first.clone().upcast()));
                    if let Some(p) = first.get_logical_parent().and_then(|p| p.cast::<Panel>()) {
                        p.children().remove(first.clone());
                    }
                }
                "orphanVisible" | "orphanHidden" => {
                    g.items().remove(&Some(first.clone().upcast()));
                    let (copy, _) = item("copy", "Ctrl+B");
                    RibbonItemHelper::set_source_item(&copy, first.clone());
                    g.items().insert(0, Some(copy.upcast()));
                    host.children().add(first.clone());
                    first.set_is_visible(state == "orphanVisible");
                }
                "normal" => {}
                _ => panic!("unknown source state"),
            }
            assert_eq!(
                r.try_route_shortcut(Key::B, RibbonModifierKeys::CONTROL),
                c["result"].as_bool().unwrap(),
                "{c}"
            );
            assert_eq!(serde_json::json!([n.get(), m.get()]), c["invokes"], "{c}");
            w.close();
            w.set_content(None);
        }
        for row in corpus["keyNames"].as_array().unwrap() {
            let key: Key = row["native"].as_str().unwrap().parse().unwrap();
            assert_eq!(names::key_name(key), row["name"].as_str());
        }
        assert_eq!(
            names::key_name(Key::FnLeftArrow),
            None,
            "no WinUI virtual key equivalent is invented"
        );
    });
}

#[test]
fn native_shared_root_handling_precedence_cache_release_and_group_mutations() {
    const NAME: &str = "controls::ribbon::shortcuts::tests::native_shared_root_handling_precedence_cache_release_and_group_mutations";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let before = RibbonItemHelper::items_version();
        let observed = std::thread::spawn(|| {
            let before = RibbonItemHelper::items_version();
            RibbonItemHelper::increment_items_version();
            before
        })
        .join()
        .unwrap();
        let data = fixture();
        let source = data["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "globalVersion")
            .unwrap();
        assert_eq!(observed == before, source["shared"].as_bool().unwrap());
        assert_eq!(
            RibbonItemHelper::items_version().wrapping_sub(before) as i64,
            source["delta"].as_i64().unwrap()
        );
        let (r, w, g) = setup();
        let (other, unused, second_group) = setup();
        unused.close();
        unused.set_content(None);
        let host = Panel::new();
        w.set_content(None);
        host.children().add(r.clone());
        host.children().add(other.clone());
        w.set_content(Some(Control::boxed(host)));
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        r.set_is_shortcut_routing_enabled(true);
        other.set_is_shortcut_routing_enabled(true);
        let (a, n) = item("a", "Ctrl+B");
        let (b, m) = item("b", "Ctrl+B");
        g.items().add(Some(a.clone().upcast()));
        second_group.items().add(Some(b.upcast()));
        assert!(event(&r, Key::B, KeyModifiers::CONTROL, false).handled());
        assert_eq!((n.get(), m.get()), (1, 0));
        let weak_item = a.downgrade();
        g.items().clear();
        assert!(
            r.shortcuts.cache.borrow().is_none(),
            "source group mutation invalidates retained cache synchronously"
        );
        drop(a);
        // The group panel retains the last measured children until its next layout pass.
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(
            weak_item.upgrade().is_none(),
            "removed item retained after layout"
        );
        assert!(event(&r, Key::B, KeyModifiers::CONTROL, false).handled());
        assert_eq!(m.get(), 1);
        let (conflict, conflicts) = item("conflict", "Ctrl+F1");
        g.items().add(Some(conflict.upcast()));
        assert!(event(&r, Key::F1, KeyModifiers::CONTROL, false).handled());
        assert!(r.is_minimized());
        assert_eq!(conflicts.get(), 0);
        assert!(r.is_key_tips_enabled());
        let (f10, f10_count) = item("f10", "F10");
        g.items().add(Some(f10.upcast()));
        r.set_is_key_tips_enabled(false);
        assert!(event(&r, Key::F10, KeyModifiers::NONE, false).handled());
        assert_eq!(f10_count.get(), 1);
        let (alt, alt_count) = item("alt", "Alt+LeftMenu");
        g.items().add(Some(alt.upcast()));
        assert!(!event(&r, Key::LeftAlt, KeyModifiers::ALT, false).handled());
        assert_eq!(alt_count.get(), 0);
        let (slide, slide_count) = item("slide", "Ctrl+C");
        g.slide_out_items().add(Some(slide.clone().upcast()));
        assert!(event(&r, Key::C, KeyModifiers::CONTROL, false).handled());
        assert_eq!(slide_count.get(), 1);
        g.slide_out_items().clear();
        assert!(!event(&r, Key::C, KeyModifiers::CONTROL, false).handled());
        r.try_route_shortcut(Key::F1, RibbonModifierKeys::CONTROL);
        let weak = r.downgrade();
        w.close();
        w.set_content(None);
        drop(r);
        assert!(
            weak.upgrade().is_some(),
            "source unload callback retains the root until dispatched"
        );
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn original_model_structure_notification_coalesces_and_releases_actual_shortcut_cache() {
    const NAME: &str = "controls::ribbon::shortcuts::tests::original_model_structure_notification_coalesces_and_releases_actual_shortcut_cache";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let source: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/element-factory.json"))
                .unwrap();
        let expected = source["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == "structure_queued")
            .unwrap();
        let r = Ribbon::new();
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(!r.try_route_shortcut(Key::F2, RibbonModifierKeys::NONE));
        let cached = r.shortcuts.cache.borrow().clone().unwrap();
        let weak = Rc::downgrade(&cached);
        drop(cached);
        let baseline = r.reference_counts().0;
        r.on_model_elements_changed();
        r.on_model_elements_changed();
        let queued = r.reference_counts().0 - baseline;
        assert_eq!(queued, expected["queued"].as_u64().unwrap() as usize);
        assert_eq!(
            r.model.elements_change_pending.get(),
            expected["pending"].as_bool().unwrap()
        );
        assert!(weak.upgrade().is_some());
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(r.shortcuts.cache.borrow().is_none());
        assert!(weak.upgrade().is_none());
        assert_eq!(
            !r.model.elements_change_pending.get(),
            expected["cleared"].as_bool().unwrap()
        );
        assert_eq!(r.reference_counts().0, baseline);
        assert!(!r.try_route_shortcut(Key::F2, RibbonModifierKeys::NONE));
        let weak = Rc::downgrade(r.shortcuts.cache.borrow().as_ref().unwrap());
        r.on_model_elements_changed();
        r.on_model_elements_changed();
        assert_eq!(r.reference_counts().0 - baseline, queued);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
        assert_eq!(r.reference_counts().0, baseline);
    });
}
