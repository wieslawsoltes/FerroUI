//! Not from the reference: tests of `NavigateToAndResetAction.cs`.

use super::support::{returned, router_with, segments, start_app, TestViewModel};
use crate::NavigateToAndResetAction;
use ferroui_base::BoxedValue;
use ferroui_splat::{Locator, MutableDependencyResolverExt};
use std::rc::Rc;

#[test]
fn execute_navigates_to_the_view_model_of_the_parameter_and_resets_the_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = NavigateToAndResetAction::new::<TestViewModel>();
    action.set_router(Some(router.clone()));
    let parameter: BoxedValue = Rc::new(TestViewModel::new("parameter"));

    let result = action.execute(None, Some(&parameter));

    assert_eq!(Some(true), returned(result));
    assert_eq!(vec!["parameter"], segments(&router));
}

#[test]
fn execute_resolves_the_view_model_from_the_locator_and_resets_the_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    Locator::current_mutable().register_constant::<TestViewModel>(TestViewModel::new("resolved"));
    let action = NavigateToAndResetAction::new::<TestViewModel>();
    action.set_router(Some(router.clone()));

    assert_eq!(Some(true), returned(action.execute(None, None)));
    assert_eq!(vec!["resolved"], segments(&router));
}

#[test]
fn execute_without_a_view_model_returns_false_and_keeps_the_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = NavigateToAndResetAction::new::<TestViewModel>();
    action.set_router(Some(router.clone()));

    assert_eq!(Some(false), returned(action.execute(None, None)));
    assert_eq!(vec!["first", "second"], segments(&router));
}

#[test]
fn execute_when_disabled_returns_false_and_keeps_the_stack() {
    let _app = start_app();
    let router = router_with(&["first", "second"]);
    let action = NavigateToAndResetAction::new::<TestViewModel>();
    action.set_router(Some(router.clone()));
    action.set_is_enabled(false);
    let parameter: BoxedValue = Rc::new(TestViewModel::new("parameter"));

    assert_eq!(Some(false), returned(action.execute(None, Some(&parameter))));
    assert_eq!(vec!["first", "second"], segments(&router));
}

#[test]
fn execute_without_a_router_returns_false() {
    let _app = start_app();
    let action = NavigateToAndResetAction::new::<TestViewModel>();
    let parameter: BoxedValue = Rc::new(TestViewModel::new("parameter"));

    assert_eq!(Some(false), returned(action.execute(None, Some(&parameter))));
}
