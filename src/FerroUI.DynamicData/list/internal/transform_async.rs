//! `TransformAsync<TSource, TDestination>` (`List/Internal/TransformAsync.cs`).
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream's factory returns a `Task<TDestination>`; here
//! it returns a future of `Result<TDestination, ObservableError>`, which runs as a task of the
//! thread (docs/porting/reactive.md, section 5.8), and takes its arguments by value, so that the
//! future owns them. Upstream serializes the change sets with a `SemaphoreSlim` as well as with
//! `Concat`; on one thread `Concat` alone keeps one change set at a time, so the lock is left out.

use super::{transform_change_set, UnspecifiedIndexError};
use crate::cache::internal::TransformFuture;
use crate::list::{clear_or_remove_many, Change, ChangeAwareList, ChangeSet, IExtendedList, IList, ListChangeReason};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::threading::tasks::Task;
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IObservable, ObservableError};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

/// The factory of an asynchronous transform of a list: the item, the value it had before (for a
/// replacement or a refresh that transforms), its index and the token that the disposal of the
/// subscription cancels (`Func<TSource, Optional<TDestination>, int, CancellationToken, Task<TDestination>>`).
pub(crate) type ListAsyncTransformFactory<S, D> = Rc<dyn Fn(S, Option<D>, usize, CancellationToken) -> TransformFuture<D>>;

/// A shared factory of a closure that returns a future.
pub(crate) fn list_async_transform_factory<S: 'static, D: 'static, Fut: Future<Output = Result<D, ObservableError>> + 'static>(
    factory: impl Fn(S, Option<D>, usize, CancellationToken) -> Fut + 'static,
) -> ListAsyncTransformFactory<S, D> {
    Rc::new(move |item: S, previous: Option<D>, index: usize, token: CancellationToken| -> TransformFuture<D> { Box::pin(factory(item, previous, index, token)) })
}

/// An item of the source and its transformed value (`Transformer<TSource, TDestination>.TransformedItemContainer`):
/// equal by the item of the source.
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

type State<S, D> = Rc<RefCell<ChangeAwareList<TransformedItemContainer<S, D>>>>;

/// Each item of the list in the form the future of `factory` gives it, at the same position
/// (`TransformAsync.Run`).
///
/// The change sets are handled one after another (`Concat`): the changes of one are applied in
/// order, each awaiting its factory, except that the items of a range are transformed at once
/// (`Task.WhenAll`); the change set of the transformed items is emitted when it has been applied.
/// An addition or a replacement transforms the item; a refresh is forwarded, or with
/// `transform_on_refresh` transforms the item again and emits a replacement; removals and moves
/// apply to the transformed items. A change without an index finds its item by equality (a
/// refresh or a removal of an item that is not there changes nothing). An error of a future ends
/// the sequence.
pub(crate) fn transform_async<S: Clone + PartialEq + 'static, D: Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S>>>,
    factory: ListAsyncTransformFactory<S, D>,
    transform_on_refresh: bool,
) -> Rc<dyn IObservable<ChangeSet<D>>> {
    // `Observable.Defer(RunImpl)`: each subscription has a list of its own.
    Observable::defer(move || {
        let state: State<S, D> = Rc::new(RefCell::new(ChangeAwareList::new()));
        let factory = factory.clone();
        source
            .select(move |changes: ChangeSet<S>| {
                let (state, factory) = (state.clone(), factory.clone());
                Observable::from_async_with_token(move |cancel: CancellationToken| {
                    let (state, factory, changes) = (state.clone(), factory.clone(), changes.clone());
                    async move {
                        transform(&state, &changes, &factory, transform_on_refresh, &cancel).await?;
                        let changed = state.borrow_mut().capture_changes();
                        Ok(transform_change_set(&changed, |container: &TransformedItemContainer<S, D>| container.destination.clone()))
                    }
                })
            })
            .concat_inner::<ChangeSet<D>>()
    })
}

/// `_containerFactory`: the future of the container of `item`.
fn container_factory<S: Clone + 'static, D: 'static>(
    factory: &ListAsyncTransformFactory<S, D>,
    item: &S,
    previous: Option<D>,
    index: usize,
    cancel: &CancellationToken,
) -> impl Future<Output = Result<TransformedItemContainer<S, D>, ObservableError>> + 'static {
    let source = item.clone();
    let destination = factory(item.clone(), previous, index, cancel.clone());
    async move { Ok(TransformedItemContainer { source, destination: destination.await? }) }
}

/// The index of the container of `source_item` (`FindItemIndex`).
fn find_item_index<S: Clone + PartialEq, D: Clone>(transformed: &ChangeAwareList<TransformedItemContainer<S, D>>, source_item: &S) -> Option<usize> {
    transformed.items().iter().position(|container| &container.source == source_item)
}

