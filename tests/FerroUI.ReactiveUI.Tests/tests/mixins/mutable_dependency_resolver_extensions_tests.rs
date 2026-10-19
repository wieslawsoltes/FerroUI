//! `MutableDependencyResolverExtensionsTests` (`ReactiveUI.Tests/Mixins/MutableDependencyResolverExtensionsTests.cs`).
//!
//! The tests run in an application with the core services (`AppBuilderTestExecutor`), since the
//! extensions check that ReactiveUI was initialized. `IsOfType(typeof(TestView))` is a downcast of
//! the resolved view.

use crate::mocks::with_app_builder;
use ferroui_reactiveui::*;
use ferroui_splat::{ModernDependencyResolver, ReadonlyDependencyResolverExt};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// `TestContract`.
const TEST_CONTRACT: &str = "TestContract";

/// `TestViewModel`.
#[derive(Default)]
pub struct TestViewModel {
    reactive: ReactiveObject,
}
reactive_object!(TestViewModel);

/// `AlternateTestViewModel`.
#[derive(Default)]
pub struct AlternateTestViewModel {
    reactive: ReactiveObject,
}
reactive_object!(AlternateTestViewModel);

/// `TestView`.
#[derive(Default)]
pub struct TestView {
    view_model: RefCell<Option<Rc<TestViewModel>>>,
}
view_for!(TestView, TestViewModel, view_model);

/// `AlternateTestView`.
#[derive(Default)]
pub struct AlternateTestView {
    view_model: RefCell<Option<Rc<AlternateTestViewModel>>>,
}
view_for!(AlternateTestView, AlternateTestViewModel, view_model);

fn is_test_view(view: &Rc<dyn IViewForOf<TestViewModel>>) -> bool {
    (&**view as &dyn Any).is::<TestView>()
}

#[test]
fn register_singleton_view_for_view_model_registers_singleton() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver.register_singleton_view_for_view_model::<TestView, TestViewModel>();

        let view = resolver.get_service::<dyn IViewForOf<TestViewModel>>();

        assert!(view.as_ref().is_some_and(is_test_view));
    });
}

#[test]
fn register_singleton_view_for_view_model_returns_resolver() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();

        let result = resolver.register_singleton_view_for_view_model::<TestView, TestViewModel>();

        assert!(std::ptr::eq(result, &resolver));
    });
}

#[test]
fn register_singleton_view_for_view_model_returns_same_instance() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver.register_singleton_view_for_view_model::<TestView, TestViewModel>();

        let view1 = resolver.get_service::<dyn IViewForOf<TestViewModel>>().unwrap();
        let view2 = resolver.get_service::<dyn IViewForOf<TestViewModel>>().unwrap();

        assert!(Rc::ptr_eq(&view1, &view2));
    });
}

#[test]
fn register_singleton_view_for_view_model_supports_chaining() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver
            .register_singleton_view_for_view_model::<TestView, TestViewModel>()
            .register_singleton_view_for_view_model::<AlternateTestView, AlternateTestViewModel>();

        let view1 = resolver.get_service::<dyn IViewForOf<TestViewModel>>();
        let view2 = resolver.get_service::<dyn IViewForOf<AlternateTestViewModel>>();

        assert!(view1.is_some());
        assert!(view2.is_some());
    });
}

// Not ported: RegisterSingletonViewForViewModelThrowsOnNullResolver - argument validation

#[test]
fn register_singleton_view_for_view_model_with_contract_registers_singleton() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver.register_singleton_view_for_view_model_with_contract::<TestView, TestViewModel>(Some(TEST_CONTRACT));

        let view = resolver.get_service_with_contract::<dyn IViewForOf<TestViewModel>>(Some(TEST_CONTRACT));

        assert!(view.as_ref().is_some_and(is_test_view));
    });
}

#[test]
fn register_view_for_view_model_creates_new_instance_each_time() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver.register_view_for_view_model::<TestView, TestViewModel>();

        let view1 = resolver.get_service::<dyn IViewForOf<TestViewModel>>().unwrap();
        let view2 = resolver.get_service::<dyn IViewForOf<TestViewModel>>().unwrap();

        assert!(!Rc::ptr_eq(&view1, &view2));
    });
}

#[test]
fn register_view_for_view_model_registers_view() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver.register_view_for_view_model::<TestView, TestViewModel>();

        let view = resolver.get_service::<dyn IViewForOf<TestViewModel>>();

        assert!(view.as_ref().is_some_and(is_test_view));
    });
}

#[test]
fn register_view_for_view_model_returns_resolver() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();

        let result = resolver.register_view_for_view_model::<TestView, TestViewModel>();

        assert!(std::ptr::eq(result, &resolver));
    });
}

#[test]
fn register_view_for_view_model_supports_chaining() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver
            .register_view_for_view_model::<TestView, TestViewModel>()
            .register_view_for_view_model::<AlternateTestView, AlternateTestViewModel>();

        let view1 = resolver.get_service::<dyn IViewForOf<TestViewModel>>();
        let view2 = resolver.get_service::<dyn IViewForOf<AlternateTestViewModel>>();

        assert!(view1.is_some());
        assert!(view2.is_some());
    });
}

// Not ported: RegisterViewForViewModelThrowsOnNullResolver - argument validation

#[test]
fn register_view_for_view_model_with_contract_registers_view() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver.register_view_for_view_model_with_contract::<TestView, TestViewModel>(Some(TEST_CONTRACT));

        let view = resolver.get_service_with_contract::<dyn IViewForOf<TestViewModel>>(Some(TEST_CONTRACT));

        assert!(view.as_ref().is_some_and(is_test_view));
    });
}
