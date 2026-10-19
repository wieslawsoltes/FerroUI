//! `OrderedComparer` (`ReactiveUI.Core/Comparers/OrderedComparer.cs`), the builder of comparers
//! for a type that the caller infers from a collection, with its `OrderedComparerTypeWrapper<T>`.

use super::{Comparer, IComparerBuilder, OrderedComparer};
use std::marker::PhantomData;

impl<T: 'static> OrderedComparer<T> {
    /// The builder of comparers of the elements of `items`, whose type it infers; useful for the
    /// types that have no name, such as tuples (`For<T>(IEnumerable<T>)`).
    pub fn for_items(_items: &[T]) -> OrderedComparerTypeWrapper<T> {
        Self::for_type()
    }

    /// The builder of comparers of `T` (`For<T>()`). The builder is not a comparer itself: its
    /// `order_by` methods give one.
    pub fn for_type() -> OrderedComparerTypeWrapper<T> {
        OrderedComparerTypeWrapper(PhantomData)
    }
}

/// The builder of comparers that [`OrderedComparer::for_type`] gives (`OrderedComparerTypeWrapper<T>`):
/// it forwards to the functions of [`OrderedComparer`].
pub struct OrderedComparerTypeWrapper<T>(PhantomData<fn(&T)>);

impl<T> Clone for OrderedComparerTypeWrapper<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for OrderedComparerTypeWrapper<T> {}

impl<T: 'static> IComparerBuilder<T> for OrderedComparerTypeWrapper<T> {
    fn order_by<V: Ord + 'static>(&self, selector: impl Fn(&T) -> V + 'static) -> Comparer<T> {
        OrderedComparer::<T>::order_by(selector)
    }

    fn order_by_with<V: 'static>(&self, selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T> {
        OrderedComparer::<T>::order_by_with(selector, comparer)
    }

    fn order_by_descending<V: Ord + 'static>(&self, selector: impl Fn(&T) -> V + 'static) -> Comparer<T> {
        OrderedComparer::<T>::order_by_descending(selector)
    }

    fn order_by_descending_with<V: 'static>(&self, selector: impl Fn(&T) -> V + 'static, comparer: Comparer<V>) -> Comparer<T> {
        OrderedComparer::<T>::order_by_descending_with(selector, comparer)
    }
}
