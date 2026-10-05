//! The navigation page tests, continued: the pop all modals, replace,
//! lifecycle, modal transition cancellation, swipe gesture, navigating
//! state, visual tree lifecycle and content coercion groups of the
//! reference tests.

use super::navigation_page_tests::*;
use super::{
    ContentPage, DrawerPage, ModalPoppedEventArgs, NavigatedFromEventArgs, NavigatedToEventArgs, NavigationEventArgs,
    NavigationPage, NavigationType, Page, PageList, PageNavigationExtensions,
};
use crate::mouse_test_helper::MouseTestHelper;
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::Control;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::{
    IPointer, KeyModifiers, MouseButton, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType,
    PointerUpdateKind, RawInputModifiers, SwipeGestureEventArgs,
};
use ferroui_base::interactivity::Interactive;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Point, Ref, Size, StaticType, Vector, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// --- PopAllModalsTests ---

#[test]
fn pop_all_modals_empty_stack_does_nothing() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    wait(nav.pop_all_modals_async());
    assert_eq!(0, nav.modal_stack().len());
}

#[test]
fn pop_all_modals_single_modal_clears_modal_stack() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    wait(nav.push_modal_async(page()));
    wait(nav.pop_all_modals_async());
    assert_eq!(0, nav.modal_stack().len());
}

#[test]
fn pop_all_modals_multiple_modals_clears_entire_stack() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    wait(nav.push_modal_async(page_h("M1")));
    wait(nav.push_modal_async(page_h("M2")));
    wait(nav.push_modal_async(page_h("M3")));
    wait(nav.pop_all_modals_async());
    assert_eq!(0, nav.modal_stack().len());
}

#[test]
fn pop_all_modals_fires_modal_popped_for_each_modal() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let m1 = page_h("M1");
    let m2 = page_h("M2");
    let m3 = page_h("M3");
    wait(nav.push_modal_async(&m1));
    wait(nav.push_modal_async(&m2));
    wait(nav.push_modal_async(&m3));

    let popped: Rc<RefCell<Vec<Ref<Page>>>> = Rc::default();
    let p = popped.clone();
    nav.modal_popped(move |e: &ModalPoppedEventArgs| p.borrow_mut().push(e.modal()));

    wait(nav.pop_all_modals_async());

    assert_eq!(3, popped.borrow().len());
    // LIFO order: m3 was pushed last so must be popped first.
    assert!(stack_is(&popped.borrow(), &[&m3, &m2, &m1]));
}

#[test]
fn pop_all_modals_fires_navigated_from_on_all_modals() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let m1 = page_h("M1");
    let m2 = page_h("M2");
    wait(nav.push_modal_async(&m1));
    wait(nav.push_modal_async(&m2));

    let navigated_from = log();
    m1.navigated_from(record(&navigated_from, "M1"));
    m2.navigated_from(record(&navigated_from, "M2"));

    wait(nav.pop_all_modals_async());

    // LIFO order: M2 was pushed last, so it navigates from first.
    assert_eq!(*navigated_from.borrow(), ["M2", "M1"]);
}

#[test]
fn pop_all_modals_fires_navigated_to_on_underlying_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));
    wait(nav.push_modal_async(page_h("M1")));
    wait(nav.push_modal_async(page_h("M2")));

    let navigated_to = flag();
    root.navigated_to(raise_flag(&navigated_to));

    wait(nav.pop_all_modals_async());

    assert!(navigated_to.get());
}

#[test]
fn pop_all_modals_navigated_from_navigation_type_is_pop_modal() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let m1 = page();
    let m2 = page();
    wait(nav.push_modal_async(&m1));
    wait(nav.push_modal_async(&m2));

    let m1_args = slot::<NavigatedFromEventArgs>();
    let m2_args = slot::<NavigatedFromEventArgs>();
    m1.navigated_from(store(&m1_args));
    m2.navigated_from(store(&m2_args));

    wait(nav.pop_all_modals_async());

    assert_eq!(NavigationType::PopModal, taken(&m1_args).navigation_type());
    assert_eq!(NavigationType::PopModal, taken(&m2_args).navigation_type());
}

