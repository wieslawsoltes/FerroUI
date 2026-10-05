//! The navigation page tests, continued: the back button content, pop to
//! root, insert/remove and modal groups of the reference tests.

use super::navigation_page_tests::*;
use super::{
    DrawerBehavior, DrawerPage, ModalPoppedEventArgs, ModalPushedEventArgs, NavigatedFromEventArgs, NavigatedToEventArgs, NavigationPage,
    NavigationType, PageInsertedEventArgs, PageRemovedEventArgs,
};
use crate::presenters::ContentPresenter;
use crate::shapes::Path;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::{Border, Button, Control, Panel, PathIcon};
use ferroui_base::media::StreamGeometry;
use ferroui_base::Ref;
use std::cell::RefCell;
use std::rc::Rc;

// --- BackButtonContentTests ---

#[derive(Default)]
struct BackButtonParts {
    default_icon: RefCell<Option<Ref<Path>>>,
    content_presenter: RefCell<Option<Ref<ContentPresenter>>>,
}

fn named_presenter(name: &str) -> Ref<ContentPresenter> {
    let presenter = ContentPresenter::new();
    presenter.set_name(Some(name.to_string()));
    presenter
}

fn create_navigation_page_with_back_button_parts(parts: &Rc<BackButtonParts>) -> Ref<NavigationPage> {
    let nav = NavigationPage::new();
    let parts = parts.clone();
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::new(move |_, ns| {
        let default_icon = Path::new();
        default_icon.set_name(Some("PART_BackButtonDefaultIcon".to_string()));
        let default_icon = default_icon.register_in_name_scope(&**ns);
        *parts.default_icon.borrow_mut() = Some(default_icon.clone());
        let content_presenter = named_presenter("PART_BackButtonContentPresenter").register_in_name_scope(&**ns);
        *parts.content_presenter.borrow_mut() = Some(content_presenter.clone());

        let button_content = Panel::new();
        button_content.children().add(default_icon);
        button_content.children().add(content_presenter);
        let back_button = Button::new();
        back_button.set_name(Some("PART_BackButton".to_string()));
        back_button.set_content(Some(Control::boxed(button_content)));
        let navigation_bar = Border::new();
        navigation_bar.set_name(Some("PART_NavigationBar".to_string()));
        navigation_bar.set_child(back_button.register_in_name_scope(&**ns));

        let content_host = Panel::new();
        content_host.set_name(Some("PART_ContentHost".to_string()));
        content_host.children().add(named_presenter("PART_PageBackPresenter").register_in_name_scope(&**ns));
        content_host.children().add(named_presenter("PART_PagePresenter").register_in_name_scope(&**ns));

        let panel = Panel::new();
        panel.children().add(navigation_bar.register_in_name_scope(&**ns));
        panel.children().add(content_host.register_in_name_scope(&**ns));
        panel.children().add(named_presenter("PART_TopCommandBar").register_in_name_scope(&**ns));
        panel.children().add(named_presenter("PART_ModalBackPresenter").register_in_name_scope(&**ns));
        panel.children().add(named_presenter("PART_ModalPresenter").register_in_name_scope(&**ns));
        panel.upcast()
    });
    nav.set_template(Some(template));
    nav
}

#[test]
fn drawer_toggle_uses_menu_icon_without_mutating_page_back_button_content() {
    let _scope = test_scope();
    let parts = Rc::new(BackButtonParts::default());
    let nav = create_navigation_page_with_back_button_parts(&parts);
    nav.resources().add_value("NavigationPageMenuIcon", StreamGeometry::new());
    let drawer = DrawerPage::new();
    let root = TestRoot::with_child(nav.clone());
    root.execute_initial_layout_pass();
    nav.set_drawer_page(Some(&drawer));

    let page = page_h("Root");
    wait(nav.push_async(&page));

    let default_icon = parts.default_icon.borrow().clone().expect("the default icon was created");
    let content_presenter = parts.content_presenter.borrow().clone().expect("the content presenter was created");
    assert!(NavigationPage::get_back_button_content(&page).is_none());
    assert!(!default_icon.is_visible());
    let content = content_presenter.content().as_ref().and_then(Control::from_boxed);
    assert!(content.is_some_and(|content| content.is::<PathIcon>()));
}

