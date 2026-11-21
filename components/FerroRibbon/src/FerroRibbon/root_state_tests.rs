//! Unchanged State.cs/Core oracle plus actual native identity/binding/lifecycle checks.
use super::*;
use crate::controls::RibbonButton;
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{RibbonQuickAccessPosition, state::*};
use ferroui_base::{Rect, Size, threading::Dispatcher};
use ferroui_controls::Window;
use serde_json::{Value, json};
fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_STATE_ISOLATED").as_deref() == Ok(name) {
        return false;
    }
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_STATE_ISOLATED", name)
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
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/root-state.json")).unwrap()
}
fn case(name: &str) -> Value {
    corpus()["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap()
        .clone()
}
fn canonical(v: &Value) -> Value {
    match v {
        Value::Number(n) => json!(n.as_f64().unwrap()),
        Value::Array(a) => Value::Array(a.iter().map(canonical).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, v)| (k.clone(), canonical(v))).collect())
        }
        _ => v.clone(),
    }
}
fn text(v: Option<RibbonString>) -> Option<String> {
    v.map(|v| v.to_utf8().unwrap())
}
fn setup() -> (Ref<Ribbon>, Ref<Window>) {
    let r = Ribbon::new();
    r.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Top);
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    let t = RibbonTab::new();
    t.set_id(Some("home".into()));
    t.set_header(Some("Home".into()));
    let g = RibbonGroup::new();
    g.set_id(Some("tools".into()));
    g.set_header(Some("Tools".into()));
    for id in ["a", "b", "c"] {
        let item = RibbonButton::new();
        item.set_id(Some(id.into()));
        item.set_name(Some(id.into()));
        item.set_label(Some(id.into()));
        g.items().add(Some(item.upcast()));
    }
    t.groups().add(Some(g));
    r.tabs().add(Some(t));
    let t = RibbonTab::new();
    t.set_id(Some("insert".into()));
    t.set_header(Some("Insert".into()));
    r.tabs().add(Some(t));
    let save = RibbonButton::new();
    save.set_id(Some("save".into()));
    save.set_name(Some("save".into()));
    r.quick_access_tool_bar()
        .unwrap()
        .items()
        .add(Some(save.upcast()));
    let w = crate::ribbon_tests::mount(&r);
    w.set_height(600.);
    w.measure(Size::new(1400., 600.));
    w.arrange(Rect::new(0., 0., 1400., 600.));
    Dispatcher::ui_thread().run_jobs(None);
    (r, w)
}
fn snapshot(r: &Ribbon) -> Value {
    json!({"selected":r.selected_tab().and_then(|t|t.effective_id()),"tabs":r.tabs().iter().map(|t|{
        let t=t.unwrap();json!({"id":t.id(),"header":t.header(),"hidden":t.hidden_by_customization(),"custom":t.is_custom.get(),"order":t.group_order().map(|o|o.iter().map(text).collect::<Vec<_>>()),"groups":t.groups().iter().map(|g|{let g=g.unwrap();json!({"id":g.id(),"header":g.header(),"hidden":g.hidden_by_customization(),"custom":g.is_custom(),"items":g.items().iter().flatten().map(|i|json!({"id":text(as_item(&i).unwrap().id()),"source":RibbonItemHelper::get_source_item(&i).and_then(|s|as_item(&s)).and_then(|i|text(i.id())),"linked":RibbonItemHelper::get_source_item(&i).is_some()})).collect::<Vec<_>>()})}).collect::<Vec<_>>()})
    }).collect::<Vec<_>>(),"visible":r.visible_tabs().iter().map(|t|t.id()).collect::<Vec<_>>(),"qat":r.get_quick_access_item_ids().iter().map(text).collect::<Vec<_>>(),"recent":r.search_engine().recent().iter().map(text).collect::<Vec<_>>(),"floating":r.get_state().unwrap().floating_groups().unwrap().iter().map(|s|{let s=s.unwrap();json!({"GroupId":text(s.group_id()),"X":s.x(),"Y":s.y()})}).collect::<Vec<_>>()})
}
fn assert_snapshot(r: &Ribbon, name: &str) {
    assert_eq!(
        canonical(&snapshot(r)),
        canonical(&case(name)["snapshot"]),
        "source case {name}"
    );
}
fn customization() -> Rc<RibbonCustomization> {
    let c = RibbonCustomization::new();
    for id in ["custom", "insert", "home"] {
        c.tab_order().unwrap().add(Some(id.into()));
    }
    c.hidden_group_ids().unwrap().add(Some("tools".into()));
    c.labels()
        .unwrap()
        .insert("home".into(), Some("Renamed".into()));
    c.labels()
        .unwrap()
        .insert("tools".into(), Some("Tools renamed".into()));
    let t = RibbonCustomTab::new();
    t.set_id(Some("custom".into()));
    t.set_label(Some("Custom".into()));
    let group = RibbonCustomGroup::new();
    group.set_id(Some("mine".into()));
    group.set_label(Some("Mine".into()));
    for id in ["b", "missing", "a", "b"] {
        group.item_ids().unwrap().add(Some(id.into()));
    }
    t.groups().unwrap().add(Some(group));
    c.custom_tabs().unwrap().add(Some(t));
    let group = RibbonCustomGroup::new();
    group.set_id(Some("extra".into()));
    group.set_label(Some("Extra".into()));
    group.set_tab_id(Some("home".into()));
    group.item_ids().unwrap().add(Some("c".into()));
    c.custom_groups().unwrap().add(Some(group));
    c
}
#[test]
fn source_defaults_capture_and_json_use_actual_state_pipeline() {
    const NAME: &str = "controls::ribbon::state::tests::source_defaults_capture_and_json_use_actual_state_pipeline";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        assert_snapshot(&r, "defaults");
        assert_eq!(
            r.save_state_to_json().unwrap().to_utf8().unwrap(),
            case("defaults")["json"].as_str().unwrap()
        );
        let first = r.get_state().unwrap();
        let second = r.get_state().unwrap();
        assert!(!Rc::ptr_eq(&first, &second));
        assert!(!Rc::ptr_eq(
            &first.customization().unwrap(),
            &r.customization()
        ));
        assert!(Rc::ptr_eq(&r.customization(), &r.customization()));
        w.close();
    });
}
#[test]
fn source_customize_clones_rebuilds_selection_and_unlinks_removed_actual_copies() {
    const NAME: &str = "controls::ribbon::state::tests::source_customize_clones_rebuilds_selection_and_unlinks_removed_actual_copies";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        let c = customization();
        r.apply_customization(Some(c.clone())).unwrap();
        assert!(!Rc::ptr_eq(&c, &r.customization()));
        assert_snapshot(&r, "customize");
        assert_eq!(
            r.save_state_to_json().unwrap().to_utf8().unwrap(),
            case("customize")["json"].as_str().unwrap()
        );
        r.select_tab_id("custom");
        let old = r.selected_tab().unwrap();
        let old_copy = old
            .groups()
            .get(0)
            .unwrap()
            .items()
            .get(0)
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        let source = r
            .find_item(Some("b".into()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        source.set_label(Some("source changed".into()));
        assert_eq!(old_copy.label(), source.label());
        r.apply_customization(Some(c)).unwrap();
        assert!(!r.selected_tab().unwrap().ptr_eq(&old));
        assert_snapshot(&r, "custom_selection");
        source.set_label(Some("after unlink".into()));
        assert_eq!(old_copy.label().as_deref(), Some("source changed"));
        r.reset_customization().unwrap();
        assert_snapshot(&r, "reset");
        w.close();
    });
}
#[test]
fn source_state_reorders_surviving_qat_copies_and_reset_uses_defaults() {
    const NAME: &str = "controls::ribbon::state::tests::source_state_reorders_surviving_qat_copies_and_reset_uses_defaults";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        for id in ["a", "b", "c"] {
            assert!(r.add_to_quick_access_id(Some(id.into())));
        }
        let qat = r.quick_access_tool_bar().unwrap();
        let a = qat.items().get(1).unwrap();
        let b = qat.items().get(2).unwrap();
        let dropped = qat.items().get(3).unwrap().cast::<RibbonButton>().unwrap();
        let state = r.get_state().unwrap();
        state.set_quick_access_item_ids(Some(RibbonItems::from_values(
            ["b", "a", "b", "missing", "save"]
                .map(|v| Some(v.into()))
                .to_vec(),
        )));
        r.apply_saved_state(Some(state)).unwrap();
        assert!(qat.items().get(0).unwrap().ptr_eq(&b));
        assert!(qat.items().get(1).unwrap().ptr_eq(&a));
        assert_snapshot(&r, "qat_reorder");
        let source = r
            .find_item(Some("c".into()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        source.set_label(Some("changed after drop".into()));
        assert_eq!(dropped.label().as_deref(), Some("c"));
        r.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
        r.set_is_quick_access_visible(false);
        r.reset_quick_access().unwrap();
        assert_snapshot(&r, "qat_reset");
        assert!(r.is_quick_access_visible());
        assert_eq!(
            r.quick_access_position(),
            RibbonQuickAccessPosition::AboveRibbon
        );
        w.close();
    });
}
#[test]
fn source_state_floating_capture_first_duplicate_and_empty_restore() {
    const NAME: &str = "controls::ribbon::state::tests::source_state_floating_capture_first_duplicate_and_empty_restore";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        let group = r.tabs().get(0).unwrap().groups().get(0).unwrap();
        group.float(Some(ferroui_base::Point::new(80.25, 120.5)));
        let state = r.get_state().unwrap();
        assert_snapshot(&r, "floating_capture");
        let (restored, restored_window) = setup();
        let restored_group = restored.tabs().get(0).unwrap().groups().get(0).unwrap();
        let original_panel = restored_group.items_panel();
        restored
            .load_state_from_json(Some(r.save_state_to_json().unwrap()))
            .unwrap();
        assert!(restored_group.is_floating_panel_open());
        assert_eq!(
            restored_group.floating_position(),
            ferroui_base::Point::new(80.25, 120.5)
        );
        assert!(restored_group.items_panel().ptr_eq(&original_panel));
        restored_group.return_to_ribbon();
        restored_window.close();
        let floating = RibbonItems::new();
        for (x, y) in [(30.5, 60.25), (999., 999.)] {
            let s = RibbonFloatingGroupState::new();
            s.set_group_id(Some("tools".into()));
            s.set_x(x);
            s.set_y(y);
            floating.add(Some(s));
        }
        state.set_floating_groups(Some(floating));
        r.apply_saved_state(Some(state.clone())).unwrap();
        assert_snapshot(&r, "floating_first_duplicate");
        assert!(group.is_floating_panel_open());
        state.set_floating_groups(Some(RibbonItems::new()));
        r.apply_saved_state(Some(state)).unwrap();
        assert_snapshot(&r, "floating_clear");
        assert!(!group.is_floating_panel_open());
        w.close();
    });
}
#[test]
fn source_invalid_json_and_unpaired_missing_lookup_ids() {
    const NAME: &str =
        "controls::ribbon::state::tests::source_invalid_json_and_unpaired_missing_lookup_ids";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        let values = [
            None,
            Some(""),
            Some("{"),
            Some("null"),
            Some("{\"unknown\":123}"),
        ];
        let result = values.map(|v| r.load_state_from_json(v.map(Into::into)).unwrap());
        assert_eq!(json!(result), case("invalid_json")["results"]);
        let state = r.get_state().unwrap();
        let bad = RibbonString::from_utf16(vec![0xd800]);
        state.set_selected_tab_id(Some(bad.clone()));
        state.set_quick_access_item_ids(Some(RibbonItems::from_values(vec![
            Some("save".into()),
            Some(bad),
        ])));
        r.apply_saved_state(Some(state)).unwrap();
        assert_snapshot(&r, "unpaired_missing_ids");
        assert!(r.apply_saved_state(None).unwrap_err().contains("state"));
        assert!(
            r.apply_customization(None)
                .unwrap_err()
                .contains("customization")
        );
        w.close();
    });
}

