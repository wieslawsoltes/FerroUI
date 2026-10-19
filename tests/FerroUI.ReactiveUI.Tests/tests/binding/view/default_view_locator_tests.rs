//! `DefaultViewLocatorTests` (ReactiveUI.Binding.Tests, `View/DefaultViewLocatorTests.cs`).
//!
//! A view model of a known type is resolved by `resolve_view_of` (upstream's
//! `ResolveView<TViewModel>(TViewModel)`), one held as `dyn Any` by `resolve_view`
//! (`ResolveView(object?)`).

use super::{dispatch, displays, initialize, is_same_view, is_view};
use ferroui_reactiveui::*;
use ferroui_splat::{AppLocator, MutableDependencyResolverExt};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct TestViewModel {
    name: RefCell<Option<String>>,
}

impl TestViewModel {
    fn named(name: &str) -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(Some(name.to_owned())) })
    }
}

#[derive(Default)]
struct TestView {
    view_model: RefCell<Option<Rc<TestViewModel>>>,
}
view_for!(TestView, TestViewModel, view_model);

fn new_view_model() -> Rc<TestViewModel> {
    Rc::new(TestViewModel::default())
}

fn as_any(view_model: &Rc<TestViewModel>) -> Option<Rc<dyn Any>> {
    Some(view_model.clone())
}

#[test]
fn resolve_view_null_view_model_returns_null() {
    initialize();
    let locator = DefaultViewLocator::new();

    let result = locator.resolve_view(None, None);

    assert!(result.is_none());
}

// Not ported: ResolveViewGeneric_NullViewModel_ReturnsNull - argument validation (a view model of a known type is an `Rc`, never null)

#[test]
fn map_then_resolve_returns_view() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(None);

    let result = locator.resolve_view_of(&new_view_model(), None);

    assert!(result.is_some());
    assert!(is_view::<TestView>(&result));
}

#[test]
fn map_with_factory_then_resolve_uses_factory() {
    initialize();
    let locator = DefaultViewLocator::new();
    let factory_called = Rc::new(Cell::new(false));
    let called = factory_called.clone();
    locator.map_factory::<TestViewModel>(
        move || {
            called.set(true);
            Rc::new(TestView::default())
        },
        None,
    );

    let result = locator.resolve_view_of(&new_view_model(), None);

    assert!(factory_called.get());
    assert!(result.is_some());
}

#[test]
fn resolve_view_generic_with_mapping_returns_view() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(None);

    let view_model = new_view_model();
    let result = locator.resolve_view_of(&view_model, None);

    assert!(result.is_some());
    assert!(displays(&result, &view_model));
}

#[test]
fn unmap_removes_mapping() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(None);

    let removed = locator.unmap::<TestViewModel>(None);
    let result = locator.resolve_view_of(&new_view_model(), None);

    assert!(removed);
    assert!(result.is_none());
}

#[test]
fn unmap_no_mapping_returns_false() {
    initialize();
    let locator = DefaultViewLocator::new();

    let removed = locator.unmap::<TestViewModel>(None);

    assert!(!removed);
}

#[test]
fn map_with_contract_resolves_by_contract() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(Some("desktop"));

    let with_contract = locator.resolve_view_of(&new_view_model(), Some("desktop"));
    let without_contract = locator.resolve_view_of(&new_view_model(), None);

    assert!(with_contract.is_some());
    assert!(without_contract.is_none());
}

#[test]
fn resolve_view_sets_view_model_on_view() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(None);

    let view_model = TestViewModel::named("test");
    let view = locator.resolve_view_of(&view_model, None);

    assert!(view.is_some());
    assert!(displays(&view, &view_model));
    assert_eq!(view_model.name.borrow().as_deref(), Some("test"));
}

#[test]
fn set_generated_view_dispatch_is_used_by_resolve() {
    initialize();
    let locator = DefaultViewLocator::new();
    let dispatch_called = Rc::new(Cell::new(false));
    let called = dispatch_called.clone();

    DefaultViewLocator::set_generated_view_dispatch(dispatch(move |_, _| {
        called.set(true);
        Some(Rc::new(TestView::default()))
    }));

    let result = locator.resolve_view_of(&new_view_model(), None);

    assert!(dispatch_called.get());
    assert!(result.is_some());
}

