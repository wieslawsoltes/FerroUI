//! Pinned shared CommandTests.cs / QuickAccessTests using complete SampleRibbon.
//! Copyright Wiesław Šoltés, MIT. Headless integration is not native parity.
use crate::{
    controls::*,
    runtime_sample::{self as sample, Mounted},
};
use ferroribbon_core::{RibbonQuickAccessPosition, commands::RibbonCommandCatalog};
use ferroui_base::{
    Ref,
    data::core::{ValueType, ValueTypes},
    input::{InputElement, Key, KeyEventArgs, KeyModifiers},
};
use ferroui_controls::{Control, TopLevel};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn item(control: Ref<Control>) -> Rc<dyn IRibbonItem> {
    ValueTypes::try_cast(
        &Control::boxed(control),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .unwrap()
    .downcast_ref::<Rc<dyn IRibbonItem>>()
    .unwrap()
    .clone()
}
#[test]
fn command_id_resolves_through_catalog_and_state_is_pushed() {
    crate::group_tests::run(|| {
        let ribbon = sample::create();
        let catalog = RibbonCommandCatalog::new();
        let executed = Rc::new(Cell::new(0));
        let count = executed.clone();
        catalog.register_action(
            Some("copy".into()),
            Some("Copy".into()),
            Rc::new(move |_| count.set(count.get() + 1)),
            None,
            Default::default(),
        );
        ribbon.set_command_catalog(Some(catalog.clone()));
        let m = Mounted::with_ribbon(ribbon, 1400.);
        let copy = sample::item::<RibbonButton>(&m.ribbon, "copy");
        assert!(copy.command().is_some());
        copy.perform_click();
        m.settle();
        assert_eq!(executed.get(), 1);
        catalog.set_enabled(Some(&"copy".into()), false);
        m.settle();
        assert!(!copy.is_enabled());
    });
}
#[test]
fn item_invoked_reports_ids_and_parameters() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let invoked = Rc::new(RefCell::new(Vec::new()));
        let log = invoked.clone();
        let handler = crate::root_command_tests::invocation(move |e| log.borrow_mut().push(e));
        m.ribbon.add_item_invoked(Some(handler.clone()));
        sample::item::<RibbonButton>(&m.ribbon, "cut").perform_click();
        item(sample::item::<RibbonToggleButton>(&m.ribbon, "bold").upcast()).invoke();
        m.settle();
        assert!(
            invoked
                .borrow()
                .iter()
                .any(|e| e.item_id == Some("cut".into()))
        );
        assert!(
            invoked
                .borrow()
                .iter()
                .any(|e| e.item_id == Some("bold".into())
                    && e.parameter.as_ref().and_then(|p| p.downcast_ref::<bool>()) == Some(&true))
        );
        m.ribbon.remove_item_invoked(Some(handler));
    });
}
#[test]
fn set_command_enabled_by_id_applies_to_items() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let copy = sample::item::<RibbonButton>(&m.ribbon, "copy");
        m.ribbon.set_command_enabled(Some("copy".into()), false);
        assert!(!copy.is_enabled());
        m.ribbon.set_command_enabled(Some("copy".into()), true);
        assert!(copy.is_enabled());
    });
}
#[test]
fn shortcut_routing_invokes_items() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.set_is_shortcut_routing_enabled(true);
        let bold = sample::item::<RibbonToggleButton>(&m.ribbon, "bold");
        // The source reflects into TryRouteShortcut; exercise the actual native
        // routed-key entry instead, without exposing a new public control API.
        let mut event = KeyEventArgs::new();
        event.key = Key::B;
        event.key_modifiers = KeyModifiers::CONTROL;
        event.set_routed_event(Some(InputElement::key_down_event()));
        bold.raise_event(&event);
        assert!(event.handled());
        assert_eq!(bold.is_checked(), Some(true));
    });
}
#[test]
fn add_and_remove_linked_copies() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let bold = sample::item::<RibbonToggleButton>(&m.ribbon, "bold");
        assert!(m.ribbon.add_to_quick_access(&bold));
        assert!(!m.ribbon.add_to_quick_access(&bold));
        m.settle();
        let copy = m
            .ribbon
            .find_quick_access_copy(Some(bold.clone().upcast()))
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        copy.set_is_checked(Some(true));
        assert_eq!(bold.is_checked(), Some(true));
        bold.set_is_checked(Some(false));
        assert_eq!(copy.is_checked(), Some(false));
        let ids = || {
            m.ribbon
                .get_quick_access_item_ids()
                .iter()
                .map(|id| id.unwrap().to_utf8().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(), ["save", "bold"]);
        assert!(m.ribbon.remove_from_quick_access(&bold));
        assert_eq!(ids(), ["save"]);
        bold.set_is_checked(Some(true));
        bold.set_label(Some("Strong".into()));
        assert_eq!(copy.is_checked(), Some(false));
        assert_eq!(copy.label().as_deref(), Some("Bold"));
    });
}
#[test]
fn clicking_copy_runs_original_click_once() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let cut = sample::item::<RibbonButton>(&m.ribbon, "cut");
        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        let click = cut.click(move |_, _| count.set(count.get() + 1));
        let invoked = Rc::new(Cell::new(0));
        let count = invoked.clone();
        let handler = crate::root_command_tests::invocation(move |e| {
            if e.item_id == Some("cut".into()) {
                count.set(count.get() + 1)
            }
        });
        m.ribbon.add_item_invoked(Some(handler.clone()));
        assert!(m.ribbon.add_to_quick_access(&cut));
        m.settle();
        m.ribbon
            .find_quick_access_copy(Some(cut.clone().upcast()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap()
            .perform_click();
        m.settle();
        assert_eq!(clicks.get(), 1);
        assert_eq!(invoked.get(), 1);
        m.ribbon.remove_item_invoked(Some(handler));
        cut.remove_handler(ferroui_controls::Button::click_event(), click);
    });
}
#[test]
fn position_moves_between_hosts() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let above = m.ribbon.bounds().height;
        m.ribbon
            .set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
        m.settle();
        assert!(
            TopLevel::get_top_level(Some(&m.ribbon.quick_access_tool_bar().unwrap())).is_some()
        );
        m.ribbon.set_is_quick_access_visible(false);
        m.settle();
        assert!(m.ribbon.bounds().height < above);
    });
}
