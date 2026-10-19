//! `SourceListEx` (`List/SourceListEx.cs`), with the operators of `ObservableListEx` whose receiver
//! is a list rather than a sequence of change sets: `DeferUntilLoaded` on `IObservableList`,
//! `ExpireAfter` and `LimitSizeTo` on `ISourceList`.
//!
//! The trait is implemented for every observable list; the methods of a source list require it to
//! be one, and those that keep the list in a subscription a handle that clones.

use super::{ChangeSet, IObservableList, ISourceList};
use std::rc::Rc;

/// The operators of lists (`SourceListEx`, and the extension methods of `ObservableListEx` on
/// `IObservableList` and `ISourceList`).
pub trait SourceListEx<T: Clone + PartialEq + 'static>: IObservableList<T> {
    /// The items and their changes, each item converted by `conversion_factory`
    /// (`Cast(ISourceList<TSource>, Func<TSource, TDestination>)`).
    fn cast_list<U: Clone + 'static>(&self, conversion_factory: impl Fn(&T) -> U + 'static) -> Rc<dyn ferroui_system::IObservable<ChangeSet<U>>>
    where
        Self: ISourceList<T>,
    {
        ferroui_reactive::ObservableExt::select(&IObservableList::connect(self), move |changes: ChangeSet<T>| super::transform_change_set(&changes, &conversion_factory))
    }

    super::ops::defer_until_loaded::defer_until_loaded_source_methods!();
    super::ops::expire_after::expire_after_source_methods!();
    super::ops::limit_size_to::limit_size_to_source_methods!();
}

impl<T: Clone + PartialEq + 'static, L: IObservableList<T> + ?Sized> SourceListEx<T> for L {}