#[test]
fn generated_dispatch_takes_priority_over_mappings() {
    initialize();
    let locator = DefaultViewLocator::new();
    let generated_view = Rc::new(TestView::default());
    locator.map_factory::<TestViewModel>(|| Rc::new(TestView::default()), None);

    let returned = generated_view.clone();
    DefaultViewLocator::set_generated_view_dispatch(dispatch(move |_, _| Some(returned.clone())));

    let result = locator.resolve_view_of(&new_view_model(), None);

    assert!(is_same_view(&result, &generated_view));
}

#[test]
fn generated_dispatch_returns_null_falls_back_to_mappings() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(None);

    DefaultViewLocator::set_generated_view_dispatch(dispatch(|_, _| None));

    let result = locator.resolve_view_of(&new_view_model(), None);

    assert!(result.is_some());
    assert!(is_view::<TestView>(&result));
}

#[test]
fn resolve_view_generic_uses_generated_dispatch() {
    initialize();
    let locator = DefaultViewLocator::new();
    let generated_view = Rc::new(TestView::default());

    let returned = generated_view.clone();
    DefaultViewLocator::set_generated_view_dispatch(dispatch(move |_, _| Some(returned.clone())));

    let result = locator.resolve_view_of(&new_view_model(), None);

    assert!(is_same_view(&result, &generated_view));
}

#[test]
fn resolve_view_null_contract_normalized_to_empty() {
    initialize();
    let locator = DefaultViewLocator::new();
    locator.map::<TestViewModel, TestView>(None);

    let with_null = locator.resolve_view_of(&new_view_model(), None);
    let with_empty = locator.resolve_view_of(&new_view_model(), Some(""));

    assert!(with_null.is_some());
    assert!(with_empty.is_some());
}

// Not ported: SetGeneratedViewDispatch_NullThrows - argument validation

// Not ported: MapWithFactory_NullFactory_Throws - argument validation

#[test]
fn resolve_view_generic_service_locator_fallback() {
    initialize();
    let locator = DefaultViewLocator::new();
    let view_model = new_view_model();

    // An IViewFor<TestViewModel> in the service locator.
    AppLocator::current_mutable().register::<dyn IViewForOf<TestViewModel>>(|| Some(Rc::new(TestView::default()) as Rc<dyn IViewForOf<TestViewModel>>));

    let result = locator.resolve_view_of(&view_model, None);

    assert!(result.is_some());
    assert!(is_view::<TestView>(&result));
    assert!(displays(&result, &view_model));
}

#[test]
fn resolve_view_generic_service_locator_with_contract() {
    initialize();
    let locator = DefaultViewLocator::new();
    let view_model = new_view_model();

    AppLocator::current_mutable()
        .register_with_contract::<dyn IViewForOf<TestViewModel>>(|| Some(Rc::new(TestView::default()) as Rc<dyn IViewForOf<TestViewModel>>), Some("custom"));

    let result = locator.resolve_view_of(&view_model, Some("custom"));

    assert!(result.is_some());
    assert!(displays(&result, &view_model));
}

#[test]
fn resolve_view_non_generic_generated_dispatch_sets_view_model() {
    initialize();
    let locator = DefaultViewLocator::new();
    let view_model = TestViewModel::named("dispatched");
    let dispatch_view = Rc::new(TestView::default());

    let returned = dispatch_view.clone();
    DefaultViewLocator::set_generated_view_dispatch(dispatch(move |_, _| Some(returned.clone())));

    let result = locator.resolve_view(as_any(&view_model), None);

    assert!(result.is_some());
    assert!(displays(&result, &view_model));
}

#[test]
fn resolve_view_non_generic_mapping_fallback_sets_view_model() {
    initialize();
    let locator = DefaultViewLocator::new();
    let view_model = TestViewModel::named("mapped");
    locator.map::<TestViewModel, TestView>(None);

    DefaultViewLocator::set_generated_view_dispatch(dispatch(|_, _| None));

    let result = locator.resolve_view(as_any(&view_model), None);

    assert!(result.is_some());
    assert!(displays(&result, &view_model));
}

#[test]
fn resolve_view_non_generic_no_match_returns_null() {
    initialize();
    let locator = DefaultViewLocator::new();

    let result = locator.resolve_view(as_any(&new_view_model()), None);

    assert!(result.is_none());
}

#[test]
fn create_mapping_builder_registers_on_locator() {
    initialize();
    let locator = DefaultViewLocator::new();
    let builder = locator.create_mapping_builder();

    let _ = builder.map::<TestViewModel, TestView>(None);

    let view_model = new_view_model();
    let result = locator.resolve_view_of(&view_model, None);

    assert!(result.is_some());
    assert!(is_view::<TestView>(&result));
}
