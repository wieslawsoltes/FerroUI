//! `AmountView` (`TestModels/AmountView.cs`).

use ferroui_reactiveui::{IActivatableView, IViewFor};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The writes after which a binding that keeps writing is stopped.
const WRITE_LIMIT: usize = 1_000;

/// A view whose `Text` counts its writes and panics past a thousand, so that a binding that never
/// settles fails instead of running forever (`AmountView`).
pub struct AmountView {
    text: RefCell<String>,
    writes: Cell<usize>,
    view_model: RefCell<Option<Rc<dyn Any>>>,
    property_changed: Event<str>,
}

#[allow(dead_code)]
impl AmountView {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { text: RefCell::new(String::new()), writes: Cell::new(0), view_model: RefCell::new(None), property_changed: Event::new() })
    }

    /// The number of writes of `Text` (`Writes`).
    pub fn writes(&self) -> usize {
        self.writes.get()
    }

    pub fn text(&self) -> String {
        self.text.borrow().clone()
    }

    pub fn set_text(&self, value: String) {
        self.writes.set(self.writes.get() + 1);
        if self.writes.get() > WRITE_LIMIT {
            panic!("The binding kept writing: {value}");
        }
        *self.text.borrow_mut() = value;
        self.property_changed.raise("Text");
    }
}

impl INotifyPropertyChanged for AmountView {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl IActivatableView for AmountView {}

impl IViewFor for AmountView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        self.view_model.borrow().clone()
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        *self.view_model.borrow_mut() = view_model;
    }
}