#[test]
fn pop_all_modals_navigated_to_navigation_type_is_pop_modal() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));
    wait(nav.push_modal_async(page()));

    let args = slot::<NavigatedToEventArgs>();
    root.navigated_to(store(&args));

    wait(nav.pop_all_modals_async());

    assert_eq!(NavigationType::PopModal, taken(&args).navigation_type());
}

#[test]
fn pop_all_modals_navigated_to_previous_page_is_top_modal() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let m1 = page();
    let m2 = page();
    wait(nav.push_async(&root));
    wait(nav.push_modal_async(&m1));
    wait(nav.push_modal_async(&m2));

    let args = slot::<NavigatedToEventArgs>();
    root.navigated_to(store(&args));

    wait(nav.pop_all_modals_async());

    assert!(same(&m2, &taken(&args).previous_page()));
}

#[test]
fn pop_all_modals_multiple_modals_navigated_to_fired_only_on_base_current_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));
    let m1 = page_h("M1");
    let m2 = page_h("M2");
    wait(nav.push_modal_async(&m1));
    wait(nav.push_modal_async(&m2));

    let navigated_to_pages = log();
    root.navigated_to(record(&navigated_to_pages, "root"));
    m1.navigated_to(record(&navigated_to_pages, "m1"));
    m2.navigated_to(record(&navigated_to_pages, "m2"));

    wait(nav.pop_all_modals_async());

    assert_eq!(*navigated_to_pages.borrow(), ["root"]);
}

#[test]
fn pop_all_modals_when_top_modal_cancels_navigating_does_not_clear_modal_stack() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page_h("Root")));
    let m1 = page_h("M1");
    let m2 = page_h("M2");
    wait(nav.push_modal_async(&m1));
    wait(nav.push_modal_async(&m2));

    let handler_invoked = flag();
    m2.navigating(cancel_navigation(&handler_invoked));

    wait(nav.pop_all_modals_async());

    assert!(handler_invoked.get());
    assert!(stack_is(&nav.modal_stack(), &[&m1, &m2]));
}

// --- ReplaceTests ---

// `ReplaceAsync_NullPage_Throws` has no counterpart: a page reference cannot be null.

#[test]
fn replace_async_on_empty_stack_pushes_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let page = page_h("Replaced");
    wait(nav.replace_async(&page));
    assert_eq!(1, nav.stack_depth());
    assert!(same(&page, &nav.current_page()));
}

#[test]
fn replace_async_replaces_top_page_stack_depth_unchanged() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));

    let replacement = page_h("Replacement");
    wait(nav.replace_async(&replacement));

    assert_eq!(1, nav.stack_depth());
    assert!(same(&replacement, &nav.current_page()));
    assert!(!nav.navigation_stack().iter().any(|page| same(page, &root)));
}

#[test]
fn replace_async_when_replacement_already_exists_in_navigation_stack_throws() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    assert!(throws_async(nav.replace_async(&root)));
}

#[test]
fn replace_async_when_replacement_already_exists_in_modal_stack_throws() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page_h("Root")));
    wait(nav.push_async(page_h("Top")));

    let modal = page_h("Modal");
    wait(nav.push_modal_async(&modal));

    assert!(throws_async(nav.replace_async(&modal)));
}

#[test]
fn replace_async_with_current_page_is_no_op() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let lifecycle_events = counter();
    top.navigated_from(count(&lifecycle_events));
    top.navigated_to(count(&lifecycle_events));

    wait(nav.replace_async(&top));

    assert_eq!(2, nav.stack_depth());
    assert!(same(&top, &nav.current_page()));
    assert_eq!(0, lifecycle_events.get());
}

