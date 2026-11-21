//! Original transition corpus and native retained-command-bar/input integration.
use super::*;
use crate::controls::{RibbonAutomationPeer, RibbonButton, RibbonGroup};
use ferroribbon_core::{RibbonDisplayMode, RibbonGroupState};
use ferroui_base::{
    Ref, Size,
    metadata::{MarkupDelegate, MarkupTyped},
};
use ferroui_controls::{Panel, Window, automation::provider::IExpandCollapseProvider};
struct StateAction(Box<dyn Fn()>);
impl PartialEq for StateAction {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class StateAction{this:Rc<StateAction>,handles:[Rc<StateAction>],methods:[fn Invoke(Option<BoxedValue>,ferroui_base::utilities::EventArgs)=>|h:&Rc<StateAction>,_,_|(h.0)()]});
fn state_event(f: impl Fn() + 'static) -> MarkupDelegate {
    ferroui_base::data::core::ValueTypes::register_reference::<StateAction>();
    MarkupDelegate::for_method(
        Some(Rc::new(StateAction(Box::new(f)))),
        StateAction::MARKUP,
        &StateAction::MARKUP.methods[0],
    )
}
fn click(b: &Button) {
    ferroui_controls::automation::peers::ButtonAutomationPeer::new(b)
        .invoke()
        .unwrap();
}
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../tests/fixtures/visibility.json")).unwrap()
}
fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_VISIBILITY_ISOLATED").as_deref() == Ok(name) {
        return false;
    }
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_VISIBILITY_ISOLATED", name)
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
fn resize(w: &Window, width: f64, height: f64) {
    w.set_width(width);
    w.set_height(height);
    w.measure(Size::new(width, height));
    w.arrange(Rect::new(0., 0., width, height));
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
}
fn setup() -> (Ref<Ribbon>, Ref<Window>) {
    let r = Ribbon::new();
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    let t = RibbonTab::new();
    t.set_header(Some("Home".into()));
    let g = RibbonGroup::new();
    g.set_header(Some("Tools".into()));
    for _ in 0..3 {
        let b = RibbonButton::new();
        b.set_label(Some("Command".into()));
        b.set_icon(Some(Rc::new("A".to_owned())));
        g.items().add(Some(b.upcast()));
    }
    t.groups().add(Some(g));
    r.tabs().add(Some(t));
    let w = crate::ribbon_tests::mount(&r);
    (r, w)
}
#[test]
fn source_cycle_direct_minimized_and_peer_matrix() {
    const NAME: &str =
        "controls::ribbon::visibility::tests::source_cycle_direct_minimized_and_peer_matrix";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        for c in fixture()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "cycle")
        {
            let r = Ribbon::new();
            r.set_minimize_behavior(RibbonMinimizeBehavior::from_raw(
                c["behavior"].as_i64().unwrap() as i32,
            ));
            r.set_display_mode(RibbonDisplayMode::from_raw(
                c["display"].as_i64().unwrap() as i32
            ));
            r.set_visibility_mode(RibbonVisibilityMode::from_raw(
                c["initial"].as_i64().unwrap() as i32,
            ));
            for expected in c["steps"].as_array().unwrap() {
                assert_eq!(
                    serde_json::json!({"mode":r.visibility_mode().raw(),"minimized":r.is_minimized(),"presentation":r.current_panel_presentation().raw(),"next":r.next_minimize_state().raw()}),
                    *expected,
                    "{c}"
                );
                r.toggle_minimized();
            }
        }
        for c in fixture()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "direct")
        {
            let r = Ribbon::new();
            r.set_minimize_behavior(RibbonMinimizeBehavior::from_raw(
                c["behavior"].as_i64().unwrap() as i32,
            ));
            r.set_visibility_mode(RibbonVisibilityMode::from_raw(
                c["initial"].as_i64().unwrap() as i32,
            ));
            let w = crate::ribbon_tests::mount(&r);
            let events = Rc::new(RefCell::new(Vec::<String>::new()));
            let log = events.clone();
            let weak = r.downgrade();
            r.add_state_changed(Some(state_event(move || {
                let r = weak.upgrade().unwrap();
                log.borrow_mut().push(format!(
                    "state:{}:{}",
                    r.visibility_mode().raw(),
                    if r.is_minimized() { "True" } else { "False" }
                ));
            })));
            r.set_is_minimized(c["value"].as_bool().unwrap());
            assert_eq!(
                r.visibility_mode().raw(),
                c["mode"].as_i64().unwrap() as i32
            );
            assert_eq!(r.is_minimized(), c["minimized"].as_bool().unwrap());
            let expected = c["trace"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|v| v.as_str())
                .filter(|s| s.starts_with("state:"))
                .collect::<Vec<_>>();
            assert_eq!(*events.borrow(), expected, "{c}");
            w.close();
            w.set_content(None);
        }
        for c in fixture()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "peer")
        {
            let r = Ribbon::new();
            r.set_minimize_behavior(RibbonMinimizeBehavior::from_raw(
                c["behavior"].as_i64().unwrap() as i32,
            ));
            r.set_is_collapsible(c["collapsible"].as_bool().unwrap());
            let p = RibbonAutomationPeer::new(&r);
            p.collapse();
            for expected in c["steps"].as_array().unwrap() {
                assert_eq!(
                    serde_json::json!({"mode":r.visibility_mode().raw(),"state":format!("{:?}",p.expand_collapse_state()),"pattern":p.get_provider::<dyn IExpandCollapseProvider>().is_some()}),
                    *expected,
                    "{c}"
                );
                p.expand();
            }
        }
    });
}
#[test]
fn upstream_minimize_cycle_tabs_popup_full_screen_and_reveal() {
    const NAME: &str = "controls::ribbon::visibility::tests::upstream_minimize_cycle_tabs_popup_full_screen_and_reveal";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        let full = r.bounds().height;
        r.set_minimize_behavior(RibbonMinimizeBehavior::CycleAll);
        r.toggle_minimized();
        resize(&w, 1400., 300.);
        assert_eq!(r.visibility_mode(), RibbonVisibilityMode::PanelButtons);
        let g = r.tabs().get(0).unwrap().groups().get(0).unwrap();
        assert_eq!(g.state(), RibbonGroupState::Collapsed);
        assert!(g.collapsed_button().unwrap().is_visible());
        r.toggle_minimized();
        resize(&w, 1400., 300.);
        assert_eq!(r.visibility_mode(), RibbonVisibilityMode::PanelTitles);
        assert!(g.collapsed_button().unwrap().icon().is_none());
        assert!(r.bounds().height < full - 20.);
        r.toggle_minimized();
        resize(&w, 1400., 300.);
        assert_eq!(r.visibility_mode(), RibbonVisibilityMode::TabsOnly);
        assert!(r.bounds().height < full - 40.);
        r.open_minimized_popup();
        assert!(r.is_minimized_popup_open());
        r.close_minimized_popup();
        r.toggle_minimized();
        resize(&w, 1400., 300.);
        assert_eq!(r.visibility_mode(), RibbonVisibilityMode::AlwaysShow);
        assert!((r.bounds().height - full).abs() < 2.);
        assert_ne!(g.state(), RibbonGroupState::Collapsed);
        r.set_visibility_mode(RibbonVisibilityMode::FullScreen);
        resize(&w, 1400., 300.);
        assert!(r.bounds().height < 40.);
        let reveal = r.parts.borrow().reveal.clone().unwrap();
        click(&reveal);
        resize(&w, 1400., 300.);
        assert!(r.bounds().height > 100.);
        g.apply_state(RibbonGroupState::Collapsed, false, g.metrics(), None);
        g.open_popup();
        assert!(g.is_popup_open());
        let revealed_on_close = Rc::new(RefCell::new(Vec::new()));
        let seen = revealed_on_close.clone();
        let weak = r.downgrade();
        let weak_group = g.downgrade();
        let subscription = Popup::is_open_property().changed().subscribe(move |e| {
            if let Some(p) = e.sender().downcast_ref::<Popup>()
                && !p.is_open()
                && let (Some(r), Some(g)) = (weak.upgrade(), weak_group.upgrade())
                && !g.is_popup_open()
            {
                seen.borrow_mut().push(r.is_full_screen_revealed());
            }
        });
        r.on_item_invoked(g.items().get(0).unwrap(), None, None);
        subscription.dispose();
        let corpus = fixture();
        let source = corpus["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "invocationOrder")
            .unwrap();
        let observed = revealed_on_close.borrow();
        assert_eq!(
            observed.len() as u64,
            source["PopupCloses"].as_u64().unwrap()
        );
        assert_eq!(
            format!("closing:{}", if observed[0] { "True" } else { "False" }),
            source["trace"][0].as_str().unwrap()
        );
        assert!(!r.is_full_screen_revealed());
        w.close();
        w.set_content(None);
    });
}
#[test]
fn source_minimized_popup_preserves_bar_parent_index_fractional_offsets_and_escape() {
    const NAME: &str = "controls::ribbon::visibility::tests::source_minimized_popup_preserves_bar_parent_index_fractional_offsets_and_escape";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        for c in fixture()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "popup")
        {
            let (r, w) = setup();
            // The frozen source seam specifies the TAB ROW origin at the ribbon
            // margin. Hide the new actual QAT to realize that declared geometry;
            // QAT host/height transitions are tested with the full template separately.
            r.set_is_quick_access_visible(false);
            r.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
            w.set_use_layout_rounding(false);
            r.set_margin(ferroui_base::Thickness::new(13.5, 40.25, 0., 0.));
            r.set_custom_metrics(Some(r.metrics().with_values(|v| v.tab_height = 30.5)));
            resize(&w, 914., 300.);
            let bar = r.parts.borrow().command_bar.clone().unwrap();
            let original = bar.visual_parent().unwrap().cast::<Panel>().unwrap();
            let at = original.children().index_of(&bar.clone().upcast()).unwrap();
            original.children().remove(bar.clone());
            let parent = Panel::new();
            let before = Border::new();
            before.set_name(Some("before".into()));
            let after = Border::new();
            after.set_name(Some("after".into()));
            parent.children().add(before);
            parent.children().add(bar.clone());
            parent.children().add(after);
            ferroui_controls::Grid::set_row(&parent, 2);
            original.children().insert(at, parent.clone());
            resize(&w, 914., 300.);
            r.open_minimized_popup();
            assert!(r.is_minimized_popup_open());
            let p = r.visibility.popup.borrow().clone().unwrap();
            assert!(p.chrome.child().unwrap().ptr_eq(&bar));
            assert!(!p.popup.is_light_dismiss_enabled());
            assert_eq!(p.chrome.width(), c["width"].as_f64().unwrap());
            assert_eq!(p.popup.horizontal_offset(), c["x"].as_f64().unwrap());
            assert_eq!(p.popup.vertical_offset(), c["y"].as_f64().unwrap());
            p.index.set(c["index"].as_i64().unwrap() as i32);
            r.close_minimized_popup();
            assert!(bar.visual_parent().unwrap().ptr_eq(&parent));
            let labels = parent
                .children()
                .snapshot()
                .iter()
                .map(|v| {
                    if v.ptr_eq(&bar) {
                        "bar".to_owned()
                    } else {
                        v.name().unwrap()
                    }
                })
                .collect::<Vec<_>>();
            assert_eq!(serde_json::json!(labels), c["order"]);
            r.open_minimized_popup();
            let mut e = KeyEventArgs::new();
            e.key = Key::Escape;
            e.set_routed_event(Some(InputElement::key_down_event()));
            bar.raise_event(&e);
            assert!(e.handled());
            assert!(!r.is_minimized_popup_open());
            assert!(
                r.selected_tab()
                    .unwrap()
                    .header_element()
                    .unwrap()
                    .is_focused()
            );
            w.close();
            w.set_content(None);
        }
    });
}
#[test]
fn source_deferred_header_and_item_exemptions_and_window_keyboard() {
    const NAME: &str = "controls::ribbon::visibility::tests::source_deferred_header_and_item_exemptions_and_window_keyboard";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        r.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        resize(&w, 1400., 300.);
        let h = r.selected_tab().unwrap().header_element().unwrap();
        click(&h);
        assert!(!r.is_minimized_popup_open());
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(r.is_minimized_popup_open());
        click(&h);
        assert!(!r.is_minimized_popup_open());
        click(&h);
        r.set_visibility_mode(RibbonVisibilityMode::AlwaysShow);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(!r.is_minimized_popup_open());
        r.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        r.open_minimized_popup();
        r.on_item_invoked(h.clone().upcast(), None, None);
        assert!(r.is_minimized_popup_open());
        let b = RibbonButton::new();
        b.set_flyout(Some(MenuFlyout::new().upcast()));
        r.on_item_invoked(b.upcast(), None, None);
        assert!(r.is_minimized_popup_open());
        let g = r.selected_tab().unwrap().groups().get(0).unwrap();
        r.on_item_invoked(g.items().get(0).unwrap(), None, None);
        assert!(!r.is_minimized_popup_open());
        let mut e = KeyEventArgs::new();
        e.key = Key::F1;
        e.key_modifiers = KeyModifiers::CONTROL;
        e.set_routed_event(Some(InputElement::key_down_event()));
        r.raise_event(&e);
        assert!(e.handled());
        assert_eq!(r.visibility_mode(), RibbonVisibilityMode::AlwaysShow);
        let mut blocked = KeyEventArgs::new();
        blocked.key = Key::F1;
        blocked.key_modifiers = KeyModifiers::CONTROL;
        blocked.set_routed_event(Some(InputElement::key_down_event()));
        blocked.set_handled(true);
        r.raise_event(&blocked);
        assert_eq!(r.visibility_mode(), RibbonVisibilityMode::AlwaysShow);
        r.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        r.open_minimized_popup();
        resize(&w, 1300., 300.);
        assert!(!r.is_minimized_popup_open());
        r.open_minimized_popup();
        assert!(r.is_minimized_popup_open());
        r.suspend_popups();
        assert!(!r.is_minimized_popup_open());
        r.open_minimized_popup();
        assert!(r.is_minimized_popup_open());
        w.close();
        assert!(!r.is_minimized_popup_open());
        assert!(r.visibility.input.borrow().is_none());
        w.set_content(None);
    });
}

