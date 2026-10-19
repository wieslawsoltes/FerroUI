//! `ReactiveBindingTests` (`ReactiveUI.Binding.Tests/Bindings/ReactiveBindingTests.cs`).

use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::linq::Observable;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObservable};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A minimal view (`FakeView`).
#[derive(Default)]
struct FakeView {
    view_model: RefCell<Option<Rc<dyn Any>>>,
}

impl IActivatableView for FakeView {}

impl IViewFor for FakeView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        self.view_model.borrow().clone()
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        *self.view_model.borrow_mut() = view_model;
    }
}

#[test]
fn dispose_disposes_subscription() {
    let disposed = Rc::new(Cell::new(false));
    let flag = disposed.clone();
    let subscription = Disposable::create(move || flag.set(true));
    let view = Rc::new(FakeView::default());
    let changed = Observable::empty::<String>();

    let binding = ReactiveBinding::new(view, changed, BindingDirection::OneWay, subscription);

    binding.dispose();

    assert!(disposed.get());
}

/// Counts every disposal, the repeated ones too (`Disposable.Create(disposeCount, state => state.Value++)`
/// with a counter).
struct CountingDisposable(Rc<Cell<usize>>);

impl IDisposable for CountingDisposable {
    fn dispose(&self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn dispose_twice_does_not_throw() {
    let dispose_count = Rc::new(Cell::new(0));
    let subscription: Rc<dyn IDisposable> = Rc::new(CountingDisposable(dispose_count.clone()));
    let view = Rc::new(FakeView::default());
    let changed = Observable::empty::<String>();

    let binding = ReactiveBinding::new(view, changed, BindingDirection::OneWay, subscription);

    binding.dispose();
    binding.dispose();

    assert_eq!(dispose_count.get(), 1);
}

#[test]
fn properties_return_constructor_values() {
    let view = Rc::new(FakeView::default());
    let changed: Rc<dyn IObservable<i32>> = Observable::empty::<i32>();

    let binding = ReactiveBinding::new(view.clone(), changed.clone(), BindingDirection::TwoWay, Disposable::empty());

    assert!(Rc::ptr_eq(binding.view(), &view));
    assert_eq!(binding.direction(), BindingDirection::TwoWay);
    assert!(std::ptr::addr_eq(Rc::as_ptr(&binding.changed()), Rc::as_ptr(&changed)));
    assert!(binding.view_model_expression().is_none());
    assert!(binding.view_expression().is_none());
}