#[test]
fn replace_async_fires_lifecycle_events_in_order() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let original = page_h("Original");
    wait(nav.push_async(&original));

    let replacement = page_h("Replacement");
    let order = log();
    original.navigated_from(record(&order, "Original: NavigatedFrom"));
    replacement.navigated_to(record(&order, "Replacement: NavigatedTo"));

    wait(nav.replace_async(&replacement));

    assert_eq!(*order.borrow(), ["Original: NavigatedFrom", "Replacement: NavigatedTo"]);
}

#[test]
fn replace_async_navigation_type_is_replace() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let original = page_h("Original");
    wait(nav.push_async(&original));

    let replacement = page_h("Replacement");
    let arrived = slot::<NavigatedToEventArgs>();
    let departed = slot::<NavigatedFromEventArgs>();
    replacement.navigated_to(store(&arrived));
    original.navigated_from(store(&departed));

    wait(nav.replace_async(&replacement));

    assert_eq!(NavigationType::Replace, taken(&arrived).navigation_type());
    assert_eq!(NavigationType::Replace, taken(&departed).navigation_type());
}

#[test]
fn replace_async_when_cancelled_does_not_replace() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let original = page_h("Original");
    wait(nav.push_async(&original));

    original.navigating(cancel_navigation(&flag()));

    let replacement = page_h("Replacement");
    wait(nav.replace_async(&replacement));

    assert_eq!(1, nav.stack_depth());
    assert!(same(&original, &nav.current_page()));
}

#[test]
fn replace_passes_parameter_with_event_args() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let param: BoxedValue = Rc::new(());
    let args = slot::<NavigatedToEventArgs>();
    root.navigated_to(store(&args));
    wait(nav.replace_async_with_parameter(&root, None, Some(param.clone())));
    assert!(taken(&args).parameter().is_some_and(|parameter| Rc::ptr_eq(&parameter, &param)));
}

#[test]
fn replace_generic_replace_page_with_correct_type() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.as_navigation().replace_async_of::<ContentPage>(None, None));

    assert!(nav
        .current_page()
        .is_some_and(|page| std::ptr::eq(page.get_type(), <ContentPage as StaticType>::TYPE)));
}

// --- PopToRootLifecycleTests ---

#[test]
fn pop_to_root_async_previous_page_is_not_null() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let second = page_h("Second");
    let third = page_h("Third");
    wait(nav.push_async(&root));
    wait(nav.push_async(&second));
    wait(nav.push_async(&third));

    let args = slot::<NavigatedToEventArgs>();
    root.navigated_to(store(&args));

    wait(nav.pop_to_root_async());

    assert!(same(&third, &taken(&args).previous_page()));
}

#[test]
fn pop_to_root_async_navigation_type_is_pop_to_root() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));
    wait(nav.push_async(page()));

    let args = slot::<NavigatedToEventArgs>();
    root.navigated_to(store(&args));

    wait(nav.pop_to_root_async());

    assert_eq!(NavigationType::PopToRoot, taken(&args).navigation_type());
}

// --- PopToPageLifecycleTests ---

#[test]
fn pop_to_page_async_previous_page_is_not_null() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let target = page_h("Target");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&target));
    wait(nav.push_async(&top));

    let args = slot::<NavigatedToEventArgs>();
    target.navigated_to(store(&args));

    wait(nav.pop_to_page_async(&target));

    assert!(same(&top, &taken(&args).previous_page()));
}

#[test]
fn pop_to_page_async_navigation_type_is_pop() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let target = page_h("Target");
    wait(nav.push_async(&root));
    wait(nav.push_async(&target));
    wait(nav.push_async(page()));

    let args = slot::<NavigatedToEventArgs>();
    target.navigated_to(store(&args));

    wait(nav.pop_to_page_async(&target));

    assert_eq!(NavigationType::Pop, taken(&args).navigation_type());
}

