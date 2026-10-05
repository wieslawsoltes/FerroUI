//! The tabbed page tests: the property, pages collection, selection,
//! lifecycle, tab enabling and keyboard navigation groups of the reference
//! tests. The data template, swipe gesture, system back button and visual
//! tree lifecycle groups are in `tabbed_page_tests_data_template`.
//!
//! The reference tests derive a testable class from the tabbed page to reach
//! its protected members; here those members are public (or visible in the
//! crate), so the tests use the tabbed page itself.
//!
//! The upstream tests are grouped in nested classes; the groups are kept, in
//! order, as sections of the files.

use super::navigation_page_tests::{page, page_h, same};
use super::{
    ContentPage, NavigatedFromEventArgs, NavigatedToEventArgs, NavigationType, Page, PageList,
    PageSelectionChangedEventArgs, SelectingMultiPage, TabPlacement, TabbedPage,
};
use crate::templates::{FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IDataTemplate};
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Control, PathIcon, TabControl};
use ferroui_base::animation::{CrossFade, IPageTransition, TimeSpan};
use ferroui_base::collections::{NotifyCollectionChangedAction, SharedNotifyCollectionChangedEventArgs};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::media::{EllipseGeometry, FlowDirection};
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{BoxedValue, FerroObject, FerroObjectExtensions, Rect, Ref, StyledElement};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// --- helpers shared by the tabbed page and carousel page test files ---

/// `new ContentPage { Header = header }` as a page.
pub(super) fn hp(header: &str) -> Ref<Page> {
    page_h(header).upcast()
}

/// A new observable list of the given pages.
pub(super) fn pages_of<const N: usize>(pages: [&Ref<Page>; N]) -> PageList {
    PageList::from_items(pages.into_iter().cloned())
}

/// Whether the page is a logical child of the element.
pub(super) fn is_logical_child(parent: &StyledElement, page: &Page) -> bool {
    let page: &StyledElement = page;
    parent.logical_children().snapshot().iter().any(|child| std::ptr::eq::<StyledElement>(&**child, page))
}

/// The content pages among the logical children of the element whose
/// header is a string, by header.
pub(super) fn logical_page_headers(parent: &StyledElement) -> Vec<String> {
    parent
        .logical_children()
        .snapshot()
        .iter()
        .filter_map(|child| child.clone().cast::<ContentPage>())
        .filter_map(|page| header_of(&page))
        .collect()
}

/// `page.Header?.ToString()`.
pub(super) fn header_of(page: &Page) -> Option<String> {
    page.header().as_ref().and_then(crate::test_support::string_of)
}

/// The shared args of the last change of the pages collection.
pub(super) type ReceivedChange = Rc<RefCell<Option<SharedNotifyCollectionChangedEventArgs<Ref<Page>>>>>;

pub(super) fn key_down(key: Key, modifiers: KeyModifiers) -> KeyEventArgs {
    let mut e = KeyEventArgs::new();
    e.set_routed_event(Some(InputElement::key_down_event()));
    e.key = key;
    e.key_modifiers = modifiers;
    e
}

fn simulate_key_down(tp: &TabbedPage, key: Key) {
    simulate_key_down_returns_handled(tp, key);
}

fn simulate_key_down_returns_handled(tp: &TabbedPage, key: Key) -> bool {
    let e = key_down(key, KeyModifiers::NONE);
    tp.on_key_down(&e);
    e.handled()
}

fn simulate_key_down_with_modifiers_returns_handled(tp: &TabbedPage, key: Key, modifiers: KeyModifiers) -> bool {
    let e = key_down(key, modifiers);
    tp.on_key_down(&e);
    e.handled()
}

fn call_commit_selection(tp: &TabbedPage, index: i32, page: Option<&Ref<Page>>) {
    tp.commit_selection(index, page.cloned(), NavigationType::Replace);
}

fn border_template() -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(|_| true, |_, _| Some(Border::new().upcast()), false)
}

fn template_is(actual: &Option<Rc<dyn IDataTemplate>>, expected: &Rc<dyn IDataTemplate>) -> bool {
    actual.as_ref().is_some_and(|actual| **actual == **expected)
}

