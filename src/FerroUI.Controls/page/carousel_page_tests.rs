//! The carousel page tests: the property, selection, event, lifecycle,
//! system back button, logical children and keyboard navigation groups of
//! the reference tests. The wheel, data template, swipe gesture, interactive
//! transition, carousel swipe and visual tree lifecycle groups are in
//! `carousel_page_tests_interaction`.
//!
//! The reference tests derive a testable class from the carousel page to
//! reach its protected members; here those members are reachable, so the
//! tests use the carousel page itself.

use super::navigation_page_tests::{page, same};
use super::tabbed_page_tests::{hp, is_logical_child, key_down, pages_of, ReceivedChange};
use super::{
    CarouselPage, NavigatedFromEventArgs, NavigatedToEventArgs, NavigationType, Page, PageList,
    PageSelectionChangedEventArgs,
};
use crate::templates::{FuncDataTemplate, FuncTemplate, IDataTemplate, ITemplateOf};
use crate::test_support::{test_scope, TestRoot};
use crate::{ContentControl, Panel, StackPanel};
use ferroui_base::animation::IPageTransition;
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::FlowDirection;
use ferroui_base::threading::{CancellationToken, DispatcherTask};
use ferroui_base::{Ref, Visual};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// --- helpers shared by the carousel page test files ---

/// The pages of the carousel page: the cast of `cp.Pages` to the observable
/// list of pages of the reference tests.
pub(super) fn pages(cp: &CarouselPage) -> PageList {
    cp.pages().expect("the carousel page has pages")
}

pub(super) fn simulate_key_down(cp: &CarouselPage, key: Key) {
    simulate_key_down_returns_handled(cp, key);
}

pub(super) fn simulate_key_down_returns_handled(cp: &CarouselPage, key: Key) -> bool {
    let e = key_down(key, KeyModifiers::NONE);
    cp.on_key_down(&e);
    e.handled()
}

/// A page transition that completes at once.
struct TestPageTransition;

impl IPageTransition for TestPageTransition {
    fn start(
        &self,
        _from: Option<&Ref<Visual>>,
        _to: Option<&Ref<Visual>>,
        _forward: bool,
        _cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        super::completed_task()
    }
}

fn test_page_transition() -> Rc<dyn IPageTransition> {
    Rc::new(TestPageTransition)
}

/// A carousel page with the two pages "A" and "B".
fn carousel_with_two_pages() -> (Ref<CarouselPage>, Ref<Page>, Ref<Page>) {
    let cp = CarouselPage::new();
    let page1 = hp("A");
    let page2 = hp("B");
    pages(&cp).add_range([page1.clone(), page2.clone()]);
    (cp, page1, page2)
}

// --- PropertyDefaults ---

#[test]
fn selected_index_default_is_minus_one() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    assert_eq!(-1, cp.selected_index());
}

#[test]
fn selected_page_default_is_null() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    assert!(cp.selected_page().is_none());
}

#[test]
fn current_page_default_is_null() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    assert!(cp.current_page().is_none());
}

#[test]
fn page_transition_default_is_null() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    assert!(cp.page_transition().is_none());
}

#[test]
fn is_gesture_enabled_default_is_true() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    assert!(cp.is_gesture_enabled());
}

#[test]
fn is_keyboard_navigation_enabled_default_is_true() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    assert!(cp.is_keyboard_navigation_enabled());
}

// --- PropertyRoundTrips ---

#[test]
fn is_gesture_enabled_round_trips() {
    let _scope = test_scope();
    for value in [true, false] {
        let cp = CarouselPage::new();
        cp.set_is_gesture_enabled(value);
        assert_eq!(value, cp.is_gesture_enabled());
    }
}

#[test]
fn is_keyboard_navigation_enabled_round_trips() {
    let _scope = test_scope();
    for value in [true, false] {
        let cp = CarouselPage::new();
        cp.set_is_keyboard_navigation_enabled(value);
        assert_eq!(value, cp.is_keyboard_navigation_enabled());
    }
}

#[test]
fn page_transition_round_trip() {
    let _scope = test_scope();
    let transition = test_page_transition();
    let cp = CarouselPage::new();
    cp.set_page_transition(Some(transition.clone()));
    assert!(cp.page_transition().is_some_and(|actual| Rc::ptr_eq(&actual, &transition)));
}

#[test]
fn page_transition_can_be_set_to_null() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    cp.set_page_transition(Some(test_page_transition()));
    cp.set_page_transition(None);
    assert!(cp.page_transition().is_none());
}

