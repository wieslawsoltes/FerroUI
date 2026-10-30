//! Not from the reference: tests of `NavigateAction.cs`.

use super::support::{boxed_routable, returned, routable, router, router_with, segments, start_app, TestViewModel};
use crate::NavigateAction;
use ferroui_base::BoxedValue;
use std::rc::Rc;

#[test]
fn execute_navigates_to_the_view_model() {
    let _app = start_app();
    let router = router_with(&["first"]);
    let action = NavigateAction::new();
    action.set_router(Some(router.clone()));
    action.set_view_model(Some(routable(&TestViewModel::new("second"))));

    let result = action.execute(None, None);

    assert_eq!(Some(true), returned(result));
    assert_eq!(vec!["first", "second"], segments(&router));
}

#[test]
fn execute_navigates_to_the_view_model_of_the_parameter() {
    let _app = start_app();
    let router = router();
    let action = NavigateAction::new();
    action.set_router(Some(router.clone()));
    let parameter = boxed_routable(routable(&TestViewModel::new("parameter")));

    let result = action.execute(None, Some(&parameter));

    assert_eq!(Some(true), returned(result));
    assert_eq!(vec!["parameter"], segments(&router));
}

#[test]
fn execute_prefers_the_view_model_to_the_parameter() {
    let _app = start_app();
    let router = router();
    let action = NavigateAction::new();
    action.set_router(Some(router.clone()));
    action.set_view_model(Some(routable(&TestViewModel::new("property"))));
    let parameter = boxed_routable(routable(&TestViewModel::new("parameter")));

    action.execute(None, Some(&parameter));

    assert_eq!(vec!["property"], segments(&router));
}

#[test]
fn execute_without_a_view_model_returns_false() {
    let _app = start_app();
    let router = router();
    let action = NavigateAction::new();
    action.set_router(Some(router.clone()));
    let parameter: BoxedValue = Rc::new("not a view model".to_owned());

    assert_eq!(Some(false), returned(action.execute(None, None)));
    assert_eq!(Some(false), returned(action.execute(None, Some(&parameter))));
    assert!(segments(&router).is_empty());
}

#[test]
fn execute_without_a_router_returns_false() {
    let _app = start_app();
    let action = NavigateAction::new();
    action.set_view_model(Some(routable(&TestViewModel::new("second"))));

    assert_eq!(Some(false), returned(action.execute(None, None)));
}

#[test]
fn execute_when_disabled_returns_false_and_does_not_navigate() {
    let _app = start_app();
    let router = router();
    let action = NavigateAction::new();
    action.set_router(Some(router.clone()));
    action.set_view_model(Some(routable(&TestViewModel::new("second"))));
    action.set_is_enabled(false);

    assert_eq!(Some(false), returned(action.execute(None, None)));
    assert!(segments(&router).is_empty());
}
