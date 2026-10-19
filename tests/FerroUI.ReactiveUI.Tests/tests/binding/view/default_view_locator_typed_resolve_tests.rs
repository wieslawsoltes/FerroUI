//! `DefaultViewLocatorTypedResolveTests` (ReactiveUI.Binding.Tests,
//! `View/DefaultViewLocatorTypedResolveTests.cs`).
//!
//! Upstream's `ResolveView<TViewModel>(string?)` is `resolve_view_for::<VM>(contract)`, which gives an
//! `Rc<dyn IViewForOf<VM>>`; a registration of `IViewFor<TViewModel>` is one of
//! `dyn IViewForOf<VM>`.

use super::{initialize, is_view};
use crate::mocks::panic_message;
use ferroui_reactiveui::*;
use ferroui_splat::{AppLocator, IEnableLogger, MutableDependencyResolverExt};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

const CONTRACT: &str = "compact";

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

/// A view that names no type of view model.
#[derive(Default)]
struct UnrelatedView {
    view_model: RefCell<Option<Rc<dyn Any>>>,
}

impl IActivatableView for UnrelatedView {}

impl IViewFor for UnrelatedView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        self.view_model.borrow().clone()
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        *self.view_model.borrow_mut() = view_model;
    }
}

/// A locator that resolves nothing.
struct EmptyLocator;

impl IEnableLogger for EmptyLocator {}

impl IViewLocator for EmptyLocator {
    fn resolve_view_typed(&self, _view_model: Rc<dyn Any>, _view_model_type: &ViewModelType, _contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        None
    }

    fn resolve_view(&self, _view_model: Option<Rc<dyn Any>>, _contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        None
    }

    fn resolve_view_unsafe(&self, _view_model: Option<Rc<dyn Any>>, _contract: Option<&str>) -> Option<Rc<dyn IViewFor>> {
        None
    }
}

/// Registers `V` as `dyn IViewForOf<TestViewModel>` under `contract`.
fn register_view<V: IViewForOf<TestViewModel> + Default>(contract: Option<&str>) {
    AppLocator::current_mutable()
        .register_with_contract::<dyn IViewForOf<TestViewModel>>(|| Some(Rc::new(V::default()) as Rc<dyn IViewForOf<TestViewModel>>), contract);
}

/// A typed view as an untyped one, to check its type and its view model.
fn untyped(view: Option<Rc<dyn IViewForOf<TestViewModel>>>) -> Option<Rc<dyn IViewFor>> {
    view.map(|view| view as Rc<dyn IViewFor>)
}

/// `ResolveThroughInterface`.
fn resolve_through_interface(locator: &dyn IViewLocator, contract: Option<&str>) -> Option<Rc<dyn IViewForOf<TestViewModel>>> {
    locator.resolve_view_for::<TestViewModel>(contract)
}

#[test]
fn resolve_view_from_a_mapping_returns_the_view_without_a_view_model() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(None);
    register_view::<OtherTestView>(None);

    let result = untyped(locator.resolve_view_for::<TestViewModel>(None));

    assert!(is_view::<TestView>(&result));
    assert!(result.and_then(|view| view.view_model()).is_none());
}

#[test]
fn resolve_view_with_a_contract_uses_the_contracts_mapping() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(Some(CONTRACT));

    let under_contract = untyped(locator.resolve_view_for::<TestViewModel>(Some(CONTRACT)));
    let by_default = locator.resolve_view_for::<TestViewModel>(None);

    assert!(is_view::<TestView>(&under_contract));
    assert!(by_default.is_none());
}

#[test]
fn resolve_view_mapping_of_another_view_type_falls_back_to_the_service_locator() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map_factory::<TestViewModel>(|| Rc::new(UnrelatedView::default()), None);
    register_view::<OtherTestView>(None);

    let result = untyped(locator.resolve_view_for::<TestViewModel>(None));

    assert!(is_view::<OtherTestView>(&result));
}

#[test]
fn resolve_view_without_a_mapping_uses_the_service_locator() {
    initialize();
    let locator = DefaultViewLocator::new();
    register_view::<OtherTestView>(None);

    let result = untyped(locator.resolve_view_for::<TestViewModel>(Some("")));

    assert!(is_view::<OtherTestView>(&result));
    assert!(result.and_then(|view| view.view_model()).is_none());
}

#[test]
fn resolve_view_without_a_mapping_uses_the_service_locators_contract() {
    initialize();
    let locator = DefaultViewLocator::new();
    register_view::<OtherTestView>(Some(CONTRACT));

    let result = untyped(locator.resolve_view_for::<TestViewModel>(Some(CONTRACT)));

    assert!(is_view::<OtherTestView>(&result));
}

#[test]
fn resolve_view_nothing_mapped_or_registered_returns_null() {
    initialize();
    let locator = DefaultViewLocator::new();

    let result = locator.resolve_view_for::<TestViewModel>(None);

    assert!(result.is_none());
}