#[test]
fn current_page_back_button_content_updates_rendered_presenter() {
    let _scope = test_scope();
    let parts = Rc::new(BackButtonParts::default());
    let nav = create_navigation_page_with_back_button_parts(&parts);
    let root = TestRoot::with_child(nav.clone());
    root.execute_initial_layout_pass();

    wait(nav.push_async(page_h("Root")));
    let detail = page_h("Detail");
    wait(nav.push_async(&detail));

    let custom_content = "Custom";
    NavigationPage::set_back_button_content(&detail, boxed_str(custom_content));

    let default_icon = parts.default_icon.borrow().clone().expect("the default icon was created");
    let content_presenter = parts.content_presenter.borrow().clone().expect("the content presenter was created");
    assert!(!default_icon.is_visible());
    assert_eq!(content_presenter.content().as_ref().and_then(string_of).as_deref(), Some(custom_content));

    NavigationPage::set_back_button_content(&detail, None);

    assert!(default_icon.is_visible());
    assert!(content_presenter.content().is_none());
}

#[test]
fn drawer_behavior_change_does_not_clear_custom_path_icon() {
    let _scope = test_scope();
    let parts = Rc::new(BackButtonParts::default());
    let nav = create_navigation_page_with_back_button_parts(&parts);
    let drawer = DrawerPage::new();
    let root = TestRoot::with_child(nav.clone());
    root.execute_initial_layout_pass();
    nav.set_drawer_page(Some(&drawer));

    let custom_icon = PathIcon::new();
    let page = page_h("Root");
    NavigationPage::set_back_button_content(&page, Some(Control::boxed(custom_icon.clone())));

    wait(nav.push_async(&page));
    drawer.set_drawer_behavior(DrawerBehavior::Locked);
    nav.set_drawer_page(Some(&drawer));

    let default_icon = parts.default_icon.borrow().clone().expect("the default icon was created");
    let content_presenter = parts.content_presenter.borrow().clone().expect("the content presenter was created");
    let held = NavigationPage::get_back_button_content(&page).as_ref().and_then(Control::from_boxed);
    assert!(same(&custom_icon, &held));
    let content = content_presenter.content().as_ref().and_then(Control::from_boxed);
    assert!(same(&custom_icon, &content));
    assert!(!default_icon.is_visible());
}

// --- PopToRootTests ---

#[test]
fn pop_to_root_leaves_only_first_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));
    wait(nav.push_async(page()));
    wait(nav.push_async(page()));
    wait(nav.push_async(page()));
    wait(nav.pop_to_root_async());
    assert_eq!(1, nav.stack_depth());
    assert!(same(&root, &nav.current_page()));
}

#[test]
fn pop_to_root_fires_popped_to_root_event() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let fired = flag();
    nav.popped_to_root(raise_flag(&fired));
    wait(nav.push_async(page()));
    wait(nav.push_async(page()));
    wait(nav.pop_to_root_async());
    assert!(fired.get());
}

#[test]
fn pop_to_root_invokes_navigated_from_on_current_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let args = slot::<NavigatedFromEventArgs>();
    top.navigated_from(store(&args));

    wait(nav.pop_to_root_async());

    let args = taken(&args);
    assert_eq!(NavigationType::PopToRoot, args.navigation_type());
    assert!(same(&root, &args.destination_page()));
}

#[test]
fn pop_to_root_invokes_navigated_to_on_root_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let args = slot::<NavigatedToEventArgs>();
    root.navigated_to(store(&args));

    wait(nav.pop_to_root_async());

    let args = taken(&args);
    assert_eq!(NavigationType::PopToRoot, args.navigation_type());
    assert!(same(&top, &args.previous_page()));
}