// --- PropertyDefaults ---

#[test]
fn tab_placement_default_is_auto() {
    let _scope = test_scope();
    // Auto resolves to Bottom on iOS/Android and Top everywhere else.
    let tp = TabbedPage::new();
    assert_eq!(TabPlacement::Auto, tp.tab_placement());
}

#[test]
fn selected_index_initially_minus_one() {
    let _scope = test_scope();
    // -1 is the "no selection" sentinel used throughout the selection API.
    let tp = TabbedPage::new();
    assert_eq!(-1, tp.selected_index());
}

#[test]
fn selected_page_initially_null() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    assert!(tp.selected_page().is_none());
}

// --- PropertyRoundTrips ---

#[test]
fn tab_placement_round_trips() {
    let _scope = test_scope();
    for placement in
        [TabPlacement::Auto, TabPlacement::Top, TabPlacement::Bottom, TabPlacement::Left, TabPlacement::Right]
    {
        let tp = TabbedPage::new();
        tp.set_tab_placement(placement);
        assert_eq!(placement, tp.tab_placement());
    }
}

#[test]
fn is_keyboard_navigation_enabled_round_trips() {
    let _scope = test_scope();
    for enabled in [true, false] {
        let tp = TabbedPage::new();
        tp.set_is_keyboard_navigation_enabled(enabled);
        assert_eq!(enabled, tp.is_keyboard_navigation_enabled());
    }
}

#[test]
fn selected_index_stored_before_template_applied() {
    let _scope = test_scope();
    for index in [0, 1, 3] {
        let tp = TabbedPage::new();
        tp.set_selected_index(index);
        assert_eq!(index, tp.selected_index());
    }
}

#[test]
fn page_template_can_be_set_to_null() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    tp.set_page_template(None);
    assert!(tp.page_template().is_none());
}

#[test]
fn page_transition_round_trips() {
    let _scope = test_scope();
    let transition: Rc<dyn IPageTransition> = Rc::new(CrossFade::with_duration(TimeSpan::from_milliseconds(200.0)));
    let tp = TabbedPage::new();
    tp.set_page_transition(Some(transition.clone()));
    assert!(tp.page_transition().is_some_and(|actual| Rc::ptr_eq(&actual, &transition)));
}

#[test]
fn indicator_template_round_trips() {
    let _scope = test_scope();
    let template = border_template();
    let tp = TabbedPage::new();
    tp.set_indicator_template(Some(template.clone()));
    assert!(template_is(&tp.indicator_template(), &template));
}

#[test]
fn indicator_template_can_be_set_to_null() {
    let _scope = test_scope();
    let template = border_template();
    let tp = TabbedPage::new();
    tp.set_indicator_template(Some(template));
    tp.set_indicator_template(None);
    assert!(tp.indicator_template().is_none());
}

// --- PagesCollectionTests ---

#[test]
fn pages_initially_non_null_empty_list() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    assert!(tp.pages().is_some());
}

#[test]
fn pages_set_new_list_updates_property() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let pages = pages_of([&hp("A")]);
    tp.set_pages(Some(pages.clone()));
    assert!(tp.pages().is_some_and(|actual| actual.ptr_eq(&pages)));
}

#[test]
fn pages_added_become_logical_children() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let pages = PageList::new();
    tp.set_pages(Some(pages.clone()));

    let page1 = hp("Tab 1");
    let page2 = hp("Tab 2");
    pages.add(page1.clone());
    pages.add(page2.clone());

    assert!(is_logical_child(&tp, &page1));
    assert!(is_logical_child(&tp, &page2));
}

#[test]
fn pages_removed_removed_from_logical_children() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page1 = hp("Tab 1");
    let page2 = hp("Tab 2");
    let pages = pages_of([&page1, &page2]);
    tp.set_pages(Some(pages.clone()));

    pages.remove(&page1);

    assert!(!is_logical_child(&tp, &page1));
    assert!(is_logical_child(&tp, &page2));
}