#[test]
fn pop_to_page_async_intermediate_pages_navigation_type_is_pop() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let target = page();
    let middle = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&target));
    wait(nav.push_async(&middle));
    wait(nav.push_async(&top));

    let middle_args = slot::<NavigatedFromEventArgs>();
    let top_args = slot::<NavigatedFromEventArgs>();
    middle.navigated_from(store(&middle_args));
    top.navigated_from(store(&top_args));

    wait(nav.pop_to_page_async(&target));

    assert_eq!(NavigationType::Pop, taken(&top_args).navigation_type());
    assert_eq!(NavigationType::Pop, taken(&middle_args).navigation_type());
}

#[test]
fn pop_to_page_async_page_not_in_stack_throws_argument_exception() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let stranger = page();
    assert!(throws_async(nav.pop_to_page_async(&stranger)));
}

#[test]
fn pop_to_page_async_target_is_already_top_is_no_op() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let navigated_to_count = counter();
    top.navigated_to(count(&navigated_to_count));

    wait(nav.pop_to_page_async(&top));

    assert!(same(&top, &nav.current_page()));
    assert_eq!(2, nav.stack_depth());
    assert_eq!(0, navigated_to_count.get());
}

// --- ModalTransitionCancellationTests ---

#[test]
fn push_modal_async_cancelled_transition_still_fires_lifecycle_events() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));

    let modal = page();
    let navigated_from_fired = flag();
    let navigated_to_fired = flag();
    let modal_pushed_fired = flag();
    root.navigated_from(raise_flag(&navigated_from_fired));
    modal.navigated_to(raise_flag(&navigated_to_fired));
    nav.modal_pushed(raise_flag(&modal_pushed_fired));

    wait(nav.push_modal_async_with_transition(&modal, None));

    assert!(navigated_from_fired.get());
    assert!(navigated_to_fired.get());
    assert!(modal_pushed_fired.get());
    assert!(stack_is(&nav.modal_stack(), &[&modal]));
}

#[test]
fn pop_all_modals_async_cancelled_transition_still_clears_stack_and_fires_events() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));

    let m1 = page();
    let m2 = page();
    wait(nav.push_modal_async(&m1));
    wait(nav.push_modal_async(&m2));

    let m1_popped_fired = flag();
    let m2_popped_fired = flag();
    let root_navigated_to_fired = flag();
    {
        let (m1, m2) = (m1.clone(), m2.clone());
        let (m1_fired, m2_fired) = (m1_popped_fired.clone(), m2_popped_fired.clone());
        nav.modal_popped(move |e: &ModalPoppedEventArgs| {
            if same(&e.modal(), &m1) {
                m1_fired.set(true);
            }
            if same(&e.modal(), &m2) {
                m2_fired.set(true);
            }
        });
    }
    root.navigated_to(raise_flag(&root_navigated_to_fired));

    wait(nav.pop_all_modals_async_with_transition(None));

    assert_eq!(0, nav.modal_stack().len());
    assert!(m1_popped_fired.get());
    assert!(m2_popped_fired.get());
    assert!(root_navigated_to_fired.get());
}

// --- LifecycleAfterTransitionTests ---

/// A handler that records whether it ran while the gate was still closed.
fn during_transition<T>(recorded: &Rc<Cell<bool>>, gate: &Gate) -> impl Fn(&T) + 'static {
    let recorded = recorded.clone();
    let gate = gate.clone();
    move |_| recorded.set(!gate.is_completed())
}

#[test]
fn push_async_lifecycle_events_fire_after_transition() {
    let _scope = test_scope();
    let tcs = Gate::new();
    let transition = ControllableTransition::new(&tcs);
    let (nav, _root) = create_navigation_page(Some(transition));

    let root = page_h("Root");
    wait(nav.push_async(&root));

    let navigated_from_during_transition = flag();
    let navigated_to_during_transition = flag();
    let pushed_during_transition = flag();

    let second = page_h("Second");
    root.navigated_from(during_transition(&navigated_from_during_transition, &tcs));
    second.navigated_to(during_transition(&navigated_to_during_transition, &tcs));
    nav.pushed(during_transition::<NavigationEventArgs>(&pushed_during_transition, &tcs));

    let push_task = nav.push_async(&second);
    tcs.set_result();
    wait(push_task);

    assert!(!navigated_from_during_transition.get());
    assert!(!navigated_to_during_transition.get());
    assert!(!pushed_during_transition.get());
}

