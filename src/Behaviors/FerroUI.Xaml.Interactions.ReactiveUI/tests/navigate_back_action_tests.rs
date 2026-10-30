//! Not from the reference: tests of `NavigateBackAction.cs`.

use super::support::{returned, router_with, segments, start_app};
use crate::NavigateBackAction;

#[test]
fn execute_navigates_back() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = NavigateBackAction::new();
    action.set_router(Some(router.clone()));

    let result = action.execute(None, None);

    assert_eq!(Some(true), returned(result));
    assert_eq!(vec!["first"], segments(&router));
}

#[test]
fn execute_with_an_empty_stack_returns_false() {
    let _app = start_app();
    let router = router_with(&[]);
    let action = NavigateBackAction::new();
    action.set_router(Some(router.clone()));

    assert_eq!(Some(false), returned(action.execute(None, None)));
    assert!(segments(&router).is_empty());
}

#[test]
fn execute_without_a_router_returns_false() {
    let _app = start_app();
    let action = NavigateBackAction::new();

    assert_eq!(Some(false), returned(action.execute(None, None)));
}

#[test]
fn execute_when_disabled_returns_false_and_keeps_the_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = NavigateBackAction::new();
    action.set_router(Some(router.clone()));
    action.set_is_enabled(false);

    assert_eq!(Some(false), returned(action.execute(None, None)));
    assert_eq!(vec!["first", "second"], segments(&router));
}
