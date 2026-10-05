//! The tabbed page tests, continued: the data template, swipe gesture,
//! system back button and visual tree lifecycle groups of the reference
//! tests.

use super::tabbed_page_tests::{header_of, hp, is_logical_child, logical_page_headers, pages_of};
use super::{ContentPage, NavigationType, Page, PageList, TabPlacement, TabbedPage};
use crate::mouse_test_helper::MouseTestHelper;
use crate::presenters::ItemsPresenter;
use crate::templates::{
    FuncControlTemplate, FuncDataTemplate, FuncTemplateNameScopeExtensions, IControlTemplate, IDataTemplate,
};
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::{ItemsSource, TabControl};
use ferroui_base::collections::FerroList;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::{MouseButton, SwipeGestureEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, Point, Ref, Size, StaticType, Vector};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

// --- DataTemplateTests ---

#[derive(Clone, Debug, PartialEq)]
struct DataItem(String);

fn data_item(name: &str) -> DataItem {
    DataItem(name.to_string())
}

/// `new ObservableCollection<DataItem> { .. }`.
fn observable_items<const N: usize>(names: [&str; N]) -> Rc<FerroList<DataItem>> {
    Rc::new(FerroList::from_items(names.into_iter().map(data_item)))
}

/// `new FuncDataTemplate<DataItem>((item, _) => new ContentPage { Header = prefix + item.Name }, false)`.
fn page_template(prefix: &'static str) -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::for_type::<DataItem>(
        move |item, _| {
            let page = ContentPage::new();
            page.set_header(boxed_str(&format!("{prefix}{}", item.0)));
            Some(page.upcast())
        },
        false,
    )
}

fn create_tabbed_page_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<TabbedPage>(|_parent, scope| {
        let tc = TabControl::new();
        tc.set_name(Some("PART_TabControl".to_string()));
        tc.set_template(Some(FuncControlTemplate::for_type::<TabControl>(|_, tc_scope| {
            let presenter = ItemsPresenter::new();
            presenter.set_name(Some("PART_ItemsPresenter".to_string()));
            presenter.register_in_name_scope(&**tc_scope).upcast()
        })));
        tc.register_in_name_scope(&**scope).upcast()
    })
}

struct Hosted {
    tp: Ref<TabbedPage>,
    root: Ref<TestRoot>,
}

impl Hosted {
    fn layout(&self) {
        self.root.layout_manager().execute_layout_pass();
        run_jobs();
    }
}

/// A tabbed page showing the items through the page template, hosted in a
/// root and laid out.
fn create_hosted(items: ItemsSource, page_template: Rc<dyn IDataTemplate>) -> Hosted {
    let tp = TabbedPage::new();
    tp.set_width(400.0);
    tp.set_height(300.0);
    tp.set_items_source(Some(items));
    tp.set_page_template(Some(page_template));
    tp.set_template(Some(create_tabbed_page_template()));

    let root = TestRoot::new();
    root.set_client_size(Size::new(400.0, 300.0));
    root.set_child(tp.clone());
    root.execute_initial_layout_pass();
    run_jobs();
    Hosted { tp, root }
}

#[test]
fn custom_page_template_build_creates_content_page_with_correct_header() {
    let _scope = test_scope();
    let template: Rc<dyn IDataTemplate> = FuncDataTemplate::for_type::<DataItem>(
        |item, _| {
            let page = ContentPage::new();
            page.set_header(boxed_str(&item.0));
            Some(page.upcast())
        },
        true,
    );
    let tp = TabbedPage::new();
    tp.set_page_template(Some(template));

    let item: BoxedValue = Rc::new(data_item("Electronics"));
    let built = tp
        .page_template()
        .expect("the page template")
        .build(&Some(item))
        .and_then(|control| control.cast::<ContentPage>());

    let built = built.expect("a content page");
    assert_eq!(header_of(&built).as_deref(), Some("Electronics"));
}

#[test]
fn default_page_data_template_is_non_null() {
    let _scope = test_scope();
    assert!(TabbedPage::new().page_template().is_some());
}

