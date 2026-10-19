//! `RefCount<T>` (`List/Internal/RefCount.cs`): one list of the source, shared by the subscribers,
//! made by the first and disposed when the last unsubscribes.

use crate::list::{ChangeSet, IObservableList, ObservableList, ObservableListEx};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct State<T> {
    list: RefCell<Option<ObservableList<T>>>,
    ref_count: Cell<usize>,
}

/// `RefCount.Run`.
pub(crate) fn ref_count<T: Clone + PartialEq + 'static>(source: Rc<dyn IObservable<ChangeSet<T>>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
    let state = Rc::new(State::<T> { list: RefCell::new(None), ref_count: Cell::new(0) });
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        state.ref_count.set(state.ref_count.get() + 1);
        if state.ref_count.get() == 1 {
            let list = source.as_observable_list();
            *state.list.borrow_mut() = Some(list);
        }
        let list = state.list.borrow().clone().expect("The list is null despite having reference counting.");
        let subscriber = list.connect().subscribe_safe(observer);
        let state = state.clone();
        Disposable::create(move || {
            subscriber.dispose();
            state.ref_count.set(state.ref_count.get() - 1);
            let list_to_dispose = if state.ref_count.get() == 0 { state.list.borrow_mut().take() } else { None };
            if let Some(list) = list_to_dispose {
                list.dispose();
            }
        })
    })
}