#[test]
fn pages_replaced_old_logical_children_cleared_new_added() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let old = hp("Old");
    tp.set_pages(Some(pages_of([&old])));

    let fresh = hp("Fresh");
    tp.set_pages(Some(pages_of([&fresh])));

    assert!(!is_logical_child(&tp, &old));
    assert!(is_logical_child(&tp, &fresh));
}

#[test]
fn pages_set_null_clears_logical_children() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page: Ref<Page> = page().upcast();
    tp.set_pages(Some(pages_of([&page])));
    tp.set_pages(None);
    assert!(!is_logical_child(&tp, &page));
}

#[test]
fn pages_set_null_clears_current_page() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page: Ref<Page> = page().upcast();
    tp.set_pages(Some(pages_of([&page])));
    call_commit_selection(&tp, 0, Some(&page));
    assert!(tp.current_page().is_some());
    tp.set_pages(None);
    assert!(tp.current_page().is_none());
}

#[test]
fn pages_add_multiple_all_become_logical_children() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let pages = PageList::new();
    tp.set_pages(Some(pages.clone()));

    let mut list = Vec::new();
    for i in 0..5 {
        let p = hp(&format!("Tab {i}"));
        list.push(p.clone());
        pages.add(p);
    }

    for p in &list {
        assert!(is_logical_child(&tp, p));
    }
}

#[test]
fn pages_clear_removes_all_logical_children() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let a = hp("A");
    let b = hp("B");
    let pages = pages_of([&a, &b]);
    tp.set_pages(Some(pages.clone()));

    pages.clear();

    assert!(!is_logical_child(&tp, &a));
    assert!(!is_logical_child(&tp, &b));
}

// --- PagesChangedEventTests ---

#[test]
fn pages_changed_fires_on_add() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let pages = PageList::new();
    tp.set_pages(Some(pages.clone()));

    let received: ReceivedChange = Rc::default();
    let r = received.clone();
    tp.pages_changed(move |e| *r.borrow_mut() = Some(e.share()));

    pages.add(page().upcast());

    let received = received.borrow().clone().expect("pages changed");
    assert_eq!(NotifyCollectionChangedAction::Add, received.action());
}

#[test]
fn pages_changed_fires_on_remove() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page: Ref<Page> = page().upcast();
    let pages = pages_of([&page]);
    tp.set_pages(Some(pages.clone()));

    let received: ReceivedChange = Rc::default();
    let r = received.clone();
    tp.pages_changed(move |e| *r.borrow_mut() = Some(e.share()));

    pages.remove(&page);

    let received = received.borrow().clone().expect("pages changed");
    assert_eq!(NotifyCollectionChangedAction::Remove, received.action());
}

#[test]
fn pages_changed_not_fired_after_pages_replaced() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let old_pages = PageList::new();
    tp.set_pages(Some(old_pages.clone()));
    let fired = Rc::new(Cell::new(false));
    let f = fired.clone();
    tp.pages_changed(move |_| f.set(true));

    tp.set_pages(Some(PageList::new()));
    old_pages.add(page().upcast());
    assert!(!fired.get());
}

#[test]
fn pages_changed_add_args_contain_added_page() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let pages = PageList::new();
    tp.set_pages(Some(pages.clone()));

    let received: ReceivedChange = Rc::default();
    let r = received.clone();
    tp.pages_changed(move |e| *r.borrow_mut() = Some(e.share()));

    let page = hp("New");
    pages.add(page.clone());

    let received = received.borrow().clone().expect("pages changed");
    assert!(!received.new_items().is_empty());
    assert!(received.new_items().iter().any(|item| item.ptr_eq(&page)));
}

#[test]
fn pages_changed_remove_args_contain_removed_page() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page = hp("ToRemove");
    let pages = pages_of([&page]);
    tp.set_pages(Some(pages.clone()));

    let received: ReceivedChange = Rc::default();
    let r = received.clone();
    tp.pages_changed(move |e| *r.borrow_mut() = Some(e.share()));

    pages.remove(&page);

    let received = received.borrow().clone().expect("pages changed");
    assert!(!received.old_items().is_empty());
    assert!(received.old_items().iter().any(|item| item.ptr_eq(&page)));
}

