//! `ObservableCache<TObject, TKey>` (`Cache/ObservableCache.cs`), with `IConnectableCache`,
//! `IObservableCache` (`Cache/IConnectableCache.cs`, `Cache/IObservableCache.cs`).

use super::internal::{KeySelector, ReaderWriter};
use super::{Change, ChangeReason, ChangeSet, ICacheUpdater, IChangeSet, ISourceUpdater, ObservableCacheEx};
use crate::internal::DeliveryQueue;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{BehaviorSubject, Subject, SubjectBase};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::hash::Hash;
use std::rc::{Rc, Weak};

/// A cache that can be observed (`IConnectableCache<TObject, TKey>`).
pub trait IConnectableCache<T, K> {
    /// The number of items, now and whenever it changes (`CountChanged`).
    fn count_changed(&self) -> Rc<dyn IObservable<usize>>;
    /// The items as additions, then the changes, of the items that `predicate` accepts; empty
    /// change sets are left out unless `suppress_empty_change_sets` is false (`Connect`).
    fn connect_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>, suppress_empty_change_sets: bool) -> Rc<dyn IObservable<ChangeSet<T, K>>>;
    /// The changes before they are applied, while the items are as they were (`Preview`).
    fn preview_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T, K>>>;
    /// The changes of the item of `key` (`Watch`).
    fn watch(&self, key: K) -> Rc<dyn IObservable<Change<T, K>>>;

    /// The items and their changes (`Connect()`).
    fn connect(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.connect_with(None, true)
    }

    /// The items that `predicate` accepts and their changes (`Connect(Func<TObject, bool>)`).
    fn connect_filtered(&self, predicate: impl Fn(&T) -> bool + 'static) -> Rc<dyn IObservable<ChangeSet<T, K>>>
    where
        Self: Sized,
    {
        self.connect_with(Some(Rc::new(predicate)), true)
    }

    /// The changes before they are applied (`Preview()`).
    fn preview(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        self.preview_with(None)
    }
}

/// A cache that can be observed and read (`IObservableCache<TObject, TKey>`).
pub trait IObservableCache<T, K>: IConnectableCache<T, K> + IDisposable {
    /// The number of items (`Count`).
    fn count(&self) -> usize;
    /// The items (`Items`).
    fn items(&self) -> Vec<T>;
    /// The keys (`Keys`).
    fn keys(&self) -> Vec<K>;
    /// The keys and items (`KeyValues`).
    fn key_values(&self) -> Dictionary<K, T>;
    /// The item of `key` (`Lookup`).
    fn lookup(&self, key: &K) -> Option<T>;
}

/// A notification of the queue of the cache (`CacheUpdate`).
#[derive(Clone)]
struct CacheUpdate<T, K> {
    changes: Option<ChangeSet<T, K>>,
    count: usize,
    version: u64,
}

/// The notifications that a suspension holds back (`SuspensionTracker`).
struct SuspensionTracker<T, K> {
    are_notifications_suspended: Rc<BehaviorSubject<bool>>,
    pending_changes: Vec<Change<T, K>>,
    count_suspend_count: usize,
    notify_suspend_count: usize,
    disposed: bool,
}

impl<T: Clone + 'static, K: Clone + 'static> SuspensionTracker<T, K> {
    fn new() -> Self {
        Self { are_notifications_suspended: BehaviorSubject::new(false), pending_changes: Vec::new(), count_suspend_count: 0, notify_suspend_count: 0, disposed: false }
    }

    fn is_count_suspended(&self) -> bool {
        self.count_suspend_count > 0
    }

    fn are_notifications_suspended(&self) -> bool {
        self.notify_suspend_count > 0
    }
}

