//! `DefaultViewLocatorObjectResolveTests` (ReactiveUI.Binding.Tests,
//! `View/DefaultViewLocatorObjectResolveTests.cs`).
//!
//! A view model held as an `object` is an `Rc<dyn Any>`. A view registered in the service locator
//! for `resolve_view_unsafe` is registered with `register_view_for`, which records the view model
//! type, as upstream's `ResolveViewUnsafe` closes `IViewFor<>` over the run-time type by
//! reflection.

use super::{dispatch, displays, initialize, is_same_view, is_view};
use crate::mocks::register_test_logger;
use ferroui_reactiveui::*;
use ferroui_splat::{AppLocator, LogLevel};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const CONTRACT: &str = "compact";

const VALUE_TYPE_VIEW_MODEL: i32 = 42;

#[derive(Default)]
struct TestViewModel;

#[derive(Default)]
struct TestView {
    view_model: RefCell<Option<Rc<TestViewModel>>>,
}
view_for!(TestView, TestViewModel, view_model);

#[derive(Default)]
struct OtherTestView {
    view_model: RefCell<Option<Rc<TestViewModel>>>,
}
view_for!(OtherTestView, TestViewModel, view_model);

fn object(view_model: &Rc<TestViewModel>) -> Option<Rc<dyn Any>> {
    Some(view_model.clone())
}

/// Registers `V` for the view model type in the service locator under `contract`, counting the
/// views it creates in `calls`.
fn register_view<V: IViewForOf<TestViewModel> + Default>(contract: Option<&str>, calls: Rc<Cell<usize>>) {
    AppLocator::current_mutable().register_view_for::<TestViewModel>(
        move || {
            calls.set(calls.get() + 1);
            Rc::new(V::default())
        },
        contract,
    );
}

#[test]
fn resolve_view_generated_lookup_returns_the_view_with_its_view_model() {
    initialize();
    let generated_view = Rc::new(TestView::default());
    let returned = generated_view.clone();
    DefaultViewLocator::set_generated_view_dispatch(dispatch(move |view_model, contract| {
        (view_model.is::<TestViewModel>() && contract.is_empty()).then(|| returned.clone() as Rc<dyn IViewFor>)
    }));
    let view_model = Rc::new(TestViewModel);

    let result = DefaultViewLocator::new().resolve_view(object(&view_model), None);

    assert!(is_same_view(&result, &generated_view));
    assert!(displays(&result, &view_model));
}

/// `ResolveView_MappingForTheRuntimeType_ReturnsTheView`, for one row of its arguments.
fn resolve_view_mapping_for_the_runtime_type_returns_the_view(contract: Option<&str>) {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(contract);
    let view_model = Rc::new(TestViewModel);

    let result = locator.resolve_view(object(&view_model), contract);

    assert!(is_view::<TestView>(&result));
    assert!(displays(&result, &view_model));
}

#[test]
fn resolve_view_mapping_for_the_runtime_type_returns_the_view_null() {
    resolve_view_mapping_for_the_runtime_type_returns_the_view(None);
}

#[test]
fn resolve_view_mapping_for_the_runtime_type_returns_the_view_contract() {
    resolve_view_mapping_for_the_runtime_type_returns_the_view(Some(CONTRACT));
}

#[test]
fn resolve_view_mapping_under_another_contract_returns_null() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(Some(CONTRACT));
    let view_model = Rc::new(TestViewModel);

    let result = locator.resolve_view(object(&view_model), None);

    assert!(result.is_none());
}

/// `ResolveView_ServiceLocatorOnlyRegistration_ReturnsNullWithoutAskingTheServiceLocator`, for one
/// row of its arguments.
fn resolve_view_service_locator_only_registration_returns_null_without_asking_the_service_locator(contract: Option<&str>) {
    initialize();
    let factory_calls = Rc::new(Cell::new(0));
    register_view::<TestView>(contract, factory_calls.clone());
    let view_model = Rc::new(TestViewModel);

    let result = DefaultViewLocator::new().resolve_view(object(&view_model), contract);

    assert!(result.is_none());
    assert_eq!(factory_calls.get(), 0);
}

#[test]
fn resolve_view_service_locator_only_registration_returns_null_without_asking_the_service_locator_null() {
    resolve_view_service_locator_only_registration_returns_null_without_asking_the_service_locator(None);
}

#[test]
fn resolve_view_service_locator_only_registration_returns_null_without_asking_the_service_locator_contract() {
    resolve_view_service_locator_only_registration_returns_null_without_asking_the_service_locator(Some(CONTRACT));
}

