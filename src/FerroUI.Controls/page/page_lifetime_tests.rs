//! Tests of the lifetime of the pages a navigation page or a tabbed page
//! hosted: a page that left its host is freed once the caller lets go of it.
//!
//! Not ports: the reference relies on the garbage collector and has no tests
//! of this. Each test covers one way a page leaves its host, so that a
//! reference a host (or a part of its template) keeps to a page that left
//! shows as the failure of the test of that way. The pages have no control
//! theme here: what a theme adds to a page is not under test.

use super::navigation_page_tests::{create_navigation_page, page_h, wait, ControllableTransition, Gate};
use super::{ContentPage, NavigationPage, Page, TabbedPage};
use crate::presenters::ItemsPresenter;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Control, TabControl};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Ref, Size, WeakRef};
use std::rc::Rc;

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// A content page with a header and a control as its content.
fn page_with_content(header: &str) -> Ref<ContentPage> {
    let page = page_h(header);
    page.set_content(Some(Control::boxed(Border::new())));
    page
}

/// Lets go of the page and returns what remains of it.
fn release(page: Ref<ContentPage>) -> WeakRef<ContentPage> {
    let weak = page.downgrade();
    drop(page);
    weak
}

/// Lays the root out and runs the jobs of the dispatcher (the loaded events
/// among them).
fn layout(root: &TestRoot) {
    root.layout_manager().execute_layout_pass();
    run_jobs();
}

fn is_freed(page: &WeakRef<ContentPage>) -> bool {
    page.upgrade().is_none()
}

// --- a page alone ---

#[test]
fn page_that_was_never_hosted_is_freed() {
    let _scope = test_scope();
    let page = release(page_with_content("Alone"));

    assert!(is_freed(&page));
}

#[test]
fn page_removed_from_its_root_is_freed() {
    let _scope = test_scope();
    let page = page_with_content("Shown");
    let root = TestRoot::with_child(page.clone());
    root.execute_initial_layout_pass();
    run_jobs();

    root.set_child(None);
    layout(&root);
    let page = release(page);

    assert!(is_freed(&page));
}

// --- NavigationPage ---

#[test]
fn navigation_page_popped_page_is_freed() {
    let _scope = test_scope();
    let (nav, root) = create_navigation_page(None);
    wait(nav.push_async(page_h("Root")));
    let top = page_with_content("Top");
    wait(nav.push_async(&top));
    layout(&root);

    let popped = wait(nav.pop_async());
    drop(popped);
    layout(&root);
    let top = release(top);

    assert_eq!(1, nav.stack_depth());
    assert!(is_freed(&top));
}

#[test]
fn navigation_page_popped_page_that_was_never_laid_out_is_freed() {
    let _scope = test_scope();
    let (nav, _root) = create_navigation_page(None);
    wait(nav.push_async(page_h("Root")));
    let top = page_with_content("Top");
    wait(nav.push_async(&top));

    drop(wait(nav.pop_async()));
    let top = release(top);

    assert!(is_freed(&top));
}

#[test]
fn navigation_page_pages_popped_to_the_root_are_freed() {
    let _scope = test_scope();
    let (nav, root) = create_navigation_page(None);
    wait(nav.push_async(page_h("Root")));
    let mut pushed = Vec::new();
    for index in 2..=6 {
        let page = page_with_content(&format!("Page {index}"));
        wait(nav.push_async(&page));
        pushed.push(page);
    }
    layout(&root);

    wait(nav.pop_to_root_async());
    layout(&root);
    let pushed: Vec<WeakRef<ContentPage>> = pushed.into_iter().map(release).collect();

    assert_eq!(1, nav.stack_depth());
    let alive = pushed.iter().filter(|page| !is_freed(page)).count();
    assert_eq!(0, alive);
}

#[test]
fn navigation_page_pages_popped_to_a_page_are_freed() {
    let _scope = test_scope();
    let (nav, root) = create_navigation_page(None);
    wait(nav.push_async(page_h("Root")));
    let target = page_h("Target");
    wait(nav.push_async(&target));
    let above = page_with_content("Above");
    wait(nav.push_async(&above));
    let top = page_with_content("Top");
    wait(nav.push_async(&top));
    layout(&root);

    wait(nav.pop_to_page_async(&target));
    layout(&root);
    let above = release(above);
    let top = release(top);

    assert_eq!(2, nav.stack_depth());
    assert!(is_freed(&above));
    assert!(is_freed(&top));
}

