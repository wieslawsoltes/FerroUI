//! `RefCount<TObject, TKey>` (`Cache/Internal/RefCount.cs`): one cache of the source, shared by the
//! subscribers, made by the first and disposed when the last unsubscribes.

use crate::cache::{ChangeSet, IConnectableCache, ObservableCache, ObservableCacheEx};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

struct State<T, K> {
    cache: RefCell<Option<ObservableCache<T, K>>>,
    ref_count: Cell<usize>,
}

/// `RefCount.Run`.
pub(crate) fn ref_count<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(source: Rc<dyn IObservable<ChangeSet<T, K>>>) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    let state = Rc::new(State::<T, K> { cache: RefCell::new(None), ref_count: Cell::new(0) });
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        state.ref_count.set(state.ref_count.get() + 1);
        if state.ref_count.get() == 1 {
            let cache = source.as_observable_cache();
            *state.cache.borrow_mut() = Some(cache);
        }
        let cache = state.cache.borrow().clone().expect("the cache is there while it is counted");
        let subscriber = cache.connect().subscribe_safe(observer);
        let state = state.clone();
        Disposable::create(move || {
            subscriber.dispose();
            state.ref_count.set(state.ref_count.get() - 1);
            let cache_to_dispose = if state.ref_count.get() == 0 { state.cache.borrow_mut().take() } else { None };
            if let Some(cache) = cache_to_dispose {
                cache.dispose();
            }
        })
    })
}
