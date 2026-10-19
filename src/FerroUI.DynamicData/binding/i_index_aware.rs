//! `IIndexAware` (`Binding/IIndexAware.cs`).

use std::rc::Rc;

/// An item that knows its index in a sorted collection (`IIndexAware`), which `UpdateIndex` sets.
///
/// Deviation (DEVIATIONS.md, DynamicData): the setter takes `&self`, as the items are shared
/// (`Rc`) and keep the index in a `Cell`; the index is a `usize`.
pub trait IIndexAware {
    /// The index (`Index`).
    fn index(&self) -> usize;
    /// Sets the index (`Index`).
    fn set_index(&self, index: usize);
}

impl<X: IIndexAware + ?Sized> IIndexAware for Rc<X> {
    fn index(&self) -> usize {
        (**self).index()
    }

    fn set_index(&self, index: usize) {
        (**self).set_index(index);
    }
}