#[test]
fn pop_to_root_navigated_from_fires_after_state_change() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let stack_depth_at_event = Rc::new(std::cell::Cell::new(-1));
    let depth = stack_depth_at_event.clone();
    let weak = nav.downgrade();
    top.navigated_from(move |_| depth.set(weak.upgrade().expect("the navigation page is alive").stack_depth()));

    wait(nav.pop_to_root_async());

    assert_eq!(1, stack_depth_at_event.get());
}

#[test]
fn pop_to_root_when_already_at_root_does_nothing() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let fired = flag();
    nav.popped_to_root(raise_flag(&fired));
    wait(nav.push_async(page()));
    wait(nav.pop_to_root_async());
    assert_eq!(1, nav.stack_depth());
    assert!(!fired.get());
}

// --- InsertRemoveTests ---

#[test]
fn insert_page_adds_page_before_target() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let middle = page_h("Middle");
    nav.insert_page(&middle, &top);

    assert!(stack_is(&nav.navigation_stack(), &[&root, &middle, &top]));
}

#[test]
fn remove_page_removes_from_middle_of_stack() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let middle = page_h("Middle");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&middle));
    wait(nav.push_async(&top));

    nav.remove_page(&middle);

    assert!(stack_is(&nav.navigation_stack(), &[&root, &top]));
}

#[test]
fn insert_page_fires_page_inserted_event() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let args = slot::<PageInsertedEventArgs>();
    nav.page_inserted(store(&args));

    let inserted = page();
    nav.insert_page(&inserted, &top);

    assert!(same(&inserted, &taken(&args).page()));
}

#[test]
fn remove_page_fires_page_removed_event() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let mid = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&mid));
    wait(nav.push_async(&top));

    let args = slot::<PageRemovedEventArgs>();
    nav.page_removed(store(&args));

    nav.remove_page(&mid);

    assert!(same(&mid, &taken(&args).page()));
}

// `InsertPage_NullPage_ThrowsArgumentNullException`, `InsertPage_NullBefore_ThrowsArgumentNullException` and
// `RemovePage_NullPage_ThrowsArgumentNullException` have no counterpart: a page reference cannot be null.

#[test]
fn insert_page_does_not_fire_navigated_to_on_inserted_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let args = slot::<NavigatedToEventArgs>();
    let inserted = page();
    inserted.navigated_to(store(&args));

    nav.insert_page(&inserted, &top);

    assert!(args.borrow().is_none());
}

#[test]
fn insert_page_fires_navigated_to_when_inserted_page_becomes_current() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let inserted = page();
    let navigated_to_count = counter();
    let args = slot::<NavigatedToEventArgs>();
    inserted.navigated_to(count(&navigated_to_count));
    inserted.navigated_to(store(&args));

    nav.insert_page(&inserted, &top);
    assert_eq!(0, navigated_to_count.get());

    wait(nav.pop_async());

    assert_eq!(1, navigated_to_count.get());
    let args = taken(&args);
    assert_eq!(NavigationType::Pop, args.navigation_type());
    assert!(same(&top, &args.previous_page()));
    assert!(same(&inserted, &nav.current_page()));
}

#[test]
fn insert_page_duplicate_page_throws_invalid_operation_exception() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let top = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    assert!(throws(|| nav.insert_page(&root, &top)));
}

#[test]
fn insert_page_page_already_presented_modally_throws_invalid_operation_exception() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    let top = page();
    let modal = page();
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));
    wait(nav.push_modal_async(&modal));

    assert!(throws(|| nav.insert_page(&modal, &top)));
}

#[test]
fn insert_page_before_not_in_stack_throws_invalid_operation_exception() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page();
    wait(nav.push_async(&root));

    let stranger = page();
    assert!(throws(|| nav.insert_page(page(), &stranger)));
}

