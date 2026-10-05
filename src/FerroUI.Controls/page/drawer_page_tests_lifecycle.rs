//! The drawer page tests, continued: the lifecycle, display mode mapping,
//! disabled behavior closing, drawer length validation, escape key and
//! system back button groups of the reference tests.

use super::drawer_page_tests::boxed;
use super::navigation_page_tests::{
    flag, log, page, page_h, record, same, slot, store, taken, throws, wait, BackHandlingPage,
};
use super::tabbed_page_tests::key_down;
use super::{
    DrawerBehavior, DrawerLayoutBehavior, DrawerPage, DrawerPlacement, NavigatedFromEventArgs, NavigatedToEventArgs,
    NavigationPage, NavigationType, Page,
};
use crate::test_support::{test_scope, TestRoot};
use crate::{SplitViewDisplayMode, StackPanel};
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Ref, Size};

// --- LifecycleEventTests ---

#[test]
fn is_open_changes_never_fire_page_lifecycle_events() {
    let _scope = test_scope();
    // Toggling IsOpen (open, close, repeated) must never raise page lifecycle events.
    let page = page_h("Content");
    let dp = DrawerPage::new();
    let _root = TestRoot::with_child(dp.clone());
    dp.set_content(boxed(&page)); // fires initial NavigatedTo

    let events = log();
    page.navigated_to(record(&events, "NavigatedTo"));
    page.navigated_from(record(&events, "NavigatedFrom"));

    dp.set_is_open(true);
    dp.set_is_open(false);
    dp.set_is_open(false); // same value

    assert!(events.borrow().is_empty());
}

#[test]
fn content_set_initially_fires_navigated_to() {
    let _scope = test_scope();
    let page = page_h("Home");
    let events = log();
    page.navigated_to(record(&events, "NavigatedTo"));

    let dp = DrawerPage::new();
    let _root = TestRoot::with_child(dp.clone());
    dp.set_content(boxed(&page));

    assert_eq!(vec!["NavigatedTo"], *events.borrow());
}

#[test]
fn content_set_initially_sets_current_page() {
    let _scope = test_scope();
    let page = page_h("Home");
    let dp = DrawerPage::new();
    dp.set_content(boxed(&page));
    assert!(same(&page, &dp.current_page()));
}

#[test]
fn content_changed_fires_lifecycle_events_in_order() {
    let _scope = test_scope();
    let first = page_h("First");
    let second = page_h("Second");
    let dp = DrawerPage::new();
    let _root = TestRoot::with_child(dp.clone());
    dp.set_content(boxed(&first));

    let order = log();
    first.navigated_from(record(&order, "NavigatedFrom"));
    second.navigated_to(record(&order, "NavigatedTo"));

    dp.set_content(boxed(&second));

    assert_eq!(vec!["NavigatedFrom", "NavigatedTo"], *order.borrow());
}

#[test]
fn content_set_initially_navigated_to_navigation_type_is_replace() {
    let _scope = test_scope();
    let page = page_h("Home");
    let args = slot::<NavigatedToEventArgs>();
    page.navigated_to(store(&args));

    let dp = DrawerPage::new();
    let _root = TestRoot::with_child(dp.clone());
    dp.set_content(boxed(&page));

    assert!(args.borrow().is_some());
    assert_eq!(NavigationType::Replace, taken(&args).navigation_type());
}

#[test]
fn content_changed_navigated_from_and_to_navigation_type_is_replace() {
    let _scope = test_scope();
    let first = page_h("First");
    let second = page_h("Second");
    let dp = DrawerPage::new();
    let _root = TestRoot::with_child(dp.clone());
    dp.set_content(boxed(&first));

    let from_args = slot::<NavigatedFromEventArgs>();
    let to_args = slot::<NavigatedToEventArgs>();
    first.navigated_from(store(&from_args));
    second.navigated_to(store(&to_args));

    dp.set_content(boxed(&second));

    assert!(from_args.borrow().is_some());
    assert_eq!(NavigationType::Replace, taken(&from_args).navigation_type());
    assert!(to_args.borrow().is_some());
    assert_eq!(NavigationType::Replace, taken(&to_args).navigation_type());
}