struct Inner<T, K> {
    this: Weak<Inner<T, K>>,
    changes: Rc<Subject<ChangeSet<T, K>>>,
    changes_preview: Rc<Subject<ChangeSet<T, K>>>,
    count_changed: RefCell<Option<Rc<Subject<usize>>>>,
    suspension: RefCell<Option<SuspensionTracker<T, K>>>,
    reader_writer: ReaderWriter<T, K>,
    notifications: Rc<DeliveryQueue<CacheUpdate<T, K>>>,
    is_edit_in_progress: RefCell<Option<Rc<BehaviorSubject<bool>>>>,
    edit_level: Cell<usize>,
    current_version: Cell<u64>,
    current_delivery_version: Cell<u64>,
    loader: RefCell<Option<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

/// A cache whose edits and contents can be observed (`ObservableCache<TObject, TKey>`): the core
/// of `SourceCache`, and the cache of `AsObservableCache`.
pub struct ObservableCache<T, K> {
    inner: Rc<Inner<T, K>>,
}

impl<T, K> Clone for ObservableCache<T, K> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> ObservableCache<T, K> {
    fn create(key_selector: Option<KeySelector<T, K>>) -> Self {
        let inner = Rc::new_cyclic(|this: &Weak<Inner<T, K>>| Inner {
            this: this.clone(),
            changes: Subject::new(),
            changes_preview: Subject::new(),
            count_changed: RefCell::new(None),
            suspension: RefCell::new(None),
            reader_writer: ReaderWriter::new(key_selector),
            notifications: DeliveryQueue::new(Rc::new(CacheUpdateObserver { cache: this.clone() })),
            is_edit_in_progress: RefCell::new(None),
            edit_level: Cell::new(0),
            current_version: Cell::new(0),
            current_delivery_version: Cell::new(0),
            loader: RefCell::new(None),
            disposed: Cell::new(false),
        });
        Self { inner }
    }

    /// An empty cache whose items are keyed by `key_selector`
    /// (`ObservableCache(Func<TObject, TKey>)`).
    pub fn new(key_selector: impl Fn(&T) -> K + 'static) -> Self {
        Self::create(Some(Rc::new(key_selector)))
    }

    /// A cache without a key selector, for edits that give the keys
    /// (`ObservableCache(Func<TObject, TKey>? = null)`).
    pub fn without_key_selector() -> Self {
        Self::create(None)
    }

    /// A cache that applies the change sets of `source` (`ObservableCache(IObservable<IChangeSet>)`).
    pub fn from_source<C: IChangeSet<T, K>>(source: Rc<dyn IObservable<C>>) -> Self {
        let cache = Self::create(None);
        let inner = Rc::downgrade(&cache.inner);
        let (error_target, completed_target) = (inner.clone(), inner.clone());
        let loader = source.change_sets().subscribe_all(
            move |change_set| {
                if let Some(inner) = inner.upgrade() {
                    let notifications = inner.notifications.acquire();
                    let preview_handler = inner.preview_handler();
                    let changes = inner.reader_writer.write_changes(&change_set, preview_handler.as_ref().map(|h| h as &dyn Fn(&ChangeSet<T, K>)), inner.changes.has_observers());
                    let version = inner.current_version.get() + 1;
                    inner.current_version.set(version);
                    notifications.enqueue_next(CacheUpdate { changes: Some(changes), count: inner.reader_writer.count(), version });
                }
            },
            move |error| {
                if let Some(inner) = error_target.upgrade() {
                    inner.notify_error(error);
                }
            },
            move || {
                if let Some(inner) = completed_target.upgrade() {
                    inner.notify_completed();
                }
            },
        );
        *cache.inner.loader.borrow_mut() = Some(loader);
        cache
    }

    /// The key selector of the cache.
    pub fn key_selector(&self) -> Option<&KeySelector<T, K>> {
        self.inner.reader_writer.key_selector()
    }

    /// Runs an edit with the cache's key selector (`UpdateFromSource`); the changes are delivered
    /// once the outermost edit ends.
    pub fn update_from_source(&self, update: impl FnOnce(&mut dyn ISourceUpdater<T, K>)) {
        let mut update = Some(update);
        self.inner.update(&mut |updater| (update.take().expect("an edit runs once"))(updater));
    }

    /// Runs an edit that gives the keys (`UpdateFromIntermediate`).
    pub fn update_from_intermediate(&self, update: impl FnOnce(&mut dyn ICacheUpdater<T, K>)) {
        let mut update = Some(update);
        self.inner.update(&mut |updater| (update.take().expect("an edit runs once"))(updater));
    }

