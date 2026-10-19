//! `TrueFor<TObject, TKey, TValue>` (`Cache/Internal/TrueFor.cs`): whether a condition holds for
//! the latest values of the observables of the items, as those values and the items change.
//!
//! Upstream composes `Transform`, `Publish`, `MergeMany`, `ToCollection`, `CombineLatest` and
//! `DistinctUntilChanged`; the port runs the same steps in one operator: the items become
//! observables with their latest values (the transform), each is subscribed while its item is
//! there (the merge, whose errors are ignored and which ends when the source and every observable
//! have), the collection of them is taken after each change set (the collection), and the
//! condition is evaluated over the collection whenever a value arrives once both have (the
//! combination), each result different from the last one delivered.
//!
//! [`merge_many_values`] is the merge alone, for `QueryWhenChanged`.

use super::ObservableWithValue;
use crate::cache::{ChangeReason, ChangeSet};
use crate::internal::DeliveryQueue;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::collections::generic::Dictionary;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

/// The subscriptions of the observables of the items (`SubscribeMany` within `MergeMany`): a
/// subscription per key, whose end (completion, error or disposal) counts down once.
struct ItemSubscriptions<K> {
    subscriptions: RefCell<HashMap<K, Rc<dyn IDisposable>>>,
    counter: Cell<usize>,
}

impl<K: Eq + Hash + Clone> ItemSubscriptions<K> {
    fn new() -> Rc<Self> {
        Rc::new(Self { subscriptions: RefCell::new(HashMap::new()), counter: Cell::new(1) })
    }

    /// `CheckCompleted`: whether the merge has ended.
    fn count_down(&self) -> bool {
        let counter = self.counter.get() - 1;
        self.counter.set(counter);
        counter == 0
    }

    /// Subscribes `observable` for `key`: `on_value` receives its values; `on_end` runs once when it
    /// ends or is disposed. The subscription of an update is made before the previous one of the key
    /// is disposed, as `Transform` and `DisposeMany` do.
    fn subscribe<V: 'static>(self: &Rc<Self>, key: K, observable: &Rc<dyn IObservable<V>>, on_value: Rc<dyn Fn(V)>, on_end: Rc<dyn Fn()>) {
        self.counter.set(self.counter.get() + 1);
        let ended = Rc::new(Cell::new(false));
        let end = {
            let ended = ended.clone();
            Rc::new(move || {
                if !ended.replace(true) {
                    on_end();
                }
            })
        };
        let (completed_end, error_end) = (end.clone(), end.clone());
        let subscription = observable.subscribe_all(move |value: V| on_value(value), move |_| error_end(), move || completed_end());
        let subscription: Rc<dyn IDisposable> = Disposable::create(move || {
            subscription.dispose();
            end();
        });
        let previous = self.subscriptions.borrow_mut().insert(key, subscription);
        if let Some(previous) = previous {
            previous.dispose();
        }
    }

    fn unsubscribe(&self, key: &K) {
        let subscription = self.subscriptions.borrow_mut().remove(key);
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn dispose_all(&self) {
        let subscriptions: Vec<Rc<dyn IDisposable>> = self.subscriptions.borrow_mut().drain().map(|(_, subscription)| subscription).collect();
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
}

/// The values of the observables that `selector` gives for the items of `source`, merged
/// (`MergeMany(Func<TObject, IObservable<TDestination>>)`): the observable of an item is
/// subscribed while the item is there, its errors are ignored, and the result ends once the source
/// and every observable have ended.
pub(crate) fn merge_many_values<T: Clone + 'static, K: Eq + Hash + Clone + 'static, V: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    selector: Rc<dyn Fn(&T) -> Rc<dyn IObservable<V>>>,
) -> Rc<dyn IObservable<V>> {
    Observable::create(move |observer: Rc<dyn IObserver<V>>| {
        let queue = DeliveryQueue::new(observer.clone());
        let subscriptions = ItemSubscriptions::<K>::new();
        let check_completed: Rc<dyn Fn()> = {
            let (queue, subscriptions) = (queue.clone(), Rc::downgrade(&subscriptions));
            Rc::new(move || {
                if subscriptions.upgrade().is_some_and(|subscriptions| subscriptions.count_down()) && !queue.is_terminated() {
                    queue.acquire().enqueue_completed();
                }
            })
        };
        let on_value: Rc<dyn Fn(V)> = {
            let queue = queue.clone();
            Rc::new(move |value| queue.acquire().enqueue_next(value))
        };

        let source_subscription = {
            let (subscriptions, selector, completed) = (subscriptions.clone(), selector.clone(), check_completed.clone());
            source.subscribe_all(
                move |changes: ChangeSet<T, K>| {
                    for change in &changes {
                        match change.reason() {
                            ChangeReason::Add | ChangeReason::Update => {
                                let observable = selector(change.current());
                                subscriptions.subscribe(change.key().clone(), &observable, on_value.clone(), check_completed.clone());
                            }
                            ChangeReason::Remove => subscriptions.unsubscribe(change.key()),
                            ChangeReason::Refresh | ChangeReason::Moved => {}
                        }
                    }
                },
                move |error| observer.on_error(error),
                move || completed(),
            )
        };

        Disposable::create(move || {
            queue.dispose();
            source_subscription.dispose();
            subscriptions.dispose_all();
        })
    })
}

type Collection<T, V> = Vec<Rc<ObservableWithValue<T, V>>>;

