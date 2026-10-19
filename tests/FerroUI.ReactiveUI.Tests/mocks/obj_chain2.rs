//! `ObjChain2` (`ReactiveUI.Tests/WhenAny/Mockups/ObjChain2.cs`).

use super::ObjChain3;
use ferroui_reactiveui::{reactive_object, reactive_properties, ReactiveObject};
use std::cell::RefCell;
use std::rc::Rc;

reactive_properties! {
    /// The second link of a chain of nested reactive objects, for the observation of deep
    /// property paths (`ObjChain2`).
    pub struct ObjChain2 {
        reactive: ReactiveObject,
        #[reactive]
        model: Rc<ObjChain3>,
    }
}
reactive_object!(ObjChain2);

impl Default for ObjChain2 {
    /// A link holding a new `ObjChain3` (`Model = new()`).
    fn default() -> Self {
        Self { reactive: ReactiveObject::new(), model: RefCell::new(ObjChain3::new()) }
    }
}

/// Links compare by identity, as the reference types of upstream do.
impl PartialEq for ObjChain2 {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ObjChain2 {
    /// A link holding a new model, shared as view models are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}
