//! Not from the reference: tests of `NavigateAndReset.cs`.

use super::support::{boxed_routable, returned, routable, router_with, segments, start_app, TestViewModel};
use crate::NavigateAndReset;

#[test]
fn execute_navigates_to_the_view_model_and_resets_the_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = NavigateAndReset::new();
    action.set_router(Some(router.clone()));
    action.set_view_model(Some(routable(&TestViewModel::new("reset"))));

    let result = action.execute(None, None);

    assert_eq!(Some(true), returned(result));
    assert_eq!(vec!["reset"], segments(&router));
}

#[test]
fn execute_navigates_to_the_view_model_of_the_parameter_and_resets_the_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = NavigateAndReset::new();
    action.set_router(Some(router.clone()));
    let parameter = boxed_routable(routable(&TestViewModel::new("parameter")));

    let result = action.execute(None, Some(&parameter));

    assert_eq!(Some(true), returned(result));
    assert_eq!(vec!["parameter"], segments(&router));
}

#[test]
fn execute_without_a_view_model_returns_false_and_keeps_the_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = NavigateAndReset::new();
    action.set_router(Some(router.clone()));

    assert_eq!(Some(false), returned(action.execute(None, None)));
    assert_eq!(vec!["first", "second"], segments(&router));
}

#[test]
fn execute_when_disabled_returns_false_and_keeps_the_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = NavigateAndReset::new();
    action.set_router(Some(router.clone()));
    action.set_view_model(Some(routable(&TestViewModel::new("reset"))));
    action.set_is_enabled(false);

    assert_eq!(Some(false), returned(action.execute(None, None)));
    assert_eq!(vec!["first", "second"], segments(&router));
}

#[test]
fn execute_without_a_router_returns_false() {
    let _app = start_app();
    let action = NavigateAndReset::new();
    action.set_view_model(Some(routable(&TestViewModel::new("reset"))));

    assert_eq!(Some(false), returned(action.execute(None, None)));
}
