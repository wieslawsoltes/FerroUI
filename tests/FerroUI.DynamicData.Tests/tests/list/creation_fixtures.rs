//! `ListCreationFixtures` (`DynamicData.Tests/List/CreationFixtures.cs`): the sequences of change
//! sets of lists made by `ObservableChangeSet.Create` with an asynchronous function.

use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ObservableListEx, SourceList, SourceListEditConvenienceEx};
use ferroui_dynamic_data::ObservableChangeSet;
use ferroui_reactive::ObservableExt;
use ferroui_system::threading::tasks::Task;
use ferroui_system::{IDisposable, IObservable, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `CreateTask`: `value` as a completed task (`Task.FromResult`).
fn create_task<T: Clone + 'static>(value: T) -> Task<T> {
    Task::from_result(value)
}

#[test]
fn create() {
    subscribe_and_assert(
        ObservableChangeSet::create_list_async_with_action(|list: SourceList<i32>| async move {
            let value = create_task(10).await?;
            list.add(value);
            Ok::<_, ObservableError>(|| {})
        }),
        false,
    );
}

/// `SubscribeAndAssert`: the sequence as a list, with its error checked; disposing the list ends
/// the sequence.
fn subscribe_and_assert<T: Clone + PartialEq + 'static>(observable_changeset: Rc<dyn IObservable<ChangeSet<T>>>, expects_error: bool) {
    let error: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let complete = Rc::new(Cell::new(false));
    let changes: Rc<RefCell<Option<ChangeSet<T>>>> = Rc::new(RefCell::new(None));

    {
        let on_finally = complete.clone();
        let my_list = observable_changeset.finally(move || on_finally.set(true)).as_observable_list();
        let subscription = {
            let (changes, error) = (changes.clone(), error.clone());
            my_list.connect().subscribe_next_error(move |result| *changes.borrow_mut() = Some(result), move |ex| *error.borrow_mut() = Some(ex))
        };

        if !expects_error {
            assert!(error.borrow().is_none());
        } else {
            assert!(error.borrow().is_some());
        }

        subscription.dispose();
        my_list.dispose();
    }

    assert!(complete.get());
}