    /// The items as additions, those that `filter` accepts (`GetInitialUpdates`).
    pub(crate) fn get_initial_updates(&self, filter: Option<&dyn Fn(&T) -> bool>) -> ChangeSet<T, K> {
        self.inner.reader_writer.get_initial_updates(filter)
    }

    /// Holds back the notifications of count changes until the result is disposed
    /// (`SuspendCount`).
    pub fn suspend_count(&self) -> Rc<dyn IDisposable> {
        self.inner.suspension_tracker(|tracker| tracker.count_suspend_count += 1);
        let inner = self.inner.clone();
        Disposable::create(move || inner.resume_count())
    }

    /// Holds back the notifications of changes until the result is disposed; the changes made
    /// meanwhile are delivered as one change set (`SuspendNotifications`).
    pub fn suspend_notifications(&self) -> Rc<dyn IDisposable> {
        let suspended = self.inner.suspension_tracker(|tracker| {
            tracker.notify_suspend_count += 1;
            (tracker.notify_suspend_count == 1 && !tracker.disposed).then(|| tracker.are_notifications_suspended.clone())
        });
        if let Some(subject) = suspended {
            subject.on_next(true);
        }
        let inner = self.inner.clone();
        Disposable::create(move || inner.resume_notifications())
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Inner<T, K> {
    fn preview_handler(&self) -> Option<impl Fn(&ChangeSet<T, K>)> {
        if !self.changes_preview.has_observers() {
            return None;
        }
        let this = self.this.clone();
        Some(move |changes: &ChangeSet<T, K>| {
            if let Some(inner) = this.upgrade() {
                if changes.count() != 0 && !inner.notifications.is_terminated() {
                    inner.changes_preview.on_next(changes.clone());
                }
            }
        })
    }

    fn edit_in_progress_subject(&self) -> Option<Rc<BehaviorSubject<bool>>> {
        self.is_edit_in_progress.borrow().clone()
    }

    /// `UpdateFromSource` and `UpdateFromIntermediate`.
    fn update(&self, update: &mut dyn FnMut(&mut dyn ISourceUpdater<T, K>)) {
        let notifications = self.notifications.acquire();
        let level = self.edit_level.get() + 1;
        self.edit_level.set(level);
        if level == 1 {
            if let Some(subject) = self.edit_in_progress_subject() {
                subject.on_next(true);
            }
        }
        // An edit that panics still brings the level down (`finally`); what else the `finally`
        // does runs on the normal path below, in upstream's order.
        struct Unwind<'a> {
            level: &'a Cell<usize>,
            armed: bool,
        }
        impl Drop for Unwind<'_> {
            fn drop(&mut self) {
                if self.armed {
                    self.level.set(self.level.get() - 1);
                }
            }
        }
        let mut unwind = Unwind { level: &self.edit_level, armed: true };
        let changes = if level == 1 {
            let preview_handler = self.preview_handler();
            Some(self.reader_writer.write(update, preview_handler.as_ref().map(|h| h as &dyn Fn(&ChangeSet<T, K>)), self.changes.has_observers()))
        } else {
            self.reader_writer.write_nested(update);
            None
        };
        unwind.armed = false;
        drop(unwind);
        // `finally`: the level comes down, the changes are enqueued, then the edit is no longer
        // in progress; the delivery follows when the scope of the queue ends.
        let level = self.edit_level.get() - 1;
        self.edit_level.set(level);
        if let Some(changes) = changes {
            if level == 0 {
                let version = self.current_version.get() + 1;
                self.current_version.set(version);
                notifications.enqueue_next(CacheUpdate { changes: Some(changes), count: self.reader_writer.count(), version });
            }
        }
        if level == 0 {
            if let Some(subject) = self.edit_in_progress_subject() {
                subject.on_next(false);
            }
        }
        drop(notifications);
    }

    fn suspension_tracker<R>(&self, action: impl FnOnce(&mut SuspensionTracker<T, K>) -> R) -> R {
        let mut tracker = self.suspension.borrow_mut();
        action(tracker.get_or_insert_with(SuspensionTracker::new))
    }

    fn notify_completed(&self) {
        self.notifications.acquire().enqueue_completed();
    }