#[test]
fn pop_async_lifecycle_events_fire_after_transition() {
    let _scope = test_scope();
    let tcs = Gate::new();
    let (nav, _root) = create_navigation_page(None);

    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    nav.set_page_transition(Some(ControllableTransition::new(&tcs)));

    let navigated_from_during_transition = flag();
    let navigated_to_during_transition = flag();
    let popped_during_transition = flag();

    top.navigated_from(during_transition(&navigated_from_during_transition, &tcs));
    root.navigated_to(during_transition(&navigated_to_during_transition, &tcs));
    nav.popped(during_transition::<NavigationEventArgs>(&popped_during_transition, &tcs));

    let pop_task = nav.pop_async();
    tcs.set_result();
    wait(pop_task);

    assert!(!navigated_from_during_transition.get());
    assert!(!navigated_to_during_transition.get());
    assert!(!popped_during_transition.get());
}

#[test]
fn pop_to_root_async_lifecycle_events_fire_after_transition() {
    let _scope = test_scope();
    let tcs = Gate::new();
    let (nav, _root) = create_navigation_page(None);

    let root = page_h("Root");
    let second = page_h("Second");
    let third = page_h("Third");
    wait(nav.push_async(&root));
    wait(nav.push_async(&second));
    wait(nav.push_async(&third));

    nav.set_page_transition(Some(ControllableTransition::new(&tcs)));

    let navigated_from_during_transition = flag();
    let navigated_to_during_transition = flag();
    let popped_to_root_during_transition = flag();

    second.navigated_from(during_transition(&navigated_from_during_transition, &tcs));
    third.navigated_from(during_transition(&navigated_from_during_transition, &tcs));
    root.navigated_to(during_transition(&navigated_to_during_transition, &tcs));
    nav.popped_to_root(during_transition::<NavigationEventArgs>(&popped_to_root_during_transition, &tcs));

    let pop_task = nav.pop_to_root_async();
    tcs.set_result();
    wait(pop_task);

    assert!(!navigated_from_during_transition.get());
    assert!(!navigated_to_during_transition.get());
    assert!(!popped_to_root_during_transition.get());
}

#[test]
fn replace_async_lifecycle_events_fire_after_transition() {
    let _scope = test_scope();
    let tcs = Gate::new();
    let (nav, _root) = create_navigation_page(None);

    let root = page_h("Root");
    wait(nav.push_async(&root));

    nav.set_page_transition(Some(ControllableTransition::new(&tcs)));

    let navigated_from_during_transition = flag();
    let navigated_to_during_transition = flag();

    let replacement = page_h("Replacement");
    root.navigated_from(during_transition(&navigated_from_during_transition, &tcs));
    replacement.navigated_to(during_transition(&navigated_to_during_transition, &tcs));

    let replace_task = nav.replace_async(&replacement);
    tcs.set_result();
    wait(replace_task);

    assert!(!navigated_from_during_transition.get());
    assert!(!navigated_to_during_transition.get());
}

// --- SwipeGestureTests ---

fn raise_handled_pointer_pressed(target: &NavigationPage, position: Point) {
    let pointer: Rc<dyn IPointer> = Pointer::new(Pointer::get_next_free_id(), PointerType::Touch, true);
    let visual: Ref<Visual> = target.to_ref().upcast();
    let source: Ref<Interactive> = target.to_ref().upcast();
    let args = PointerPressedEventArgs::new(
        source,
        pointer,
        &visual,
        position,
        1,
        PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE,
        1,
    );
    args.set_handled(true);

    target.raise_event(&args);
}

