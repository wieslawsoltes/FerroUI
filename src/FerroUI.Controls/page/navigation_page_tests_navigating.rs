//! The navigation page tests, continued: the system back button, property,
//! attached property, initial content, transition cancellation, navigating
//! event and logical children groups of the reference tests.

use super::navigation_page_tests::*;
use super::{BarLayoutBehavior, NavigationPage, Page};
use crate::test_support::test_scope;
use crate::Control;
use ferroui_base::animation::{CrossFade, IPageTransition, TimeSpan};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::Thickness;
use std::cell::Cell;
use std::rc::Rc;

// --- SystemBackButtonTests ---

fn raise_back_button(nav: &NavigationPage) -> RoutedEventArgs {
    let args = RoutedEventArgs::with_event(Page::page_navigation_system_back_button_pressed_event());
    nav.raise_event(&args);
    args
}

#[test]
fn back_button_with_modal_on_root_pops_modal() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let modal = page_h("Modal");
    wait(nav.push_async(&root));
    wait(nav.push_modal_async(&modal));

    let args = raise_back_button(&nav);

    assert!(args.handled());
    assert!(nav.modal_stack().is_empty());
    assert_eq!(1, nav.stack_depth());
    assert!(same(&root, &nav.current_page()));
}

#[test]
fn back_button_with_modal_and_deep_stack_pops_modal_before_stack() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let detail = page_h("Detail");
    let modal = page_h("Modal");
    wait(nav.push_async(&root));
    wait(nav.push_async(&detail));
    wait(nav.push_modal_async(&modal));

    let args = raise_back_button(&nav);

    assert!(args.handled());
    assert!(nav.modal_stack().is_empty());
    assert_eq!(2, nav.stack_depth());
    assert!(same(&detail, &nav.current_page()));
}

#[test]
fn back_button_forwards_to_top_modal_before_auto_pop() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page_h("Root")));
    let modal = BackHandlingPage::new(true);
    wait(nav.push_modal_async(&modal));

    let args = raise_back_button(&nav);

    assert!(args.handled());
    assert_eq!(1, modal.back_button_press_count());
    let modal_stack = nav.modal_stack();
    assert_eq!(1, modal_stack.len());
    assert!(same(&modal, &modal_stack[0]));
}

#[test]
fn back_button_with_modal_does_not_forward_to_covered_current_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = BackHandlingPage::new(true);
    let modal = BackHandlingPage::new(false);
    wait(nav.push_async(&root));
    wait(nav.push_modal_async(&modal));

    let args = raise_back_button(&nav);

    assert!(args.handled());
    assert_eq!(0, root.back_button_press_count());
    assert_eq!(1, modal.back_button_press_count());
    assert!(nav.modal_stack().is_empty());
    assert_eq!(1, nav.stack_depth());
    assert!(same(&root, &nav.current_page()));
}

#[test]
fn back_button_with_handled_modal_does_not_forward_to_covered_current_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = BackHandlingPage::new(true);
    let modal = BackHandlingPage::new(true);
    wait(nav.push_async(&root));
    wait(nav.push_modal_async(&modal));

    let args = raise_back_button(&nav);

    assert!(args.handled());
    assert_eq!(0, root.back_button_press_count());
    assert_eq!(1, modal.back_button_press_count());
    let modal_stack = nav.modal_stack();
    assert_eq!(1, modal_stack.len());
    assert!(same(&modal, &modal_stack[0]));
}

// --- PropertyTests ---

#[test]
fn is_gesture_enabled_default_is_true() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    assert!(nav.is_gesture_enabled());
}

#[test]
fn is_gesture_enabled_round_trips() {
    let _scope = test_scope();
    for value in [true, false] {
        let nav = NavigationPage::new();
        nav.set_is_gesture_enabled(value);
        assert_eq!(value, nav.is_gesture_enabled());
    }
}

#[test]
fn safe_area_padding_affeccts_nav_bar_height() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    nav.set_safe_area_padding(Thickness::uniform(10.0));
    let page = page();

    NavigationPage::set_bar_height_override(&page, Some(60.0));
    wait(nav.push_async(&page));
    assert_eq!(70.0, nav.effective_bar_height());
}

// --- AttachedPropertyTests ---

#[test]
fn effective_bar_height_uses_page_override() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let page = page();
    NavigationPage::set_bar_height_override(&page, Some(60.0));
    wait(nav.push_async(&page));
    assert_eq!(60.0, nav.effective_bar_height());
}

#[test]
fn effective_bar_height_falls_back_to_global_bar_height() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    nav.set_bar_height(56.0);
    wait(nav.push_async(page()));
    assert_eq!(56.0, nav.effective_bar_height());
}

