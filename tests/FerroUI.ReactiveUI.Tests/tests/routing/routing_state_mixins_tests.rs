//! `RoutingStateMixinsTests` (`ReactiveUI.Tests/Routing/RoutingStateMixinsTests.cs`).

use ferroui_reactiveui::*;
use std::rc::Rc;

/// A routable view model used to populate the navigation stack (`FirstViewModel`).
struct FirstViewModel {
    reactive: ReactiveObject,
}
reactive_object!(FirstViewModel);

impl IRoutableViewModel for FirstViewModel {
    fn url_path_segment(&self) -> Option<String> {
        Some("first".to_owned())
    }

    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        None
    }
}

/// A second routable view model type used to populate the navigation stack (`SecondViewModel`).
struct SecondViewModel {
    reactive: ReactiveObject,
}
reactive_object!(SecondViewModel);

impl IRoutableViewModel for SecondViewModel {
    fn url_path_segment(&self) -> Option<String> {
        Some("second".to_owned())
    }

    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        None
    }
}

fn first() -> Rc<FirstViewModel> {
    Rc::new(FirstViewModel { reactive: ReactiveObject::new() })
}

fn second() -> Rc<SecondViewModel> {
    Rc::new(SecondViewModel { reactive: ReactiveObject::new() })
}

#[test]
fn find_view_model_in_stack_returns_match_after_skipping_non_matches() {
    let state = RoutingState::new();
    let first = first();
    state.navigation_stack().add(first.clone());
    state.navigation_stack().add(second());

    // Reverse search visits the SecondViewModel (no match) before the FirstViewModel (match),
    // exercising both branches of the type test.
    let found = state.find_view_model_in_stack::<FirstViewModel>();
    assert!(found.is_some_and(|found| Rc::ptr_eq(&found, &first)));
}

#[test]
fn find_view_model_in_stack_returns_null_when_no_match() {
    let state = RoutingState::new();
    state.navigation_stack().add(second());

    assert!(state.find_view_model_in_stack::<FirstViewModel>().is_none());
}

#[test]
fn get_current_view_model_returns_top_or_null() {
    let state = RoutingState::new();
    assert!(state.get_current_view_model().is_none());

    let first: Rc<dyn IRoutableViewModel> = first();
    let second: Rc<dyn IRoutableViewModel> = second();
    state.navigation_stack().add(first);
    state.navigation_stack().add(second.clone());

    assert_eq!(state.get_current_view_model(), Some(second));
}