#[test]
fn handled_pointer_pressed_at_edge_allows_swipe_pop() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root_page = page_h("Root");
    let top_page = page_h("Top");
    wait(nav.push_async(&root_page));
    wait(nav.push_async(&top_page));

    let root = TestRoot::with_child(nav.clone());
    root.execute_initial_layout_pass();

    raise_handled_pointer_pressed(&nav, Point::new(5.0, 5.0));

    let swipe = SwipeGestureEventArgs::new(1, Vector::new(-20.0, 0.0), Vector::default());
    nav.raise_event(&swipe);
    Dispatcher::ui_thread().run_jobs(None);

    assert!(swipe.handled());
    assert_eq!(1, nav.stack_depth());
    assert!(same(&root_page, &nav.current_page()));
}

#[test]
fn mouse_edge_drag_allows_swipe_pop() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    nav.set_width(400.0);
    nav.set_height(300.0);
    nav.gesture_recognizers()
        .to_vec()
        .into_iter()
        .find_map(|recognizer| recognizer.cast::<SwipeGestureRecognizer>())
        .expect("the navigation page has a swipe gesture recognizer")
        .set_is_mouse_enabled(true);

    let root_page = page_h("Root");
    let top_page = page_h("Top");
    wait(nav.push_async(&root_page));
    wait(nav.push_async(&top_page));

    let root = TestRoot::new();
    root.set_client_size(Size::new(400.0, 300.0));
    root.set_child(nav.clone());
    root.execute_initial_layout_pass();

    let mouse = MouseTestHelper::new();
    mouse.down_at(&nav, MouseButton::Left, Point::new(5.0, 5.0), 1);
    mouse.move_(&nav, Point::new(40.0, 5.0));
    mouse.up_at(&nav, MouseButton::Left, Point::new(40.0, 5.0));
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(1, nav.stack_depth());
    assert!(same(&root_page, &nav.current_page()));
}

#[test]
fn same_gesture_id_only_pops_one_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    nav.set_width(400.0);
    nav.set_height(300.0);
    let page1 = page_h("1");
    let page2 = page_h("2");
    let page3 = page_h("3");
    wait(nav.push_async(&page1));
    wait(nav.push_async(&page2));
    wait(nav.push_async(&page3));

    let root = TestRoot::new();
    root.set_client_size(Size::new(400.0, 300.0));
    root.set_child(nav.clone());
    root.execute_initial_layout_pass();

    raise_handled_pointer_pressed(&nav, Point::new(5.0, 5.0));

    nav.raise_event(&SwipeGestureEventArgs::new(42, Vector::new(-20.0, 0.0), Vector::default()));
    nav.raise_event(&SwipeGestureEventArgs::new(42, Vector::new(-30.0, 0.0), Vector::default()));
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(2, nav.stack_depth());
    assert!(same(&page2, &nav.current_page()));
}

// --- IsNavigatingTests ---

#[test]
fn is_navigating_false_by_default() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    assert!(!nav.is_navigating());
}

#[test]
fn is_navigating_true_while_push_in_progress() {
    let _scope = test_scope();
    let tcs = Gate::new();
    let (nav, _root) = create_navigation_page(Some(ControllableTransition::new(&tcs)));

    wait(nav.push_async(page()));

    let push_task = nav.push_async(page());
    let during_push = nav.is_navigating();

    tcs.set_result();
    wait(push_task);

    assert!(during_push);
}

#[test]
fn is_navigating_false_after_push_completes() {
    let _scope = test_scope();
    let tcs = Gate::new();
    let (nav, _root) = create_navigation_page(Some(ControllableTransition::new(&tcs)));

    wait(nav.push_async(page()));

    let push_task = nav.push_async(page());
    tcs.set_result();
    wait(push_task);

    assert!(!nav.is_navigating());
}

// --- VisualTreeLifecycleTests ---