#[test]
fn bar_layout_behavior_default_applies_nav_bar_inset_pseudo_class() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    assert!(nav.classes().contains(":nav-bar-inset"));
}

#[test]
fn bar_layout_behavior_overlay_removes_nav_bar_inset_pseudo_class() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let page = page();
    NavigationPage::set_bar_layout_behavior(&page, Some(BarLayoutBehavior::Overlay));
    wait(nav.push_async(&page));
    assert!(!nav.classes().contains(":nav-bar-inset"));
}

// --- InitialContentTests ---

#[test]
fn content_set_before_push_is_used_as_initial_page() {
    let _scope = test_scope();
    let page = page_h("Initial");
    let nav = NavigationPage::new();
    nav.set_content(Some(Control::boxed(page.clone())));
    assert_eq!(1, nav.stack_depth());
    assert!(same(&page, &nav.current_page()));
}

#[test]
fn content_set_after_push_is_ignored() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let first = page_h("First");
    wait(nav.push_async(&first));

    // Setting the content when the stack is already populated should not push again.
    nav.set_content(Some(Control::boxed(page_h("Second"))));

    assert_eq!(1, nav.stack_depth());
    assert!(same(&first, &nav.current_page()));
}

// --- TransitionCancellationTests ---

fn cross_fade() -> Option<Rc<dyn IPageTransition>> {
    Some(Rc::new(CrossFade::with_duration(TimeSpan::from_milliseconds(100.0))))
}

#[test]
fn push_async_with_transition_when_cancelled_stack_unchanged_after_cancel() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));

    // Cancel the first navigation.
    let should_cancel = Rc::new(Cell::new(true));
    let cancel = should_cancel.clone();
    root.navigating(move |args| {
        if cancel.get() {
            args.set_cancel(true);
        }
        completed()
    });

    let custom_transition = cross_fade();
    wait(nav.push_async_with_transition(page(), custom_transition));
    assert_eq!(1, nav.stack_depth());

    // Stop cancelling and push again: the override should not leak.
    should_cancel.set(false);
    let second = page();
    wait(nav.push_async(&second));
    assert_eq!(2, nav.stack_depth());
    assert!(same(&second, &nav.current_page()));
}

#[test]
fn pop_async_with_transition_when_cancelled_stack_unchanged_after_cancel() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let top = page();
    wait(nav.push_async(&top));

    top.navigating(cancel_navigation(&flag()));

    wait(nav.pop_async_with_transition(cross_fade()));

    assert_eq!(2, nav.stack_depth());
    assert!(same(&top, &nav.current_page()));
}

// --- NavigatingEventTests ---

#[test]
fn pop_to_root_async_awaits_async_navigating_handler() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let top = page();
    wait(nav.push_async(&top));

    let handler_invoked = flag();
    top.navigating(cancel_navigation(&handler_invoked));

    wait(nav.pop_to_root_async());

    assert!(handler_invoked.get());
    assert_eq!(2, nav.stack_depth());
    assert!(same(&top, &nav.current_page()));
}

#[test]
fn pop_to_page_async_awaits_async_navigating_handler() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let middle = page();
    wait(nav.push_async(&middle));
    let top = page();
    wait(nav.push_async(&top));

    let handler_invoked = flag();
    top.navigating(cancel_navigation(&handler_invoked));

    wait(nav.pop_to_page_async(&root));

    assert!(handler_invoked.get());
    assert_eq!(3, nav.stack_depth());
    assert!(same(&top, &nav.current_page()));
}

#[test]
fn push_sync_invokes_navigating_event() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));

    let handler_invoked = flag();
    root.navigating(cancel_navigation(&handler_invoked));

    wait(nav.push_async(page()));

    assert!(handler_invoked.get());
    assert_eq!(1, nav.stack_depth());
}

#[test]
fn pop_sync_invokes_navigating_event() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let top = page();
    wait(nav.push_async(&top));

    let handler_invoked = flag();
    top.navigating(cancel_navigation(&handler_invoked));

    wait(nav.pop_async());

    assert!(handler_invoked.get());
    assert_eq!(2, nav.stack_depth());
}

#[test]
fn navigating_cancel_in_first_handler_skips_subsequent_handlers() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let top = page();
    wait(nav.push_async(&top));

    let second_handler_invoked = flag();
    top.navigating(cancel_navigation(&flag()));
    let invoked = second_handler_invoked.clone();
    top.navigating(move |_| {
        invoked.set(true);
        completed()
    });

    wait(nav.pop_async());

    assert!(!second_handler_invoked.get());
    assert_eq!(2, nav.stack_depth());
}