    fn notify_error(&self, error: ObservableError) {
        self.notifications.acquire().enqueue_error(error);
    }

    fn resume_count(&self) {
        let notifications = self.notifications.acquire();
        let resumed = self.suspension_tracker(|tracker| {
            tracker.count_suspend_count -= 1;
            tracker.count_suspend_count == 0
        });
        if resumed && self.count_changed.borrow().is_some() {
            notifications.enqueue_next(CacheUpdate { changes: None, count: self.reader_writer.count(), version: 0 });
        }
    }

    fn resume_notifications(&self) {
        let notifications = self.notifications.acquire();
        let (changes, emit_resume) = self.suspension_tracker(|tracker| {
            tracker.notify_suspend_count -= 1;
            if tracker.notify_suspend_count == 0 && !tracker.disposed {
                let changes = (!tracker.pending_changes.is_empty()).then(|| ChangeSet::from(std::mem::take(&mut tracker.pending_changes)));
                (changes, true)
            } else {
                (None, false)
            }
        });
        if let Some(changes) = changes {
            let version = self.current_version.get() + 1;
            self.current_version.set(version);
            notifications.enqueue_next(CacheUpdate { changes: Some(changes), count: self.reader_writer.count(), version });
        }
        if emit_resume {
            let subject = self.suspension_tracker(|tracker| (!tracker.disposed).then(|| tracker.are_notifications_suspended.clone()));
            if let Some(subject) = subject {
                subject.on_next(false);
            }
        }
    }

    /// The changes of the subject, without those that a subscriber that connects now has in its
    /// snapshot already: when notifications are pending, the ones up to the current version are
    /// skipped (the version guard of `CreateConnectObservable` and `CreateWatchObservable`).
    fn changes_after_snapshot(&self) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        let changes = self.changes.as_observable();
        if !self.notifications.has_pending() {
            return changes;
        }
        let snapshot_version = self.current_version.get();
        let this = self.this.clone();
        changes.skip_while(move |_| this.upgrade().is_some_and(|inner| inner.current_delivery_version.get() <= snapshot_version))
    }