#[test]
fn detach_and_reattach_preserves_modal_stack() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    nav.set_template(Some(create_navigation_page_template()));
    let root = TestRoot::with_child(nav.clone());
    root.execute_initial_layout_pass();

    let page = page_h("Root");
    let modal = page_h("Modal");
    wait(nav.push_async(&page));
    wait(nav.push_modal_async(&modal));

    root.set_child(None);

    assert!(stack_is(&nav.modal_stack(), &[&modal]));
    assert!(modal.navigation().is_none());
    assert!(!modal.is_in_navigation_page());

    root.set_child(nav.clone());
    root.execute_initial_layout_pass();

    assert!(stack_is(&nav.modal_stack(), &[&modal]));
    assert!(navigation_is(&modal, &nav));
    assert!(modal.is_in_navigation_page());
}

// --- PagesPropertyTests ---

#[test]
fn pages_direct_assignment_throws_invalid_operation_exception() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let original_pages = nav.pages();

    assert!(throws(|| nav.set_pages(Some(PageList::from_items([page().upcast::<Page>()])))));

    assert!(original_pages.is_some());
    assert!(original_pages == nav.pages());
    assert!(nav.navigation_stack().is_empty());
    assert!(nav.current_page().is_none());
}

#[test]
fn pages_assign_null_throws_invalid_operation_exception() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let original_pages = nav.pages();

    assert!(throws(|| nav.set_pages(None)));

    assert!(original_pages.is_some());
    assert!(original_pages == nav.pages());
    assert!(nav.navigation_stack().is_empty());
    assert!(nav.current_page().is_none());
}

#[test]
fn pages_direct_assignment_does_not_corrupt_existing_stack() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let top = page();
    wait(nav.push_async(root.clone()));
    wait(nav.push_async(top.clone()));
    let original_pages = nav.pages();

    assert!(throws(|| nav.set_pages(Some(PageList::from_items([page().upcast::<Page>()])))));

    assert!(original_pages.is_some());
    assert!(original_pages == nav.pages());
    assert_eq!(2, nav.navigation_stack().len());
    assert!(same(&root, &nav.navigation_stack()[0]));
    assert!(same(&top, &nav.navigation_stack()[1]));
    assert!(same(&top, &nav.current_page()));
}

// The tests below are not in the reference tests: they pin the enumeration order of the `Pages` property
// of a navigation page to the order of the reference stack (the visible page first, the root page last),
// while the navigation stack property lists the root page first.

/// Whether the `Pages` property of the navigation page enumerates exactly these pages, in order.
fn pages_are(nav: &NavigationPage, expected: &[&Ref<ContentPage>]) -> bool {
    let pages = nav.pages().expect("the navigation page has pages").snapshot();
    stack_is(&pages, expected)
}

fn nav_with_three() -> (Ref<NavigationPage>, Ref<ContentPage>, Ref<ContentPage>, Ref<ContentPage>) {
    let nav = NavigationPage::new();
    let (a, b, c) = (page_h("a"), page_h("b"), page_h("c"));
    wait(nav.push_async(a.clone()));
    wait(nav.push_async(b.clone()));
    wait(nav.push_async(c.clone()));
    (nav, a, b, c)
}

#[test]
fn pages_enumerates_top_first_after_push() {
    let _scope = test_scope();
    let (nav, a, b, c) = nav_with_three();

    assert!(pages_are(&nav, &[&c, &b, &a]));
    assert!(stack_is(&nav.navigation_stack(), &[&a, &b, &c]));
}

#[test]
fn pages_enumerates_top_first_after_pop() {
    let _scope = test_scope();
    let (nav, a, b, _c) = nav_with_three();

    wait(nav.pop_async());

    assert!(pages_are(&nav, &[&b, &a]));
    assert!(stack_is(&nav.navigation_stack(), &[&a, &b]));
}

#[test]
fn pages_enumerates_top_first_after_insert() {
    let _scope = test_scope();
    let (nav, a, b, c) = nav_with_three();
    let x = page_h("x");

    nav.insert_page(x.clone(), b.clone());

    assert!(pages_are(&nav, &[&c, &b, &x, &a]));
    assert!(stack_is(&nav.navigation_stack(), &[&a, &x, &b, &c]));
}