struct State<T, K, V> {
    items: Dictionary<K, Rc<ObservableWithValue<T, V>>>,
    collection: Option<Collection<T, V>>,
    merged_has_value: bool,
    merged_completed: bool,
    collection_completed: bool,
    last: Option<bool>,
    terminated: bool,
}

struct Context<T: 'static, K: 'static, V: 'static> {
    state: RefCell<State<T, K, V>>,
    subscriptions: Rc<ItemSubscriptions<K>>,
    observer: Rc<dyn IObserver<bool>>,
    collection_matcher: Rc<dyn Fn(&[Rc<ObservableWithValue<T, V>>]) -> bool>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static, V: Clone + 'static> Context<T, K, V> {
    /// `CombineLatest(..).DistinctUntilChanged()`: the condition over the latest collection.
    fn evaluate(&self) {
        let collection = {
            let state = self.state.borrow();
            if state.terminated || !state.merged_has_value {
                return;
            }
            match &state.collection {
                Some(collection) => collection.clone(),
                None => return,
            }
        };
        let result = (self.collection_matcher)(&collection);
        let changed = {
            let mut state = self.state.borrow_mut();
            let changed = state.last != Some(result);
            state.last = Some(result);
            changed
        };
        if changed {
            self.observer.on_next(result);
        }
    }

    /// A value of the merge.
    fn on_merged_value(&self, item: &Rc<ObservableWithValue<T, V>>, value: V) {
        item.set_latest_value(value);
        self.state.borrow_mut().merged_has_value = true;
        self.evaluate();
    }

    fn on_error(&self, error: ObservableError) {
        let first = !std::mem::replace(&mut self.state.borrow_mut().terminated, true);
        if first {
            self.observer.on_error(error);
        }
    }

    /// The end of one side of the combination: the result ends when the side ends without a value,
    /// or when both have ended.
    fn on_side_completed(&self, merged: bool) {
        let complete = {
            let mut state = self.state.borrow_mut();
            if state.terminated {
                return;
            }
            let has_value = if merged {
                state.merged_completed = true;
                state.merged_has_value
            } else {
                state.collection_completed = true;
                state.collection.is_some()
            };
            let complete = !has_value || (state.merged_completed && state.collection_completed);
            if complete {
                state.terminated = true;
            }
            complete
        };
        if complete {
            self.observer.on_completed();
        }
    }
}

/// `TrueFor.Run`.
pub(crate) fn true_for<T: Clone + 'static, K: Eq + Hash + Clone + 'static, V: Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T, K>>>,
    observable_selector: Rc<dyn Fn(&T) -> Rc<dyn IObservable<V>>>,
    collection_matcher: Rc<dyn Fn(&[Rc<ObservableWithValue<T, V>>]) -> bool>,
) -> Rc<dyn IObservable<bool>> {
    Observable::create(move |observer: Rc<dyn IObserver<bool>>| {
        let context = Rc::new(Context {
            state: RefCell::new(State { items: Dictionary::new(), collection: None, merged_has_value: false, merged_completed: false, collection_completed: false, last: None, terminated: false }),
            subscriptions: ItemSubscriptions::new(),
            observer,
            collection_matcher: collection_matcher.clone(),
        });
        let check_completed: Rc<dyn Fn()> = {
            let context = Rc::downgrade(&context);
            Rc::new(move || {
                if let Some(context) = context.upgrade() {
                    if context.subscriptions.count_down() {
                        context.on_side_completed(true);
                    }
                }
            })
        };

        let source_subscription = {
            let (next_context, error_context, completed_context) = (context.clone(), context.clone(), context.clone());
            let observable_selector = observable_selector.clone();
            let completed = check_completed.clone();
            source.subscribe_all(
                move |changes: ChangeSet<T, K>| {
                    let context = &next_context;
                    for change in &changes {
                        match change.reason() {
                            ChangeReason::Add | ChangeReason::Update => {
                                // The transform.
                                let item = Rc::new(ObservableWithValue::new(change.current().clone(), observable_selector(change.current())));
                                context.state.borrow_mut().items.insert(change.key().clone(), item.clone());
                                // The merge.
                                let on_value: Rc<dyn Fn(V)> = {
                                    let (context, item) = (Rc::downgrade(context), item.clone());
                                    Rc::new(move |value| {
                                        if let Some(context) = context.upgrade() {
                                            context.on_merged_value(&item, value);
                                        }
                                    })
                                };
                                context.subscriptions.subscribe(change.key().clone(), item.observable(), on_value, check_completed.clone());
                            }
                            ChangeReason::Remove => {
                                context.state.borrow_mut().items.remove(change.key());
                                context.subscriptions.unsubscribe(change.key());
                            }
                            ChangeReason::Refresh | ChangeReason::Moved => {}
                        }
                    }
                    // The collection.
                    {
                        let mut state = context.state.borrow_mut();
                        let collection: Collection<T, V> = state.items.values().cloned().collect();
                        state.collection = Some(collection);
                    }
                    context.evaluate();
                },
                move |error| error_context.on_error(error),
                move || {
                    // The merge counts the end of the source down, then the collection ends.
                    completed();
                    completed_context.on_side_completed(false);
                },
            )
        };

        Disposable::create(move || {
            context.state.borrow_mut().terminated = true;
            source_subscription.dispose();
            context.subscriptions.dispose_all();
        })
    })
}