// A real downstream-style native override throws while State.cs is cloning a command.
#[repr(C)]
struct FailingCopyButton {
    base: RibbonButton,
}
ferroui_base::ferro_class!(FailingCopyButton:RibbonButton);
ferroui_base::ferro_class_info!(FailingCopyButton {});
ferroui_base::ferro_impl_classes!(FailingCopyButton:
 ferroui_base::FerroObjectImpl,ferroui_base::StyledElementImpl,ferroui_base::VisualImpl,
 ferroui_base::layout::LayoutableImpl,ferroui_base::interactivity::InteractiveImpl,
 ferroui_base::input::InputElementImpl,ferroui_controls::ControlImpl,
 ferroui_controls::primitives::TemplatedControlImpl,ferroui_controls::ContentControlImpl,ferroui_controls::ButtonImpl);
impl crate::controls::RibbonButtonImpl for FailingCopyButton {
    fn create_linked_copy_core(_this: &Self) -> Option<Ref<Control>> {
        panic!("copy failed")
    }
}
#[test]
fn source_copy_failure_resets_finally_guard_and_later_tab_changes_work() {
    const NAME: &str = "controls::ribbon::state::tests::source_copy_failure_resets_finally_guard_and_later_tab_changes_work";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        let item = ferroui_base::instantiate(FailingCopyButton {
            base: RibbonButton::construct(),
        });
        item.set_id(Some("bad".into()));
        r.tabs()
            .get(0)
            .unwrap()
            .groups()
            .get(0)
            .unwrap()
            .items()
            .add(Some(item.upcast()));
        let c = RibbonCustomization::new();
        let g = RibbonCustomGroup::new();
        g.set_id(Some("badgroup".into()));
        g.set_tab_id(Some("home".into()));
        g.item_ids().unwrap().add(Some("bad".into()));
        c.custom_groups().unwrap().add(Some(g));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            r.apply_customization(Some(c))
        }));
        let error = result.unwrap_err();
        assert_eq!(
            error.downcast_ref::<&str>().copied(),
            case("copy_failure")["error"].as_str()
        );
        assert_eq!(
            r.state.applying.get(),
            case("copy_failure")["applying"].as_bool().unwrap()
        );
        let t = RibbonTab::new();
        t.set_id(Some("after".into()));
        r.tabs().add(Some(t.clone()));
        assert!(r.visible_tabs().iter().any(|v| v.ptr_eq(&t)));
        r.reset_customization().unwrap();
        w.close();
    });
}
#[test]
fn source_recent_alias_and_custom_group_qat_permission_are_preserved() {
    const NAME: &str = "controls::ribbon::state::tests::source_recent_alias_and_custom_group_qat_permission_are_preserved";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        r.search_engine().mark_used(Some("a".into()));
        r.search_engine().mark_used(Some("b".into()));
        let state = r.get_state().unwrap();
        let recent = r.search_engine().recent();
        state.set_recent_search_ids(Some(recent.clone()));
        r.apply_saved_state(Some(state)).unwrap();
        assert!(Rc::ptr_eq(&recent, &r.search_engine().recent()));
        assert_snapshot(&r, "recent_alias");
        let item = r
            .find_item(Some("a".into()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        item.set_can_add_to_quick_access(false);
        let c = RibbonCustomization::new();
        let g = RibbonCustomGroup::new();
        g.set_id(Some("copies".into()));
        g.set_tab_id(Some("home".into()));
        g.item_ids().unwrap().add(Some("a".into()));
        c.custom_groups().unwrap().add(Some(g));
        r.apply_customization(Some(c)).unwrap();
        assert_snapshot(&r, "custom_ignores_qat_permission");
        w.close();
    });
}
#[test]
fn source_state_event_reentry_uses_new_customization_and_releases_subscriptions() {
    const NAME: &str = "controls::ribbon::state::tests::source_state_event_reentry_uses_new_customization_and_releases_subscriptions";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let weak = r.downgrade();
        let handler = MarkupDelegate::new(move |_| {
            count.set(count.get() + 1);
            if count.get() == 1 {
                weak.upgrade().unwrap().reset_customization().unwrap();
            }
            None
        });
        r.add_state_changed(Some(handler.clone()));
        let c = RibbonCustomization::new();
        c.labels()
            .unwrap()
            .insert("home".into(), Some("Temporary".into()));
        r.apply_customization(Some(c)).unwrap();
        assert_eq!(
            calls.get(),
            case("event_reentry")["calls"].as_u64().unwrap() as usize
        );
        assert_snapshot(&r, "event_reentry");
        r.remove_state_changed(Some(handler));
        let before = calls.get();
        r.reset_customization().unwrap();
        assert_eq!(calls.get(), before);
        w.close();
    });
}
#[test]
fn upstream_state_round_trip_and_group_order_preserve_original_collections() {
    const NAME: &str = "controls::ribbon::state::tests::upstream_state_round_trip_and_group_order_preserve_original_collections";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        let home = r.tabs().get(0).unwrap();
        let tools = home.groups().get(0).unwrap();
        let second = RibbonGroup::new();
        second.set_id(Some("other".into()));
        home.groups().add(Some(second.clone()));
        r.select_tab_id("insert");
        r.set_display_mode(ferroribbon_core::RibbonDisplayMode::Simplified);
        r.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
        r.set_show_group_captions(false);
        r.add_to_quick_access_id(Some("a".into()));
        let c = customization();
        c.group_order().unwrap().insert(
            "home".into(),
            Some(RibbonItems::from_values(vec![
                Some("other".into()),
                Some("tools".into()),
            ])),
        );
        r.apply_customization(Some(c)).unwrap();
        assert!(home.groups().get(0).unwrap().ptr_eq(&tools));
        assert!(home.display_groups().get(0).unwrap().ptr_eq(&second));
        let json = r.save_state_to_json().unwrap();
        r.reset_customization().unwrap();
        assert!(home.display_groups().get(0).unwrap().ptr_eq(&tools));
        r.load_state_from_json(Some(json.clone())).unwrap();
        assert!(home.display_groups().get(0).unwrap().ptr_eq(&second));
        let (restored, other_window) = setup();
        restored.load_state_from_json(Some(json)).unwrap();
        assert_eq!(
            restored.selected_tab().unwrap().id().as_deref(),
            Some("insert")
        );
        assert_eq!(
            restored.display_mode(),
            ferroribbon_core::RibbonDisplayMode::Simplified
        );
        assert_eq!(
            restored.quick_access_position(),
            RibbonQuickAccessPosition::BelowRibbon
        );
        assert!(!restored.get_state().unwrap().show_group_captions());
        assert_eq!(
            restored
                .get_quick_access_item_ids()
                .iter()
                .collect::<Vec<_>>(),
            vec![Some("save".into()), Some("a".into())]
        );
        assert!(
            !restored
                .tabs()
                .get(0)
                .unwrap()
                .groups()
                .get(0)
                .unwrap()
                .is_visible()
        );
        assert_eq!(
            restored
                .find_tab("custom")
                .unwrap()
                .groups()
                .get(0)
                .unwrap()
                .items()
                .count(),
            3
        );
        restored.reset_customization().unwrap();
        assert!(restored.find_tab("custom").is_none());
        assert_eq!(
            restored.find_tab("home").unwrap().header().as_deref(),
            Some("Home")
        );
        other_window.close();
        w.close();
    });
}
#[test]
fn source_normalizes_absent_definitions_but_keeps_duplicate_ids_and_copies() {
    const NAME: &str = "controls::ribbon::state::tests::source_normalizes_absent_definitions_but_keeps_duplicate_ids_and_copies";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let (r, w) = setup();
        let c = RibbonCustomization::new();
        c.custom_tabs().unwrap().add(None);
        let empty = ferroribbon_core::state::RibbonCustomTab::new();
        empty.set_id(Some("".into()));
        empty.set_label(Some("drop".into()));
        c.custom_tabs().unwrap().add(Some(empty));
        for (label, group_label, ids) in [
            ("First", "One", vec![None, Some(""), Some("a"), Some("a")]),
            ("Second", "Two", vec![Some("b")]),
        ] {
            let tab = ferroribbon_core::state::RibbonCustomTab::new();
            tab.set_id(Some("duplicate".into()));
            tab.set_label(Some(label.into()));
            if label == "First" {
                tab.groups().unwrap().add(None);
            }
            let group = RibbonCustomGroup::new();
            group.set_id(Some("same".into()));
            group.set_label(Some(group_label.into()));
            for id in ids {
                group.item_ids().unwrap().add(id.map(Into::into));
            }
            tab.groups().unwrap().add(Some(group));
            c.custom_tabs().unwrap().add(Some(tab));
        }
        r.apply_customization(Some(c)).unwrap();
        r.select_tab_id("duplicate");
        assert_snapshot(&r, "duplicate_custom_ids");
        assert_eq!(
            r.save_state_to_json().unwrap().to_utf8().unwrap(),
            case("duplicate_custom_ids")["json"].as_str().unwrap()
        );
        let first = r.tabs().get(2).unwrap().groups().get(0).unwrap();
        assert_eq!(first.items().count(), 2);
        assert!(
            !first
                .items()
                .get(0)
                .unwrap()
                .ptr_eq(&first.items().get(1).unwrap())
        );
        w.close();
    });
}
