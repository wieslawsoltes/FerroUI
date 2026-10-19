//! `OwnerClass` (`ReactiveUI.Tests/WhenAny/Mockups/OwnerClass.cs`).

use ferroui_reactiveui::{reactive_object, reactive_properties, ReactiveObject};
use std::rc::Rc;

reactive_properties! {
    /// An owner with a name (`OwnerClass`).
    #[derive(Default)]
    pub struct OwnerClass {
        reactive: ReactiveObject,
        #[reactive]
        name: Option<String>,
    }
}
reactive_object!(OwnerClass);

/// Owners compare by identity, as the reference types of upstream do
/// (`EqualityComparer<OwnerClass>.Default`).
impl PartialEq for OwnerClass {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl OwnerClass {
    /// An owner without a name, shared as view models are.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// An owner named `name` (`new() { Name = .. }`).
    pub fn with_name(name: &str) -> Rc<Self> {
        let owner = Self::new();
        owner.set_name(Some(name.to_owned()));
        owner
    }
}