#[test]
fn interface_resolve_view_default_locator_uses_its_mappings() {
    initialize();
    let default_locator = Rc::new(DefaultViewLocator::new());
    default_locator.map::<TestViewModel, TestView>(Some(CONTRACT));
    let locator: Rc<dyn IViewLocator> = default_locator;

    let by_default = locator.resolve_view_for::<TestViewModel>(None);
    let under_contract = untyped(locator.resolve_view_for::<TestViewModel>(Some(CONTRACT)));

    assert!(by_default.is_none());
    assert!(is_view::<TestView>(&under_contract));
}

/// `InterfaceResolveView_OtherLocator_UsesTheServiceLocator`, for one row of its arguments.
fn interface_resolve_view_other_locator_uses_the_service_locator(contract: Option<&str>) {
    initialize();
    register_view::<OtherTestView>(contract.filter(|contract| !contract.is_empty()));

    let result = untyped(resolve_through_interface(&EmptyLocator, contract));

    assert!(is_view::<OtherTestView>(&result));
}

#[test]
fn interface_resolve_view_other_locator_uses_the_service_locator_null() {
    interface_resolve_view_other_locator_uses_the_service_locator(None);
}

#[test]
fn interface_resolve_view_other_locator_uses_the_service_locator_empty() {
    interface_resolve_view_other_locator_uses_the_service_locator(Some(""));
}

#[test]
fn interface_resolve_view_other_locator_uses_the_service_locator_contract() {
    interface_resolve_view_other_locator_uses_the_service_locator(Some(CONTRACT));
}

// Not ported: InterfaceResolveView_NullLocator_Throws - argument validation

#[test]
fn map_from_service_locator_resolves_the_registered_view() {
    initialize();
    let locator = DefaultViewLocator::new();
    let _ = locator.create_mapping_builder().map_from_service_locator::<TestViewModel, TestView>(None, None);
    AppLocator::current_mutable().register::<TestView>(|| Some(Rc::new(TestView::default())));
    let view_model = Rc::new(TestViewModel);

    let result = locator.resolve_view_of(&view_model, None);

    assert!(is_view::<TestView>(&result));
    assert!(super::displays(&result, &view_model));
}

#[test]
fn map_from_service_locator_with_a_contract_resolves_under_that_contract() {
    initialize();
    let locator = DefaultViewLocator::new();
    let builder = locator.create_mapping_builder();
    AppLocator::current_mutable().register::<TestView>(|| Some(Rc::new(TestView::default())));

    let returned = builder.map_from_service_locator::<TestViewModel, TestView>(Some(CONTRACT), None);

    assert!(std::ptr::eq(returned, &builder));
    assert!(is_view::<TestView>(&untyped(locator.resolve_view_for::<TestViewModel>(Some(CONTRACT)))));
    assert!(locator.resolve_view_for::<TestViewModel>(None).is_none());
}

#[test]
fn map_from_service_locator_unregistered_throws_when_resolved() {
    initialize();
    let locator = DefaultViewLocator::new();
    let _ = locator.create_mapping_builder().map_from_service_locator::<TestViewModel, TestView>(None, None);

    let message = panic_message(|| {
        let _ = locator.resolve_view_for::<TestViewModel>(None);
    });

    assert!(message.contains("InvalidOperationException"));
}

#[test]
fn map_from_service_locator_with_a_service_contract_resolves_the_contracted_registration() {
    initialize();
    let locator = DefaultViewLocator::new();
    register_view::<TestView>(None);
    register_view::<OtherTestView>(Some(CONTRACT));
    let view_model = Rc::new(TestViewModel);

    let _ = locator
        .create_mapping_builder()
        .map_from_service_locator::<TestViewModel, dyn IViewForOf<TestViewModel>>(None, None)
        .map_from_service_locator::<TestViewModel, dyn IViewForOf<TestViewModel>>(Some(CONTRACT), Some(CONTRACT));

    assert!(is_view::<TestView>(&locator.resolve_view_of(&view_model, None)));
    assert!(is_view::<OtherTestView>(&locator.resolve_view_of(&view_model, Some(CONTRACT))));
}

#[test]
fn map_from_service_locator_unregistered_service_contract_throws_naming_the_contract() {
    initialize();
    let locator = DefaultViewLocator::new();
    register_view::<TestView>(None);
    let _ = locator
        .create_mapping_builder()
        .map_from_service_locator::<TestViewModel, dyn IViewForOf<TestViewModel>>(Some(CONTRACT), Some(CONTRACT));

    let message = panic_message(|| {
        let _ = locator.resolve_view_for::<TestViewModel>(Some(CONTRACT));
    });

    assert!(message.contains("InvalidOperationException"));
    assert!(message.contains(&format!("'{CONTRACT}'")));
}
