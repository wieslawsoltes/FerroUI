use super::{
    CarouselPage, ContentPage, NavigatedFromEventArgs, NavigatedToEventArgs, NavigationType, Page, PageList,
    PageNavigationHost,
};
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::testing::{create_page_test_theme, TestServices, UnitTestApplication};
use crate::{Control, TopLevel, Window};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{Ref, Thickness};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

fn page_with_header(header: &str) -> Ref<ContentPage> {
    let page = ContentPage::new();
    page.set_header(boxed_str(header));
    page
}

fn presenter_child_is(host: &PageNavigationHost, page: &ContentPage) -> bool {
    let page: &Control = page;
    host.presenter()
        .expect("the host has a presenter")
        .child()
        .is_some_and(|child| std::ptr::eq::<Control>(&*child, page))
}

// --- LifecycleEventTests ---

#[test]
fn page_set_fires_navigated_to() {
    let _scope = test_scope();
    let page = page_with_header("Home");
    let fired = Rc::new(RefCell::new(false));
    let f = fired.clone();
    page.navigated_to(move |_| *f.borrow_mut() = true);

    let host = PageNavigationHost::new();
    let _root = TestRoot::with_child(host.clone());
    host.set_page(&page);

    assert!(*fired.borrow());
}

#[test]
fn page_set_navigated_to_navigation_type_is_replace() {
    let _scope = test_scope();
    let page = page_with_header("Home");
    let args: Rc<RefCell<Option<NavigatedToEventArgs>>> = Rc::default();
    let a = args.clone();
    page.navigated_to(move |e| *a.borrow_mut() = Some(e.clone()));

    let host = PageNavigationHost::new();
    let _root = TestRoot::with_child(host.clone());
    host.set_page(&page);

    let args = args.borrow().clone().expect("navigated to");
    assert_eq!(NavigationType::Replace, args.navigation_type());
}

#[test]
fn page_set_navigated_to_previous_page_is_null() {
    let _scope = test_scope();
    let page = page_with_header("Home");
    let args: Rc<RefCell<Option<NavigatedToEventArgs>>> = Rc::default();
    let a = args.clone();
    page.navigated_to(move |e| *a.borrow_mut() = Some(e.clone()));

    let host = PageNavigationHost::new();
    let _root = TestRoot::with_child(host.clone());
    host.set_page(&page);

    let args = args.borrow().clone().expect("navigated to");
    assert!(args.previous_page().is_none());
}

#[test]
fn page_changed_fires_navigated_from_on_old_page() {
    let _scope = test_scope();
    let first = page_with_header("First");
    let second = page_with_header("Second");
    let host = PageNavigationHost::new();
    let _root = TestRoot::with_child(host.clone());
    host.set_page(&first);

    let args: Rc<RefCell<Option<NavigatedFromEventArgs>>> = Rc::default();
    let a = args.clone();
    first.navigated_from(move |e| *a.borrow_mut() = Some(e.clone()));

    host.set_page(&second);

    let args = args.borrow().clone().expect("navigated from");
    assert_eq!(NavigationType::Replace, args.navigation_type());
    assert!(args.destination_page().is_some_and(|page| page.ptr_eq(&second)));
}

#[test]
fn page_changed_fires_navigated_to_on_new_page() {
    let _scope = test_scope();
    let first = page_with_header("First");
    let second = page_with_header("Second");
    let host = PageNavigationHost::new();
    let _root = TestRoot::with_child(host.clone());
    host.set_page(&first);

    let args: Rc<RefCell<Option<NavigatedToEventArgs>>> = Rc::default();
    let a = args.clone();
    second.navigated_to(move |e| *a.borrow_mut() = Some(e.clone()));

    host.set_page(&second);

    let args = args.borrow().clone().expect("navigated to");
    assert_eq!(NavigationType::Replace, args.navigation_type());
    assert!(args.previous_page().is_some_and(|page| page.ptr_eq(&first)));
}

