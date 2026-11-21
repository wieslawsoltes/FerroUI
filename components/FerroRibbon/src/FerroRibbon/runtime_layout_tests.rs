//! The twelve pinned shared LayoutTests.cs / SelectionTests scenarios, using the complete
//! SampleRibbon fixture and native compiled themes in Headless. Copyright Wiesław Šoltés, MIT.
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
#[test]
fn narrow_ribbon_reduces_then_collapses_groups() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.resize(620.);
        assert!(
            m.states().iter().any(|s| *s != RibbonGroupState::Large),
            "{:?}",
            m.states()
        );
        m.resize(300.);
        assert!(
            m.states()
                .iter()
                .filter(|s| **s == RibbonGroupState::Collapsed)
                .count()
                >= 3,
            "{:?}",
            m.states()
        );
        m.resize(1400.);
        assert!(m.states().iter().all(|s| *s == RibbonGroupState::Large));
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