    /// `CreateConnectObservable`.
    fn create_connect_observable(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>, suppress_empty_change_sets: bool) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        let this = self.this.clone();
        Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
            let Some(inner) = this.upgrade() else { return Disposable::empty() };
            let (snapshot_inner, snapshot_predicate) = (this.clone(), predicate.clone());
            let initial = Observable::defer(move || {
                let initial = snapshot_inner.upgrade().map(|inner| inner.reader_writer.get_initial_updates(snapshot_predicate.as_deref())).unwrap_or_default();
                Observable::return_(initial)
            });
            let mut changes = initial.concat(inner.changes_after_snapshot());
            if let Some(predicate) = &predicate {
                let predicate = predicate.clone();
                changes = changes.filter_with(move |item: &T| predicate(item), suppress_empty_change_sets);
            } else if suppress_empty_change_sets {
                changes = changes.not_empty();
            }
            changes.subscribe(observer)
        })
    }

    /// `CreateWatchObservable`.
    fn create_watch_observable(&self, key: K) -> Rc<dyn IObservable<Change<T, K>>> {
        let this = self.this.clone();
        Observable::create(move |observer: Rc<dyn IObserver<Change<T, K>>>| {
            let Some(inner) = this.upgrade() else { return Disposable::empty() };
            if let Some(initial) = inner.reader_writer.lookup(&key) {
                observer.on_next(Change::new(ChangeReason::Add, key.clone(), initial));
            }
            let (completed, key) = (observer.clone(), key.clone());
            inner.changes_after_snapshot().finally(move || completed.on_completed()).subscribe_next(move |changes: ChangeSet<T, K>| {
                for change in &changes {
                    if change.key() == &key {
                        observer.on_next(change.clone());
                    }
                }
            })
        })
    }

    /// `CreateDeferredNotificationObservable`: a connection made while an edit or a suspension is
    /// in progress waits for both to end.
    fn create_deferred_notification_observable<V: Clone + 'static>(&self, factory: Rc<dyn Fn() -> Rc<dyn IObservable<V>>>) -> Rc<dyn IObservable<V>> {
        let this = self.this.clone();
        Observable::create(move |observer: Rc<dyn IObserver<V>>| {
            let Some(inner) = this.upgrade() else { return Disposable::empty() };
            let suspension_active = inner.suspension.borrow().is_some();
            let edit_active = inner.is_edit_in_progress.borrow().is_some() || inner.edit_level.get() != 0;
            let observable: Rc<dyn IObservable<V>> = match (suspension_active, edit_active) {
                (false, false) => factory(),
                (false, true) => {
                    let (inner2, factory2, this2) = (inner.clone(), factory.clone(), this.clone());
                    inner
                        .edit_in_progress()
                        .where_(|in_progress| !*in_progress)
                        .take(1)
                        .select_many(move |_| {
                            let suspended = inner2.suspension.borrow().as_ref().is_some_and(SuspensionTracker::are_notifications_suspended);
                            match suspended {
                                true => fully_deferred(&this2, &factory2),
                                false => factory2(),
                            }
                        })
                }
                (true, false) => {
                    let (inner2, factory2, this2) = (inner.clone(), factory.clone(), this.clone());
                    inner
                        .notifications_suspended()
                        .where_(|suspended| !*suspended)
                        .take(1)
                        .select_many(move |_| {
                            let editing = inner2.is_edit_in_progress.borrow().as_ref().is_some_and(|subject| subject.value().unwrap_or(false));
                            match editing {
                                true => fully_deferred(&this2, &factory2),
                                false => factory2(),
                            }
                        })
                }
                (true, true) => fully_deferred(&this, &factory),
            };
            observable.subscribe(observer)
        })
    }

    /// The subject of edits in progress, made on first use (`_isEditInProgress`).
    fn edit_in_progress(&self) -> Rc<dyn IObservable<bool>> {
        let mut subject = self.is_edit_in_progress.borrow_mut();
        subject.get_or_insert_with(|| BehaviorSubject::new(self.edit_level.get() != 0)).as_observable()
    }

    /// The subject of suspensions (`NotificationsSuspendedObservable`).
    fn notifications_suspended(&self) -> Rc<dyn IObservable<bool>> {
        self.suspension_tracker(|tracker| tracker.are_notifications_suspended.as_observable())
    }
}

/// `CreateFullyDeferredConnection`: waits until no suspension and no edit is in progress.
fn fully_deferred<T: Clone + 'static, K: Eq + Hash + Clone + 'static, V: Clone + 'static>(
    this: &Weak<Inner<T, K>>,
    factory: &Rc<dyn Fn() -> Rc<dyn IObservable<V>>>,
) -> Rc<dyn IObservable<V>> {
    let Some(inner) = this.upgrade() else { return Observable::empty() };
    let factory = factory.clone();
    inner
        .notifications_suspended()
        .combine_latest(inner.edit_in_progress(), |suspended, editing| suspended || editing)
        .where_(|deferred| !*deferred)
        .take(1)
        .select(move |_| factory())
        .switch()
}