#[test]
fn navigating_cancel_in_on_navigating_from_skips_navigating_event() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let top = CancellingPage::new();
    wait(nav.push_async(&top));

    let async_handler_invoked = flag();
    let invoked = async_handler_invoked.clone();
    top.navigating(move |_| {
        invoked.set(true);
        completed()
    });

    wait(nav.pop_async());

    assert!(!async_handler_invoked.get());
    assert_eq!(2, nav.stack_depth());
}

// --- LogicalChildrenTests ---

#[test]
fn pop_removes_page_from_logical_children() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let top = page();
    wait(nav.push_async(&top));

    wait(nav.pop_async());

    assert!(!logical_children_contain(&nav, &top));
    assert!(logical_children_contain(&nav, &root));
}

#[test]
fn pop_to_root_async_removes_intermediate_pages_from_logical_children() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let middle = page();
    wait(nav.push_async(&middle));
    let top = page();
    wait(nav.push_async(&top));

    wait(nav.pop_to_root_async());

    assert!(logical_children_contain(&nav, &root));
    assert!(!logical_children_contain(&nav, &middle));
    assert!(!logical_children_contain(&nav, &top));
}

#[test]
fn replace_async_swaps_logical_children() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let original = page();
    wait(nav.push_async(&original));

    let replacement = page();
    wait(nav.replace_async(&replacement));

    assert!(!logical_children_contain(&nav, &original));
    assert!(logical_children_contain(&nav, &replacement));
}

#[test]
fn push_async_adds_page_to_logical_children() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    let second = page();
    wait(nav.push_async(&second));

    assert!(logical_children_contain(&nav, &root));
    assert!(logical_children_contain(&nav, &second));
}

#[test]
fn push_modal_modal_page_is_not_in_direct_logical_children() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let modal = page();
    wait(nav.push_modal_async(&modal));

    assert!(!logical_children_contain(&nav, &modal));
    assert!(navigation_is(&modal, &nav));
    assert!(modal.is_in_navigation_page());
}

#[test]
fn pop_modal_clears_navigation_and_is_in_navigation_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let modal = page();
    wait(nav.push_modal_async(&modal));

    wait(nav.pop_modal_async());

    assert!(modal.navigation().is_none());
    assert!(!modal.is_in_navigation_page());
}

#[test]
fn pop_keeps_outgoing_page_in_logical_tree_until_transition_completes() {
    let _scope = test_scope();
    let gate = Gate::new();
    let (nav, _root) = create_navigation_page(None);
    let root = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    nav.set_page_transition(Some(ControllableTransition::new(&gate)));
    let pop_task = nav.pop_async();

    // The outgoing page is still animating out, so it must remain in the logical tree.
    assert!(logical_children_contain(&nav, &top));

    gate.set_result();
    wait(pop_task);

    // Once the transition completes, the page is detached.
    assert!(!logical_children_contain(&nav, &top));
    assert!(logical_children_contain(&nav, &root));
}

#[test]
fn replace_keeps_outgoing_page_in_logical_tree_until_transition_completes() {
    let _scope = test_scope();
    let gate = Gate::new();
    let (nav, _root) = create_navigation_page(None);
    let original = page();
    wait(nav.push_async(&original));

    nav.set_page_transition(Some(ControllableTransition::new(&gate)));
    let replacement = page();
    let replace_task = nav.replace_async(&replacement);

    // The replaced page is still animating out, so both pages are in the logical tree.
    assert!(logical_children_contain(&nav, &original));
    assert!(logical_children_contain(&nav, &replacement));

    gate.set_result();
    wait(replace_task);

    assert!(!logical_children_contain(&nav, &original));
    assert!(logical_children_contain(&nav, &replacement));
}

#[test]
fn pop_to_root_keeps_visible_top_page_in_logical_tree_until_transition_completes() {
    let _scope = test_scope();
    let gate = Gate::new();
    let (nav, _root) = create_navigation_page(None);
    let root = page();
    let middle = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&middle));
    wait(nav.push_async(&top));

    nav.set_page_transition(Some(ControllableTransition::new(&gate)));
    let pop_task = nav.pop_to_root_async();

    // The hidden intermediate page is detached eagerly, but the visible top page is
    // animating out and must stay attached until the transition completes.
    assert!(!logical_children_contain(&nav, &middle));
    assert!(logical_children_contain(&nav, &top));

    gate.set_result();
    wait(pop_task);

    assert!(!logical_children_contain(&nav, &top));
    assert!(logical_children_contain(&nav, &root));
}
