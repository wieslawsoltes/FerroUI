//! The comparers (`ReactiveUI.Core/Comparers`): orders built by chaining keys
//! (`OrderedComparer<T>::order_by(..).then_by(..)`).
//!
//! A comparer is a shared comparison function, [`Comparer<T>`], the port's form of
//! `IComparer<T>` (`docs/porting/reactive.md`, section 5.4): `items.sort_by(|x, y| comparer(x, y))`.

mod chained_comparer;
mod comparer_chaining_extensions;
mod i_comparer_builder;
mod ordered_comparer;
mod ordered_comparer_t;

use std::cmp::Ordering;
use std::rc::Rc;

pub use chained_comparer::ChainedComparer;
pub use comparer_chaining_extensions::ComparerChainingExtensions;
pub use i_comparer_builder::IComparerBuilder;
pub use ordered_comparer::OrderedComparerTypeWrapper;
pub use ordered_comparer_t::OrderedComparer;

/// A comparer of `T` (`IComparer<T>`): a shared comparison function.
pub type Comparer<T> = Rc<dyn Fn(&T, &T) -> Ordering>;