#[test]
fn content_changed_while_overlay_drawer_open_non_page_drawer_fires_lifecycle_events() {
    let _scope = test_scope();
    let home = page_h("Home");
    let profile = page_h("Profile");
    let dp = DrawerPage::new();
    dp.set_display_mode(SplitViewDisplayMode::Overlay);
    let _root = TestRoot::with_child(dp.clone());
    dp.set_drawer(boxed(&StackPanel::new()));
    dp.set_content(boxed(&home));

    let events = log();
    home.navigated_from(record(&events, "Home: NavigatedFrom"));
    profile.navigated_to(record(&events, "Profile: NavigatedTo"));

    dp.set_is_open(true);
    assert!(events.borrow().is_empty());

    dp.set_content(boxed(&profile));
    assert_eq!(vec!["Home: NavigatedFrom", "Profile: NavigatedTo"], *events.borrow());
}

// --- Initial-attach lifecycle (the first page / loaded fix) ---

#[test]
fn content_set_before_attach_suppressed_until_load() {
    let _scope = test_scope();
    // Events must NOT fire during markup parsing (before the visual root is set).
    let page = page_h("Home");
    let events = log();
    page.navigated_to(record(&events, "NavigatedTo"));

    let dp = DrawerPage::new();
    dp.set_content(boxed(&page));

    assert!(events.borrow().is_empty());
}

#[test]
fn content_set_before_attach_fires_lifecycle_events_on_load() {
    let _scope = test_scope();
    // Content set before the control enters the visual tree (simulating markup parsing).
    // Events must fire exactly once when the control is attached and Loaded fires.
    let page = page_h("Home");
    let events = log();
    page.navigated_to(record(&events, "NavigatedTo"));

    let dp = DrawerPage::new();
    dp.set_content(boxed(&page));
    assert!(events.borrow().is_empty()); // suppressed before visual tree

    let _root = TestRoot::with_child(dp.clone());
    Dispatcher::ui_thread().run_jobs(None); // pump the posted Loaded dispatch

    assert_eq!(vec!["NavigatedTo"], *events.borrow());
}

#[test]
fn content_set_before_attach_then_changed_after_attach_no_double_fire() {
    let _scope = test_scope();
    let first = page_h("First");
    let second = page_h("Second");

    let dp = DrawerPage::new();
    dp.set_content(boxed(&first));
    let _root = TestRoot::with_child(dp.clone());
    Dispatcher::ui_thread().run_jobs(None); // fire the deferred NavigatedTo on first

    let events = log();
    first.navigated_from(record(&events, "First: NavigatedFrom"));
    second.navigated_to(record(&events, "Second: NavigatedTo"));

    dp.set_content(boxed(&second));

    assert_eq!(vec!["First: NavigatedFrom", "Second: NavigatedTo"], *events.borrow());
}

#[test]
fn content_set_before_attach_navigated_to_navigation_type_is_push() {
    let _scope = test_scope();
    let page = page_h("Home");
    let args = slot::<NavigatedToEventArgs>();
    page.navigated_to(store(&args));

    let dp = DrawerPage::new();
    dp.set_content(boxed(&page));
    let _root = TestRoot::with_child(dp.clone());
    Dispatcher::ui_thread().run_jobs(None);

    assert!(args.borrow().is_some());
    assert_eq!(NavigationType::Push, taken(&args).navigation_type());
}

