//! The twelve pinned shared LayoutTests.cs / SelectionTests scenarios, using the complete
//! SampleRibbon fixture and native compiled themes in Headless.
use crate::{
    controls::*,
    runtime_sample::{self as sample, Mounted},
};
use ferroribbon_core::{RibbonDensity, RibbonDisplayMode, RibbonGroupState, RibbonVisibilityMode};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupDelegate, from_markup_value},
};
use ferroui_controls::Control;
use std::{cell::RefCell, rc::Rc};

#[test]
fn wide_ribbon_shows_all_groups_large() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        assert_eq!(
            m.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("home")
        );
        assert!(
            m.states().iter().all(|s| *s == RibbonGroupState::Large),
            "{:?}",
            m.states()
        );
    });
}
/// What the reduction decided from: the width of the panel and, for every group, the widths by
/// state that the panel cached next to the widths measured again now. Measuring again applies
/// the states, so this is only called for the message of a failed assertion.
fn reduction_report(m: &Mounted) -> String {
    let panel = m.ribbon.selected_tab().unwrap().groups_panel();
    let mut report = format!(
        "panel width {} extent {}",
        panel.bounds().width,
        panel.extent_width()
    );
    let top_level = ferroui_controls::TopLevel::get_top_level(Some(&m.ribbon));
    report.push_str(&format!(
        " render scaling {:?} winui rounding {} ribbon font {}",
        top_level.as_ref().map(|t| t.render_scaling()),
        m.ribbon.use_winui_measure_rounding(),
        m.ribbon.font_size()
    ));
    let metrics = panel.metrics().unwrap();
    for group in m.groups() {
        let cached = panel.cached_classic_widths(&group);
        let state = group.state();
        let height = group.desired_size().height;
        let measured = (0..4)
            .map(|raw| {
                group.measure_width(RibbonGroupState::from_raw(raw), metrics.clone(), height)
            })
            .collect::<Vec<_>>();
        report.push_str(&format!(
            "\n{:?} {state:?} order {} collapse {} cached {cached:?} measured {measured:?}",
            group.id(),
            group.reduction_order(),
            group.can_collapse(),
        ));
    }
    for group in m.groups() {
        let Some(button) = group.collapsed_button() else {
            continue;
        };
        report.push_str(&format!(
            "\n{:?} collapsed button font {} theme {}",
            group.id(),
            button.font_size(),
            button.theme().is_some()
        ));
        for text in button
            .get_visual_descendants()
            .filter_map(|v| v.cast::<ferroui_controls::TextBlock>())
        {
            report.push_str(&format!(
                " | {:?} size {} family {} visible {} width {}",
                text.text(),
                text.font_size(),
                text.font_family(),
                text.is_visible(),
                text.desired_size().width
            ));
        }
    }
    report
}
/// Writes the inputs of the reduction that the panel cached, without measuring again, to the
/// standard error of the process: the case runs on the session thread, whose captured output is
/// not the output of this test. Temporary, to compare a passing run with a failing one.
fn trace_reduction(m: &Mounted, step: &str) {
    use std::io::Write;
    let panel = m.ribbon.selected_tab().unwrap().groups_panel();
    let mut line = format!(
        "ribbon-trace {step}: container {} panel {} extent {}",
        m.ribbon.bounds().width,
        panel.bounds().width,
        panel.extent_width()
    );
    for group in m.groups() {
        line.push_str(&format!(
            " | {:?} {:?} {:?}",
            group.id(),
            group.state(),
            panel.cached_classic_widths(&group)
        ));
    }
    line.push('\n');
    let _ = std::io::stderr().write_all(line.as_bytes());
}
#[test]
fn narrow_ribbon_reduces_then_collapses_groups() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        trace_reduction(&m, "1400");
        m.resize(620.);
        trace_reduction(&m, "620");
        assert!(
            m.states().iter().any(|s| *s != RibbonGroupState::Large),
            "{:?}\n{}",
            m.states(),
            reduction_report(&m)
        );
        m.resize(300.);
        trace_reduction(&m, "300");
        assert!(
            m.states()
                .iter()
                .filter(|s| **s == RibbonGroupState::Collapsed)
                .count()
                >= 3,
            "{:?}\n{}",
            m.states(),
            reduction_report(&m)
        );
        m.resize(1400.);
        assert!(
            m.states().iter().all(|s| *s == RibbonGroupState::Large),
            "{:?}\n{}",
            m.states(),
            reduction_report(&m)
        );
    });
}
#[test]
fn reduction_order_shrinks_high_priority_group_first() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let width = m
            .ribbon
            .selected_tab()
            .unwrap()
            .groups_panel()
            .extent_width();
        m.resize(width - 20.);
        assert_ne!(
            sample::find_group(&m.ribbon, "styles").state(),
            RibbonGroupState::Large
        );
        assert_eq!(
            sample::find_group(&m.ribbon, "clipboard").state(),
            RibbonGroupState::Large
        );
    });
}
#[test]
fn collapsed_group_opens_popup_with_items() {
    crate::group_tests::run(|| {
        let m = Mounted::new(300.);
        let group = m
            .groups()
            .into_iter()
            .find(|g| g.state() == RibbonGroupState::Collapsed)
            .unwrap();
        group.open_popup();
        m.settle();
        assert!(group.is_popup_open());
        group.close_popup();
        m.settle();
        assert!(!group.is_popup_open());
        assert_eq!(group.state(), RibbonGroupState::Collapsed);
    });
}
#[test]
fn simplified_mode_moves_items_to_overflow() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.set_display_mode(RibbonDisplayMode::Simplified);
        m.settle();
        assert!(m.groups().iter().all(|g| g.is_simplified()));
        m.resize(360.);
        assert!(
            m.ribbon
                .selected_tab()
                .unwrap()
                .groups_panel()
                .has_overflow()
        );
        assert!(
            m.groups()
                .iter()
                .any(|g| g.get_overflow_items().iter().next().is_some())
        );
        m.ribbon.set_display_mode(RibbonDisplayMode::Classic);
        m.settle();
        assert!(!m.groups().iter().any(|g| g.is_simplified()));
    });
}
#[test]
fn density_changes_metrics() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let comfortable = m.ribbon.bounds().height;
        m.ribbon.set_density(RibbonDensity::Touch);
        m.settle();
        assert!(
            m.ribbon.bounds().height > comfortable,
            "Touch {} vs comfortable {comfortable}",
            m.ribbon.bounds().height
        );
        m.ribbon.set_density(RibbonDensity::Compact);
        m.settle();
        assert!(m.ribbon.bounds().height < comfortable);
    });
}
#[test]
fn minimized_ribbon_hides_commands_and_shows_popup() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let full = m.ribbon.bounds().height;
        m.ribbon.toggle_minimized();
        m.settle();
        assert!(m.ribbon.is_minimized());
        assert!(m.ribbon.bounds().height < full - 40.);
        m.ribbon.open_minimized_popup();
        m.settle();
        assert!(m.ribbon.is_minimized_popup_open());
        m.ribbon.close_minimized_popup();
        m.settle();
        m.ribbon
            .set_visibility_mode(RibbonVisibilityMode::AlwaysShow);
        m.settle();
        assert!(!m.ribbon.is_minimized());
        assert!((m.ribbon.bounds().height - full).abs() < 2.);
    });
}
#[test]
fn full_screen_mode_hides_ribbon_until_revealed() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon
            .set_visibility_mode(RibbonVisibilityMode::FullScreen);
        m.settle();
        assert!(m.ribbon.bounds().height < 40.);
        m.ribbon.set_is_full_screen_revealed(true);
        m.settle();
        assert!(m.ribbon.bounds().height > 100.);
    });
}
#[test]
fn selecting_tabs_by_id_updates_selection() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let changed = Rc::new(RefCell::new(None));
        let log = changed.clone();
        let handler = MarkupDelegate::new(move |args| {
            let e = from_markup_value::<Rc<RibbonTabChangedEventArgs>>(&args[1]).unwrap();
            log.replace(e.new_tab.clone());
            None
        });
        m.ribbon.add_selected_tab_changed(Some(handler.clone()));
        assert!(m.ribbon.select_tab_id("insert"));
        assert_eq!(m.ribbon.selected_tab_id().as_deref(), Some("insert"));
        assert_eq!(
            changed.borrow().as_ref().unwrap().id().as_deref(),
            Some("insert")
        );
        m.ribbon.set_selected_tab_id(Some("home".into()));
        assert_eq!(
            m.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("home")
        );
        assert!(!m.ribbon.select_tab_id("tableDesign"));
        m.ribbon.remove_selected_tab_changed(Some(handler));
    });
}
#[test]
fn contextual_group_shows_selects_and_restores_previous_tab() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.select_tab_id("insert");
        assert_eq!(m.ribbon.visible_tabs().len(), 2);
        m.ribbon.set_active_contextual_groups(&[Some("tableTools")]);
        m.settle();
        assert_eq!(m.ribbon.visible_tabs().len(), 3);
        assert_eq!(
            m.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("tableDesign")
        );
        assert!(m.ribbon.tab_headers().last().unwrap().is_contextual());
        m.ribbon.set_active_contextual_groups(&[]);
        m.settle();
        assert_eq!(
            m.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("insert")
        );
    });
}
#[test]
fn radio_toggle_groups_are_exclusive() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let left = sample::item::<RibbonToggleButton>(&m.ribbon, "left");
        let center = sample::item::<RibbonToggleButton>(&m.ribbon, "center");
        let boxed = Control::boxed(center.clone());
        let value = ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn IRibbonItem>>()).unwrap();
        let item = value.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap();
        item.invoke();
        m.settle();
        assert_eq!(center.is_checked(), Some(true));
        assert_eq!(left.is_checked(), Some(false));
        item.invoke();
        assert_eq!(center.is_checked(), Some(true));
    });
}
#[test]
fn hidden_tab_and_group_disappear() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon
            .find_tab("insert")
            .unwrap()
            .set_is_tab_visible(false);
        m.settle();
        assert!(
            !m.ribbon
                .visible_tabs()
                .iter()
                .any(|t| t.id().as_deref() == Some("insert"))
        );
        let editing = sample::find_group(&m.ribbon, "editing");
        editing.set_is_group_visible(false);
        m.settle();
        assert!(!editing.is_visible());
    });
}
