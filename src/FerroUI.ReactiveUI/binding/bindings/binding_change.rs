//! `BindingChange`.

use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// A value a two-way binding moved, and which side of the binding produced it (`BindingChange`).
///
/// A two-way binding's change stream carries both directions, so a subscriber that acted on every
/// value without knowing where it came from would act twice on its own write. The flag is what
/// separates an edit from the echo of one.
///
/// The value is type-erased, as upstream's `object? Value`: a change from the view model carries
/// the value written to the view, a change from the view the value written to the view model, and
/// the two may differ in type. [`value_as`](Self::value_as) reads it back.
#[derive(Clone)]
pub struct BindingChange {
    /// The value that was written (`Value`).
    pub value: Rc<dyn Any>,
    /// Whether the view model produced the value, rather than the view (`FromViewModel`).
    pub from_view_model: bool,
}

impl BindingChange {
    /// A change of `value`, produced by the view model when `from_view_model` is `true`.
    pub fn new<T: 'static>(value: T, from_view_model: bool) -> Self {
        Self { value: Rc::new(value), from_view_model }
    }

    /// The value, when it is a `T`.
    pub fn value_as<T: 'static>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }
}

impl fmt::Debug for BindingChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side = if self.from_view_model { "ViewModel" } else { "View" };
        write!(f, "BindingChange: {side} -> {:p}", Rc::as_ptr(&self.value))
    }
}