#[test]
fn pages_changed_fires_on_clear_with_reset_action() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let pages = pages_of([&hp("A"), &hp("B")]);
    tp.set_pages(Some(pages.clone()));

    let received: ReceivedChange = Rc::default();
    let r = received.clone();
    tp.pages_changed(move |e| *r.borrow_mut() = Some(e.share()));

    pages.clear();

    let received = received.borrow().clone().expect("pages changed");
    assert_eq!(NotifyCollectionChangedAction::Reset, received.action());
}

// --- SelectionTests ---

#[test]
fn selection_changed_fires_when_selection_changes() {
    let _scope = test_scope();
    let tp = TabbedPage::new();

    let received: Rc<RefCell<Option<PageSelectionChangedEventArgs>>> = Rc::default();
    let r = received.clone();
    tp.selection_changed(move |_, e| *r.borrow_mut() = Some(e.clone()));

    let page1 = hp("A");
    let page2 = hp("B");
    call_commit_selection(&tp, 0, Some(&page1));
    call_commit_selection(&tp, 1, Some(&page2));

    let received = received.borrow().clone().expect("selection changed");
    assert!(same(&page1, &received.previous_page()));
    assert!(same(&page2, &received.current_page()));
}

#[test]
fn selection_changed_not_fired_when_same_page_selected() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    tp.selection_changed(move |_, _| c.set(c.get() + 1));

    let page = hp("A");
    call_commit_selection(&tp, 0, Some(&page));
    let count_after_first = count.get();
    call_commit_selection(&tp, 0, Some(&page));

    assert_eq!(1, count_after_first); // first commit must fire exactly once
    assert_eq!(1, count.get()); // second commit (same page) must not fire again
}

#[test]
fn commit_selection_updates_current_page() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page = hp("X");
    call_commit_selection(&tp, 0, Some(&page));

    assert!(same(&page, &tp.current_page()));
    assert!(same(&page, &tp.selected_page()));
    assert_eq!(0, tp.selected_index());
}

#[test]
fn commit_selection_sequential_selections_tracks_correct_pages() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let pages = [hp("Feed"), hp("Explore"), hp("Profile")];

    type Events = Rc<RefCell<Vec<(Option<Ref<Page>>, Option<Ref<Page>>)>>>;
    let events: Events = Rc::default();
    let ev = events.clone();
    tp.selection_changed(move |_, e| ev.borrow_mut().push((e.previous_page(), e.current_page())));

    call_commit_selection(&tp, 0, Some(&pages[0]));
    call_commit_selection(&tp, 1, Some(&pages[1]));
    call_commit_selection(&tp, 2, Some(&pages[2]));
    call_commit_selection(&tp, 0, Some(&pages[0]));

    let events = events.borrow();
    assert_eq!(4, events.len());
    assert!(events[0].0.is_none());
    assert!(same(&pages[0], &events[0].1));
    assert!(same(&pages[0], &events[1].0));
    assert!(same(&pages[1], &events[1].1));
    assert!(same(&pages[1], &events[2].0));
    assert!(same(&pages[2], &events[2].1));
    assert!(same(&pages[2], &events[3].0));
    assert!(same(&pages[0], &events[3].1));
}

#[test]
fn commit_selection_null_page_sets_current_page_to_null() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    call_commit_selection(&tp, 0, Some(&page().upcast()));
    call_commit_selection(&tp, -1, None);

    assert!(tp.current_page().is_none());
    assert!(tp.selected_page().is_none());
    assert_eq!(-1, tp.selected_index());
}

#[test]
fn commit_selection_rapid_changes_tracks_final_state() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let pages = [hp("A"), hp("B"), hp("C")];

    for (i, page) in pages.iter().enumerate() {
        call_commit_selection(&tp, i as i32, Some(page));
    }

    assert!(same(&pages[2], &tp.current_page()));
    assert!(same(&pages[2], &tp.selected_page()));
    assert_eq!(2, tp.selected_index());
}

#[test]
fn current_page_changed_fires_on_commit_selection() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    tp.current_page_changed(move || c.set(c.get() + 1));

    call_commit_selection(&tp, 0, Some(&page().upcast()));

    assert_eq!(1, count.get());
}

