//! `DispatchStubView` (`TestModels/DispatchStubView.cs`).
//!
//! The controls of the command bindings (`Control`, `Surface`) are left out: the tests that use
//! them belong to the command binding.

use ferroui_reactiveui::{IActivatableView, IViewFor};
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// A view that names no type of view model, whose `Caption` raises `PropertyChanged` on every
/// write (`DispatchStubView`). Its view model does not notify.
pub struct DispatchStubView {
    caption: RefCell<String>,
    view_model: RefCell<Option<Rc<dyn Any>>>,
    property_changed: Event<str>,
}

#[allow(dead_code)]
impl DispatchStubView {
    /// The view with the caption `"a"`.
    pub fn new() -> Rc<Self> {
        Rc::new(Self { caption: RefCell::new("a".to_string()), view_model: RefCell::new(None), property_changed: Event::new() })
    }

    /// The view with `caption`.
    pub fn with_caption(caption: &str) -> Rc<Self> {
        let view = Self::new();
        *view.caption.borrow_mut() = caption.to_string();
        view
    }

    pub fn caption(&self) -> String {
        self.caption.borrow().clone()
    }

    pub fn set_caption(&self, value: String) {
        *self.caption.borrow_mut() = value;
        self.property_changed.raise("Caption");
    }
}

impl INotifyPropertyChanged for DispatchStubView {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl IActivatableView for DispatchStubView {}

impl IViewFor for DispatchStubView {
    fn view_model(&self) -> Option<Rc<dyn Any>> {
        self.view_model.borrow().clone()
    }

    fn set_view_model(&self, view_model: Option<Rc<dyn Any>>) {
        *self.view_model.borrow_mut() = view_model;
    }
}
