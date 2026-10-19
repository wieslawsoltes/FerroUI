//! `BindPaged<TObject, TKey>` and `BindVirtualized<TObject, TKey>` (`Binding/BindPaged.cs`,
//! `Binding/BindVirtualized.cs`): the binding of the results of `SortAndPage` and
//! `SortAndVirtualize`, sorted by the comparer of their context.
//!
//! The two classes of upstream are the same but for the type of the context (BindPaged is "a
//! direct lift from BindVirtualized"); here one function serves both, over [`SortContext`].

use super::sort_and_bind::{sort_and_bind_dynamic, Comparer};
use super::SortAndBindOptions;
use crate::cache::{ChangeSet, ChangeSetWithContext, IChangeSet, PageContext, VirtualContext};
use crate::list::IList;
use crate::DynamicDataOptions;
use ferroui_reactive::disposables::{CompositeDisposable, SingleAssignmentDisposable};
use ferroui_reactive::subjects::{IConnectableObservable, ReplaySubject, Subject};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::hash::Hash;
use std::rc::Rc;

/// The context of the change sets of `SortAndPage` and `SortAndVirtualize`, as the binding reads
/// it.
pub(crate) trait SortContext<T>: Clone + 'static {
    /// The comparer of the sort (`Context.Comparer`).
    fn comparer(&self) -> Comparer<T>;
    /// Whether the sort finds positions with a binary search (`Context.Options.UseBinarySearch`).
    fn use_binary_search(&self) -> bool;
    /// The threshold of the resets of the sort (`Context.Options.ResetThreshold`).
    fn reset_threshold(&self) -> usize;
}

impl<T: 'static> SortContext<T> for PageContext<T> {
    fn comparer(&self) -> Comparer<T> {
        self.comparer.clone()
    }

    fn use_binary_search(&self) -> bool {
        self.options.use_binary_search
    }

    fn reset_threshold(&self) -> usize {
        self.options.reset_threshold
    }
}

impl<T: 'static> SortContext<T> for VirtualContext<T> {
    fn comparer(&self) -> Comparer<T> {
        self.comparer.clone()
    }

    fn use_binary_search(&self) -> bool {
        self.options.use_binary_search
    }

    fn reset_threshold(&self) -> usize {
        self.options.reset_threshold
    }
}

/// `Run()`: with `options`, or with the options of the context of the first change set.
pub(crate) fn bind_with_context<T, K, X, L>(source: Rc<dyn IObservable<ChangeSetWithContext<T, K, X>>>, target: L, options: Option<SortAndBindOptions>) -> Rc<dyn IObservable<ChangeSet<T, K>>>
where
    T: Clone + PartialEq + 'static,
    K: Eq + Hash + Clone + 'static,
    X: SortContext<T>,
    L: IList<T> + Clone + 'static,
{
    match options {
        Some(options) => use_provided_options(source, target, options),
        None => use_context_sort_options(source, target),
    }
}

fn use_provided_options<T, K, X, L>(source: Rc<dyn IObservable<ChangeSetWithContext<T, K, X>>>, target: L, options: SortAndBindOptions) -> Rc<dyn IObservable<ChangeSet<T, K>>>
where
    T: Clone + PartialEq + 'static,
    K: Eq + Hash + Clone + 'static,
    X: SortContext<T>,
    L: IList<T> + Clone + 'static,
{
    source.publish_with(move |changes: Rc<dyn IObservable<ChangeSetWithContext<T, K, X>>>| {
        let comparer_changed = changes
            .select(|changes_with_context: ChangeSetWithContext<T, K, X>| changes_with_context.context().comparer())
            .distinct_until_changed_with_comparer(|a: &Comparer<T>, b: &Comparer<T>| Rc::ptr_eq(a, b));
        let plain = changes.select(|changes_with_context: ChangeSetWithContext<T, K, X>| changes_with_context.change_set().clone());
        sort_and_bind_dynamic(plain, comparer_changed, options.clone(), target.clone())
    })
}

fn use_context_sort_options<T, K, X, L>(source: Rc<dyn IObservable<ChangeSetWithContext<T, K, X>>>, target: L) -> Rc<dyn IObservable<ChangeSet<T, K>>>
where
    T: Clone + PartialEq + 'static,
    K: Eq + Hash + Clone + 'static,
    X: SortContext<T>,
    L: IList<T> + Clone + 'static,
{
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let connectable = source.publish();
        let shared: Rc<dyn IObservable<ChangeSetWithContext<T, K, X>>> = connectable.clone();

        let subscriber = SingleAssignmentDisposable::new();

        // Upstream: "I tried to make this work without subjects but had issues making the
        // comparedChanged observable to fire".
        let changes_subject = Subject::<ChangeSet<T, K>>::new();
        let comparer_subject = ReplaySubject::<Comparer<T>>::with_buffer_size(1);

        // Once there are the initial values, publish as normal.
        let subsequent = {
            let (changes_subject, comparer_subject) = (changes_subject.clone(), comparer_subject.clone());
            shared.skip(1).subscribe_next(move |changes_with_context: ChangeSetWithContext<T, K, X>| {
                comparer_subject.on_next(changes_with_context.context().comparer());
                changes_subject.on_next(changes_with_context.change_set().clone());
            })
        };

        // Extract the binding options from the context.
        let initial = {
            let (changes_subject, comparer_subject, subscriber, target) = (changes_subject.clone(), comparer_subject.clone(), subscriber.clone(), target.clone());
            shared.take(1).subscribe_next(move |changes_with_context: ChangeSetWithContext<T, K, X>| {
                let context = changes_with_context.context();
                let mut extracted_options = DynamicDataOptions::sort_and_bind();
                extracted_options.use_binary_search = context.use_binary_search();
                extracted_options.reset_threshold = context.reset_threshold();

                let comparer_changed = comparer_subject.as_observable().distinct_until_changed_with_comparer(|a: &Comparer<T>, b: &Comparer<T>| Rc::ptr_eq(a, b));
                subscriber.set_disposable(Some(sort_and_bind_dynamic(changes_subject.as_observable(), comparer_changed, extracted_options, target.clone()).subscribe(observer.clone())));

                comparer_subject.on_next(context.comparer());
                changes_subject.on_next(changes_with_context.change_set().clone());
            })
        };

        let subscriptions: Vec<Rc<dyn IDisposable>> = vec![initial, subscriber, subsequent, IConnectableObservable::connect(&*connectable)];
        CompositeDisposable::from_disposables(subscriptions) as Rc<dyn IDisposable>
    })
}
