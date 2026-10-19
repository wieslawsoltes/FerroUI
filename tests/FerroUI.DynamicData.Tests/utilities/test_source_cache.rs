//! `TestSourceCache<TObject, TKey>` (`Utilities/TestSourceCache.cs`).

use ferroui_dynamic_data::cache::{Change, ChangeSet, IConnectableCache, IObservableCache, ISourceCache, ISourceUpdater, KeySelector, SourceCache};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::BehaviorSubject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::hash::Hash;
use std::rc::Rc;

struct Inner<T, K> {
    count_changed: Rc<dyn IObservable<usize>>,
    error: Rc<BehaviorSubject<Option<ObservableError>>>,
    has_completed: Rc<BehaviorSubject<bool>>,
    source: SourceCache<T, K>,
}

/// A `SourceCache` whose streams a test can fail or complete (`TestSourceCache<TObject, TKey>`):
/// [`set_error`](Self::set_error) ends every stream with an error and
/// [`complete`](Self::complete) completes them; after either, an edit panics. A clone is a handle
/// to the same cache.
pub struct TestSourceCache<T, K> {
    inner: Rc<Inner<T, K>>,
}

impl<T, K> Clone for TestSourceCache<T, K> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

/// `WrapStream`: the stream of the source, ended by the error or the completion of the test
/// cache. The completion is connected after the subscription, so that a subscriber receives the
/// initial change set before an immediate completion.
fn wrap_stream<V: 'static>(error: &Rc<BehaviorSubject<Option<ObservableError>>>, has_completed: &Rc<BehaviorSubject<bool>>, source_stream: Rc<dyn IObservable<V>>) -> Rc<dyn IObservable<V>> {
    let (error, has_completed) = (error.clone(), has_completed.clone());
    Observable::create(move |downstream_observer: Rc<dyn IObserver<V>>| {
        let when_completed = has_completed.as_observable().where_(|has_completed: &bool| *has_completed).publish();
        let completion: Rc<dyn IObservable<bool>> = when_completed.clone();
        let errors: Rc<dyn IObservable<V>> = error
            .as_observable()
            .select(|error: Option<ObservableError>| match error {
                Some(error) => Observable::throw::<V>(error),
                None => Observable::empty::<V>(),
            })
            .switch();
        let subscription = errors.merge(source_stream.clone()).take_until(completion).subscribe_safe(downstream_observer);
        let connection = when_completed.connect();
        Disposable::create(move || {
            subscription.dispose();
            connection.dispose();
        })
    })
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> TestSourceCache<T, K> {
    /// An empty cache whose items are keyed by `key_selector` (`new TestSourceCache(keySelector)`).
    pub fn new(key_selector: impl Fn(&T) -> K + 'static) -> Self {
        let error = BehaviorSubject::new(None);
        let has_completed = BehaviorSubject::new(false);
        let source = SourceCache::new(key_selector);
        let count_changed = wrap_stream(&error, &has_completed, source.count_changed());
        Self { inner: Rc::new(Inner { count_changed, error, has_completed, source }) }
    }

    /// The cache that the test cache wraps.
    pub fn source(&self) -> &SourceCache<T, K> {
        &self.inner.source
    }

    /// Completes every stream of the cache (`Complete`).
    ///
    /// # Panics
    /// When the cache has failed or completed already (`InvalidOperationException`).
    pub fn complete(&self) {
        self.assert_can_mutate();
        self.inner.has_completed.on_next(true);
    }

    /// Ends every stream of the cache with `error` (`SetError`).
    ///
    /// # Panics
    /// When the cache has failed or completed already (`InvalidOperationException`).
    pub fn set_error(&self, error: ObservableError) {
        self.assert_can_mutate();
        self.inner.error.on_next(Some(error));
    }

    /// Runs an edit (`Edit`).
    ///
    /// # Panics
    /// When the cache has failed or completed (`InvalidOperationException`).
    pub fn edit(&self, update: impl FnOnce(&mut dyn ISourceUpdater<T, K>)) {
        self.assert_can_mutate();
        self.inner.source.edit(update);
    }

    fn assert_can_mutate(&self) {
        let failed = self.inner.error.value().ok().flatten().is_some();
        assert!(!failed, "The source collection is in an error state and cannot be mutated.");
        let completed = self.inner.has_completed.value().unwrap_or(false);
        assert!(!completed, "The source collection is in a completed state and cannot be mutated.");
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IConnectableCache<T, K> for TestSourceCache<T, K> {
    fn count_changed(&self) -> Rc<dyn IObservable<usize>> {
        self.inner.count_changed.clone()
    }

    fn connect_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>, suppress_empty_change_sets: bool) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        wrap_stream(&self.inner.error, &self.inner.has_completed, self.inner.source.connect_with(predicate, suppress_empty_change_sets))
    }

    fn preview_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
        wrap_stream(&self.inner.error, &self.inner.has_completed, self.inner.source.preview_with(predicate))
    }

    fn watch(&self, key: K) -> Rc<dyn IObservable<Change<T, K>>> {
        wrap_stream(&self.inner.error, &self.inner.has_completed, self.inner.source.watch(key))
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IObservableCache<T, K> for TestSourceCache<T, K> {
    fn count(&self) -> usize {
        self.inner.source.count()
    }

    fn items(&self) -> Vec<T> {
        self.inner.source.items()
    }

    fn keys(&self) -> Vec<K> {
        self.inner.source.keys()
    }

    fn key_values(&self) -> Dictionary<K, T> {
        self.inner.source.key_values()
    }

    fn lookup(&self, key: &K) -> Option<T> {
        self.inner.source.lookup(key)
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> ISourceCache<T, K> for TestSourceCache<T, K> {
    fn key_selector(&self) -> KeySelector<T, K> {
        ISourceCache::key_selector(&self.inner.source)
    }

    fn edit(&self, update: &mut dyn FnMut(&mut dyn ISourceUpdater<T, K>)) {
        self.assert_can_mutate();
        ISourceCache::edit(&self.inner.source, update);
    }
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> IDisposable for TestSourceCache<T, K> {
    /// Disposes the cache and the subjects of the error and the completion (`Dispose`).
    fn dispose(&self) {
        self.inner.source.dispose();
        self.inner.error.dispose();
        self.inner.has_completed.dispose();
    }
}
