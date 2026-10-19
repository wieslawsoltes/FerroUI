//! `TestSourceList<T>` (`Utilities/TestSourceList.cs`).

use ferroui_dynamic_data::list::{Change, ChangeSet, IExtendedList, IObservableList, ISourceList, ListChangeReason, SourceList};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{BehaviorSubject, Subject};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

struct Inner<T> {
    count_changed: Rc<dyn IObservable<usize>>,
    error: Rc<BehaviorSubject<Option<ObservableError>>>,
    has_completed: Rc<BehaviorSubject<bool>>,
    refresh_requested: Rc<Subject<ChangeSet<T>>>,
    refresh_requested_preview: Rc<Subject<ChangeSet<T>>>,
    source: SourceList<T>,
}

/// A `SourceList` whose streams a test can fail or complete, and whose items it can refresh
/// (`TestSourceList<T>`): [`set_error`](Self::set_error) ends every stream with an error and
/// [`complete`](Self::complete) completes them; after either, an edit panics. A clone is a handle
/// to the same list.
pub struct TestSourceList<T> {
    inner: Rc<Inner<T>>,
}

impl<T> Clone for TestSourceList<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

/// `WrapStream`: the stream of the source, ended by the error or the completion of the test list.
/// The completion is connected after the subscription, so that a subscriber receives the initial
/// change set before an immediate completion.
fn wrap_stream<V: 'static>(error: &Rc<BehaviorSubject<Option<ObservableError>>>, has_completed: &Rc<BehaviorSubject<bool>>, source_stream: Rc<dyn IObservable<V>>) -> Rc<dyn IObservable<V>> {
    let (error, has_completed) = (error.clone(), has_completed.clone());
    Observable::create(move |downstream_observer: Rc<dyn IObserver<V>>| {
        let has_completed = has_completed.as_observable().publish();
        let completion: Rc<dyn IObservable<bool>> = has_completed.clone();
        let errors: Rc<dyn IObservable<V>> = error
            .as_observable()
            .select(|error: Option<ObservableError>| match error {
                Some(error) => Observable::throw::<V>(error),
                None => Observable::empty::<V>(),
            })
            .switch();
        let subscription = errors.merge(source_stream.clone()).take_until(completion.where_(|has_completed: &bool| *has_completed)).subscribe_safe(downstream_observer);
        let connection = has_completed.connect();
        Disposable::create(move || {
            connection.dispose();
            subscription.dispose();
        })
    })
}

impl<T: Clone + PartialEq + 'static> Default for TestSourceList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq + 'static> TestSourceList<T> {
    /// An empty list (`new TestSourceList()`).
    pub fn new() -> Self {
        let error = BehaviorSubject::new(None);
        let has_completed = BehaviorSubject::new(false);
        let source = SourceList::new();
        let count_changed = wrap_stream(&error, &has_completed, source.count_changed());
        Self { inner: Rc::new(Inner { count_changed, error, has_completed, refresh_requested: Subject::new(), refresh_requested_preview: Subject::new(), source }) }
    }

    /// The list that the test list wraps.
    pub fn source(&self) -> &SourceList<T> {
        &self.inner.source
    }

    /// Completes every stream of the list (`Complete`).
    ///
    /// # Panics
    /// When the list has failed or completed already (`InvalidOperationException`).
    pub fn complete(&self) {
        self.assert_can_mutate();
        self.inner.has_completed.on_next(true);
    }

    /// Ends every stream of the list with `error` (`SetError`).
    ///
    /// # Panics
    /// When the list has failed or completed already (`InvalidOperationException`).
    pub fn set_error(&self, error: ObservableError) {
        self.assert_can_mutate();
        self.inner.error.on_next(Some(error));
    }

    /// Runs an edit (`Edit`).
    ///
    /// # Panics
    /// When the list has failed or completed (`InvalidOperationException`).
    pub fn edit(&self, update: impl FnOnce(&mut dyn IExtendedList<T>)) {
        self.assert_can_mutate();
        self.inner.source.edit(update);
    }

    fn request_refresh(&self, changes: ChangeSet<T>) {
        self.inner.refresh_requested_preview.on_next(changes.clone());
        self.inner.refresh_requested.on_next(changes);
    }

    /// Refreshes every item, previews first (`Refresh()`).
    pub fn refresh(&self) {
        let changes = self.inner.source.items().into_iter().enumerate().map(|(index, item)| Change::new(ListChangeReason::Refresh, item, Some(index))).collect::<Vec<_>>();
        self.request_refresh(ChangeSet::from(changes));
    }

    /// Refreshes the item at `index` (`Refresh(int index)`).
    ///
    /// # Panics
    /// When there is no item at `index`.
    pub fn refresh_at(&self, index: usize) {
        let item = self.inner.source.items().into_iter().nth(index).expect("the index is in the list");
        self.request_refresh(ChangeSet::from(vec![Change::new(ListChangeReason::Refresh, item, Some(index))]));
    }

    /// Refreshes the items at `indexes`, in their order (`Refresh(IEnumerable<int> indexes)`).
    ///
    /// # Panics
    /// When there is no item at one of the indexes.
    pub fn refresh_indexes(&self, indexes: impl IntoIterator<Item = usize>) {
        let items = self.inner.source.items();
        let changes = indexes.into_iter().map(|index| Change::new(ListChangeReason::Refresh, items[index].clone(), Some(index))).collect::<Vec<_>>();
        self.request_refresh(ChangeSet::from(changes));
    }

    fn assert_can_mutate(&self) {
        let failed = self.inner.error.value().ok().flatten().is_some();
        assert!(!failed, "The source collection is in an error state and cannot be mutated.");
        let completed = self.inner.has_completed.value().unwrap_or(false);
        assert!(!completed, "The source collection is in a completed state and cannot be mutated.");
    }
}

impl<T: Clone + PartialEq + 'static> IObservableList<T> for TestSourceList<T> {
    fn count(&self) -> usize {
        self.inner.source.count()
    }

    fn count_changed(&self) -> Rc<dyn IObservable<usize>> {
        self.inner.count_changed.clone()
    }

    fn items(&self) -> Vec<T> {
        self.inner.source.items()
    }

    /// The items and their changes, with the refreshes requested (`Connect`).
    fn connect_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
        let changes = self.inner.source.connect_with(predicate).merge(self.inner.refresh_requested.as_observable());
        wrap_stream(&self.inner.error, &self.inner.has_completed, changes)
    }

    /// The changes before they are applied, with the refreshes requested (`Preview`).
    fn preview_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
        let changes = self.inner.source.preview_with(predicate).merge(self.inner.refresh_requested_preview.as_observable());
        wrap_stream(&self.inner.error, &self.inner.has_completed, changes)
    }
}

impl<T: Clone + PartialEq + 'static> ISourceList<T> for TestSourceList<T> {
    fn edit_dyn(&self, update: &mut dyn FnMut(&mut dyn IExtendedList<T>)) {
        self.assert_can_mutate();
        self.inner.source.edit_dyn(update);
    }
}

impl<T: Clone + PartialEq + 'static> IDisposable for TestSourceList<T> {
    /// Disposes the list and the subjects (`Dispose`).
    fn dispose(&self) {
        self.inner.source.dispose();
        self.inner.error.dispose();
        self.inner.has_completed.dispose();
        self.inner.refresh_requested.dispose();
        self.inner.refresh_requested_preview.dispose();
    }
}