#[test]
fn default_page_data_template_wraps_non_page_item_in_content_page() {
    let _scope = test_scope();
    // Data items must be wrapped in a content page, not shown as raw headers.
    let tp = TabbedPage::new();
    let item: BoxedValue = Rc::new(data_item("Test"));
    let built = tp.page_template().expect("the page template").build(&Some(item));
    assert!(built.is_some_and(|built| std::ptr::eq(built.get_type(), <ContentPage as StaticType>::TYPE)));
}

#[test]
fn view_model_items_with_page_template_build_containers_as_content_pages() {
    let _scope = test_scope();
    let items = observable_items(["Electronics", "Books"]);

    let hosted = create_hosted(items.into(), page_template(""));

    let logicals = logical_page_headers(&hosted.tp);
    assert!(logicals.iter().any(|header| header == "Electronics"));
    assert!(logicals.iter().any(|header| header == "Books"));
}

#[test]
fn items_source_selected_page_is_not_null_after_containers_realized() {
    let _scope = test_scope();
    let items = observable_items(["First", "Second"]);

    let hosted = create_hosted(items.into(), page_template(""));
    let tp = &hosted.tp;

    let selected_page = tp.selected_page().expect("a selected page");
    assert_eq!(0, tp.selected_index());
    assert!(std::ptr::eq(selected_page.get_type(), <ContentPage as StaticType>::TYPE));
    assert_eq!(header_of(&selected_page).as_deref(), Some("First"));
}

#[test]
fn non_list_items_source_selected_page_and_automation_name_are_resolved() {
    let _scope = test_scope();
    // A source that only enumerates is copied into a list when it is assigned.
    let hosted =
        create_hosted(ItemsSource::from_values([data_item("First"), data_item("Second")]), page_template(""));
    let tp = &hosted.tp;

    let selected_page = tp.selected_page().expect("a selected page");
    assert_eq!(header_of(&selected_page).as_deref(), Some("First"));
    // AUTOMATION-SEAM: the name of the automation peer of the tabbed page is "Tab 1 of 2: First".

    tp.set_selected_index(1);
    hosted.layout();

    let selected_page = tp.selected_page().expect("a selected page");
    assert_eq!(header_of(&selected_page).as_deref(), Some("Second"));
    // AUTOMATION-SEAM: the name of the automation peer of the tabbed page is "Tab 2 of 2: Second".
}

#[test]
fn items_source_selection_changed_reports_correct_page() {
    let _scope = test_scope();
    let items = observable_items(["Alpha", "Beta"]);

    let hosted = create_hosted(items.into(), page_template(""));
    let tp = &hosted.tp;

    let reported_page: Rc<RefCell<Option<Ref<Page>>>> = Rc::default();
    let r = reported_page.clone();
    tp.selection_changed(move |_, e| *r.borrow_mut() = e.current_page());

    tp.set_selected_index(1);
    hosted.layout();

    let selected_page = tp.selected_page().expect("a selected page");
    assert_eq!(1, tp.selected_index());
    assert_eq!(header_of(&selected_page).as_deref(), Some("Beta"));
    let reported_page = reported_page.borrow().clone().expect("a reported page");
    assert_eq!(header_of(&reported_page).as_deref(), Some("Beta"));
}

#[test]
fn view_model_items_removed_from_collection_template_created_page_removed_from_logical_children() {
    let _scope = test_scope();
    let items = observable_items(["A", "B"]);

    let hosted = create_hosted(items.clone().into(), page_template(""));

    items.remove_at(1);
    hosted.layout();

    let logicals = logical_page_headers(&hosted.tp);
    assert!(!logicals.iter().any(|header| header == "B"));
}

#[test]
fn items_source_replaced_no_phantom_logical_children() {
    let _scope = test_scope();
    let first = observable_items(["One", "Two"]);
    let second = observable_items(["Three", "Four"]);

    let hosted = create_hosted(first.into(), page_template(""));

    hosted.tp.set_items_source(Some(second.into()));
    hosted.layout();

    let logicals = logical_page_headers(&hosted.tp);
    assert!(!logicals.iter().any(|header| header == "One"));
    assert!(!logicals.iter().any(|header| header == "Two"));
    assert!(logicals.iter().any(|header| header == "Three"));
    assert!(logicals.iter().any(|header| header == "Four"));
}