#[test]
fn current_page_changed_not_fired_when_same_page_committed() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page: Ref<Page> = page().upcast();
    call_commit_selection(&tp, 0, Some(&page));

    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    tp.current_page_changed(move || c.set(c.get() + 1));

    call_commit_selection(&tp, 0, Some(&page));

    assert_eq!(0, count.get());
}

// --- LifecycleTests ---

#[test]
fn commit_selection_fires_navigated_from_on_previous_page() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page1 = hp("A");
    let page2 = hp("B");
    call_commit_selection(&tp, 0, Some(&page1));

    let args: Rc<RefCell<Option<NavigatedFromEventArgs>>> = Rc::default();
    let a = args.clone();
    page1.navigated_from(move |e| *a.borrow_mut() = Some(e.clone()));
    call_commit_selection(&tp, 1, Some(&page2));

    let args = args.borrow().clone().expect("navigated from");
    assert!(same(&page2, &args.destination_page()));
    assert_eq!(NavigationType::Replace, args.navigation_type());
}

#[test]
fn commit_selection_fires_navigated_to_on_new_page() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page1 = hp("A");
    let page2 = hp("B");
    call_commit_selection(&tp, 0, Some(&page1));

    let args: Rc<RefCell<Option<NavigatedToEventArgs>>> = Rc::default();
    let a = args.clone();
    page2.navigated_to(move |e| *a.borrow_mut() = Some(e.clone()));
    call_commit_selection(&tp, 1, Some(&page2));

    let args = args.borrow().clone().expect("navigated to");
    assert!(same(&page1, &args.previous_page()));
    assert_eq!(NavigationType::Replace, args.navigation_type());
}

#[test]
fn commit_selection_lifecycle_order_navigated_from_navigated_to() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page1 = hp("A");
    let page2 = hp("B");
    call_commit_selection(&tp, 0, Some(&page1));

    let order: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    let o = order.clone();
    page1.navigated_from(move |_| o.borrow_mut().push("NavigatedFrom"));
    let o = order.clone();
    page2.navigated_to(move |_| o.borrow_mut().push("NavigatedTo"));
    call_commit_selection(&tp, 1, Some(&page2));

    assert_eq!(*order.borrow(), ["NavigatedFrom", "NavigatedTo"]);
}

#[test]
fn commit_selection_same_page_no_lifecycle_events() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page = hp("A");
    call_commit_selection(&tp, 0, Some(&page));

    let events: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    let ev = events.clone();
    page.navigated_to(move |_| ev.borrow_mut().push("NavigatedTo"));
    let ev = events.clone();
    page.navigated_from(move |_| ev.borrow_mut().push("NavigatedFrom"));
    call_commit_selection(&tp, 0, Some(&page));

    assert!(events.borrow().is_empty());
}

#[test]
fn commit_selection_first_page_navigated_to_has_null_previous() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page = hp("A");

    let navigated_to_args: Rc<RefCell<Option<NavigatedToEventArgs>>> = Rc::default();
    let events: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    let a = navigated_to_args.clone();
    let ev = events.clone();
    page.navigated_to(move |e| {
        *a.borrow_mut() = Some(e.clone());
        ev.borrow_mut().push("NavigatedTo");
    });
    let ev = events.clone();
    page.navigated_from(move |_| ev.borrow_mut().push("NavigatedFrom"));

    call_commit_selection(&tp, 0, Some(&page));

    assert_eq!(*events.borrow(), ["NavigatedTo"]);
    let navigated_to_args = navigated_to_args.borrow().clone().expect("navigated to");
    assert!(navigated_to_args.previous_page().is_none());
    assert_eq!(NavigationType::Replace, navigated_to_args.navigation_type());
}

#[test]
fn commit_selection_to_null_fires_navigated_from_with_null_destination() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let page = hp("A");
    call_commit_selection(&tp, 0, Some(&page));

    let navigated_from_args: Rc<RefCell<Option<NavigatedFromEventArgs>>> = Rc::default();
    let events: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    let a = navigated_from_args.clone();
    let ev = events.clone();
    page.navigated_from(move |e| {
        *a.borrow_mut() = Some(e.clone());
        ev.borrow_mut().push("NavigatedFrom");
    });

    call_commit_selection(&tp, -1, None);

    assert_eq!(*events.borrow(), ["NavigatedFrom"]);
    let navigated_from_args = navigated_from_args.borrow().clone().expect("navigated from");
    assert!(navigated_from_args.destination_page().is_none());
    assert_eq!(NavigationType::Replace, navigated_from_args.navigation_type());
}

