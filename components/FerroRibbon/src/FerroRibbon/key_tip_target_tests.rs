//! Unchanged source topology corpus against retained native controls and real popups.
use super::*;
use ferroribbon_core::collections::RibbonFixedItems;
use ferroui_controls::{TextBox, Window};
fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_KEYTIP_TARGET_ISOLATED").as_deref() == Ok(name) {
        return false;
    }
    let o = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_KEYTIP_TARGET_ISOLATED", name)
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
fn item(name: &str) -> Ref<super::super::RibbonButton> {
    let b = super::super::RibbonButton::new();
    b.set_name(Some(name.into()));
    b
}
fn trace_name(c: &Control) -> String {
    c.name()
        .or_else(|| c.tag().and_then(|v| v.downcast_ref::<String>().cloned()))
        .expect("trace identity")
}
fn names(e: ferroribbon_core::model::RibbonItemsSource<Ref<Control>>) -> Vec<String> {
    e.iter().map(|c| trace_name(&c)).collect()
}
fn setup() -> (
    Ref<super::super::Ribbon>,
    Ref<super::super::RibbonTab>,
    Ref<RibbonGroup>,
    Ref<Window>,
) {
    let root = super::super::Ribbon::new();
    let tab = super::super::RibbonTab::new();
    let g = RibbonGroup::new();
    let a = item("a");
    g.items().add(Some(a.upcast()));
    let c = super::super::RibbonStackPanel::new();
    c.set_name(Some("container".into()));
    c.items().add(Some(item("nested").upcast()));
    let hidden = item("hiddenNested");
    hidden.set_is_visible(false);
    c.items().add(Some(hidden.upcast()));
    c.items()
        .add(Some(super::super::RibbonLabel::new().upcast()));
    g.items().add(Some(c.upcast()));
    let raw = TextBox::new();
    raw.set_name(Some("raw".into()));
    g.items().add(Some(raw.upcast()));
    g.items()
        .add(Some(super::super::RibbonSeparator::new().upcast()));
    let hidden = item("hidden");
    hidden.set_is_visible(false);
    g.items().add(Some(hidden.upcast()));
    g.items().add(Some(Border::new().upcast()));
    g.slide_out_items().add(Some(item("slide").upcast()));
    let hidden = item("hiddenSlide");
    hidden.set_is_visible(false);
    g.slide_out_items().add(Some(hidden.upcast()));
    tab.groups().add(Some(g.clone()));
    root.tabs().add(Some(tab.clone()));
    let w = crate::ribbon_tests::mount(&root);
    (root, tab, g, w)
}
fn parts(g: &RibbonGroup, flags: u64) {
    let collapsed = super::super::RibbonButton::new();
    collapsed.set_name(Some("collapsed".into()));
    collapsed.set_is_visible(false);
    let expand = Button::new();
    expand.set_name(Some("expand".into()));
    expand.set_is_visible(flags & 1 != 0);
    let launcher = Button::new();
    launcher.set_name(Some("launcher".into()));
    launcher.set_is_visible(flags & 2 != 0);
    let mut p = g.parts.borrow_mut();
    p.collapsed = Some(collapsed);
    p.slide_out_button = Some(expand);
    p.launcher = Some(launcher);
    drop(p);
    if let Some(b) = g.popup_launcher() {
        b.set_tag(Some(Rc::new("popupLauncher".to_owned())));
        b.set_is_visible(flags & 4 != 0);
    }
}
#[test]
fn source_group_target_matrix_real_popup_and_deferred_state() {
    const NAME: &str = "controls::ribbon_group::key_tip_target_tests::source_group_target_matrix_real_popup_and_deferred_state";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/keytip-targets.json")).unwrap();
        for case in fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "group")
        {
            let (_r, _t, g, w) = setup();
            if case["popup"] == true {
                g.open_popup();
                assert!(g.is_popup_open());
            }
            g.set_value(
                RibbonGroup::state_property(),
                RibbonGroupState::from_raw(case["state"].as_i64().unwrap() as i32),
            );
            g.set_value(
                RibbonGroup::is_simplified_property(),
                case["simplified"].as_bool().unwrap(),
            );
            g.items_panel()
                .set_in_line_mask(Some(RibbonFixedItems::new(vec![
                    true, false, true, true, true, true,
                ])
                    as ferroribbon_core::collections::RibbonReadOnlyList<_>));
            parts(&g, case["flags"].as_u64().unwrap());
            assert_eq!(
                serde_json::json!(names(g.get_key_tip_targets())),
                case["targets"],
                "{case}"
            );
            g.close_popup();
            w.close();
            w.set_content(None);
        }
    });
}
#[test]
fn source_live_target_mutations_and_tab_overflow_tail() {
    const NAME: &str = "controls::ribbon_group::key_tip_target_tests::source_live_target_mutations_and_tab_overflow_tail";
    if isolated(NAME) {
        return;
    }
    crate::group_tests::run(|| {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/keytip-targets.json")).unwrap();
        for case in fixture["cases"].as_array().unwrap().iter().filter(|c| {
            matches!(
                c["name"].as_str(),
                Some("deferred" | "changeMode" | "append" | "tabOverflow")
            )
        }) {
            let (_r, t, g, w) = setup();
            g.set_value(RibbonGroup::state_property(), RibbonGroupState::Large);
            g.set_value(RibbonGroup::is_simplified_property(), false);
            g.items_panel()
                .set_in_line_mask(Some(RibbonFixedItems::new(vec![
                    true, false, true, true, true, true,
                ])
                    as ferroribbon_core::collections::RibbonReadOnlyList<_>));
            parts(&g, if case["name"] == "tabOverflow" { 0 } else { 3 });
            let mut e = if case["name"] == "tabOverflow" {
                t.get_key_tip_targets().iter()
            } else {
                g.get_key_tip_targets().iter()
            };
            let mut result = vec![];
            if case["name"] == "deferred" {
                g.set_value(RibbonGroup::state_property(), RibbonGroupState::Collapsed);
            } else {
                result.push(trace_name(&e.next().unwrap()));
            }
            if case["name"] == "changeMode" {
                g.set_value(RibbonGroup::is_simplified_property(), true);
            }
            if case["name"] == "append" {
                g.items_panel().children().add(item("appended"));
            }
            if case["name"] == "tabOverflow" {
                t.groups_panel().set_is_simplified(true);
                t.groups_panel().set_overflow(true);
                t.overflow_button()
                    .set_tag(Some(Rc::new("overflow".to_owned())));
            }
            result.extend(e.map(|c| trace_name(&c)));
            assert_eq!(serde_json::json!(result), case["targets"], "{case}");
            w.close();
            w.set_content(None);
        }
    });
}
