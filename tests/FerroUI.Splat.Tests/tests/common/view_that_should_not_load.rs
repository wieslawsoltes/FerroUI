//! `ViewThatShouldNotLoad`.

use super::{IViewFor, IViewForOf, ViewModelOne};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// A view whose construction panics (`InvalidOperationException`), to check that a resolver does
/// not create it.
pub struct ViewThatShouldNotLoad {
    view_model: RefCell<Option<Rc<ViewModelOne>>>,
}

impl ViewThatShouldNotLoad {
    /// Panics: "This view should not be created."
    pub fn new() -> Self {
        panic!("This view should not be created.");
    }
}

impl IViewFor for ViewThatShouldNotLoad {
    fn view_model_any(&self) -> Option<Rc<dyn Any>> {
        self.view_model.borrow().clone().map(|view_model| view_model as Rc<dyn Any>)
    }

    fn set_view_model_any(&self, view_model: Option<Rc<dyn Any>>) {
        *self.view_model.borrow_mut() = view_model.and_then(|view_model| view_model.downcast::<ViewModelOne>().ok());
    }
}

impl IViewForOf<ViewModelOne> for ViewThatShouldNotLoad {
    fn view_model(&self) -> Option<Rc<ViewModelOne>> {
        self.view_model.borrow().clone()
    }

    fn set_view_model(&self, view_model: Option<Rc<ViewModelOne>>) {
        *self.view_model.borrow_mut() = view_model;
    }
}