// --- IsTabEnabledTests ---

#[test]
fn is_tab_enabled_default_is_true() {
    let _scope = test_scope();
    let page = page();
    assert!(TabbedPage::get_is_tab_enabled(&page));
}

#[test]
fn is_tab_enabled_set_false_get_false() {
    let _scope = test_scope();
    let page = page();
    TabbedPage::set_is_tab_enabled(&page, false);
    assert!(!TabbedPage::get_is_tab_enabled(&page));
}

#[test]
fn is_tab_enabled_set_true_get_true() {
    let _scope = test_scope();
    let page = page();
    TabbedPage::set_is_tab_enabled(&page, false);
    TabbedPage::set_is_tab_enabled(&page, true);
    assert!(TabbedPage::get_is_tab_enabled(&page));
}

// --- FindNextEnabledTabTests ---

/// A tabbed page with `count` pages.
fn tabbed_with_pages(count: usize) -> (Ref<TabbedPage>, Vec<Ref<Page>>) {
    let tp = TabbedPage::new();
    let pages: Vec<Ref<Page>> = (0..count).map(|_| page().upcast()).collect();
    tp.set_pages(Some(PageList::from_items(pages.iter().cloned())));
    (tp, pages)
}

#[test]
fn forward_skips_disabled() {
    let _scope = test_scope();
    let (tp, pages) = tabbed_with_pages(3);
    TabbedPage::set_is_tab_enabled(&pages[1], false);

    let result = tp.find_next_enabled_tab(1, 1);
    assert_eq!(2, result);
}

#[test]
fn backward_skips_disabled() {
    let _scope = test_scope();
    let (tp, pages) = tabbed_with_pages(3);
    TabbedPage::set_is_tab_enabled(&pages[1], false);

    let result = tp.find_next_enabled_tab(1, -1);
    assert_eq!(0, result);
}

#[test]
fn no_enabled_tab_ahead_returns_minus_one() {
    let _scope = test_scope();
    let (tp, pages) = tabbed_with_pages(2);
    TabbedPage::set_is_tab_enabled(&pages[1], false);

    let result = tp.find_next_enabled_tab(1, 1);
    assert_eq!(-1, result);
}

#[test]
fn all_enabled_returns_start_index() {
    let _scope = test_scope();
    let (tp, _pages) = tabbed_with_pages(3);

    let result = tp.find_next_enabled_tab(1, 1);
    assert_eq!(1, result);
}

#[test]
fn multiple_consecutive_disabled_skips_all() {
    let _scope = test_scope();
    let (tp, pages) = tabbed_with_pages(4);
    TabbedPage::set_is_tab_enabled(&pages[1], false);
    TabbedPage::set_is_tab_enabled(&pages[2], false);

    let result = tp.find_next_enabled_tab(1, 1);
    assert_eq!(3, result);
}

#[test]
fn all_disabled_returns_minus_one() {
    let _scope = test_scope();
    let (tp, pages) = tabbed_with_pages(3);
    TabbedPage::set_is_tab_enabled(&pages[0], false);
    TabbedPage::set_is_tab_enabled(&pages[1], false);
    TabbedPage::set_is_tab_enabled(&pages[2], false);

    assert_eq!(-1, tp.find_next_enabled_tab(0, 1));
    assert_eq!(-1, tp.find_next_enabled_tab(2, -1));
}

// --- KeyboardNavigationTests ---

#[test]
fn is_keyboard_navigation_enabled_default_is_true() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    assert!(tp.is_keyboard_navigation_enabled());
}

