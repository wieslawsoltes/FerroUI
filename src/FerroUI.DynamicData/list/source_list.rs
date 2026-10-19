//! `SourceList<T>`, `IObservableList<T>` and `ISourceList<T>` (`List/SourceList.cs`,
//! `List/IObservableList.cs`, `List/ISourceList.cs`).

use super::internal::{filter_static, ReaderWriter};
use super::{Change, ChangeSet, IExtendedList, ListChangeReason};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::{BehaviorSubject, Subject, SubjectBase};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A list that can be observed (`IObservableList<T>`).
pub trait IObservableList<T>: IDisposable {
    /// The number of items (`Count`).
    fn count(&self) -> usize;
    /// The number of items, now and whenever it changes (`CountChanged`).
    fn count_changed(&self) -> Rc<dyn IObservable<usize>>;
    /// The items (`Items`).
    fn items(&self) -> Vec<T>;
    /// The items as one range added, then the changes, of the items that `predicate` accepts
    /// (`Connect`).
    fn connect_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T>>>;
    /// The changes before they are applied (`Preview`).
    fn preview_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T>>>;

    /// The items and their changes (`Connect()`).
    fn connect(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        self.connect_with(None)
    }

    /// The changes before they are applied (`Preview()`).
    fn preview(&self) -> Rc<dyn IObservable<ChangeSet<T>>> {
        self.preview_with(None)
    }
}

/// A list whose items are edited by its owner (`ISourceList<T>`).
pub trait ISourceList<T: Clone + PartialEq>: IObservableList<T> {
    /// Runs an edit; its changes are delivered as one change set when it ends (`Edit`).
    fn edit_dyn(&self, update: &mut dyn FnMut(&mut dyn IExtendedList<T>));
}

struct Inner<T> {
    this: Weak<Inner<T>>,
    changes: Rc<Subject<ChangeSet<T>>>,
    changes_preview: Rc<Subject<ChangeSet<T>>>,
    count_changed: RefCell<Option<Rc<Subject<usize>>>>,
    reader_writer: ReaderWriter<T>,
    is_edit_in_progress: RefCell<Option<Rc<BehaviorSubject<bool>>>>,
    edit_level: Cell<usize>,
    loader: RefCell<Option<Rc<dyn IDisposable>>>,
    disposed: Cell<bool>,
}

/// The source of a reactive list (`SourceList<T>`): edits make change sets, which `connect`
/// delivers to the operators.
pub struct SourceList<T> {
    inner: Rc<Inner<T>>,
}

impl<T> Clone for SourceList<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

impl<T: Clone + PartialEq + 'static> Default for SourceList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq + 'static> SourceList<T> {
    fn create() -> Self {
        let inner = Rc::new_cyclic(|this| Inner {
            this: this.clone(),
            changes: Subject::new(),
            changes_preview: Subject::new(),
            count_changed: RefCell::new(None),
            reader_writer: ReaderWriter::new(),
            is_edit_in_progress: RefCell::new(None),
            edit_level: Cell::new(0),
            loader: RefCell::new(None),
            disposed: Cell::new(false),
        });
        Self { inner }
    }

    /// An empty list (`SourceList()`).
    pub fn new() -> Self {
        Self::create()
    }

    /// A list that applies the change sets of `source` (`SourceList(IObservable<IChangeSet<T>>)`).
    pub fn from_source<C: super::IChangeSet<T>>(source: Rc<dyn IObservable<C>>) -> Self {
        let list = Self::create();
        let (next, error, completed) = (Rc::downgrade(&list.inner), Rc::downgrade(&list.inner), Rc::downgrade(&list.inner));
        let finally_target = completed.clone();
        let loader = source
            .finally(move || {
                if let Some(inner) = finally_target.upgrade() {
                    inner.on_completed();
                }
            })
            .subscribe_all(
                move |changes: C| {
                    if let Some(inner) = next.upgrade() {
                        let changes = inner.reader_writer.write_changes(changes.change_set());
                        inner.invoke_next(&changes);
                    }
                },
                move |e: ObservableError| {
                    if let Some(inner) = error.upgrade() {
                        inner.on_error(e);
                    }
                },
                move || {
                    if let Some(inner) = completed.upgrade() {
                        inner.on_completed();
                    }
                },
            );
        *list.inner.loader.borrow_mut() = Some(loader);
        list
    }

    /// Runs an edit (`Edit(Action<IExtendedList<T>>)`): the changes are delivered when the
    /// outermost edit ends, before the edit is no longer in progress.
    pub fn edit(&self, update: impl FnOnce(&mut dyn IExtendedList<T>)) {
        let mut update = Some(update);
        self.inner.edit(&mut |list| (update.take().expect("an edit runs once"))(list));
    }
}

