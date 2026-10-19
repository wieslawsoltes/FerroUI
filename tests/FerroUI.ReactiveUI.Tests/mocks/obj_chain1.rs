//! `ObjChain1` (`ReactiveUI.Tests/WhenAny/Mockups/ObjChain1.cs`).

use super::ObjChain2;
use ferroui_reactiveui::{reactive_object, reactive_properties, ReactiveObject};
use std::cell::RefCell;
use std::rc::Rc;

reactive_properties! {
    /// The first link of a chain of nested reactive objects, for the observation of deep
    /// property paths (`ObjChain1`).
    pub struct ObjChain1 {
        reactive: ReactiveObject,
        #[reactive]
        model: Rc<ObjChain2>,
    }
}
reactive_object!(ObjChain1);

impl Default for ObjChain1 {
    /// A link holding a new `ObjChain2` (`Model = new()`).
    fn default() -> Self {
        Self { reactive: ReactiveObject::new(), model: RefCell::new(ObjChain2::new()) }
    }
}

/// Links compare by identity, as the reference types of upstream do.
impl PartialEq for ObjChain1 {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ObjChain1 {
    /// A link holding a new model, shared as view models are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