/// `ResolveViewUnsafe_ServiceLocatorOnlyRegistration_ReturnsTheView`, for one row of its
/// arguments.
fn resolve_view_unsafe_service_locator_only_registration_returns_the_view(contract: Option<&str>) {
    initialize();
    register_view::<TestView>(contract, Rc::new(Cell::new(0)));
    let view_model = Rc::new(TestViewModel);

    let result = DefaultViewLocator::new().resolve_view_unsafe(object(&view_model), contract);

    assert!(is_view::<TestView>(&result));
    assert!(displays(&result, &view_model));
}

#[test]
fn resolve_view_unsafe_service_locator_only_registration_returns_the_view_null() {
    resolve_view_unsafe_service_locator_only_registration_returns_the_view(None);
}

#[test]
fn resolve_view_unsafe_service_locator_only_registration_returns_the_view_contract() {
    resolve_view_unsafe_service_locator_only_registration_returns_the_view(Some(CONTRACT));
}

#[test]
fn resolve_view_unsafe_registration_under_another_contract_returns_null() {
    initialize();
    register_view::<TestView>(Some(CONTRACT), Rc::new(Cell::new(0)));
    let view_model = Rc::new(TestViewModel);

    let result = DefaultViewLocator::new().resolve_view_unsafe(object(&view_model), None);

    assert!(result.is_none());
}

#[test]
fn resolve_view_unsafe_mapping_and_registration_prefers_the_mapping() {
    initialize();
    let factory_calls = Rc::new(Cell::new(0));
    register_view::<OtherTestView>(None, factory_calls.clone());
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(None);
    let view_model = Rc::new(TestViewModel);

    let result = locator.resolve_view_unsafe(object(&view_model), None);

    assert!(is_view::<TestView>(&result));
    assert_eq!(factory_calls.get(), 0);
}

#[test]
fn resolve_view_unsafe_generated_lookup_returns_the_generated_view() {
    initialize();
    let generated_view = Rc::new(TestView::default());
    let returned = generated_view.clone();
    DefaultViewLocator::set_generated_view_dispatch(dispatch(move |view_model, _| {
        view_model.is::<TestViewModel>().then(|| returned.clone() as Rc<dyn IViewFor>)
    }));
    register_view::<OtherTestView>(None, Rc::new(Cell::new(0)));
    let view_model = Rc::new(TestViewModel);

    let result = DefaultViewLocator::new().resolve_view_unsafe(object(&view_model), None);

    assert!(is_same_view(&result, &generated_view));
    assert!(displays(&result, &view_model));
}

#[test]
fn resolve_view_unsafe_null_view_model_returns_null() {
    initialize();

    let result = DefaultViewLocator::new().resolve_view_unsafe(None, None);

    assert!(result.is_none());
}

#[test]
fn resolve_view_unsafe_nothing_mapped_or_registered_returns_null() {
    initialize();

    let result = DefaultViewLocator::new().resolve_view_unsafe(object(&Rc::new(TestViewModel)), None);

    assert!(result.is_none());
}

#[test]
fn resolve_view_unsafe_value_type_view_model_returns_null() {
    initialize();

    let result = DefaultViewLocator::new().resolve_view_unsafe(Some(Rc::new(VALUE_TYPE_VIEW_MODEL) as Rc<dyn Any>), None);

    assert!(result.is_none());
}

#[test]
fn resolve_view_unsafe_extension_uses_the_default_contract() {
    initialize();
    register_view::<TestView>(None, Rc::new(Cell::new(0)));
    let locator = DefaultViewLocator::new();
    let view_model = Rc::new(TestViewModel);

    let result = locator.resolve_view_unsafe(object(&view_model), None);

    assert!(is_view::<TestView>(&result));
}

// Not ported: ResolveViewUnsafeExtension_NullLocator_Throws - argument validation

// Not ported: ResolveViewExtension_NullLocator_Throws - argument validation

#[test]
fn resolve_view_no_generated_or_mapped_view_logs_a_warning() {
    initialize();
    let logger = register_test_logger();
    // The type of a view model held as `dyn Any` is named when it is known to the locator.
    DefaultViewLocator::register_view_model_type::<TestViewModel>();
    let view_model = Rc::new(TestViewModel);
    let locator = DefaultViewLocator::new();

    let _ = locator.resolve_view_unsafe(object(&view_model), None);
    let after_unsafe = logger.count(LogLevel::Warn);
    let _ = locator.resolve_view(object(&view_model), None);

    let warnings: Vec<String> = logger.messages().into_iter().filter(|(_, _, level)| *level == LogLevel::Warn).map(|(message, _, _)| message).collect();
    assert_eq!(after_unsafe, 0);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("TestViewModel"));
    assert!(warnings[0].contains("ResolveViewUnsafe"));
}
