//! `MutableDependencyResolverAotExtensionsTests` (`ReactiveUI.Tests/Mixins/MutableDependencyResolverAotExtensionsTests.cs`).
//!
//! The tests run in an application with the core services (upstream's assembly-wide
//! `AppBuilderTestExecutor`), since the extensions check that ReactiveUI was initialized.
//! `IsTypeOf<TestView>()` is a downcast of the resolved view.

use crate::mocks::with_app_builder;
use ferroui_reactiveui::*;
use ferroui_splat::{ModernDependencyResolver, ReadonlyDependencyResolverExt};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// `SingletonContractName`.
const SINGLETON_CONTRACT_NAME: &str = "SingletonContract";

/// `TestViewModel`.
#[derive(Default)]
pub struct TestViewModel {
    reactive: ReactiveObject,
}
reactive_object!(TestViewModel);

/// `TestView`.
#[derive(Default)]
pub struct TestView {
    view_model: RefCell<Option<Rc<TestViewModel>>>,
}
view_for!(TestView, TestViewModel, view_model);

fn is_test_view(view: &Rc<dyn IViewForOf<TestViewModel>>) -> bool {
    (&**view as &dyn Any).is::<TestView>()
}

#[test]
fn register_singleton_view_for_view_model_aot_registers_singleton_view() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver.register_singleton_view_for_view_model_aot::<TestView, TestViewModel>(None);

        let view1 = resolver.get_service::<dyn IViewForOf<TestViewModel>>().unwrap();
        let view2 = resolver.get_service::<dyn IViewForOf<TestViewModel>>().unwrap();

        assert!(is_test_view(&view1));
        assert!(Rc::ptr_eq(&view1, &view2));
    });
}

#[test]
fn register_singleton_view_for_view_model_aot_returns_resolver_for_chaining() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();

        let result = resolver.register_singleton_view_for_view_model_aot::<TestView, TestViewModel>(None);

        assert!(std::ptr::eq(result, &resolver));
    });
}

// Not ported: RegisterSingletonViewForViewModelAOT_ThrowsArgumentNullException_WhenResolverIsNull - argument validation

#[test]
fn register_singleton_view_for_view_model_aot_with_contract_registers_view_with_contract() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver.register_singleton_view_for_view_model_aot::<TestView, TestViewModel>(Some(SINGLETON_CONTRACT_NAME));

        let view1 = resolver.get_service_with_contract::<dyn IViewForOf<TestViewModel>>(Some(SINGLETON_CONTRACT_NAME)).unwrap();
        let view2 = resolver.get_service_with_contract::<dyn IViewForOf<TestViewModel>>(Some(SINGLETON_CONTRACT_NAME)).unwrap();

        assert!(is_test_view(&view1));
        assert!(Rc::ptr_eq(&view1, &view2));
    });
}

#[test]
fn register_view_for_view_model_aot_registers_transient_view() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver.register_view_for_view_model_aot::<TestView, TestViewModel>(None);

        let view1 = resolver.get_service::<dyn IViewForOf<TestViewModel>>().unwrap();
        let view2 = resolver.get_service::<dyn IViewForOf<TestViewModel>>().unwrap();

        assert!(is_test_view(&view1));
        assert!(!Rc::ptr_eq(&view1, &view2));
    });
}

#[test]
fn register_view_for_view_model_aot_returns_resolver_for_chaining() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();

        let result = resolver.register_view_for_view_model_aot::<TestView, TestViewModel>(None);

        assert!(std::ptr::eq(result, &resolver));
    });
}

// Not ported: RegisterViewForViewModelAOT_ThrowsArgumentNullException_WhenResolverIsNull - argument validation

#[test]
fn register_view_for_view_model_aot_with_contract_registers_view_with_contract() {
    with_app_builder(|| {
        let resolver = ModernDependencyResolver::new();
        let _ = resolver.register_view_for_view_model_aot::<TestView, TestViewModel>(Some("MyContract"));

        let view = resolver.get_service_with_contract::<dyn IViewForOf<TestViewModel>>(Some("MyContract"));

        assert!(view.as_ref().is_some_and(is_test_view));
    });
}