#[test]
fn items_panel_round_trip() {
    let _scope = test_scope();
    let template: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
        FuncTemplate::new(|| Some(StackPanel::new().upcast::<Panel>()));
    let cp = CarouselPage::new();
    cp.set_items_panel(template.clone());
    assert!(Rc::ptr_eq(&template, &cp.items_panel()));
}

#[test]
fn page_template_round_trip() {
    let _scope = test_scope();
    let template: Rc<dyn IDataTemplate> =
        FuncDataTemplate::for_type::<Ref<Page>>(|_, _| Some(ContentControl::new().upcast()), false);
    let cp = CarouselPage::new();
    cp.set_page_template(Some(template.clone()));
    assert!(cp.page_template().is_some_and(|actual| *actual == *template));
}

#[test]
fn page_template_can_be_set_to_null() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    cp.set_page_template(None);
    assert!(cp.page_template().is_none());
}

// --- SelectionBehavior ---

#[test]
fn selected_index_set_updates_selected_page_and_current_page() {
    let _scope = test_scope();
    let (cp, _page1, page2) = carousel_with_two_pages();
    cp.set_selected_index(0);

    cp.set_selected_index(1);

    assert!(same(&page2, &cp.selected_page()));
    assert!(same(&page2, &cp.current_page()));
}

#[test]
fn selected_index_auto_selects_first_when_pages_set() {
    let _scope = test_scope();
    let (cp, page1, _page2) = carousel_with_two_pages();

    assert_eq!(0, cp.selected_index());
    assert!(same(&page1, &cp.selected_page()));
}

#[test]
fn update_active_page_selects_first_when_first_page_added_to_empty_collection() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    assert_eq!(-1, cp.selected_index());

    let page1 = hp("A");
    pages(&cp).add(page1.clone());

    assert_eq!(0, cp.selected_index());
    assert!(same(&page1, &cp.selected_page()));
}

#[test]
fn update_active_page_does_not_override_existing_selection() {
    let _scope = test_scope();
    let (cp, _page1, _page2) = carousel_with_two_pages();
    cp.set_selected_index(1);

    pages(&cp).add(hp("C"));

    assert_eq!(1, cp.selected_index());
}

#[test]
fn selected_page_same_reference_as_current_page() {
    let _scope = test_scope();
    let (cp, _page1, _page2) = carousel_with_two_pages();
    cp.set_selected_index(1);

    assert!(same(&cp.selected_page(), &cp.current_page()));
}

#[test]
fn selected_index_invalid_with_pages_coerces_to_first_page() {
    let _scope = test_scope();
    let (cp, page1, _page2) = carousel_with_two_pages();

    cp.set_selected_index(999);

    assert_eq!(0, cp.selected_index());
    assert!(same(&page1, &cp.selected_page()));
    assert!(same(&page1, &cp.current_page()));
}

#[test]
fn selected_index_set_before_pages_is_stored() {
    let _scope = test_scope();
    let cp = CarouselPage::new();

    cp.set_selected_index(2);

    assert_eq!(2, cp.selected_index());
    assert!(cp.selected_page().is_none());
    assert!(cp.current_page().is_none());
}

#[test]
fn selected_index_set_before_pages_is_applied_when_pages_set() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    cp.set_selected_index(2);

    let page1 = hp("A");
    let page2 = hp("B");
    let page3 = hp("C");

    cp.set_pages(Some(pages_of([&page1, &page2, &page3])));

    assert_eq!(2, cp.selected_index());
    assert!(same(&page3, &cp.selected_page()));
    assert!(same(&page3, &cp.current_page()));
}

#[test]
fn pages_set_null_selected_index_retains_last_value() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    pages(&cp).add(hp("A"));
    assert_eq!(0, cp.selected_index());

    cp.set_pages(None);

    assert_eq!(0, cp.selected_index());
}

// --- SelectionChangedEvent ---

#[test]
fn selection_changed_fires_when_selection_changes() {
    let _scope = test_scope();
    let (cp, page1, page2) = carousel_with_two_pages();
    cp.set_selected_index(0);

    let received: Rc<RefCell<Option<PageSelectionChangedEventArgs>>> = Rc::default();
    let r = received.clone();
    cp.selection_changed(move |_, e| *r.borrow_mut() = Some(e.clone()));

    cp.set_selected_index(1);

    let received = received.borrow().clone().expect("selection changed");
    assert!(same(&page1, &received.previous_page()));
    assert!(same(&page2, &received.current_page()));
}

#[test]
fn selection_changed_not_fired_when_same_page_selected() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    pages(&cp).add(hp("A"));
    cp.set_selected_index(0);

    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    cp.selection_changed(move |_, _| c.set(c.get() + 1));

    cp.set_selected_index(0);

    assert_eq!(0, count.get());
}