#[test]
fn is_keyboard_navigation_enabled_false_right_key_is_not_handled() {
    let _scope = test_scope();
    let (tp, _pages) = tabbed_with_pages(2);
    tp.set_is_keyboard_navigation_enabled(false);
    tp.set_selected_index(0);

    let handled = simulate_key_down_returns_handled(&tp, Key::Right);

    assert!(!handled);
}

#[test]
fn is_keyboard_navigation_enabled_false_ctrl_tab_is_not_handled() {
    let _scope = test_scope();
    let (tp, _pages) = tabbed_with_pages(2);
    tp.set_is_keyboard_navigation_enabled(false);
    tp.set_selected_index(0);

    let handled = simulate_key_down_with_modifiers_returns_handled(&tp, Key::Tab, KeyModifiers::CONTROL);

    assert!(!handled);
}

#[test]
fn is_keyboard_navigation_enabled_true_no_template_key_is_not_handled() {
    let _scope = test_scope();
    let (tp, _pages) = tabbed_with_pages(2);
    tp.set_is_keyboard_navigation_enabled(true);

    let handled = simulate_key_down_returns_handled(&tp, Key::Right);

    assert!(!handled);
}

// --- KeyboardNavigationWithTemplateTests ---

struct Tabbed {
    tp: Ref<TabbedPage>,
    /// The root owns the tree.
    _root: Ref<TestRoot>,
}

/// Builds a tabbed page with a real `PART_TabControl` wired up so that the
/// key handling can navigate.
fn make_tabbed(page_count: usize, selected_index: i32, placement: TabPlacement) -> Tabbed {
    let tp = TabbedPage::new();
    tp.set_tab_placement(placement);
    let pages = tp.pages().expect("the tabbed page has pages");
    for i in 0..page_count {
        pages.add(hp(&format!("Tab {i}")));
    }

    tp.set_template(Some(FuncControlTemplate::for_type::<TabbedPage>(|parent, scope| {
        let tab_control = TabControl::new();
        tab_control.set_name(Some("PART_TabControl".to_string()));
        tab_control.set_items_source(parent.pages_items_source());
        tab_control.register_in_name_scope(&**scope).upcast()
    })));

    let root = TestRoot::with_child(tp.clone());
    tp.apply_template();
    tp.set_selected_index(selected_index);
    Tabbed { tp, _root: root }
}

fn make_tabbed_top(page_count: usize, selected_index: i32) -> Tabbed {
    make_tabbed(page_count, selected_index, TabPlacement::Top)
}

fn page_at(tp: &TabbedPage, index: usize) -> Ref<Page> {
    tp.pages().expect("the tabbed page has pages").get(index)
}

#[test]
fn right_key_navigates_to_next_page() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 0);
    simulate_key_down(&t.tp, Key::Right);
    assert_eq!(1, t.tp.selected_index());
}

#[test]
fn left_key_navigates_to_previous_page() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 1);
    simulate_key_down(&t.tp, Key::Left);
    assert_eq!(0, t.tp.selected_index());
}

#[test]
fn down_key_with_vertical_placement_navigates_to_next_page() {
    let _scope = test_scope();
    let t = make_tabbed(3, 0, TabPlacement::Left);
    simulate_key_down(&t.tp, Key::Down);
    assert_eq!(1, t.tp.selected_index());
}

#[test]
fn up_key_with_vertical_placement_navigates_to_previous_page() {
    let _scope = test_scope();
    let t = make_tabbed(3, 1, TabPlacement::Left);
    simulate_key_down(&t.tp, Key::Up);
    assert_eq!(0, t.tp.selected_index());
}

#[test]
fn right_key_at_last_page_does_not_navigate() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 2);
    simulate_key_down(&t.tp, Key::Right);
    assert_eq!(2, t.tp.selected_index());
}

#[test]
fn left_key_at_first_page_does_not_navigate() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 0);
    simulate_key_down(&t.tp, Key::Left);
    assert_eq!(0, t.tp.selected_index());
}

#[test]
fn right_key_marks_event_handled() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 0);
    let handled = simulate_key_down_returns_handled(&t.tp, Key::Right);
    assert!(handled);
}

#[test]
fn right_key_at_last_page_does_not_mark_event_handled() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 2);
    let handled = simulate_key_down_returns_handled(&t.tp, Key::Right);
    assert!(!handled);
}