#[test]
fn pages_enumerates_top_first_after_insert_before_root_and_before_top() {
    let _scope = test_scope();
    let (nav, a, b, c) = nav_with_three();
    let x = page_h("x");
    let y = page_h("y");

    nav.insert_page(x.clone(), a.clone());
    nav.insert_page(y.clone(), c.clone());

    assert!(pages_are(&nav, &[&c, &y, &b, &a, &x]));
    assert!(stack_is(&nav.navigation_stack(), &[&x, &a, &b, &y, &c]));
}

#[test]
fn pages_enumerates_top_first_after_remove() {
    let _scope = test_scope();
    let (nav, a, b, c) = nav_with_three();

    nav.remove_page(b.clone());

    assert!(pages_are(&nav, &[&c, &a]));
    assert!(stack_is(&nav.navigation_stack(), &[&a, &c]));

    nav.remove_page(c.clone());

    assert!(pages_are(&nav, &[&a]));
}

#[test]
fn pages_enumerates_top_first_after_replace() {
    let _scope = test_scope();
    let (nav, a, b, _c) = nav_with_three();
    let d = page_h("d");

    wait(nav.replace_async(d.clone()));

    assert!(pages_are(&nav, &[&d, &b, &a]));
    assert!(stack_is(&nav.navigation_stack(), &[&a, &b, &d]));
}

#[test]
fn pages_enumerates_top_first_after_pop_to_root_and_pop_to_page() {
    let _scope = test_scope();
    let (nav, a, b, _c) = nav_with_three();

    wait(nav.pop_to_page_async(b.clone()));

    assert!(pages_are(&nav, &[&b, &a]));

    wait(nav.pop_to_root_async());

    assert!(pages_are(&nav, &[&a]));
    assert!(stack_is(&nav.navigation_stack(), &[&a]));
}

#[test]
fn pages_rejected_assignment_re_adds_logical_children_top_first() {
    let _scope = test_scope();
    let (nav, a, b, c) = nav_with_three();

    assert!(throws(|| nav.set_pages(None)));

    let children = nav.logical_children().snapshot();
    assert_eq!(3, children.len());
    for (child, expected) in children.iter().zip([&c, &b, &a]) {
        assert!(same(child, expected));
    }
}

// --- ContentPageCoerceTests ---

#[test]
fn content_page_content_set_to_page_throws_invalid_operation_exception() {
    let _scope = test_scope();
    let page = ContentPage::new();
    assert!(throws(|| page.set_content(Some(Control::boxed(ContentPage::new())))));
}

#[test]
fn content_page_content_set_to_non_page_does_not_throw() {
    let _scope = test_scope();
    let page = ContentPage::new();
    page.set_content(boxed_str("hello"));
    assert_eq!(page.content().as_ref().and_then(string_of).as_deref(), Some("hello"));
}

// --- DrawerPageFirstPageTests ---

#[test]
fn drawer_page_content_replaced_resends_navigated_to_on_load() {
    let _scope = test_scope();
    let root = TestRoot::new();
    let nav1 = NavigationPage::new();
    let page1 = page();
    let drawer = DrawerPage::new();
    drawer.set_content(Some(Control::boxed(nav1.clone())));
    root.set_child(drawer.clone());
    root.execute_initial_layout_pass();

    let first_args = slot::<NavigatedToEventArgs>();
    page1.navigated_to(store(&first_args));
    nav1.set_content(Some(Control::boxed(page1.clone())));

    assert!(first_args.borrow().is_some());

    let nav2 = NavigationPage::new();
    let page2 = page();

    let second_args = slot::<NavigatedToEventArgs>();
    page2.navigated_to(store(&second_args));

    drawer.set_content(Some(Control::boxed(nav2.clone())));
    nav2.set_content(Some(Control::boxed(page2.clone())));

    assert!(second_args.borrow().is_some());
}
