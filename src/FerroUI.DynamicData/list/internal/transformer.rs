//! `Transformer<TSource, TDestination>` (`List/Internal/Transformer.cs`), with `ChangeSetEx.Transform`
//! (`List/ChangeSetEx.cs`) for the change sets it captures.

use crate::list::{clear_or_remove_many, remove_many, Change, ChangeAwareList, ChangeSet, IExtendedList, ListChangeReason};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, ObservableError};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// The factory of a transform of a list: the item, the value it had before (for a replacement or
/// a refresh that transforms) and its index (`Func<TSource, Optional<TDestination>, int, TDestination>`).
pub(crate) type ListTransformFactory<S, D> = Rc<dyn Fn(&S, Option<&D>, usize) -> Result<D, ObservableError>>;

/// An item of the source and its transformed value (`TransformedItemContainer`): equal by the item
/// of the source.
#[derive(Clone)]
struct TransformedItemContainer<S, D> {
    source: S,
    destination: D,
}

impl<S: PartialEq, D> PartialEq for TransformedItemContainer<S, D> {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

/// The error of a change whose index cannot be found (`UnspecifiedIndexException`).
///
/// Deviation (DEVIATIONS.md, DynamicData): the message does not name the item, which would need
/// `T: Debug`.
#[derive(Debug)]
struct UnspecifiedIndexError(&'static str);

impl fmt::Display for UnspecifiedIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for UnspecifiedIndexError {}

fn unspecified_index(message: &'static str) -> ObservableError {
    Rc::new(UnspecifiedIndexError(message))
}

/// Each item of the list in the form `factory` gives it, at the same position
/// (`Transformer.Run`): an addition or a replacement transforms the item; a refresh is forwarded,
/// or with `transform_on_refresh` transforms the item again and emits a replacement; a removal
/// and a move apply to the transformed items. A change without an index finds its item by
/// equality. An error of the factory, or a change whose item cannot be found where upstream
/// throws, ends the sequence.
pub(crate) fn transformer<S: Clone + PartialEq + 'static, D: Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S>>>,
    factory: impl Fn(&S, Option<&D>, usize) -> Result<D, ObservableError> + 'static,
    transform_on_refresh: bool,
) -> Rc<dyn IObservable<ChangeSet<D>>> {
    let factory: ListTransformFactory<S, D> = Rc::new(factory);
    // `Observable.Defer(RunImpl)`: each subscription has a list of its own.
    Observable::defer(move || {
        let state = RefCell::new(ChangeAwareList::<TransformedItemContainer<S, D>>::new());
        let factory = factory.clone();
        source.try_select(move |changes: ChangeSet<S>| {
            let mut transformed = state.borrow_mut();
            transform(&mut transformed, &changes, &*factory, transform_on_refresh)?;
            let changed = transformed.capture_changes();
            Ok(transform_change_set(&changed, |container: &TransformedItemContainer<S, D>| container.destination.clone()))
        })
    })
}

/// The index of the container of `source_item` (`Transformer.FindItemIndex`).
fn find_item_index<S: PartialEq, D>(transformed: &ChangeAwareList<TransformedItemContainer<S, D>>, source_item: &S) -> Option<usize>
where
    S: Clone,
    D: Clone,
{
    transformed.items().iter().position(|container| &container.source == source_item)
}

/// `Transform(ChangeAwareList, IChangeSet)`.
fn transform<S: Clone + PartialEq, D: Clone>(
    transformed: &mut ChangeAwareList<TransformedItemContainer<S, D>>,
    changes: &ChangeSet<S>,
    factory: &dyn Fn(&S, Option<&D>, usize) -> Result<D, ObservableError>,
    transform_on_refresh: bool,
) -> Result<(), ObservableError> {
    let container_factory = |item: &S, previous: Option<&D>, index: usize| -> Result<TransformedItemContainer<S, D>, ObservableError> {
        Ok(TransformedItemContainer { source: item.clone(), destination: factory(item, previous, index)? })
    };

    for item in changes {
        match item.reason() {
            ListChangeReason::Add => {
                let change = item.item();
                match change.current_index() {
                    Some(index) if index < transformed.count() => {
                        let converted = container_factory(change.current(), None, index)?;
                        transformed.insert(index, converted);
                    }
                    _ => {
                        let converted = container_factory(change.current(), None, transformed.count())?;
                        transformed.add(converted);
                    }
                }
            }
            ListChangeReason::AddRange => {
                let range = item.range();
                let start_index = range.index().unwrap_or(transformed.count());
                let mut converted = Vec::with_capacity(range.count());
                for (offset, source_item) in range.iter().enumerate() {
                    converted.push(container_factory(source_item, None, offset + start_index)?);
                }
                crate::list::IList::add_or_insert_range(transformed, converted, range.index());
            }
            ListChangeReason::Refresh => {
                let change = item.item();
                let index = match change.current_index() {
                    Some(index) => index,
                    None => find_item_index(transformed, change.current()).ok_or_else(|| unspecified_index("Cannot find index of the refreshed item"))?,
                };
                if transform_on_refresh {
                    let previous = transformed.get(index).destination.clone();
                    let converted = container_factory(change.current(), Some(&previous), index)?;
                    transformed.set(index, converted);
                } else {
                    transformed.refresh_at(index);
                }
            }
            ListChangeReason::Replace => {
                let change = item.item();
                let previous_index = match change.previous_index() {
                    Some(index) => index,
                    None => {
                        let previous = change.previous().expect("a replacement has a previous item");
                        find_item_index(transformed, previous).ok_or_else(|| unspecified_index("Cannot find index of the replaced item"))?
                    }
                };
                let current_index = change.current_index().unwrap_or(previous_index);
                if current_index == previous_index {
                    let previous = transformed.get(previous_index).destination.clone();
                    let converted = container_factory(change.current(), Some(&previous), current_index)?;
                    transformed.set(current_index, converted);
                } else {
                    transformed.remove_at(previous_index);
                    let converted = container_factory(change.current(), None, current_index)?;
                    transformed.insert(current_index, converted);
                }
            }
            ListChangeReason::Remove => {
                let change = item.item();
                let index = match change.current_index() {
                    Some(index) => Some(index),
                    None => find_item_index(transformed, change.current()),
                };
                if let Some(index) = index {
                    transformed.remove_at(index);
                }
            }
            ListChangeReason::RemoveRange => {
                let range = item.range();
                match range.index() {
                    Some(index) => transformed.remove_range(index, range.count()),
                    None => {
                        let view: &ChangeAwareList<TransformedItemContainer<S, D>> = transformed;
                        let many_to_remove: Vec<TransformedItemContainer<S, D>> =
                            range.iter().filter_map(|current| find_item_index(view, current).map(|index| view.get(index).clone())).collect();
                        remove_many(transformed, &many_to_remove);
                    }
                }
            }
            ListChangeReason::Clear => {
                // The transformed items are kept so that the clear reports them.
                let to_clear = Change::range_change(ListChangeReason::Clear, transformed.items().to_vec(), None);
                clear_or_remove_many(transformed, &to_clear);
            }
            ListChangeReason::Moved => {
                let change = item.item();
                let (Some(current_index), Some(previous_index)) = (change.current_index(), change.previous_index()) else {
                    return Err(unspecified_index("Cannot move as an index was not specified"));
                };
                transformed.move_index(previous_index, current_index);
            }
        }
    }
    Ok(())
}

/// The change set with each item transformed, every other part kept (`ChangeSetEx.Transform`).
pub(crate) fn transform_change_set<S, D>(changes: &ChangeSet<S>, transformer: impl Fn(&S) -> D) -> ChangeSet<D> {
    changes
        .iter()
        .map(|change| match change.try_item() {
            Some(item) => Change::with_previous(change.reason(), transformer(item.current()), item.previous().map(&transformer), item.current_index(), item.previous_index()),
            None => Change::range_change(change.reason(), change.range().iter().map(&transformer).collect(), change.range().index()),
        })
        .collect()
}
