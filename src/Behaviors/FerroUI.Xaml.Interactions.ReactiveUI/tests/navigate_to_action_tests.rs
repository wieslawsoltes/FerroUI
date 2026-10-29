//! Not from the reference: tests of `NavigateToAction.cs`.

use super::support::{
    boxed_routable, returned, routable, router, router_with, segments, start_app, OtherViewModel, TestViewModel,
};
use crate::{NavigateToAction, NavigateToActionImpl, RoutableViewModelType};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_reactiveui::RoutableViewModel;
use ferroui_splat::{Locator, MutableDependencyResolverExt};
use ferroui_xaml_interactivity::StyledElementActionImpl;
use std::rc::Rc;

/// A deriving class that resolves a fixed view model (an override of `ResolveViewModel`).
#[repr(C)]
struct FixedNavigateToAction {
    base: NavigateToAction,
    view_model: RoutableViewModel,
}

ferro_class!(FixedNavigateToAction: NavigateToAction);
ferroui_base::ferro_impl_classes!(FixedNavigateToAction: FerroObjectImpl, StyledElementImpl, StyledElementActionImpl);

impl NavigateToActionImpl for FixedNavigateToAction {
    fn resolve_view_model(this: &Self) -> Option<RoutableViewModel> {
        Some(this.view_model.clone())
    }
}

impl FixedNavigateToAction {
    fn new(view_model: RoutableViewModel) -> Ref<Self> {
        instantiate(Self { base: NavigateToAction::construct(RoutableViewModelType::of::<TestViewModel>()), view_model })
    }
}

#[test]
fn execute_navigates_to_the_view_model_of_the_parameter() {
    let _app = start_app();
    let router = router_with(&["first"]);
    let action = NavigateToAction::new::<TestViewModel>();
    action.set_router(Some(router.clone()));
    let parameter: BoxedValue = Rc::new(TestViewModel::new("parameter"));

    let result = action.execute(None, Some(&parameter));

    assert_eq!(Some(true), returned(result));
    assert_eq!(vec!["first", "parameter"], segments(&router));
}

#[test]
fn execute_accepts_a_routable_view_model_of_the_type() {
    let _app = start_app();
    let router = router();
    let action = NavigateToAction::new::<TestViewModel>();
    action.set_router(Some(router.clone()));
    let parameter = boxed_routable(routable(&TestViewModel::new("parameter")));

    assert_eq!(Some(true), returned(action.execute(None, Some(&parameter))));
    assert_eq!(vec!["parameter"], segments(&router));
}

#[test]
fn execute_resolves_the_view_model_from_the_locator() {
    let _app = start_app();
    let router = router();
    Locator::current_mutable().register_constant::<TestViewModel>(TestViewModel::new("resolved"));
    let action = NavigateToAction::new::<TestViewModel>();
    action.set_router(Some(router.clone()));

    assert_eq!(Some(true), returned(action.execute(None, None)));
    assert_eq!(vec!["resolved"], segments(&router));
}

#[test]
fn execute_resolves_the_view_model_when_the_parameter_is_of_another_type() {
    let _app = start_app();
    let router = router();
    Locator::current_mutable().register_constant::<TestViewModel>(TestViewModel::new("resolved"));
    let action = NavigateToAction::new::<TestViewModel>();
    action.set_router(Some(router.clone()));
    let parameter = boxed_routable(routable(&OtherViewModel::new()));

    assert_eq!(Some(true), returned(action.execute(None, Some(&parameter))));
    assert_eq!(vec!["resolved"], segments(&router));
}

#[test]
fn execute_without_a_view_model_returns_false() {
    let _app = start_app();
    let router = router();
    let action = NavigateToAction::new::<TestViewModel>();
    action.set_router(Some(router.clone()));
    let parameter = boxed_routable(routable(&OtherViewModel::new()));

    assert_eq!(Some(false), returned(action.execute(None, None)));
    assert_eq!(Some(false), returned(action.execute(None, Some(&parameter))));
    assert!(segments(&router).is_empty());
}

#[test]
fn execute_without_a_router_returns_false() {
    let _app = start_app();
    let action = NavigateToAction::new::<TestViewModel>();
    let parameter = boxed_routable(routable(&TestViewModel::new("parameter")));

    assert_eq!(Some(false), returned(action.execute(None, Some(&parameter))));
}

#[test]
fn execute_when_disabled_returns_false_and_does_not_navigate() {
    let _app = start_app();
    let router = router();
    let action = NavigateToAction::new::<TestViewModel>();
    action.set_router(Some(router.clone()));
    action.set_is_enabled(false);
    let parameter = boxed_routable(routable(&TestViewModel::new("parameter")));

    assert_eq!(Some(false), returned(action.execute(None, Some(&parameter))));
    assert!(segments(&router).is_empty());
}

#[test]
fn a_deriving_class_resolves_the_view_model() {
    let _app = start_app();
    let router = router();
    let action = FixedNavigateToAction::new(routable(&TestViewModel::new("derived")));
    action.set_router(Some(router.clone()));

    assert_eq!(Some(true), returned(action.execute(None, None)));
    assert_eq!(vec!["derived"], segments(&router));
}

#[test]
fn view_model_type_names_the_type() {
    let _app = start_app();
    let action = NavigateToAction::new::<TestViewModel>();

    assert_eq!(std::any::type_name::<TestViewModel>(), action.view_model_type().name());
}