#[test]
fn source_reentrant_owner_publication_restore_guard_and_native_lifetime() {
    const NAME: &str = "controls::ribbon::visibility::tests::source_reentrant_owner_publication_restore_guard_and_native_lifetime";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        use crate::controls::RibbonItemHelper;
        let (r, w) = setup();
        r.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        resize(&w, 1400., 300.);
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let weak = r.downgrade();
        let sub = RibbonItemHelper::owner_property()
            .changed()
            .subscribe(move |e| {
                if e.sender().is::<Border>()
                    && let Some(r) = weak.upgrade()
                {
                    count.set(count.get() + 1);
                    if count.get() == 1 {
                        assert!(r.visibility.popup.borrow().is_some());
                        r.open_minimized_popup();
                    }
                }
            });
        r.open_minimized_popup();
        sub.dispose();
        assert_eq!(calls.get(), 1);
        assert!(r.is_minimized_popup_open());
        let state = r.visibility.popup.borrow().clone().unwrap();
        let bar = r.parts.borrow().command_bar.clone().unwrap();
        assert!(state.chrome.child().unwrap().ptr_eq(&bar));
        r.close_minimized_popup();
        assert!(bar.visual_parent().is_some());
        r.open_minimized_popup();
        r.parts.borrow_mut().command_bar = Some(Border::new());
        r.close_minimized_popup();
        assert!(!r.is_minimized_popup_open());
        assert!(
            state.chrome.child().unwrap().ptr_eq(&bar),
            "original restore identity guard retains a replaced bar"
        );
        w.close();
        w.set_content(None);
        let weak = r.downgrade();
        drop(r);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
    });
}
#[test]
fn minimize_menu_pointer_edges_template_subscription_cleanup() {
    const NAME: &str = "controls::ribbon::visibility::tests::minimize_menu_pointer_edges_template_subscription_cleanup";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        use crate::compat::RadioMenuFlyoutItem;
        use ferroui_base::input::{Pointer, PointerPointProperties, PointerType};
        let (r, w) = setup();
        r.set_is_minimize_button_visible(true);
        let minimize = r.parts.borrow().minimize.clone().unwrap();
        assert!(
            r.parts
                .borrow()
                .minimize_buttons
                .as_ref()
                .unwrap()
                .is_visible()
        );
        let menu = r.build_minimize_behavior_menu();
        let corpus = fixture();
        let source_menu = corpus["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "menu")
            .unwrap();
        let rows = source_menu["rows"].as_array().unwrap();
        assert_eq!(menu.items().count(), rows.len());
        for (i, row) in rows.iter().enumerate() {
            let value = menu.items().get_at(i).unwrap();
            let item = ferroui_base::data::core::ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<RadioMenuFlyoutItem>()
                .unwrap();
            assert_eq!(item.group_name().as_deref(), row["GroupName"].as_str());
            assert_eq!(item.text().as_deref(), row["Text"].as_str());
            assert_eq!(item.is_checked(), row["IsChecked"].as_bool().unwrap());
            item.invoke_menu_core();
            assert_eq!(
                r.minimize_behavior().raw() as i64,
                source_menu["actions"][i].as_i64().unwrap()
            );
        }
        r.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        resize(&w, 1400., 300.);
        let row = r.parts.borrow().tab_row.clone().unwrap();
        let point = row
            .translate_point(Point::new(row.bounds().width, row.bounds().height), &w)
            .unwrap();
        r.open_minimized_popup();
        let e = PointerPressedEventArgs::new(
            r.to_ref().upcast::<FerroObject>(),
            Pointer::new(1, PointerType::Mouse, true),
            &w,
            point,
            0,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
            1,
        );
        r.raise_event(&e);
        assert!(r.is_minimized_popup_open(), "inclusive tab-row boundary");
        let point = Point::new(1300., 299.);
        let e = PointerPressedEventArgs::new(
            r.to_ref().upcast::<FerroObject>(),
            Pointer::new(1, PointerType::Mouse, true),
            &w,
            point,
            0,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
            1,
        );
        r.raise_event(&e);
        assert!(!r.is_minimized_popup_open());
        r.set_visibility_mode(RibbonVisibilityMode::FullScreen);
        r.set_is_full_screen_revealed(true);
        r.raise_event(&e);
        assert!(!r.is_full_screen_revealed());
        r.set_visibility_mode(RibbonVisibilityMode::AlwaysShow);
        // Original OnApplyTemplate publishes content before minimize labels,
        // and the command bar afterward. Observe a real property callback.
        let old_bar = r.parts.borrow().command_bar.clone().unwrap();
        let next_bar = Border::new();
        let next_content = ferroui_controls::Grid::new();
        let next_button = Button::new();
        let observations = Rc::new(RefCell::new(Vec::new()));
        let seen = observations.clone();
        let weak = r.downgrade();
        let button = next_button.clone();
        let content = next_content.clone();
        let old = old_bar.clone();
        let subscription = ferroui_controls::automation::AutomationProperties::name_property()
            .changed()
            .subscribe(move |e| {
                if e.sender().to_ref().ptr_eq(&button) {
                    let r = weak.upgrade().unwrap();
                    let parts = r.parts.borrow_mut();
                    seen.borrow_mut().push((
                        parts.command_bar.as_ref().unwrap().ptr_eq(&old),
                        parts.content.as_ref().unwrap().ptr_eq(&content),
                    ));
                }
            });
        let bar = next_bar.clone();
        let content = next_content.clone();
        let button = next_button.clone();
        r.set_template(Some(
            ferroui_controls::templates::FuncControlTemplate::for_type::<Ribbon>(
                move |_, scope| {
                    let root = Panel::new();
                    scope.register("PART_MinimizeButton", button.clone().upcast());
                    scope.register("PART_CommandBar", bar.clone().upcast());
                    scope.register("PART_TabContentHost", content.clone().upcast());
                    bar.set_child(content.clone());
                    root.children().add(button.clone());
                    root.children().add(bar.clone());
                    root.upcast()
                },
            ),
        ));
        r.apply_template();
        subscription.dispose();
        let source = fixture()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "templatePhase")
            .unwrap()
            .clone();
        assert_eq!(
            &*observations.borrow(),
            &[(
                source["oldBar"].as_bool().unwrap(),
                source["newContent"].as_bool().unwrap()
            )]
        );
        assert_eq!(
            r.parts
                .borrow()
                .command_bar
                .as_ref()
                .unwrap()
                .ptr_eq(&next_bar),
            source["newBar"].as_bool().unwrap()
        );
        click(&minimize);
        assert_eq!(r.visibility_mode(), RibbonVisibilityMode::AlwaysShow);
        r.set_template(Some(
            ferroui_controls::templates::FuncControlTemplate::for_type::<Ribbon>(|_, _| {
                Panel::new().upcast()
            }),
        ));
        r.apply_template();
        click(&next_button);
        click(&minimize);
        assert_eq!(
            r.visibility_mode(),
            RibbonVisibilityMode::AlwaysShow,
            "obsolete part handler removed"
        );
        assert!(r.parts.borrow().minimize.is_none());
        w.close();
        w.set_content(None);
    });
}

#[test]
fn source_minimize_label_callback_reloads_tooltip_receiver() {
    const NAME: &str = "controls::ribbon::visibility::tests::source_minimize_label_callback_reloads_tooltip_receiver";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let original = Button::new();
        let replacement = Button::new();
        r.parts.borrow_mut().minimize = Some(original.clone());
        let weak = r.downgrade();
        let first = original.clone();
        let next = replacement.clone();
        let sub = AutomationProperties::name_property()
            .changed()
            .subscribe(move |e| {
                if e.sender().to_ref().ptr_eq(&first) {
                    weak.upgrade().unwrap().parts.borrow_mut().minimize = Some(next.clone());
                }
            });
        r.update_minimize_button();
        sub.dispose();
        let corpus = fixture();
        let source = corpus["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "minimizeReceiver")
            .unwrap();
        let tip =
            |b: &Button| ToolTip::get_tip(b).and_then(|v| v.downcast_ref::<String>().cloned());
        assert_eq!(tip(&original).as_deref(), source["originalTip"].as_str());
        assert_eq!(
            tip(&replacement).as_deref(),
            source["replacementTip"].as_str()
        );
    });
}