#[test]
fn selection_changed_tracks_sequential_selections() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let all = [hp("A"), hp("B"), hp("C")];
    pages(&cp).add_range(all.iter().cloned());
    cp.set_selected_index(0);

    type Events = Rc<RefCell<Vec<(Option<Ref<Page>>, Option<Ref<Page>>)>>>;
    let events: Events = Rc::default();
    let ev = events.clone();
    cp.selection_changed(move |_, e| ev.borrow_mut().push((e.previous_page(), e.current_page())));

    cp.set_selected_index(1);
    cp.set_selected_index(2);
    cp.set_selected_index(0);

    let events = events.borrow();
    assert_eq!(3, events.len());
    assert!(same(&all[0], &events[0].0));
    assert!(same(&all[1], &events[0].1));
    assert!(same(&all[1], &events[1].0));
    assert!(same(&all[2], &events[1].1));
    assert!(same(&all[2], &events[2].0));
    assert!(same(&all[0], &events[2].1));
}

#[test]
fn selection_changed_previous_page_is_null_on_first_auto_selection() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let page1 = hp("A");

    let received: Rc<RefCell<Option<PageSelectionChangedEventArgs>>> = Rc::default();
    let r = received.clone();
    cp.selection_changed(move |_, e| *r.borrow_mut() = Some(e.clone()));

    pages(&cp).add(page1.clone());

    let received = received.borrow().clone().expect("selection changed");
    assert!(received.previous_page().is_none());
    assert!(same(&page1, &received.current_page()));
}

// --- CurrentPageChangedEvent ---

#[test]
fn current_page_changed_fires_on_selection_change() {
    let _scope = test_scope();
    let (cp, _page1, _page2) = carousel_with_two_pages();
    cp.set_selected_index(0);

    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    cp.current_page_changed(move || c.set(c.get() + 1));

    cp.set_selected_index(1);

    assert_eq!(1, count.get());
}

#[test]
fn current_page_changed_not_fired_when_same_page_selected() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    pages(&cp).add(hp("A"));
    cp.set_selected_index(0);

    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    cp.current_page_changed(move || c.set(c.get() + 1));

    cp.set_selected_index(0);

    assert_eq!(0, count.get());
}

// --- PagesChangedEvent ---

#[test]
fn pages_changed_fires_on_add() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let received: ReceivedChange = Rc::default();
    let r = received.clone();
    cp.pages_changed(move |e| *r.borrow_mut() = Some(e.share()));

    pages(&cp).add(page().upcast());

    let received = received.borrow().clone().expect("pages changed");
    assert_eq!(NotifyCollectionChangedAction::Add, received.action());
}

#[test]
fn pages_changed_fires_on_remove() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let page: Ref<Page> = page().upcast();
    pages(&cp).add(page.clone());

    let received: ReceivedChange = Rc::default();
    let r = received.clone();
    cp.pages_changed(move |e| *r.borrow_mut() = Some(e.share()));

    pages(&cp).remove(&page);

    let received = received.borrow().clone().expect("pages changed");
    assert_eq!(NotifyCollectionChangedAction::Remove, received.action());
}

#[test]
fn pages_changed_not_fired_after_pages_collection_replaced() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let old_pages = pages(&cp);
    cp.set_pages(Some(PageList::new()));

    cp.pages_changed(|_| panic!("Should not fire for old collection"));

    old_pages.add(page().upcast());
}

// --- PageLifecycleEvents ---

#[test]
fn selection_change_fires_navigated_from_on_previous_page() {
    let _scope = test_scope();
    let (cp, page1, page2) = carousel_with_two_pages();
    cp.set_selected_index(0);

    let args: Rc<RefCell<Option<NavigatedFromEventArgs>>> = Rc::default();
    let a = args.clone();
    page1.navigated_from(move |e| *a.borrow_mut() = Some(e.clone()));

    cp.set_selected_index(1);

    let args = args.borrow().clone().expect("navigated from");
    assert!(same(&page2, &args.destination_page()));
}

#[test]
fn selection_change_fires_navigated_to_on_new_page() {
    let _scope = test_scope();
    let (cp, page1, page2) = carousel_with_two_pages();
    cp.set_selected_index(0);

    let args: Rc<RefCell<Option<NavigatedToEventArgs>>> = Rc::default();
    let a = args.clone();
    page2.navigated_to(move |e| *a.borrow_mut() = Some(e.clone()));

    cp.set_selected_index(1);

    let args = args.borrow().clone().expect("navigated to");
    assert!(same(&page1, &args.previous_page()));
}