#[test]
fn content_set_to_same_instance_no_lifecycle_events() {
    let _scope = test_scope();
    // Re-assigning the same Content instance must not re-fire lifecycle events.
    let page = page_h("Home");
    let dp = DrawerPage::new();
    let _root = TestRoot::with_child(dp.clone());
    dp.set_content(boxed(&page)); // initial assignment fires NavigatedTo

    let events = log();
    page.navigated_to(record(&events, "NavigatedTo"));
    page.navigated_from(record(&events, "NavigatedFrom"));

    dp.set_content(boxed(&page)); // same instance: must not fire anything

    assert!(events.borrow().is_empty());
}

// --- DisplayModeMappingTests ---

fn with_layout_behavior(behavior: DrawerLayoutBehavior) -> Ref<DrawerPage> {
    let dp = DrawerPage::new();
    dp.set_drawer_layout_behavior(behavior);
    dp
}

#[test]
fn drawer_layout_behavior_overlay_maps_to_overlay() {
    let _scope = test_scope();
    let dp = with_layout_behavior(DrawerLayoutBehavior::Overlay);
    assert_eq!(SplitViewDisplayMode::Overlay, dp.display_mode());
}

#[test]
fn drawer_layout_behavior_split_maps_to_inline() {
    let _scope = test_scope();
    let dp = with_layout_behavior(DrawerLayoutBehavior::Split);
    assert_eq!(SplitViewDisplayMode::Inline, dp.display_mode());
}

#[test]
fn drawer_layout_behavior_compact_overlay_maps_to_compact_overlay() {
    let _scope = test_scope();
    let dp = with_layout_behavior(DrawerLayoutBehavior::CompactOverlay);
    assert_eq!(SplitViewDisplayMode::CompactOverlay, dp.display_mode());
}

#[test]
fn drawer_layout_behavior_compact_inline_maps_to_compact_inline() {
    let _scope = test_scope();
    let dp = with_layout_behavior(DrawerLayoutBehavior::CompactInline);
    assert_eq!(SplitViewDisplayMode::CompactInline, dp.display_mode());
}

#[test]
fn drawer_behavior_locked_overrides_compact_overlay_to_inline() {
    let _scope = test_scope();
    let dp = with_layout_behavior(DrawerLayoutBehavior::CompactOverlay);
    dp.set_drawer_behavior(DrawerBehavior::Locked);
    assert_eq!(SplitViewDisplayMode::Inline, dp.display_mode());
}

#[test]
fn drawer_behavior_flyout_overrides_compact_inline_to_overlay() {
    let _scope = test_scope();
    let dp = with_layout_behavior(DrawerLayoutBehavior::CompactInline);
    dp.set_drawer_behavior(DrawerBehavior::Flyout);
    assert_eq!(SplitViewDisplayMode::Overlay, dp.display_mode());
}

#[test]
fn drawer_breakpoint_length_before_layout_does_not_override_layout_behavior() {
    let _scope = test_scope();
    let dp = with_layout_behavior(DrawerLayoutBehavior::Split);
    dp.set_drawer_breakpoint_length(1200.0);
    assert_eq!(SplitViewDisplayMode::Inline, dp.display_mode());
}

#[test]
fn drawer_breakpoint_length_zero_does_not_override_layout_behavior() {
    let _scope = test_scope();
    // Breakpoint == 0 means the feature is disabled; the layout behavior drives the display mode.
    let dp = with_layout_behavior(DrawerLayoutBehavior::Split);
    dp.set_drawer_breakpoint_length(0.0);
    assert_eq!(SplitViewDisplayMode::Inline, dp.display_mode());
}

/// A split drawer page with the breakpoint and placement, laid out at
/// 800 x 600.
fn laid_out_with_breakpoint(breakpoint: f64, placement: DrawerPlacement) -> Ref<DrawerPage> {
    let dp = with_layout_behavior(DrawerLayoutBehavior::Split);
    dp.set_drawer_breakpoint_length(breakpoint);
    dp.set_drawer_placement(placement);
    dp.measure(Size::new(800.0, 600.0));
    dp.arrange(Rect::new(0.0, 0.0, 800.0, 600.0));
    dp
}

