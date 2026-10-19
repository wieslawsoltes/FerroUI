//! `Cast` (`List/ObservableListEx.Cast.cs`).

use crate::list::{Change, ChangeSet, ChangeType};
use ferroui_reactive::linq::{AnyObject, InvalidCastException};
use ferroui_reactive::ObservableExt;
use ferroui_system::{IObservable, ObservableError};
use std::any::Any;
use std::rc::Rc;

macro_rules! cast_methods {
    () => {
        /// The change sets with each item converted by `conversion_factory`
        /// (`Cast(Func<TSource, TDestination>)`; named `cast_list` beside `ObservableExt::cast`).
        fn cast_list<U: Clone + 'static>(&self, conversion_factory: impl Fn(&T) -> U + 'static) -> std::rc::Rc<dyn ferroui_system::IObservable<crate::list::ChangeSet<U>>> {
            ferroui_reactive::ObservableExt::select(&self.change_sets(), move |changes: crate::list::ChangeSet<T>| crate::list::transform_change_set(&changes, &conversion_factory))
        }
    };
}
pub(crate) use cast_methods;

/// The change sets of a list with each item cast to a `U`: an item of another type ends the
/// sequence with an `InvalidCastException`.
fn cast_object<U: 'static>(changes: &ChangeSet<Rc<dyn Any>>) -> Result<ChangeSet<Rc<U>>, ObservableError> {
    let cast = |item: &Rc<dyn Any>| -> Result<Rc<U>, ObservableError> { item.clone().downcast_object::<U>().map_err(|_| Rc::new(InvalidCastException::new::<U>()) as ObservableError) };
    let mut result = Vec::with_capacity(changes.count());
    for change in changes {
        if change.change_type() == ChangeType::Item {
            let item = change.item();
            let previous = item.previous().map(cast).transpose()?;
            result.push(Change::with_previous(change.reason(), cast(item.current())?, previous, item.current_index(), item.previous_index()));
        } else {
            let items = change.range().iter().map(cast).collect::<Result<Vec<_>, _>>()?;
            result.push(Change::range_change(change.reason(), items, change.range().index()));
        }
    }
    Ok(ChangeSet::from(result))
}

/// The operator of the sequences of change sets of lists of objects (`Rc<dyn Any>`, whose items
/// have no equality, so that `ObservableListEx` does not apply to them).
pub trait ObservableObjectListEx {
    /// The change sets with each item cast to a `U`; an item of another type ends the sequence with
    /// an `InvalidCastException` (`Cast<TDestination>(IObservable<IChangeSet<object>>)`).
    fn cast_list_any<U: 'static>(&self) -> Rc<dyn IObservable<ChangeSet<Rc<U>>>>;
}

impl ObservableObjectListEx for Rc<dyn IObservable<ChangeSet<Rc<dyn Any>>>> {
    fn cast_list_any<U: 'static>(&self) -> Rc<dyn IObservable<ChangeSet<Rc<U>>>> {
        self.try_select(|changes: ChangeSet<Rc<dyn Any>>| cast_object::<U>(&changes))
    }
}