#[test]
fn remove_page_page_not_in_stack_is_no_op() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));

    let stranger = page();
    // Should not panic and should not change the stack depth.
    nav.remove_page(&stranger);
    assert_eq!(1, nav.stack_depth());
}

#[test]
fn remove_page_top_page_fires_navigated_from_with_remove_type() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let from_args = slot::<NavigatedFromEventArgs>();
    top.navigated_from(store(&from_args));

    nav.remove_page(&top);

    let from_args = taken(&from_args);
    assert_eq!(NavigationType::Remove, from_args.navigation_type());
    assert!(same(&root, &from_args.destination_page()));
}

#[test]
fn remove_page_top_page_fires_navigated_to_on_new_current_page_with_remove_type() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    let to_args = slot::<NavigatedToEventArgs>();
    root.navigated_to(store(&to_args));

    nav.remove_page(&top);

    let to_args = taken(&to_args);
    assert_eq!(NavigationType::Remove, to_args.navigation_type());
    assert!(same(&top, &to_args.previous_page()));
}

#[test]
fn remove_page_top_page_updates_current_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let top = page_h("Top");
    wait(nav.push_async(&root));
    wait(nav.push_async(&top));

    nav.remove_page(&top);

    assert!(same(&root, &nav.current_page()));
    assert_eq!(1, nav.stack_depth());
}

// --- ModalTests ---

#[test]
fn push_modal_adds_to_modal_stack() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let modal = page_h("Modal");
    wait(nav.push_modal_async(&modal));
    assert_eq!(1, nav.modal_stack().len());
}

#[test]
fn push_modal_page_already_in_navigation_stack_throws() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));

    assert!(throws_async(nav.push_modal_async(&root)));
}

#[test]
fn push_modal_duplicate_modal_throws() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));

    let modal = page_h("Modal");
    wait(nav.push_modal_async(&modal));

    assert!(throws_async(nav.push_modal_async(&modal)));
}

#[test]
fn pop_modal_removes_from_modal_stack() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let modal = page();
    wait(nav.push_modal_async(&modal));
    wait(nav.pop_modal_async());
    assert_eq!(0, nav.modal_stack().len());
}

#[test]
fn modal_stack_is_ordered_bottom_to_top() {
    // Index 0 = oldest (bottom-most); last index = topmost, consistent with the navigation stack.
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let m1 = page_h("M1");
    let m2 = page_h("M2");
    let m3 = page_h("M3");
    wait(nav.push_modal_async(&m1));
    wait(nav.push_modal_async(&m2));
    wait(nav.push_modal_async(&m3));

    assert!(stack_is(&nav.modal_stack(), &[&m1, &m2, &m3]));
}

#[test]
fn push_modal_fires_modal_pushed_event() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let args = slot::<ModalPushedEventArgs>();
    nav.modal_pushed(store(&args));
    let modal = page();
    wait(nav.push_modal_async(&modal));
    assert!(same(&modal, &taken(&args).modal()));
}

#[test]
fn pop_modal_fires_modal_popped_event() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let modal = page();
    wait(nav.push_modal_async(&modal));
    let args = slot::<ModalPoppedEventArgs>();
    nav.modal_popped(store(&args));
    wait(nav.pop_modal_async());
    assert!(same(&modal, &taken(&args).modal()));
}

#[test]
fn pop_modal_on_empty_stack_returns_null() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let result = wait(nav.pop_modal_async());
    assert!(result.is_none());
}

#[test]
fn push_modal_invokes_navigated_from_on_covered_page_with_push_modal_type() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));

    let args = slot::<NavigatedFromEventArgs>();
    root.navigated_from(store(&args));

    wait(nav.push_modal_async(page_h("Modal")));

    assert_eq!(NavigationType::PushModal, taken(&args).navigation_type());
}

#[test]
fn push_modal_invokes_navigated_to_on_modal_page_with_push_modal_type() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));

    let modal = page_h("Modal");
    let args = slot::<NavigatedToEventArgs>();
    modal.navigated_to(store(&args));

    wait(nav.push_modal_async(&modal));

    assert_eq!(NavigationType::PushModal, taken(&args).navigation_type());
}