#[test]
fn page_template_changed_after_containers_realized_rebuilds_existing_containers() {
    let _scope = test_scope();
    let items = observable_items(["X", "Y"]);

    let hosted = create_hosted(items.into(), page_template("old-"));

    hosted.tp.set_page_template(Some(page_template("new-")));
    hosted.layout();

    let logicals = logical_page_headers(&hosted.tp);
    assert!(!logicals.iter().any(|header| header.starts_with("old-")));
    assert!(logicals.iter().any(|header| header == "new-X"));
    assert!(logicals.iter().any(|header| header == "new-Y"));
}

#[test]
fn page_template_changed_after_containers_realized_updates_selected_page() {
    let _scope = test_scope();
    let items = observable_items(["A", "B"]);

    let hosted = create_hosted(items.into(), page_template("old-"));

    hosted.tp.set_page_template(Some(page_template("new-")));
    hosted.layout();

    let selected_page = hosted.tp.selected_page().expect("a selected page");
    assert_eq!(header_of(&selected_page).as_deref(), Some("new-A"));
}

#[test]
fn reapplying_template_does_not_leave_phantom_logical_children() {
    let _scope = test_scope();
    let items = observable_items(["A", "B"]);

    let hosted = create_hosted(items.into(), page_template(""));
    let tp = &hosted.tp;

    let first_selected_page = tp.selected_page().expect("a selected page");

    tp.set_template(Some(create_tabbed_page_template()));
    hosted.layout();

    assert_eq!(2, tp.logical_children().count());
    assert!(!is_logical_child(tp, &first_selected_page));
}

#[test]
fn page_template_set_to_null_after_containers_realized_clears_generated_pages() {
    let _scope = test_scope();
    let items = observable_items(["A", "B"]);

    let hosted = create_hosted(items.into(), page_template(""));
    let tp = &hosted.tp;

    let original_selected_page = tp.selected_page().expect("a selected page");

    tp.set_page_template(None);
    hosted.layout();

    assert!(tp.selected_page().is_none());
    assert!(tp.current_page().is_none());
    assert!(tp.logical_children().is_empty());
    assert!(!is_logical_child(tp, &original_selected_page));
}

// --- SwipeGestureTests ---

fn create_swipe_ready_tabbed_page() -> Hosted {
    let tp = TabbedPage::new();
    tp.set_is_gesture_enabled(true);
    tp.set_width(400.0);
    tp.set_height(300.0);
    tp.set_tab_placement(TabPlacement::Top);
    tp.set_selected_index(0);
    tp.set_pages(Some(pages_of([&hp("A"), &hp("B"), &hp("C")])));
    tp.set_template(Some(FuncControlTemplate::for_type::<TabbedPage>(|parent, scope| {
        let tab_control = TabControl::new();
        tab_control.set_name(Some("PART_TabControl".to_string()));
        tab_control.set_items_source(parent.pages_items_source());
        tab_control.register_in_name_scope(&**scope).upcast()
    })));
    tp.gesture_recognizers()
        .to_vec()
        .into_iter()
        .find_map(|recognizer| recognizer.cast::<SwipeGestureRecognizer>())
        .expect("a swipe gesture recognizer")
        .set_is_mouse_enabled(true);

    let root = TestRoot::new();
    root.set_client_size(Size::new(400.0, 300.0));
    root.set_child(tp.clone());
    tp.apply_template();
    run_jobs();

    Hosted { tp, root }
}

#[test]
fn same_gesture_id_only_advances_one_tab() {
    let _scope = test_scope();
    let hosted = create_swipe_ready_tabbed_page();
    let tp = &hosted.tp;

    let first_swipe = SwipeGestureEventArgs::new(7, Vector::new(20.0, 0.0), Vector::default());
    let repeated_swipe = SwipeGestureEventArgs::new(7, Vector::new(20.0, 0.0), Vector::default());

    tp.raise_event(&first_swipe);
    tp.raise_event(&repeated_swipe);
    run_jobs();

    assert!(first_swipe.handled());
    assert!(!repeated_swipe.handled());
    assert_eq!(1, tp.selected_index());
}