#[test]
fn navigation_page_page_popped_with_a_transition_is_freed() {
    let _scope = test_scope();
    let gate = Gate::new();
    let (nav, root) = create_navigation_page(None);
    wait(nav.push_async(page_h("Root")));
    let top = page_with_content("Top");
    wait(nav.push_async(&top));
    layout(&root);

    nav.set_page_transition(Some(ControllableTransition::new(&gate)));
    let pop_task = nav.pop_async();
    gate.set_result();
    drop(wait(pop_task));
    layout(&root);
    let top = release(top);

    assert!(is_freed(&top));
}

#[test]
fn navigation_page_removed_page_is_freed() {
    let _scope = test_scope();
    let (nav, root) = create_navigation_page(None);
    wait(nav.push_async(page_h("Root")));
    let middle = page_with_content("Middle");
    wait(nav.push_async(&middle));
    wait(nav.push_async(page_h("Top")));
    layout(&root);

    nav.remove_page(&middle);
    layout(&root);
    let middle = release(middle);

    assert_eq!(2, nav.stack_depth());
    assert!(is_freed(&middle));
}

#[test]
fn navigation_page_replaced_page_is_freed() {
    let _scope = test_scope();
    let (nav, root) = create_navigation_page(None);
    wait(nav.push_async(page_h("Root")));
    let replaced = page_with_content("Replaced");
    wait(nav.push_async(&replaced));
    layout(&root);

    wait(nav.replace_async(page_h("Replacement")));
    layout(&root);
    let replaced = release(replaced);

    assert_eq!(2, nav.stack_depth());
    assert!(is_freed(&replaced));
}

#[test]
fn navigation_page_popped_modal_page_is_freed() {
    let _scope = test_scope();
    let (nav, root) = create_navigation_page(None);
    wait(nav.push_async(page_h("Root")));
    let modal = page_with_content("Modal");
    wait(nav.push_modal_async(&modal));
    layout(&root);

    drop(wait(nav.pop_modal_async()));
    layout(&root);
    let modal = release(modal);

    assert!(is_freed(&modal));
}

#[test]
fn navigation_page_page_popped_after_a_reattachment_is_freed() {
    let _scope = test_scope();
    let (nav, root) = create_navigation_page(None);
    wait(nav.push_async(page_h("Root")));
    let top = page_with_content("Top");
    wait(nav.push_async(&top));
    layout(&root);

    // The navigation page leaves the tree and returns: the pages lose and regain their
    // navigation service.
    root.set_child(None);
    root.set_child(nav.clone());
    layout(&root);
    drop(wait(nav.pop_async()));
    layout(&root);
    let top = release(top);

    assert!(is_freed(&top));
}

#[test]
fn navigation_page_frees_its_pages_with_itself() {
    let _scope = test_scope();
    let (nav, root) = create_navigation_page(None);
    let first = page_with_content("Root");
    let second = page_with_content("Top");
    wait(nav.push_async(&first));
    wait(nav.push_async(&second));
    layout(&root);

    root.set_child(None);
    layout(&root);
    let weak_nav: WeakRef<NavigationPage> = nav.downgrade();
    drop(nav);
    let first = release(first);
    let second = release(second);

    assert!(weak_nav.upgrade().is_none());
    assert!(is_freed(&first));
    assert!(is_freed(&second));
}

// --- TabbedPage ---

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

/// A templated tabbed page with the pages as its tabs, hosted in a root and
/// laid out: the tab control has made a container for each page.
fn create_hosted_tabbed_page(pages: &[Ref<ContentPage>]) -> (Ref<TabbedPage>, Ref<TestRoot>) {
    let tp = TabbedPage::new();
    tp.set_width(400.0);
    tp.set_height(300.0);
    tp.set_template(Some(create_tabbed_page_template()));

    let root = TestRoot::new();
    root.set_client_size(Size::new(400.0, 300.0));
    root.set_child(tp.clone());
    root.execute_initial_layout_pass();
    run_jobs();

    let list = tp.pages().expect("the pages of the tabbed page");
    for page in pages {
        list.add(page.clone().upcast::<Page>());
    }
    layout(&root);
    (tp, root)
}

