//! The backing field of a notifying property: the `ref T backingField` of the setters of .NET
//! (`SetAndRaise`, `RaiseAndSetIfChanged`).

use std::cell::{Cell, RefCell};

/// A backing field that a setter reads and assigns through a shared reference: a `Cell` for a
/// value that is `Copy`, a `RefCell` for the others (`ref T backingField`).
///
/// The setters of the reactive libraries take `&impl BackingField<T>`, so that one setter serves
/// both kinds of field.
///
/// ```
/// use ferroui_system::component_model::BackingField;
/// use std::cell::{Cell, RefCell};
///
/// fn set<T: PartialEq>(field: &impl BackingField<T>, value: T) -> bool {
///     if field.equals(&value, &|a, b| a == b) {
///         return false;
///     }
///     field.store(value);
///     true
/// }
///
/// let age = Cell::new(3);
/// assert!(set(&age, 4));
/// assert!(!set(&age, 4));
/// let name = RefCell::new(String::from("a"));
/// assert!(set(&name, String::from("b")));
/// assert_eq!(*name.borrow(), "b");
/// ```
pub trait BackingField<T> {
    /// Whether the value of the field equals `value` by `comparer`.
    fn equals(&self, value: &T, comparer: &dyn Fn(&T, &T) -> bool) -> bool;
    /// Assigns `value` to the field.
    fn store(&self, value: T);
}

impl<T: Copy> BackingField<T> for Cell<T> {
    fn equals(&self, value: &T, comparer: &dyn Fn(&T, &T) -> bool) -> bool {
        comparer(&self.get(), value)
    }

    fn store(&self, value: T) {
        self.set(value);
    }
}

impl<T> BackingField<T> for RefCell<T> {
    fn equals(&self, value: &T, comparer: &dyn Fn(&T, &T) -> bool) -> bool {
        comparer(&self.borrow(), value)
    }

    fn store(&self, value: T) {
        // The old value is dropped after the borrow ends, so that its destructor may read the
        // field.
        let old = std::mem::replace(&mut *self.borrow_mut(), value);
        drop(old);
    }
}
