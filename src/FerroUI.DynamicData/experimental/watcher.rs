//! `Watcher<TObject, TKey>` (`Experimental/Watcher.cs`).

use super::{ISubjectWithRefCount, IWatcher, SubjectWithRefCount};
use crate::cache::{Change, ChangeReason, ChangeSet, IConnectableCache, IObservableCache, IntermediateCache, ObservableCache, ObservableCacheEx};
use ferroui_reactive::concurrency::{IScheduler, SchedulerExt};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{ISubject, ReplaySubject};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::hash::Hash;
use std::rc::Rc;

/// The subject of the watchers of one key.
type Watchers<T, K> = Rc<SubjectWithRefCount<Change<T, K>>>;

/// A watcher of a cache by key (`Watcher<TObject, TKey>`): the changes of each watched key go to a
/// subject that replays the latest one, on `scheduler`; the subject completes when its last
/// watcher goes or the watcher is disposed.
///
/// Upstream's lock (`Synchronize(_locker)`) is not needed on one thread.
pub(crate) struct Watcher<T, K> {
    disposer: Rc<dyn IDisposable>,
    source: ObservableCache<T, K>,
    subscribers: IntermediateCache<Watchers<T, K>, K>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Watcher<T, K> {
    /// A watcher of `source` that notifies on `scheduler` (`Watcher(IObservable<IChangeSet<TObject,
    /// TKey>>, IScheduler)`).
    pub(crate) fn new(source: Rc<dyn IObservable<ChangeSet<T, K>>>, scheduler: Rc<dyn IScheduler>) -> Self {
        let cache = source.as_observable_cache();
        let subscribers: IntermediateCache<Watchers<T, K>, K> = IntermediateCache::new();

        let on_complete_publisher = subscribers
            .connect()
            .observe_on(scheduler.clone())
            .subscribe_many(|subject: &Watchers<T, K>| {
                let subject = subject.clone();
                Disposable::create(move || subject.on_completed())
            })
            .subscribe_ignore();

        let source_subscriber = {
            let subscribers = subscribers.clone();
            source.subscribe_next(move |updates: ChangeSet<T, K>| {
                for update in updates.changes() {
                    if let Some(subscriber) = subscribers.lookup(update.key()) {
                        let update = update.clone();
                        scheduler.schedule_action(move || subscriber.on_next(update));
                    }
                }
            })
        };

        let disposer = {
            let (cache, subscribers) = (cache.clone(), subscribers.clone());
            Disposable::create(move || {
                on_complete_publisher.dispose();
                source_subscriber.dispose();

                cache.dispose();
                subscribers.dispose();
            })
        };

        Self { disposer, source: cache, subscribers }
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IDisposable for Watcher<T, K> {
    fn dispose(&self) {
        self.disposer.dispose();
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IWatcher<T, K> for Watcher<T, K> {
    fn watch(&self, key: K) -> Rc<dyn IObservable<Change<T, K>>> {
        let (source, subscribers) = (self.source.clone(), self.subscribers.clone());
        Observable::create(move |observer: Rc<dyn IObserver<Change<T, K>>>| {
            // Create or find the existing subscribers.
            let subject = match subscribers.lookup(&key) {
                Some(existing) => existing,
                None => {
                    let replay: Rc<dyn ISubject<Change<T, K>>> = ReplaySubject::with_buffer_size(1);
                    let subject = SubjectWithRefCount::new(Some(replay));

                    if let Some(initial) = source.lookup(&key) {
                        subject.on_next(Change::new(ChangeReason::Add, key.clone(), initial));
                    }

                    subscribers.add_or_update(subject.clone(), key.clone());
                    subject
                }
            };

            // Set up the subscription.
            let subscriber = subject.subscribe(observer);

            let (subscribers, key) = (subscribers.clone(), key.clone());
            Disposable::create(move || {
                subscriber.dispose();
                if subject.ref_count() == 0 {
                    subscribers.remove_key(key);
                }
            })
        })
    }
}
