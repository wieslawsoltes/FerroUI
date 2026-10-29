//! Not from the reference: tests of `ClearNavigationStackAction.cs`.

use super::support::{returned, router_with, segments, start_app};
use crate::ClearNavigationStackAction;

#[test]
fn execute_clears_the_navigation_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = ClearNavigationStackAction::new();
    action.set_router(Some(router.clone()));

    let result = action.execute(None, None);

    assert_eq!(Some(true), returned(result));
    assert!(segments(&router).is_empty());
}

#[test]
fn execute_without_a_router_returns_false() {
    let _app = start_app();
    let action = ClearNavigationStackAction::new();

    assert_eq!(Some(false), returned(action.execute(None, None)));
}

#[test]
fn execute_when_disabled_returns_false_and_keeps_the_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = ClearNavigationStackAction::new();
    action.set_router(Some(router.clone()));
    action.set_is_enabled(false);

    assert_eq!(Some(false), returned(action.execute(None, None)));
    assert_eq!(vec!["first", "second"], segments(&router));
}

#[test]
fn router_is_a_property_of_the_object_model() {
    let _app = start_app();
    let router = router_with(&[]);
    let action = ClearNavigationStackAction::new();

    assert!(action.router().is_none());

    action.set_router(Some(router.clone()));

    assert!(action.router().is_some_and(|value| std::rc::Rc::ptr_eq(&value, &router)));
}