#[test]
fn tabbed_page_removed_tab_that_was_not_selected_is_freed() {
    let _scope = test_scope();
    let first = page_with_content("A");
    let last = page_with_content("B");
    let (tp, root) = create_hosted_tabbed_page(&[first.clone(), last.clone()]);
    let pages = tp.pages().expect("the pages of the tabbed page");

    pages.remove_at(1);
    layout(&root);
    let last = release(last);

    assert_eq!(1, pages.count());
    assert!(is_freed(&last));
}

#[test]
fn tabbed_page_removed_tab_that_was_selected_is_freed() {
    let _scope = test_scope();
    let first = page_with_content("A");
    let last = page_with_content("B");
    let (tp, root) = create_hosted_tabbed_page(&[first.clone(), last.clone()]);
    let pages = tp.pages().expect("the pages of the tabbed page");
    assert!(tp.selected_page().is_some_and(|selected| selected.ptr_eq(&first)), "the first tab is selected");

    pages.remove_at(0);
    layout(&root);
    let first = release(first);

    assert_eq!(1, pages.count());
    assert!(is_freed(&first));
}

#[test]
fn tabbed_page_tab_removed_after_it_was_deselected_is_freed() {
    let _scope = test_scope();
    let first = page_with_content("A");
    let last = page_with_content("B");
    let (tp, root) = create_hosted_tabbed_page(&[first.clone(), last.clone()]);
    let pages = tp.pages().expect("the pages of the tabbed page");

    tp.set_selected_index(1);
    layout(&root);
    tp.set_selected_index(0);
    layout(&root);
    pages.remove_at(1);
    layout(&root);
    let last = release(last);

    assert!(is_freed(&last));
}

#[test]
fn tabbed_page_tabs_removed_from_the_last_to_the_first_are_freed() {
    let _scope = test_scope();
    let tabs: Vec<Ref<ContentPage>> = (1..=5).map(|index| page_with_content(&format!("T{index}"))).collect();
    let (tp, root) = create_hosted_tabbed_page(&tabs);
    let pages = tp.pages().expect("the pages of the tabbed page");

    while pages.count() > 0 {
        pages.remove_at(pages.count() - 1);
    }
    layout(&root);
    let tabs: Vec<WeakRef<ContentPage>> = tabs.into_iter().map(release).collect();

    let alive = tabs.iter().filter(|page| !is_freed(page)).count();
    assert_eq!(0, alive);
}

#[test]
fn tabbed_page_cleared_tabs_are_freed() {
    let _scope = test_scope();
    let tabs: Vec<Ref<ContentPage>> = (1..=3).map(|index| page_with_content(&format!("T{index}"))).collect();
    let (tp, root) = create_hosted_tabbed_page(&tabs);

    tp.pages().expect("the pages of the tabbed page").clear();
    layout(&root);
    let tabs: Vec<WeakRef<ContentPage>> = tabs.into_iter().map(release).collect();

    let alive = tabs.iter().filter(|page| !is_freed(page)).count();
    assert_eq!(0, alive);
}

#[test]
fn tabbed_page_without_a_template_frees_a_removed_tab() {
    let _scope = test_scope();
    let tp = TabbedPage::new();
    let _root = TestRoot::with_child(tp.clone());
    let pages = tp.pages().expect("the pages of the tabbed page");
    let tab = page_with_content("A");
    pages.add(tab.clone().upcast::<Page>());

    pages.remove_at(0);
    let tab = release(tab);

    assert!(is_freed(&tab));
}

#[test]
fn tabbed_page_frees_its_tabs_with_itself() {
    let _scope = test_scope();
    let first = page_with_content("A");
    let last = page_with_content("B");
    let (tp, root) = create_hosted_tabbed_page(&[first.clone(), last.clone()]);

    root.set_child(None);
    layout(&root);
    let weak_tp: WeakRef<TabbedPage> = tp.downgrade();
    drop(tp);
    let first = release(first);
    let last = release(last);

    assert!(weak_tp.upgrade().is_none());
    assert!(is_freed(&first));
    assert!(is_freed(&last));
}