/// The observer of the queue of the cache (`CacheUpdateObserver`): it delivers the changes and
/// the count.
struct CacheUpdateObserver<T, K> {
    cache: Weak<Inner<T, K>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IObserver<CacheUpdate<T, K>> for CacheUpdateObserver<T, K> {
    fn on_next(&self, value: CacheUpdate<T, K>) {
        let Some(cache) = self.cache.upgrade() else { return };
        if let Some(changes) = value.changes {
            cache.current_delivery_version.set(value.version);
            let held_back = cache.suspension.borrow_mut().as_mut().is_some_and(|tracker| {
                if tracker.are_notifications_suspended() {
                    tracker.pending_changes.extend(changes.iter().cloned());
                    true
                } else {
                    false
                }
            });
            if !held_back {
                cache.changes.on_next(changes);
            }
        }
        let count_suspended = cache.suspension.borrow().as_ref().is_some_and(SuspensionTracker::is_count_suspended);
        if !count_suspended {
            let subject = cache.count_changed.borrow().clone();
            if let Some(subject) = subject {
                subject.on_next(value.count);
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        let Some(cache) = self.cache.upgrade() else { return };
        cache.changes_preview.on_error(error.clone());
        cache.changes.on_error(error.clone());
        if let Some(subject) = cache.edit_in_progress_subject() {
            subject.on_error(error.clone());
        }
        let count = cache.count_changed.borrow().clone();
        if let Some(subject) = count {
            subject.on_error(error.clone());
        }
        let suspended = cache.suspension.borrow_mut().as_mut().map(|tracker| {
            tracker.disposed = true;
            tracker.are_notifications_suspended.clone()
        });
        if let Some(subject) = suspended {
            subject.on_error(error);
        }
    }

    fn on_completed(&self) {
        let Some(cache) = self.cache.upgrade() else { return };
        cache.changes.on_completed();
        cache.changes_preview.on_completed();
        if let Some(subject) = cache.edit_in_progress_subject() {
            subject.on_completed();
        }
        let count = cache.count_changed.borrow().clone();
        if let Some(subject) = count {
            subject.on_completed();
        }
        let suspended = cache.suspension.borrow_mut().as_mut().map(|tracker| {
            tracker.disposed = true;
            tracker.are_notifications_suspended.clone()
        });
        if let Some(subject) = suspended {
            subject.on_completed();
        }
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IConnectableCache<T, K> for ObservableCache<T, K> {
    fn count_changed(&self) -> Rc<dyn IObservable<usize>> {
        let this = Rc::downgrade(&self.inner);
        Observable::create(move |observer: Rc<dyn IObserver<usize>>| {
            let Some(inner) = this.upgrade() else { return Disposable::empty() };
            let subject = inner.count_changed.borrow_mut().get_or_insert_with(Subject::new).clone();
            let mut count_changed = subject.as_observable();
            if inner.notifications.has_pending() {
                let snapshot_version = inner.current_version.get();
                let weak = this.clone();
                count_changed = count_changed.skip_while(move |_| weak.upgrade().is_some_and(|inner| inner.current_delivery_version.get() <= snapshot_version));
            }
            count_changed.start_with([inner.reader_writer.count()]).distinct_until_changed().subscribe(observer)
        })
    }

    fn connect_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>, suppress_empty_change_sets: bool) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        let this = Rc::downgrade(&self.inner);
        let factory: Rc<dyn Fn() -> Rc<dyn IObservable<ChangeSet<T, K>>>> = Rc::new(move || match this.upgrade() {
            Some(inner) => inner.create_connect_observable(predicate.clone(), suppress_empty_change_sets),
            None => Observable::empty(),
        });
        self.inner.create_deferred_notification_observable(factory)
    }

    fn preview_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        let preview = self.inner.changes_preview.as_observable();
        match predicate {
            None => preview,
            Some(predicate) => preview.filter(move |item: &T| predicate(item)),
        }
    }

    fn watch(&self, key: K) -> Rc<dyn IObservable<Change<T, K>>> {
        let this = Rc::downgrade(&self.inner);
        let factory: Rc<dyn Fn() -> Rc<dyn IObservable<Change<T, K>>>> = Rc::new(move || match this.upgrade() {
            Some(inner) => inner.create_watch_observable(key.clone()),
            None => Observable::empty(),
        });
        self.inner.create_deferred_notification_observable(factory)
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IObservableCache<T, K> for ObservableCache<T, K> {
    fn count(&self) -> usize {
        self.inner.reader_writer.count()
    }

    fn items(&self) -> Vec<T> {
        self.inner.reader_writer.items()
    }

    fn keys(&self) -> Vec<K> {
        self.inner.reader_writer.keys()
    }

    fn key_values(&self) -> Dictionary<K, T> {
        self.inner.reader_writer.key_values()
    }

    fn lookup(&self, key: &K) -> Option<T> {
        self.inner.reader_writer.lookup(key)
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IDisposable for ObservableCache<T, K> {
    /// Ends the subscription to the source and completes the observers (`Dispose`).
    fn dispose(&self) {
        if self.inner.disposed.replace(true) {
            return;
        }
        let loader = self.inner.loader.borrow_mut().take();
        if let Some(loader) = loader {
            loader.dispose();
        }
        self.inner.notify_completed();
    }
}