impl<T: Clone + PartialEq + 'static> Inner<T> {
    fn edit_in_progress_subject(&self) -> Option<Rc<BehaviorSubject<bool>>> {
        self.is_edit_in_progress.borrow().clone()
    }

    /// `Edit`.
    fn edit(&self, update: &mut dyn FnMut(&mut dyn IExtendedList<T>)) {
        let level = self.edit_level.get() + 1;
        self.edit_level.set(level);
        if level == 1 {
            if let Some(subject) = self.edit_in_progress_subject() {
                subject.on_next(true);
            }
        }
        // The inner `finally`: the level comes down however the edit ends.
        struct Level<'a>(&'a Cell<usize>, bool);
        impl Drop for Level<'_> {
            fn drop(&mut self) {
                if self.1 {
                    self.0.set(self.0.get() - 1);
                }
            }
        }
        let mut guard = Level(&self.edit_level, true);
        let changes = if level == 1 {
            if self.changes_preview.has_observers() {
                let this = self.this.clone();
                Some(self.reader_writer.write_with_preview(update, &move |changes| {
                    if let Some(inner) = this.upgrade() {
                        inner.invoke_next_preview(changes);
                    }
                }))
            } else {
                Some(self.reader_writer.write(update))
            }
        } else {
            self.reader_writer.write_nested(update);
            None
        };
        guard.1 = false;
        drop(guard);
        let level = self.edit_level.get() - 1;
        self.edit_level.set(level);
        if let Some(changes) = changes {
            if level == 0 {
                self.invoke_next(&changes);
            }
        }
        // The outer `finally`.
        if level == 0 {
            if let Some(subject) = self.edit_in_progress_subject() {
                subject.on_next(false);
            }
        }
    }

    fn invoke_next(&self, changes: &ChangeSet<T>) {
        if changes.count() == 0 {
            return;
        }
        self.changes.on_next(changes.clone());
        let count = self.count_changed.borrow().clone();
        if let Some(subject) = count {
            subject.on_next(self.reader_writer.count());
        }
    }

    fn invoke_next_preview(&self, changes: &ChangeSet<T>) {
        if changes.count() == 0 {
            return;
        }
        self.changes_preview.on_next(changes.clone());
    }

    fn on_completed(&self) {
        self.changes_preview.on_completed();
        self.changes.on_completed();
        if let Some(subject) = self.edit_in_progress_subject() {
            subject.on_completed();
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.changes_preview.on_error(error.clone());
        self.changes.on_error(error.clone());
        if let Some(subject) = self.edit_in_progress_subject() {
            subject.on_error(error);
        }
    }

    /// `CreateConnectObservable`.
    fn create_connect_observable(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
        let this = self.this.clone();
        let observable = Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
            let Some(inner) = this.upgrade() else { return Disposable::empty() };
            let items = inner.reader_writer.items();
            if !items.is_empty() {
                observer.on_next(ChangeSet::from(vec![Change::range_change(ListChangeReason::AddRange, items, Some(0))]));
            }
            let completed = observer.clone();
            inner.changes.as_observable().finally(move || completed.on_completed()).subscribe(observer)
        });
        match predicate {
            Some(predicate) => filter_static(observable, predicate),
            None => observable,
        }
    }
}

impl<T: Clone + PartialEq + 'static> IObservableList<T> for SourceList<T> {
    fn count(&self) -> usize {
        self.inner.reader_writer.count()
    }

    fn count_changed(&self) -> Rc<dyn IObservable<usize>> {
        let this = Rc::downgrade(&self.inner);
        Observable::create(move |observer: Rc<dyn IObserver<usize>>| {
            let Some(inner) = this.upgrade() else { return Disposable::empty() };
            let subject = inner.count_changed.borrow_mut().get_or_insert_with(Subject::new).clone();
            subject.as_observable().start_with([inner.reader_writer.count()]).distinct_until_changed().subscribe(observer)
        })
    }

    fn items(&self) -> Vec<T> {
        self.inner.reader_writer.items()
    }

    fn connect_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
        let this = Rc::downgrade(&self.inner);
        Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
            let Some(inner) = this.upgrade() else { return Disposable::empty() };
            let editing = inner.is_edit_in_progress.borrow().is_some() || inner.edit_level.get() != 0;
            let observable = if editing {
                // Defer the connection until no edit is in progress.
                let subject = inner.is_edit_in_progress.borrow_mut().get_or_insert_with(|| BehaviorSubject::new(inner.edit_level.get() != 0)).clone();
                let (target, predicate) = (this.clone(), predicate.clone());
                subject.as_observable().where_(|editing| !*editing).take(1).select_many(move |_| match target.upgrade() {
                    Some(inner) => inner.create_connect_observable(predicate.clone()),
                    None => Observable::empty(),
                })
            } else {
                inner.create_connect_observable(predicate.clone())
            };
            observable.subscribe(observer)
        })
    }

    fn preview_with(&self, predicate: Option<Rc<dyn Fn(&T) -> bool>>) -> Rc<dyn IObservable<ChangeSet<T>>> {
        let preview = self.inner.changes_preview.as_observable();
        match predicate {
            Some(predicate) => filter_static(preview, predicate),
            None => preview,
        }
    }
}

impl<T: Clone + PartialEq + 'static> ISourceList<T> for SourceList<T> {
    fn edit_dyn(&self, update: &mut dyn FnMut(&mut dyn IExtendedList<T>)) {
        self.inner.edit(update);
    }
}

impl<T: Clone + PartialEq + 'static> IDisposable for SourceList<T> {
    /// Ends the subscription to the source and completes the observers (`Dispose`).
    fn dispose(&self) {
        if self.inner.disposed.replace(true) {
            return;
        }
        let loader = self.inner.loader.borrow_mut().take();
        if let Some(loader) = loader {
            loader.dispose();
        }
        self.inner.on_completed();
        let count = self.inner.count_changed.borrow().clone();
        if let Some(subject) = count {
            subject.on_completed();
        }
    }
}
