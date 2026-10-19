//! `ViewMappingBuilderTests` (ReactiveUI.Binding.Tests, `View/ViewMappingBuilderTests.cs`).

use super::{initialize, is_view};
use ferroui_reactiveui::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct TestViewModel;

#[derive(Default)]
struct OtherViewModel;

#[derive(Default)]
struct TestView {
    view_model: RefCell<Option<Rc<TestViewModel>>>,
}
view_for!(TestView, TestViewModel, view_model);

#[derive(Default)]
struct OtherView {
    view_model: RefCell<Option<Rc<OtherViewModel>>>,
}
view_for!(OtherView, OtherViewModel, view_model);

#[test]
fn map_registers_mapping() {
    initialize();
    let locator = DefaultViewLocator::new();
    let builder = ViewMappingBuilder::new(&locator);

    let _ = builder.map::<TestViewModel, TestView>(None);

    let result = locator.resolve_view_of(&Rc::new(TestViewModel), None);
    assert!(result.is_some());
    assert!(is_view::<TestView>(&result));
}

#[test]
fn map_returns_self_for_chaining() {
    initialize();
    let locator = DefaultViewLocator::new();
    let builder = ViewMappingBuilder::new(&locator);

    let returned = builder.map::<TestViewModel, TestView>(None);

    assert!(std::ptr::eq(returned, &builder));
}

#[test]
fn map_multiple_chained() {
    initialize();
    let locator = DefaultViewLocator::new();
    let builder = ViewMappingBuilder::new(&locator);

    let _ = builder.map::<TestViewModel, TestView>(None).map::<OtherViewModel, OtherView>(None);

    let result1 = locator.resolve_view_of(&Rc::new(TestViewModel), None);
    let result2 = locator.resolve_view_of(&Rc::new(OtherViewModel), None);

    assert!(result1.is_some());
    assert!(result2.is_some());
}

#[test]
fn map_with_factory_registers_factory() {
    initialize();
    let locator = DefaultViewLocator::new();
    let builder = ViewMappingBuilder::new(&locator);
    let factory_called = Rc::new(Cell::new(false));
    let called = factory_called.clone();

    let _ = builder.map_factory::<TestViewModel>(
        move || {
            called.set(true);
            Rc::new(TestView::default())
        },
        None,
    );

    let _ = locator.resolve_view_of(&Rc::new(TestViewModel), None);

    assert!(factory_called.get());
}
