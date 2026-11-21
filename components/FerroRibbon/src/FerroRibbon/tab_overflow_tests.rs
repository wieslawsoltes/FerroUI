//! Selected original display-order/overflow corpus against actual native controls.
use crate::controls::*;
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::collections::RibbonReadOnlyList;
use ferroui_base::Ref;
use ferroui_controls::{Border, Control, Flyout, ScrollViewer, StackPanel, TextBlock};
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../../tests/fixtures/tab-overflow.json")).unwrap()
}
fn run(test: impl FnOnce() + Send + 'static) {
    crate::group_tests::run(test)
}
fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_OVERFLOW_ISOLATED").as_deref() == Ok(name) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_OVERFLOW_ISOLATED", name)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
fn panel(flyout: &Flyout) -> Ref<StackPanel> {
    let scroll = Control::from_boxed(&flyout.content().unwrap())
        .unwrap()
        .cast::<ScrollViewer>()
        .unwrap();
    let padding = Control::from_boxed(&scroll.content().unwrap())
        .unwrap()
        .cast::<Border>()
        .unwrap();
    padding.child().unwrap().cast().unwrap()
}
#[test]
fn original_display_group_order_identity_duplicates_custom_and_null_ids() {
    if isolated(
        "controls::tab_overflow_tests::original_display_group_order_identity_duplicates_custom_and_null_ids",
    ) {
        return;
    }
    run(|| {
        for case in fixture()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "order")
        {
            let tab = RibbonTab::new();
            for (id, header, name, custom) in [
                (Some("a"), Some("A"), None, false),
                (Some("b"), Some("B"), None, false),
                (Some("a"), Some("A2"), None, false),
                (None, None, None, false),
                (Some(""), Some("Named"), Some("fallback"), false),
                (Some("custom"), Some("Custom"), None, true),
            ] {
                let group = RibbonGroup::new();
                group.set_id(id.map(Into::into));
                group.set_header(header.map(Into::into));
                group.set_name(name.map(Into::into));
                group.set_is_custom(custom);
                tab.groups().add(Some(group));
            }
            let order = case["order"].as_array().map(|a| {
                RibbonFixedItems::new(a.iter().map(|v| v.as_str().map(Into::into)).collect())
                    as RibbonReadOnlyList<_>
            });
            tab.set_group_order(order.clone());
            let first = tab.display_groups();
            let again = tab.display_groups();
            assert_eq!(first.ptr_eq(&again), case["sameView"].as_bool().unwrap());
            assert_eq!(
                serde_json::json!(
                    first
                        .iter()
                        .map(|g| g.unwrap().header())
                        .collect::<Vec<_>>()
                ),
                case["labels"]
            );
            if let Some(order) = order {
                let same = RibbonFixedItems::new(order.iter().collect()) as RibbonReadOnlyList<_>;
                tab.set_group_order(Some(same));
                assert!(std::rc::Rc::ptr_eq(&tab.group_order().unwrap(), &order));
            }
        }
    });
}
#[test]
fn original_overflow_snapshot_headings_membership_layout_and_ownership() {
    if isolated(
        "controls::tab_overflow_tests::original_overflow_snapshot_headings_membership_layout_and_ownership",
    ) {
        return;
    }
    run(|| {
        for case in fixture()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "overflow")
        {
            let root = Ribbon::new();
            root.set_display_mode(ferroribbon_core::RibbonDisplayMode::Simplified);
            let tab = RibbonTab::new();
            root.tabs().add(Some(tab.clone()));
            for g in 0..3 {
                let group = RibbonGroup::new();
                group.set_header(Some(format!("Group {g}")));
                if case["hidden"] == true && g == 1 {
                    group.set_is_group_visible(false);
                }
                for i in 0..2 {
                    let b = RibbonButton::new();
                    b.set_label(Some(format!("{g}.{i}")));
                    if i == 1 && g == 2 {
                        b.set_is_visible(false);
                    }
                    if i == 0 && g == 1 {
                        b.set_simplified_visibility(
                            ferroribbon_core::RibbonSimplifiedVisibility::Overflow,
                        );
                    }
                    group.items().add(Some(b.upcast()));
                }
                tab.groups().add(Some(group));
            }
            let w = crate::ribbon_tests::mount(&root);
            let mask = case["mask"].as_u64().unwrap();
            for g in 0..3 {
                let group = tab.groups().get(g).unwrap();
                group
                    .items_panel()
                    .set_in_line_mask(Some(RibbonFixedItems::new(vec![(mask & (1 << g)) != 0; 2])
                        as RibbonReadOnlyList<_>));
            }
            tab.show_overflow();
            let flyout = tab.active_overflow_flyouts().pop().unwrap();
            assert!(flyout.is_open());
            let panel = panel(&flyout);
            let mut rows = vec![];
            for c in panel.children().snapshot().iter() {
                if let Some(t) = c.cast::<TextBlock>() {
                    rows.push(format!("header:{}", t.text().unwrap()));
                } else if let Some(b) = c.cast::<RibbonButton>() {
                    rows.push(format!("item:{}", b.label().unwrap()));
                    assert_eq!(b.current_size(), ferroribbon_core::RibbonItemSize::Medium);
                    assert!(!b.is_simplified());
                    assert!(b.actual_show_label());
                    assert!(RibbonItemHelper::find_owner(Some(&b)).is_some());
                    assert_eq!(
                        b.horizontal_alignment(),
                        ferroui_base::layout::HorizontalAlignment::Stretch
                    );
                } else {
                    assert!(c.is::<ferroui_controls::shapes::Rectangle>());
                    rows.push("separator".into());
                }
            }
            assert_eq!(serde_json::json!(rows), case["rows"], "{case}");
            flyout.hide();
            assert!(tab.active_overflow_flyouts().is_empty());
            w.close();
            w.set_content(None);
        }
    });
}
#[test]
fn overflow_button_defers_opening_and_close_unlinks_copies() {
    if isolated(
        "controls::tab_overflow_tests::overflow_button_defers_opening_and_close_unlinks_copies",
    ) {
        return;
    }
    run(|| {
        let root = Ribbon::new();
        root.set_display_mode(ferroribbon_core::RibbonDisplayMode::Simplified);
        let tab = RibbonTab::new();
        let group = RibbonGroup::new();
        group.set_header(Some("Commands".into()));
        let source = RibbonSplitButton::new();
        source.set_label(Some("Toggle".into()));
        source.set_is_checkable(true);
        source.set_simplified_visibility(ferroribbon_core::RibbonSimplifiedVisibility::Overflow);
        group.items().add(Some(source.clone().upcast()));
        tab.groups().add(Some(group));
        root.tabs().add(Some(tab.clone()));
        let w = crate::ribbon_tests::mount(&root);
        tab.overflow_button().perform_click();
        assert!(tab.active_overflow_flyouts().is_empty());
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        let flyout = tab.active_overflow_flyouts().pop().unwrap();
        let content = panel(&flyout);
        let copy = content
            .children()
            .snapshot()
            .iter()
            .find_map(|c| c.cast::<RibbonSplitButton>())
            .unwrap();
        copy.invoke_primary();
        assert_eq!(source.is_checked(), Some(true));
        flyout.hide();
        source.set_is_checked(Some(false));
        assert_eq!(copy.is_checked(), Some(true));
        assert!(copy.flyout().is_none());
        let weak = copy.downgrade();
        drop(copy);
        drop(content);
        drop(flyout);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
        w.close();
        w.set_content(None);
    });
}