fn unspecified_index(message: &str) -> ObservableError {
    Rc::new(UnspecifiedIndexError(message.to_owned()))
}

/// `Transform(ChangeAwareList, IChangeSet, CancellationToken)`. No borrow of the list is held
/// while a future is awaited.
async fn transform<S: Clone + PartialEq + 'static, D: Clone + 'static>(
    transformed: &State<S, D>,
    changes: &ChangeSet<S>,
    factory: &ListAsyncTransformFactory<S, D>,
    transform_on_refresh: bool,
    cancel: &CancellationToken,
) -> Result<(), ObservableError> {
    for item in changes {
        match item.reason() {
            ListChangeReason::Add => {
                let change = item.item();
                let count = transformed.borrow().count();
                match change.current_index() {
                    Some(index) if index < count => {
                        let container = container_factory(factory, change.current(), None, index, cancel).await?;
                        transformed.borrow_mut().insert(index, container);
                    }
                    _ => {
                        let container = container_factory(factory, change.current(), None, count, cancel).await?;
                        transformed.borrow_mut().add(container);
                    }
                }
            }
            ListChangeReason::AddRange => {
                let range = item.range();
                let start_index = range.index().unwrap_or_else(|| transformed.borrow().count());
                // The transforms of the range run at once (`Task.WhenAll`): each is a task, and
                // every one is awaited before the first error, if any, ends the change set.
                let tasks: Vec<Task<TransformedItemContainer<S, D>>> = range
                    .iter()
                    .enumerate()
                    .map(|(offset, source_item)| Task::spawn_local(container_factory(factory, source_item, None, offset + start_index, cancel)))
                    .collect();
                let mut outcomes = Vec::with_capacity(tasks.len());
                for task in tasks {
                    outcomes.push(task.await);
                }
                let containers = outcomes.into_iter().collect::<Result<Vec<_>, ObservableError>>()?;
                IList::add_or_insert_range(&mut *transformed.borrow_mut(), containers, range.index());
            }
            ListChangeReason::Refresh => {
                let change = item.item();
                let index = change.current_index().or_else(|| find_item_index(&transformed.borrow(), change.current()));
                let Some(index) = index else {
                    continue;
                };
                if transform_on_refresh {
                    let previous = transformed.borrow().get(index).destination.clone();
                    let container = container_factory(factory, change.current(), Some(previous), index, cancel).await?;
                    transformed.borrow_mut().set(index, container);
                } else {
                    transformed.borrow_mut().refresh_at(index);
                }
            }
            ListChangeReason::Replace => {
                let change = item.item();
                let previous_index = match change.previous_index() {
                    Some(index) => index,
                    None => {
                        let previous = change.previous().expect("a replacement has a previous item");
                        // Upstream indexes the list with the -1 of an item it cannot find, which
                        // throws.
                        find_item_index(&transformed.borrow(), previous).ok_or_else(|| unspecified_index("Cannot find index of the replaced item"))?
                    }
                };
                let current_index = change.current_index().unwrap_or(previous_index);
                if current_index == previous_index {
                    let previous = transformed.borrow().get(previous_index).destination.clone();
                    let container = container_factory(factory, change.current(), Some(previous), current_index, cancel).await?;
                    transformed.borrow_mut().set(current_index, container);
                } else {
                    transformed.borrow_mut().remove_at(previous_index);
                    let container = container_factory(factory, change.current(), None, current_index, cancel).await?;
                    transformed.borrow_mut().insert(current_index, container);
                }
            }
            ListChangeReason::Remove => {
                let change = item.item();
                let index = change.current_index().or_else(|| find_item_index(&transformed.borrow(), change.current()));
                if let Some(index) = index {
                    transformed.borrow_mut().remove_at(index);
                }
            }
            ListChangeReason::RemoveRange => {
                let range = item.range();
                match range.index() {
                    Some(index) => transformed.borrow_mut().remove_range(index, range.count()),
                    None => {
                        for current in range.iter() {
                            let index = find_item_index(&transformed.borrow(), current);
                            if let Some(index) = index {
                                transformed.borrow_mut().remove_at(index);
                            }
                        }
                    }
                }
            }
            ListChangeReason::Clear => {
                // The transformed items are kept so that the clear reports them.
                let mut list = transformed.borrow_mut();
                let to_clear = Change::range_change(ListChangeReason::Clear, list.items().to_vec(), None);
                clear_or_remove_many(&mut *list, &to_clear);
            }
            ListChangeReason::Moved => {
                let change = item.item();
                let (Some(current_index), Some(previous_index)) = (change.current_index(), change.previous_index()) else {
                    return Err(unspecified_index("Cannot move as an index was not specified"));
                };
                IExtendedList::move_index(&mut *transformed.borrow_mut(), previous_index, current_index);
            }
        }
    }
    Ok(())
}