#[test]
fn new_gesture_id_can_advance_again() {
    let _scope = test_scope();
    let hosted = create_swipe_ready_tabbed_page();
    let tp = &hosted.tp;

    tp.raise_event(&SwipeGestureEventArgs::new(7, Vector::new(20.0, 0.0), Vector::default()));
    tp.raise_event(&SwipeGestureEventArgs::new(8, Vector::new(20.0, 0.0), Vector::default()));
    run_jobs();

    assert_eq!(2, tp.selected_index());
}

#[test]
fn mouse_swipe_advances_tab() {
    let _scope = test_scope();
    let hosted = create_swipe_ready_tabbed_page();
    let tp = &hosted.tp;
    let mouse = MouseTestHelper::new();

    mouse.down_at(tp, MouseButton::Left, Point::new(200.0, 100.0), 1);
    mouse.move_(tp, Point::new(160.0, 100.0));
    mouse.up_at(tp, MouseButton::Left, Point::new(160.0, 100.0));
    run_jobs();

    assert_eq!(1, tp.selected_index());
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
    let tp = TabbedPage::new();
    let page1 = hp("Page1");
    let page2 = hp("Page2");
    let page3 = hp("Page3");
    let pages = pages_of([&page1, &page2, &page3]);
    tp.set_pages(Some(pages));

    let raised: [Rc<Cell<bool>>; 3] = Default::default();
    for (page, raised) in [&page1, &page2, &page3].into_iter().zip(&raised) {
        let raised = raised.clone();
        page.page_navigation_system_back_button_pressed(move |_, _| raised.set(true));
    }
    let _root = TestRoot::with_child(tp.clone());
    tp.commit_selection(1, Some(page2.clone()), NavigationType::Replace);

    let args = raise_back_button(&tp);

    assert!(!raised[0].get());
    assert!(raised[1].get());
    assert!(!raised[2].get());
    assert!(!args.handled());
}

// --- VisualTreeLifecycleTests ---

#[test]
fn detach_and_reattach_collection_changed_still_fires_pages_changed() {
    let _scope = test_scope();
    let pages = PageList::new();
    let tp = TabbedPage::new();
    tp.set_pages(Some(pages.clone()));
    let root = TestRoot::with_child(tp.clone());

    root.set_child(None);
    root.set_child(tp.clone());

    let fire_count = Rc::new(Cell::new(0));
    let f = fire_count.clone();
    tp.pages_changed(move |_| f.set(f.get() + 1));

    pages.add(hp("A"));
    pages.add(hp("B"));

    assert_eq!(2, fire_count.get());
}

// The tests below are not in the reference tests: the subscription of a multi-page to its pages
// collection ends with the page.

#[test]
fn dropped_page_leaves_no_subscription_on_its_pages() {
    let _scope = test_scope();
    let pages = PageList::new();
    assert!(!pages.has_collection_changed_subscribers());

    let tp = TabbedPage::new();
    tp.set_pages(Some(pages.clone()));
    assert!(pages.has_collection_changed_subscribers());

    drop(tp);

    assert!(!pages.has_collection_changed_subscribers());
    pages.add(hp("A"));
}

#[test]
fn dropped_page_keeps_subscription_of_other_page_sharing_its_pages() {
    let _scope = test_scope();
    let pages = PageList::new();
    let first = TabbedPage::new();
    let second = TabbedPage::new();
    first.set_pages(Some(pages.clone()));
    second.set_pages(Some(pages.clone()));

    let fire_count = Rc::new(Cell::new(0));
    let f = fire_count.clone();
    second.pages_changed(move |_| f.set(f.get() + 1));

    drop(first);

    assert!(pages.has_collection_changed_subscribers());
    pages.add(hp("A"));
    assert_eq!(1, fire_count.get());

    drop(second);

    assert!(!pages.has_collection_changed_subscribers());
}