#[test]
fn page_set_to_null_fires_navigated_from_on_old_page() {
    let _scope = test_scope();
    let page = page_with_header("Home");
    let host = PageNavigationHost::new();
    let _root = TestRoot::with_child(host.clone());
    host.set_page(&page);

    let args: Rc<RefCell<Option<NavigatedFromEventArgs>>> = Rc::default();
    let a = args.clone();
    page.navigated_from(move |e| *a.borrow_mut() = Some(e.clone()));

    host.set_page(None);

    let args = args.borrow().clone().expect("navigated from");
    assert!(args.destination_page().is_none());
    assert_eq!(NavigationType::Replace, args.navigation_type());
}

#[test]
fn page_changed_fires_lifecycle_events_in_order() {
    let _scope = test_scope();
    let first = page_with_header("First");
    let second = page_with_header("Second");
    let host = PageNavigationHost::new();
    let _root = TestRoot::with_child(host.clone());
    host.set_page(&first);

    let order: Rc<RefCell<Vec<&'static str>>> = Rc::default();
    let o = order.clone();
    first.navigated_from(move |_| o.borrow_mut().push("NavigatedFrom"));
    let o = order.clone();
    second.navigated_to(move |_| o.borrow_mut().push("NavigatedTo"));

    host.set_page(&second);

    assert_eq!(*order.borrow(), ["NavigatedFrom", "NavigatedTo"]);
}

#[test]
fn initial_layout_with_existing_page_does_not_throw_when_content_presenter_child_is_assigned() {
    let _scope = test_scope();
    let page = page_with_header("Home");
    let host = PageNavigationHost::new();
    host.set_page(&page);
    let root = TestRoot::with_child(host.clone());

    let exception = catch_unwind(AssertUnwindSafe(|| root.execute_initial_layout_pass()));

    assert!(exception.is_ok());
    assert!(host.presenter().is_some());
    assert!(presenter_child_is(&host, &page));
}

#[test]
fn replacing_page_resets_old_presenter_child_safe_area_padding() {
    let _scope = test_scope();
    let first = page_with_header("First");
    let second = page_with_header("Second");
    let host = PageNavigationHost::new();
    host.set_page(&first);
    let root = TestRoot::with_child(host.clone());
    root.execute_initial_layout_pass();
    first.set_safe_area_padding(Thickness::new(1.0, 2.0, 3.0, 4.0));

    let exception = catch_unwind(AssertUnwindSafe(|| host.set_page(&second)));

    assert!(exception.is_ok());
    assert_eq!(Thickness::default(), first.safe_area_padding());
    assert!(host.presenter().is_some());
    assert!(presenter_child_is(&host, &second));
}

// --- SystemBackButtonTests ---

#[test]
fn back_requested_forwards_to_nested_current_page_once() {
    let _app = UnitTestApplication::start(TestServices::styled_window().with_theme(create_page_test_theme));
    let child = page_with_header("Child");
    let parent = CarouselPage::new();
    parent.set_pages(Some(PageList::from_items([child.clone().upcast::<Page>()])));
    let host = PageNavigationHost::new();
    host.set_page(&parent);
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(host.clone())));
    let raise_count = Rc::new(Cell::new(0));
    let r = raise_count.clone();
    child.page_navigation_system_back_button_pressed(move |_, e| {
        r.set(r.get() + 1);
        e.set_handled(true);
    });

    window.show();

    assert!(parent.current_page().is_some_and(|page| {
        let child: &Page = &child;
        std::ptr::eq::<Page>(&*page, child)
    }));
    let parent_control: &Control = &parent;
    assert!(host
        .presenter()
        .and_then(|presenter| presenter.child())
        .is_some_and(|presenter_child| std::ptr::eq::<Control>(&*presenter_child, parent_control)));

    let args = RoutedEventArgs::with_event(TopLevel::back_requested_event());
    window.raise_event(&args);

    assert_eq!(1, raise_count.get());
    assert!(args.handled());
}
