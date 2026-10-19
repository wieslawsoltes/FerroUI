//! [`AsDisposable`]: the test of upstream's `item as IDisposable`, which `DisposeMany` and
//! `KeyedDisposable` make of the items they track.

use ferroui_system::IDisposable;
use std::rc::Rc;

/// An item that may be a disposable (`item as IDisposable`): `DisposeMany` disposes the items for
/// which [`to_disposable`](AsDisposable::to_disposable) gives a disposable, and leaves the others
/// alone, as upstream disposes the items that implement `IDisposable`.
///
/// Deviation (DEVIATIONS.md, DynamicData): .NET tests the type of an item at run time; Rust has no
/// such test for a generic item, so the type of the items says whether they are disposable. Every
/// `Rc` of a disposable is one (`Rc<dyn IDisposable>` and `Rc<X>` for `X: IDisposable`); another
/// type implements the trait, or `dispose_many_with` takes the function that disposes an item.
pub trait AsDisposable {
    /// The item as a disposable, or `None` when it is not one.
    fn to_disposable(&self) -> Option<Rc<dyn IDisposable>>;
}

impl AsDisposable for Rc<dyn IDisposable> {
    fn to_disposable(&self) -> Option<Rc<dyn IDisposable>> {
        Some(self.clone())
    }
}

impl<X: IDisposable + 'static> AsDisposable for Rc<X> {
    fn to_disposable(&self) -> Option<Rc<dyn IDisposable>> {
        let disposable: Rc<dyn IDisposable> = self.clone();
        Some(disposable)
    }
}

impl<X: AsDisposable> AsDisposable for Option<X> {
    fn to_disposable(&self) -> Option<Rc<dyn IDisposable>> {
        self.as_ref().and_then(AsDisposable::to_disposable)
    }
}