#[test]
fn selection_change_lifecycle_order_navigated_from_then_navigated_to() {
    let _scope = test_scope();
    let (cp, page1, page2) = carousel_with_two_pages();
    cp.set_selected_index(0);

    let order: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    let o = order.clone();
    page1.navigated_from(move |_| o.borrow_mut().push("NavigatedFrom"));
    let o = order.clone();
    page2.navigated_to(move |_| o.borrow_mut().push("NavigatedTo"));

    cp.set_selected_index(1);

    assert_eq!(*order.borrow(), ["NavigatedFrom", "NavigatedTo"]);
}

#[test]
fn selection_change_same_page_no_lifecycle_events() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let page = hp("A");
    pages(&cp).add(page.clone());
    cp.set_selected_index(0);

    let events: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    let ev = events.clone();
    page.navigated_to(move |_| ev.borrow_mut().push("NavigatedTo"));
    let ev = events.clone();
    page.navigated_from(move |_| ev.borrow_mut().push("NavigatedFrom"));

    cp.set_selected_index(0);

    assert!(events.borrow().is_empty());
}

#[test]
fn navigated_from_navigation_type_is_replace() {
    let _scope = test_scope();
    let (cp, page1, _page2) = carousel_with_two_pages();

    let args: Rc<RefCell<Option<NavigatedFromEventArgs>>> = Rc::default();
    let a = args.clone();
    page1.navigated_from(move |e| *a.borrow_mut() = Some(e.clone()));

    cp.set_selected_index(1);

    let args = args.borrow().clone().expect("navigated from");
    assert_eq!(NavigationType::Replace, args.navigation_type());
}

#[test]
fn navigated_to_navigation_type_is_replace() {
    let _scope = test_scope();
    let (cp, _page1, page2) = carousel_with_two_pages();

    let args: Rc<RefCell<Option<NavigatedToEventArgs>>> = Rc::default();
    let a = args.clone();
    page2.navigated_to(move |e| *a.borrow_mut() = Some(e.clone()));

    cp.set_selected_index(1);

    let args = args.borrow().clone().expect("navigated to");
    assert_eq!(NavigationType::Replace, args.navigation_type());
}

#[test]
fn navigated_to_first_auto_selection_navigation_type_is_replace() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let page = hp("A");

    let args: Rc<RefCell<Option<NavigatedToEventArgs>>> = Rc::default();
    let a = args.clone();
    page.navigated_to(move |e| *a.borrow_mut() = Some(e.clone()));

    pages(&cp).add(page.clone());

    let args = args.borrow().clone().expect("navigated to");
    assert_eq!(NavigationType::Replace, args.navigation_type());
}

// --- SystemBackButtonTests ---

fn raise_back_button(page: &Page) -> RoutedEventArgs {
    let args = RoutedEventArgs::with_event(Page::page_navigation_system_back_button_pressed_event());
    page.raise_event(&args);
    args
}

#[test]
fn back_event_is_forwarded_to_content() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let page: Ref<Page> = page().upcast();
    let is_raised = Rc::new(Cell::new(false));
    let r = is_raised.clone();
    page.page_navigation_system_back_button_pressed(move |_, _| r.set(true));
    let _root = TestRoot::with_child(cp.clone());
    pages(&cp).add(page.clone());

    let args = raise_back_button(&cp);

    assert!(is_raised.get());
    assert!(!args.handled());
}

// --- LogicalChildrenTests ---

#[test]
fn pages_added_become_logical_children() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let page = hp("A");

    pages(&cp).add(page.clone());

    assert!(is_logical_child(&cp, &page));
}

#[test]
fn pages_removed_removed_from_logical_children() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let page = hp("A");
    pages(&cp).add(page.clone());

    pages(&cp).remove(&page);

    assert!(!is_logical_child(&cp, &page));
}

#[test]
fn pages_clear_removes_all_logical_children() {
    let _scope = test_scope();
    let (cp, page1, page2) = carousel_with_two_pages();

    pages(&cp).clear();

    assert!(!is_logical_child(&cp, &page1));
    assert!(!is_logical_child(&cp, &page2));
}

#[test]
fn pages_replaced_old_pages_removed_new_pages_added() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let old1 = hp("Old1");
    let old2 = hp("Old2");
    pages(&cp).add_range([old1.clone(), old2.clone()]);

    let new_pages = pages_of([&hp("New1")]);
    cp.set_pages(Some(new_pages.clone()));

    assert!(!is_logical_child(&cp, &old1));
    assert!(!is_logical_child(&cp, &old2));
    assert!(is_logical_child(&cp, &new_pages.get(0)));
}

