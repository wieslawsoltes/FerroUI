use super::{
    ContentPage, ModalPoppedEventArgs, ModalPushedEventArgs, NavigatedFromEventArgs, NavigatedToEventArgs,
    NavigatingFromEventArgs, NavigationEventArgs, NavigationType, Page, PageSelectionChangedEventArgs,
    SelectingMultiPage,
};
use crate::test_support::{boxed_str, test_scope};
use ferroui_base::Ref;

fn page_with_header(header: &str) -> Ref<Page> {
    let page = ContentPage::new();
    page.set_header(boxed_str(header));
    page.upcast()
}

// --- NavigatedToEventArgsTests ---

#[test]
fn navigated_to_event_args_properties_round_trip() {
    let _scope = test_scope();
    let prev = page_with_header("Prev");
    let args = NavigatedToEventArgs::new(Some(prev.clone()), NavigationType::Push);
    assert!(args.previous_page().is_some_and(|page| page.ptr_eq(&prev)));
    assert_eq!(NavigationType::Push, args.navigation_type());
}

#[test]
fn navigated_to_event_args_null_previous_page_is_allowed() {
    let _scope = test_scope();
    let args = NavigatedToEventArgs::new(None, NavigationType::PopToRoot);
    assert!(args.previous_page().is_none());
}

// --- NavigatedFromEventArgsTests ---

#[test]
fn navigated_from_event_args_properties_round_trip() {
    let _scope = test_scope();
    let dest = page_with_header("Dest");
    let args = NavigatedFromEventArgs::new(Some(dest.clone()), NavigationType::Pop);
    assert!(args.destination_page().is_some_and(|page| page.ptr_eq(&dest)));
    assert_eq!(NavigationType::Pop, args.navigation_type());
}

// --- NavigatingFromEventArgsTests ---

#[test]
fn navigating_from_event_args_cancel_default_is_false() {
    let _scope = test_scope();
    let args = NavigatingFromEventArgs::new(None, NavigationType::Push);
    assert!(!args.cancel());
}

#[test]
fn navigating_from_event_args_cancel_can_be_set_true() {
    let _scope = test_scope();
    let args = NavigatingFromEventArgs::new(None, NavigationType::Push);
    args.set_cancel(true);
    assert!(args.cancel());
}

// --- NavigationEventArgsConstructionTests ---

#[test]
fn navigation_event_args_construction_properties_round_trip() {
    let _scope = test_scope();
    let page: Ref<Page> = ContentPage::new().upcast();
    let args = NavigationEventArgs::new(page.clone(), NavigationType::Push);
    assert!(args.page().ptr_eq(&page));
    assert_eq!(NavigationType::Push, args.navigation_type());
}

// --- ModalPushedEventArgsTests ---

#[test]
fn modal_pushed_event_args_properties_round_trip() {
    let _scope = test_scope();
    let modal: Ref<Page> = ContentPage::new().upcast();
    let args = ModalPushedEventArgs::new(modal.clone());
    assert!(args.modal().ptr_eq(&modal));
}

// --- ModalPoppedEventArgsTests ---

#[test]
fn modal_popped_event_args_properties_round_trip() {
    let _scope = test_scope();
    let modal: Ref<Page> = ContentPage::new().upcast();
    let args = ModalPoppedEventArgs::new(modal.clone());
    assert!(args.modal().ptr_eq(&modal));
}

// --- PageSelectionChangedEventArgsTests ---

#[test]
fn page_selection_changed_event_args_properties_round_trip() {
    let _scope = test_scope();
    let prev = page_with_header("Tab 1");
    let current = page_with_header("Tab 2");
    let args = PageSelectionChangedEventArgs::new(
        Some(SelectingMultiPage::selection_changed_event()),
        Some(prev.clone()),
        Some(current.clone()),
    );
    assert!(args.previous_page().is_some_and(|page| page.ptr_eq(&prev)));
    assert!(args.current_page().is_some_and(|page| page.ptr_eq(&current)));
}

// --- NavigationTypeEnumTests ---

#[test]
fn all_values_are_defined() {
    let values = [
        NavigationType::Push,
        NavigationType::Pop,
        NavigationType::PopToRoot,
        NavigationType::Insert,
        NavigationType::Remove,
        NavigationType::Replace,
        NavigationType::PushModal,
        NavigationType::PopModal,
    ];
    assert!(values.contains(&NavigationType::Push));
    assert!(values.contains(&NavigationType::Pop));
    assert!(values.contains(&NavigationType::PopToRoot));
    assert!(values.contains(&NavigationType::Insert));
    assert!(values.contains(&NavigationType::Remove));
    assert!(values.contains(&NavigationType::Replace));
    assert_eq!(values.map(|value| value as i32), [0, 1, 2, 3, 4, 5, 6, 7]);
}
