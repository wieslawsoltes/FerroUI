//! `ObjChain3` (`ReactiveUI.Tests/WhenAny/Mockups/ObjChain3.cs`).

use super::HostTestFixture;
use ferroui_reactiveui::{reactive_object, reactive_properties, ReactiveObject};
use std::cell::RefCell;
use std::rc::Rc;

reactive_properties! {
    /// The third link of a chain of nested reactive objects, for the observation of deep
    /// property paths (`ObjChain3`).
    pub struct ObjChain3 {
        reactive: ReactiveObject,
        #[reactive]
        model: Rc<HostTestFixture>,
    }
}
reactive_object!(ObjChain3);

impl Default for ObjChain3 {
    /// A link holding a new `HostTestFixture` (`Model = new()`).
    fn default() -> Self {
        Self { reactive: ReactiveObject::new(), model: RefCell::new(HostTestFixture::new()) }
    }
}

/// Links compare by identity, as the reference types of upstream do.
impl PartialEq for ObjChain3 {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ObjChain3 {
    /// A link holding a new model, shared as view models are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