#[test]
fn ctrl_tab_navigates_to_next_page() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 0);
    let handled = simulate_key_down_with_modifiers_returns_handled(&t.tp, Key::Tab, KeyModifiers::CONTROL);
    assert_eq!(1, t.tp.selected_index());
    assert!(handled);
}

#[test]
fn ctrl_shift_tab_navigates_to_previous_page() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 1);
    let handled = simulate_key_down_with_modifiers_returns_handled(
        &t.tp,
        Key::Tab,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    );
    assert_eq!(0, t.tp.selected_index());
    assert!(handled);
}

#[test]
fn rtl_flow_direction_left_key_navigates_to_next_page() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 0);
    t.tp.set_flow_direction(FlowDirection::RightToLeft);
    simulate_key_down(&t.tp, Key::Left);
    assert_eq!(1, t.tp.selected_index());
}

#[test]
fn rtl_flow_direction_right_key_navigates_to_previous_page() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 1);
    t.tp.set_flow_direction(FlowDirection::RightToLeft);
    simulate_key_down(&t.tp, Key::Right);
    assert_eq!(0, t.tp.selected_index());
}

#[test]
fn right_key_skips_disabled_tab() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 0);
    TabbedPage::set_is_tab_enabled(&page_at(&t.tp, 1), false);
    simulate_key_down(&t.tp, Key::Right);
    assert_eq!(2, t.tp.selected_index());
}

#[test]
fn right_key_all_tabs_ahead_disabled_does_not_navigate() {
    let _scope = test_scope();
    let t = make_tabbed_top(3, 0);
    TabbedPage::set_is_tab_enabled(&page_at(&t.tp, 1), false);
    TabbedPage::set_is_tab_enabled(&page_at(&t.tp, 2), false);
    simulate_key_down(&t.tp, Key::Right);
    assert_eq!(0, t.tp.selected_index());
}

// --- SelectingMultiPageTests ---

#[test]
fn selected_index_direct_property_raises_changed_event() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let raised = Rc::new(Cell::new(false));
    let r = raised.clone();
    let object: &FerroObject = &tp;
    let _subscription = FerroObjectExtensions::get_observable(object, SelectingMultiPage::selected_index_property())
        .subscribe_fn(move |_| r.set(true));
    call_commit_selection(&tp, 0, Some(&page().upcast()));
    assert!(raised.get());
}

#[test]
fn selected_page_direct_property_raises_changed_event() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let raised = Rc::new(Cell::new(false));
    let r = raised.clone();
    let object: &FerroObject = &tp;
    let _subscription = FerroObjectExtensions::get_observable(object, SelectingMultiPage::selected_page_property())
        .subscribe_fn(move |_| r.set(true));
    call_commit_selection(&tp, 0, Some(&page().upcast()));
    assert!(raised.get());
}

// --- PageIconTemplateTests ---

#[test]
fn page_icon_accepts_control_value() {
    let _scope = test_scope();
    let icon = PathIcon::new();
    icon.set_data(EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0)));
    let page = page();
    page.set_icon(Some(Control::boxed(icon.clone())));
    let held = page.icon().as_ref().and_then(Control::from_boxed);
    assert!(same(&icon, &held));
}

#[test]
fn page_icon_accepts_non_control_value() {
    let _scope = test_scope();
    let geometry: BoxedValue = Rc::new(EllipseGeometry::with_rect(Rect::new(0.0, 0.0, 10.0, 10.0)));
    let page = page();
    page.set_icon(Some(geometry.clone()));
    assert!(page.icon().is_some_and(|icon| Rc::ptr_eq(&icon, &geometry)));
}

#[test]
fn page_icon_template_round_trips() {
    let _scope = test_scope();
    let template = border_template();
    let page = page();
    page.set_icon_template(Some(template.clone()));
    assert!(template_is(&page.icon_template(), &template));
}

// DRAWER-SEAM: `DrawerPage_DrawerIconTemplate_RoundTrips` and
// `DrawerPage_DrawerIcon_With_Geometry_Does_Not_Throw` test the drawer page and belong to its port.
