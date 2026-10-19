//! [`IAsyncDisposable`] and [`AsAsyncDisposable`]: the tests of upstream's `item is IDisposable`
//! and `item is IAsyncDisposable`, which `AsyncDisposeMany` makes of the items it tracks.
//!
//! Deviation (DEVIATIONS.md, DynamicData): `IAsyncDisposable` belongs to the base library upstream;
//! neither `ferroui-system` nor `ferroui-reactive` has it, so it lives here until `ferroui-system`
//! does. Its `DisposeAsync` gives a `ValueTask`; here it gives a [`Task<()>`], the task type of
//! `ferroui-system` (docs/porting/reactive.md, section 5.8).

use ferroui_system::threading::tasks::Task;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// An object whose disposal ends later (`IAsyncDisposable`).
pub trait IAsyncDisposable {
    /// Starts the disposal and gives the task that ends with it; an error of the disposal faults
    /// the task (`DisposeAsync`).
    fn dispose_async(&self) -> Task<()>;
}

/// How an item is disposed: at once, or by a task (`item is IDisposable`, `item is
/// IAsyncDisposable`).
#[derive(Clone)]
pub enum ItemDisposal {
    /// The item is disposed at once (`IDisposable`).
    Disposable(Rc<dyn IDisposable>),
    /// The item is disposed by a task (`IAsyncDisposable`).
    AsyncDisposable(Rc<dyn IAsyncDisposable>),
}

/// An item that may be disposable, at once or by a task: `AsyncDisposeMany` disposes the items for
/// which [`to_item_disposal`](AsAsyncDisposable::to_item_disposal) gives a disposal, and leaves the
/// others alone, as upstream disposes the items that implement `IDisposable` or
/// `IAsyncDisposable`.
///
/// Deviation (DEVIATIONS.md, DynamicData): .NET tests the type of an item at run time, and prefers
/// `IDisposable` when an item implements both. Rust has no such test for a generic item, so the
/// type of the items says how they are disposed: an `Rc` of an [`IAsyncDisposable`] is disposed by
/// its task, an `Rc<dyn IDisposable>` at once. Another type implements the trait, or
/// `async_dispose_many_with` takes the function that tells how an item is disposed.
pub trait AsAsyncDisposable {
    /// How the item is disposed, or `None` when it is not disposable.
    fn to_item_disposal(&self) -> Option<ItemDisposal>;
}

impl AsAsyncDisposable for Rc<dyn IDisposable> {
    fn to_item_disposal(&self) -> Option<ItemDisposal> {
        Some(ItemDisposal::Disposable(self.clone()))
    }
}

impl AsAsyncDisposable for Rc<dyn IAsyncDisposable> {
    fn to_item_disposal(&self) -> Option<ItemDisposal> {
        Some(ItemDisposal::AsyncDisposable(self.clone()))
    }
}

impl<X: IAsyncDisposable + 'static> AsAsyncDisposable for Rc<X> {
    fn to_item_disposal(&self) -> Option<ItemDisposal> {
        let disposable: Rc<dyn IAsyncDisposable> = self.clone();
        Some(ItemDisposal::AsyncDisposable(disposable))
    }
}

impl<X: AsAsyncDisposable> AsAsyncDisposable for Option<X> {
    fn to_item_disposal(&self) -> Option<ItemDisposal> {
        self.as_ref().and_then(AsAsyncDisposable::to_item_disposal)
    }
}
