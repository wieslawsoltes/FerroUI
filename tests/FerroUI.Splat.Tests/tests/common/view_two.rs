//! `ViewTwo`.

use super::{IViewFor, IViewForOf, ViewModelTwo};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// A view of [`ViewModelTwo`].
#[derive(Default)]
pub struct ViewTwo {
    view_model: RefCell<Option<Rc<ViewModelTwo>>>,
}

impl IViewFor for ViewTwo {
    fn view_model_any(&self) -> Option<Rc<dyn Any>> {
        self.view_model.borrow().clone().map(|view_model| view_model as Rc<dyn Any>)
    }

    fn set_view_model_any(&self, view_model: Option<Rc<dyn Any>>) {
        *self.view_model.borrow_mut() = view_model.and_then(|view_model| view_model.downcast::<ViewModelTwo>().ok());
    }
}

impl IViewForOf<ViewModelTwo> for ViewTwo {
    fn view_model(&self) -> Option<Rc<ViewModelTwo>> {
        self.view_model.borrow().clone()
    }

    fn set_view_model(&self, view_model: Option<Rc<ViewModelTwo>>) {
        *self.view_model.borrow_mut() = view_model;
    }
}