#[test]
fn drawer_breakpoint_length_horizontal_below_breakpoint_forces_overlay() {
    let _scope = test_scope();
    for placement in [DrawerPlacement::Left, DrawerPlacement::Right] {
        let dp = laid_out_with_breakpoint(1200.0, placement);

        assert_eq!(SplitViewDisplayMode::Overlay, dp.display_mode());
    }
}

#[test]
fn drawer_breakpoint_length_horizontal_above_breakpoint_uses_configured_layout() {
    let _scope = test_scope();
    for placement in [DrawerPlacement::Left, DrawerPlacement::Right] {
        let dp = laid_out_with_breakpoint(600.0, placement);

        assert_eq!(SplitViewDisplayMode::Inline, dp.display_mode());
    }
}

#[test]
fn drawer_breakpoint_length_vertical_below_breakpoint_forces_overlay() {
    let _scope = test_scope();
    for placement in [DrawerPlacement::Top, DrawerPlacement::Bottom] {
        let dp = laid_out_with_breakpoint(800.0, placement);

        // Vertical: the breakpoint compares against the height of the bounds (600 < 800: overlay).
        assert_eq!(SplitViewDisplayMode::Overlay, dp.display_mode());
    }
}

#[test]
fn drawer_breakpoint_length_vertical_above_breakpoint_uses_configured_layout() {
    let _scope = test_scope();
    for placement in [DrawerPlacement::Top, DrawerPlacement::Bottom] {
        let dp = laid_out_with_breakpoint(400.0, placement);

        // Vertical: the breakpoint compares against the height of the bounds (600 > 400: inline).
        assert_eq!(SplitViewDisplayMode::Inline, dp.display_mode());
    }
}

// --- DisabledBehaviorClosingTests ---

#[test]
fn closing_cancel_cannot_prevent_disabled_close() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_is_open(true);
    dp.closing(|_, e| e.set_cancel(true));

    dp.set_drawer_behavior(DrawerBehavior::Disabled);

    assert!(!dp.is_open());
}

#[test]
fn closing_not_fired_when_disabled_forces_close() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_is_open(true);
    let closing_fired = flag();
    let f = closing_fired.clone();
    dp.closing(move |_, _| f.set(true));

    dp.set_drawer_behavior(DrawerBehavior::Disabled);

    assert!(!closing_fired.get());
}

// --- DrawerLengthValidationTests ---

const INVALID_LENGTHS: [f64; 5] = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, -100.0];

#[test]
fn drawer_length_rejects_invalid_values() {
    let _scope = test_scope();
    for invalid in INVALID_LENGTHS {
        let dp = DrawerPage::new();
        dp.set_drawer_length(200.0);
        assert!(throws(|| dp.set_drawer_length(invalid)));
        assert_eq!(200.0, dp.drawer_length());
    }
}

#[test]
fn compact_drawer_length_rejects_invalid_values() {
    let _scope = test_scope();
    for invalid in INVALID_LENGTHS {
        let dp = DrawerPage::new();
        assert!(throws(|| dp.set_compact_drawer_length(invalid)));
        assert_eq!(48.0, dp.compact_drawer_length());
    }
}

#[test]
fn drawer_length_accepts_zero() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_drawer_length(0.0);
    assert_eq!(0.0, dp.drawer_length());
}

#[test]
fn compact_drawer_length_accepts_zero() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_compact_drawer_length(0.0);
    assert_eq!(0.0, dp.compact_drawer_length());
}

// --- EscapeKeyTests ---

/// An open drawer page with the display mode, in a root, after the escape
/// key was pressed on it.
fn open_after_escape(display_mode: SplitViewDisplayMode) -> bool {
    let dp = DrawerPage::new();
    dp.set_display_mode(display_mode);
    dp.set_is_open(true);
    let _root = TestRoot::with_child(dp.clone());

    dp.raise_event(&key_down(Key::Escape, KeyModifiers::NONE));

    dp.is_open()
}