#[test]
fn pages_set_null_clears_logical_children() {
    let _scope = test_scope();
    let cp = CarouselPage::new();
    let page = hp("A");
    pages(&cp).add(page.clone());

    cp.set_pages(None);

    assert!(!is_logical_child(&cp, &page));
}

// --- KeyboardNavigationTests ---

fn make_carousel(count: usize, selected_index: i32) -> Ref<CarouselPage> {
    let cp = CarouselPage::new();
    for i in 0..count {
        pages(&cp).add(hp(&format!("P{i}")));
    }
    cp.set_selected_index(selected_index);
    cp
}

#[test]
fn right_key_navigates_to_next_page() {
    let _scope = test_scope();
    let cp = make_carousel(3, 0);
    simulate_key_down(&cp, Key::Right);
    assert_eq!(1, cp.selected_index());
}

#[test]
fn down_key_navigates_to_next_page() {
    let _scope = test_scope();
    let cp = make_carousel(3, 0);
    simulate_key_down(&cp, Key::Down);
    assert_eq!(1, cp.selected_index());
}

#[test]
fn left_key_navigates_to_previous_page() {
    let _scope = test_scope();
    let cp = make_carousel(3, 2);
    simulate_key_down(&cp, Key::Left);
    assert_eq!(1, cp.selected_index());
}

#[test]
fn up_key_navigates_to_previous_page() {
    let _scope = test_scope();
    let cp = make_carousel(3, 2);
    simulate_key_down(&cp, Key::Up);
    assert_eq!(1, cp.selected_index());
}

#[test]
fn home_key_navigates_to_first_page() {
    let _scope = test_scope();
    let cp = make_carousel(3, 2);
    simulate_key_down(&cp, Key::Home);
    assert_eq!(0, cp.selected_index());
}

#[test]
fn end_key_navigates_to_last_page() {
    let _scope = test_scope();
    let cp = make_carousel(3, 0);
    simulate_key_down(&cp, Key::End);
    assert_eq!(2, cp.selected_index());
}

#[test]
fn left_key_at_first_page_does_not_navigate() {
    let _scope = test_scope();
    let cp = make_carousel(3, 0);
    simulate_key_down(&cp, Key::Left);
    assert_eq!(0, cp.selected_index());
}

#[test]
fn right_key_at_last_page_does_not_navigate() {
    let _scope = test_scope();
    let cp = make_carousel(3, 2);
    simulate_key_down(&cp, Key::Right);
    assert_eq!(2, cp.selected_index());
}

#[test]
fn key_navigation_disabled_ignores_arrow_key() {
    let _scope = test_scope();
    let cp = make_carousel(3, 0);
    cp.set_is_keyboard_navigation_enabled(false);
    simulate_key_down(&cp, Key::Right);
    assert_eq!(0, cp.selected_index());
}

#[test]
fn right_key_marks_event_handled() {
    let _scope = test_scope();
    let cp = make_carousel(3, 0);
    let handled = simulate_key_down_returns_handled(&cp, Key::Right);
    assert!(handled);
}

#[test]
fn right_key_at_last_page_does_not_mark_event_handled() {
    let _scope = test_scope();
    let cp = make_carousel(3, 2);
    let handled = simulate_key_down_returns_handled(&cp, Key::Right);
    assert!(!handled);
}

#[test]
fn rtl_flow_direction_left_key_navigates_to_next_page() {
    let _scope = test_scope();
    let cp = make_carousel(3, 0);
    cp.set_flow_direction(FlowDirection::RightToLeft);
    simulate_key_down(&cp, Key::Left);
    assert_eq!(1, cp.selected_index());
}

#[test]
fn rtl_flow_direction_right_key_navigates_to_previous_page() {
    let _scope = test_scope();
    let cp = make_carousel(3, 2);
    cp.set_flow_direction(FlowDirection::RightToLeft);
    simulate_key_down(&cp, Key::Right);
    assert_eq!(1, cp.selected_index());
}

#[test]
fn rtl_flow_direction_left_key_at_last_page_does_not_navigate() {
    let _scope = test_scope();
    let cp = make_carousel(3, 2);
    cp.set_flow_direction(FlowDirection::RightToLeft);
    simulate_key_down(&cp, Key::Left);
    assert_eq!(2, cp.selected_index());
}

#[test]
fn rtl_flow_direction_right_key_at_first_page_does_not_navigate() {
    let _scope = test_scope();
    let cp = make_carousel(3, 0);
    cp.set_flow_direction(FlowDirection::RightToLeft);
    simulate_key_down(&cp, Key::Right);
    assert_eq!(0, cp.selected_index());
}
