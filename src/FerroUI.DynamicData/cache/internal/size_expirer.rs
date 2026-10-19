//! `SizeExpirer<TObject, TKey>` (`Cache/Internal/SizeExpirer.cs`): the items of a stream, the
//! oldest removed beyond a size.

use super::{transform_to_expirable, SizeLimiter};
use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, IConnectableCache, IntermediateCache};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::Rc;

/// `SizeExpirer.Run`.
///
/// # Panics
/// For a size of zero (`ArgumentException`).
pub(crate) fn size_expirer<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(source: Rc<dyn IObservable<ChangeSet<T, K>>>, size: usize) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    assert!(size > 0, "Size limit must be greater than zero");
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let size_limiter = RefCell::new(SizeLimiter::<T, K>::new(size));
        let root = IntermediateCache::from_source(source.clone());
        let transformed = RefCell::new(ChangeAwareCache::new());
        let next_index = Cell::new(0u64);

        let editor = root.clone();
        let completion = observer.clone();
        let subscriber = root
            .connect()
            .select(move |changes: ChangeSet<T, K>| {
                let expirable = transform_to_expirable(&mut transformed.borrow_mut(), &changes, &next_index);
                let result = size_limiter.borrow_mut().change(&expirable);
                let removes: Vec<K> = result.iter().filter(|change| change.reason() == ChangeReason::Remove).map(|change| change.key().clone()).collect();
                editor.edit(|updater| {
                    for key in &removes {
                        updater.remove_key(key);
                    }
                });
                result
            })
            .finally(move || completion.on_completed())
            .subscribe_safe(observer);

        Disposable::create(move || {
            subscriber.dispose();
            root.dispose();
        })
    })
}