#[test]
fn escape_key_closes_overlay_drawer() {
    let _scope = test_scope();
    assert!(!open_after_escape(SplitViewDisplayMode::Overlay));
}

#[test]
fn escape_key_closes_compact_overlay_drawer() {
    let _scope = test_scope();
    assert!(!open_after_escape(SplitViewDisplayMode::CompactOverlay));
}

#[test]
fn escape_key_does_not_close_inline_drawer() {
    let _scope = test_scope();
    assert!(open_after_escape(SplitViewDisplayMode::Inline));
}

// --- SystemBackButtonTests ---

fn raise_back_button(dp: &DrawerPage) -> RoutedEventArgs {
    let args = RoutedEventArgs::with_event(Page::page_navigation_system_back_button_pressed_event());
    dp.raise_event(&args);
    args
}

#[test]
fn back_event_is_forwarded_to_content() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let page = page();
    let is_raised = flag();
    let r = is_raised.clone();
    page.page_navigation_system_back_button_pressed(move |_, _| r.set(true));
    let _root = TestRoot::with_child(dp.clone());
    dp.set_current_page(page.clone().upcast::<Page>());

    let args = raise_back_button(&dp);

    assert!(is_raised.get());
    assert!(!args.handled());
}

#[test]
fn back_button_closes_open_drawer() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_is_open(true);
    let _root = TestRoot::with_child(dp.clone());

    let args = raise_back_button(&dp);

    assert!(!dp.is_open());
    assert!(args.handled());
}

#[test]
fn back_button_does_not_close_locked_drawer() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_drawer_behavior(DrawerBehavior::Locked);
    dp.set_is_open(true);
    let _root = TestRoot::with_child(dp.clone());

    let args = raise_back_button(&dp);

    assert!(dp.is_open());
    assert!(!args.handled());
}

#[test]
fn back_button_does_not_act_on_disabled_drawer() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_drawer_behavior(DrawerBehavior::Disabled);
    let _root = TestRoot::with_child(dp.clone());

    let args = raise_back_button(&dp);

    assert!(!dp.is_open());
    assert!(!args.handled());
}

#[test]
fn back_button_does_not_act_when_already_closed() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let _root = TestRoot::with_child(dp.clone());

    let args = raise_back_button(&dp);

    assert!(!dp.is_open());
    assert!(!args.handled());
}

#[test]
fn back_button_forwards_through_navigation_page_to_modal_before_covered_page() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    let nav = NavigationPage::new();
    let covered_page = BackHandlingPage::new(true);
    let modal = BackHandlingPage::new(false);
    wait(nav.push_async(&covered_page));
    wait(nav.push_modal_async(&modal));
    dp.set_content(boxed(&nav));
    let _root = TestRoot::with_child(dp.clone());

    let args = raise_back_button(&dp);

    assert!(args.handled());
    assert_eq!(0, covered_page.back_button_press_count());
    assert_eq!(1, modal.back_button_press_count());
    assert!(nav.modal_stack().is_empty());
    assert!(same(&covered_page, &nav.current_page()));
}

#[test]
fn back_button_closes_open_drawer_before_forwarding_to_nested_navigation_page() {
    let _scope = test_scope();
    let dp = DrawerPage::new();
    dp.set_is_open(true);
    let nav = NavigationPage::new();
    let covered_page = BackHandlingPage::new(true);
    let modal = BackHandlingPage::new(false);
    wait(nav.push_async(&covered_page));
    wait(nav.push_modal_async(&modal));
    dp.set_content(boxed(&nav));
    let _root = TestRoot::with_child(dp.clone());

    let args = raise_back_button(&dp);

    assert!(args.handled());
    assert!(!dp.is_open());
    assert_eq!(0, covered_page.back_button_press_count());
    assert_eq!(0, modal.back_button_press_count());
    assert_eq!(1, nav.modal_stack().len());
    assert!(same(&modal, &nav.modal_stack()[0]));
}