#[test]
fn pop_modal_invokes_navigated_from_on_popped_modal_with_pop_modal_type() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page()));
    let modal = page_h("Modal");
    wait(nav.push_modal_async(&modal));

    let args = slot::<NavigatedFromEventArgs>();
    modal.navigated_from(store(&args));

    wait(nav.pop_modal_async());

    assert_eq!(NavigationType::PopModal, taken(&args).navigation_type());
}

#[test]
fn pop_modal_invokes_navigated_to_on_revealed_page_with_pop_modal_type() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));
    wait(nav.push_modal_async(page_h("Modal")));

    let args = slot::<NavigatedToEventArgs>();
    root.navigated_to(store(&args));

    wait(nav.pop_modal_async());

    assert_eq!(NavigationType::PopModal, taken(&args).navigation_type());
}

#[test]
fn push_modal_navigated_from_destination_page_is_the_modal() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));

    let modal = page_h("Modal");
    let args = slot::<NavigatedFromEventArgs>();
    root.navigated_from(store(&args));

    wait(nav.push_modal_async(&modal));

    assert!(same(&modal, &taken(&args).destination_page()));
}

#[test]
fn push_modal_navigated_to_previous_page_is_covered_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));

    let modal = page_h("Modal");
    let args = slot::<NavigatedToEventArgs>();
    modal.navigated_to(store(&args));

    wait(nav.push_modal_async(&modal));

    assert!(same(&root, &taken(&args).previous_page()));
}

#[test]
fn pop_modal_navigated_from_destination_page_is_revealed_page() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let modal = page_h("Modal");
    wait(nav.push_async(&root));
    wait(nav.push_modal_async(&modal));

    let args = slot::<NavigatedFromEventArgs>();
    modal.navigated_from(store(&args));

    wait(nav.pop_modal_async());

    assert!(same(&root, &taken(&args).destination_page()));
}

#[test]
fn pop_modal_navigated_to_previous_page_is_popped_modal() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    let modal = page_h("Modal");
    wait(nav.push_async(&root));
    wait(nav.push_modal_async(&modal));

    let args = slot::<NavigatedToEventArgs>();
    root.navigated_to(store(&args));

    wait(nav.pop_modal_async());

    assert!(same(&modal, &taken(&args).previous_page()));
}

#[test]
fn push_modal_when_covered_page_cancels_navigating_does_not_push_modal() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    let root = page_h("Root");
    wait(nav.push_async(&root));

    let handler_invoked = flag();
    root.navigating(cancel_navigation(&handler_invoked));

    wait(nav.push_modal_async(page_h("Modal")));

    assert!(handler_invoked.get());
    assert!(nav.modal_stack().is_empty());
}

#[test]
fn push_modal_when_top_modal_cancels_navigating_does_not_push_another_modal() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page_h("Root")));

    let first_modal = page_h("Modal 1");
    wait(nav.push_modal_async(&first_modal));

    let handler_invoked = flag();
    first_modal.navigating(cancel_navigation(&handler_invoked));

    wait(nav.push_modal_async(page_h("Modal 2")));

    assert!(handler_invoked.get());
    assert!(stack_is(&nav.modal_stack(), &[&first_modal]));
}

#[test]
fn pop_modal_when_modal_cancels_navigating_does_not_pop_modal() {
    let _scope = test_scope();
    let nav = NavigationPage::new();
    wait(nav.push_async(page_h("Root")));

    let modal = page_h("Modal");
    wait(nav.push_modal_async(&modal));

    let handler_invoked = flag();
    modal.navigating(cancel_navigation(&handler_invoked));

    let result = wait(nav.pop_modal_async());

    assert!(handler_invoked.get());
    assert!(result.is_none());
    assert!(stack_is(&nav.modal_stack(), &[&modal]));
}
