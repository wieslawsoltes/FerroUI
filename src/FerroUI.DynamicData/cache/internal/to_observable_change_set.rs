//! `ToObservableChangeSet<TObject, TKey>` (`Cache/Internal/ToObservableChangeSet.cs`): the items of
//! a sequence as the changes of a cache, which may expire them after a time or evict the oldest
//! beyond a size.

use crate::cache::{ChangeAwareCache, ChangeSet, ICache, IQuery};
use crate::kernel::dotnet_binary_search;
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::disposables::{Disposable, SingleAssignmentDisposable};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// The time cut to the whole millisecond, as upstream cuts the ticks of its `DateTimeOffset`.
pub(crate) fn truncate_to_millisecond(time: SystemTime) -> SystemTime {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(since_epoch) => SystemTime::UNIX_EPOCH + Duration::from_millis(since_epoch.as_millis() as u64),
        Err(_) => time,
    }
}

/// An expiration (`Expiration`): equal when both parts are, ordered by the time.
#[derive(Clone, PartialEq)]
struct Expiration<K> {
    expire_at: SystemTime,
    key: K,
}

struct ScheduledExpiration<K> {
    cancellation: Rc<SingleAssignmentDisposable>,
    expiration: Expiration<K>,
}

struct Subscription<T, K> {
    downstream_items: ChangeAwareCache<T, K>,
    eviction_queue: VecDeque<K>,
    expiration_queue: Vec<Expiration<K>>,
    expire_after: Option<Rc<dyn Fn(&T) -> Option<Duration>>>,
    expire_ats_by_key: HashMap<K, SystemTime>,
    key_selector: Rc<dyn Fn(&T) -> K>,
    limit_size_to: Option<usize>,
    scheduler: Rc<dyn IScheduler>,
    has_initialized: bool,
    has_source_completed: bool,
    scheduled_expiration: Option<ScheduledExpiration<K>>,
}

/// What a step does once the state is released.
struct Effects<T: 'static, K: 'static> {
    to_cancel: Vec<Rc<SingleAssignmentDisposable>>,
    next: Option<ChangeSet<T, K>>,
    completed: bool,
    unfinished_expiration: Option<(Expiration<K>, Rc<SingleAssignmentDisposable>)>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Subscription<T, K> {
    fn new_effects() -> Effects<T, K> {
        Effects { to_cancel: Vec::new(), next: None, completed: false, unfinished_expiration: None }
    }

    /// `OnScheduledExpirationInvoked`.
    fn on_scheduled_expiration_invoked(&mut self, intended_expiration: &Expiration<K>, effects: &mut Effects<T, K>) {
        self.scheduled_expiration = None;
        let mut processed_expiration_count = 0;
        for expiration in &self.expiration_queue {
            if expiration.expire_at > intended_expiration.expire_at {
                break;
            }
            processed_expiration_count += 1;
            if self.expire_ats_by_key.get(&expiration.key).is_some_and(|expire_at| *expire_at <= intended_expiration.expire_at) {
                self.downstream_items.remove(&expiration.key);
                self.expire_ats_by_key.remove(&expiration.key);
            }
        }
        self.expiration_queue.drain(0..processed_expiration_count);
        self.try_begin_scheduling_expiration(effects);
        self.try_publish_downstream_changes(effects);
        self.try_publish_completion(effects);
    }

    /// `OnSourceNext`.
    fn on_source_next(&mut self, upstream_items: Vec<T>, effects: &mut Effects<T, K>) {
        let now = self.scheduler.now();
        for item in upstream_items {
            let key = (self.key_selector)(&item);
            if let Some(limit_size_to) = self.limit_size_to {
                if self.downstream_items.lookup(&key).is_none() {
                    while self.downstream_items.count() >= limit_size_to {
                        match self.eviction_queue.pop_front() {
                            Some(evicted) => self.downstream_items.remove(&evicted),
                            None => break,
                        }
                    }
                    self.eviction_queue.push_back(key.clone());
                }
            }
            let lifetime = self.expire_after.as_ref().and_then(|expire_after| expire_after(&item));
            self.downstream_items.add_or_update(item, key.clone());
            match lifetime {
                Some(lifetime) => {
                    let expire_at = truncate_to_millisecond(now + lifetime);
                    self.expire_ats_by_key.insert(key.clone(), expire_at);
                    let expiration = Expiration { expire_at, key };
                    let insertion_index = dotnet_binary_search(&self.expiration_queue, &expiration, |item, value| item.expire_at.cmp(&value.expire_at)).unwrap_or_else(|index| index);
                    self.expiration_queue.insert(insertion_index, expiration);
                }
                None => {
                    self.expire_ats_by_key.remove(&key);
                }
            }
        }
        self.try_begin_scheduling_expiration(effects);
        self.try_publish_downstream_changes(effects);
    }

    /// `TryPublishCompletion`.
    fn try_publish_completion(&mut self, effects: &mut Effects<T, K>) {
        if self.has_initialized && self.has_source_completed && self.expiration_queue.is_empty() {
            effects.completed = true;
        }
    }

    /// `TryPublishDownstreamChanges`: the first change set is published even when empty.
    fn try_publish_downstream_changes(&mut self, effects: &mut Effects<T, K>) {
        let downstream_changes = self.downstream_items.capture_changes();
        if downstream_changes.count() != 0 || !self.has_initialized {
            effects.next = Some(downstream_changes);
            self.has_initialized = true;
        }
    }

    /// `TryBeginSchedulingExpiration`: the expiration to schedule, if the next one has changed.
    fn try_begin_scheduling_expiration(&mut self, effects: &mut Effects<T, K>) {
        let Some(next_expiration) = self.expiration_queue.first().cloned() else { return };
        if self.scheduled_expiration.as_ref().is_some_and(|scheduled| scheduled.expiration == next_expiration) {
            return;
        }
        if let Some(scheduled) = self.scheduled_expiration.take() {
            effects.to_cancel.push(scheduled.cancellation);
        }
        let cancellation = SingleAssignmentDisposable::new();
        self.scheduled_expiration = Some(ScheduledExpiration { cancellation: cancellation.clone(), expiration: next_expiration.clone() });
        effects.unfinished_expiration = Some((next_expiration, cancellation));
    }
}

struct Context<T: 'static, K: 'static> {
    state: RefCell<Subscription<T, K>>,
    observer: Rc<dyn IObserver<ChangeSet<T, K>>>,
    this: Weak<Context<T, K>>,
}

impl<T: Clone + 'static, K: Eq + Hash + Clone + 'static> Context<T, K> {
    /// Runs a step under the state, then publishes what it made (within the state upstream) and
    /// finishes the scheduling of the next expiration (outside of it upstream).
    fn run(&self, step: &mut dyn FnMut(&mut Subscription<T, K>, &mut Effects<T, K>)) {
        let mut effects = Subscription::new_effects();
        let scheduler = {
            let mut state = self.state.borrow_mut();
            step(&mut state, &mut effects);
            state.scheduler.clone()
        };
        for cancellation in effects.to_cancel {
            cancellation.dispose();
        }
        if let Some(next) = effects.next {
            self.observer.on_next(next);
        }
        if effects.completed {
            self.observer.on_completed();
        }
        if let Some((expiration, cancellation)) = effects.unfinished_expiration {
            self.finish_scheduling_expiration(&scheduler, expiration, cancellation);
        }
    }

    /// `FinishSchedulingExpiration`: the scheduler holds the subscription weakly.
    fn finish_scheduling_expiration(&self, scheduler: &Rc<dyn IScheduler>, expiration: Expiration<K>, cancellation: Rc<SingleAssignmentDisposable>) {
        let this = self.this.clone();
        let due_time = expiration.expire_at;
        let scheduled = scheduler.schedule_at(
            due_time,
            Box::new(move |_| {
                if let Some(this) = this.upgrade() {
                    this.run(&mut |state, effects| state.on_scheduled_expiration_invoked(&expiration, effects));
                }
                Disposable::empty()
            }),
        );
        cancellation.set_disposable(Some(scheduled));
    }

    fn on_error(&self, error: ObservableError) {
        self.observer.on_error(error);
    }
}

/// `ToObservableChangeSet.Create(IObservable<IEnumerable<TObject>>, ..)`, for any sequence of
/// collections of items.
pub(crate) fn to_observable_change_set<T: Clone + 'static, K: Eq + Hash + Clone + 'static, I: IntoIterator<Item = T> + 'static>(
    source: Rc<dyn IObservable<I>>,
    key_selector: Rc<dyn Fn(&T) -> K>,
    expire_after: Option<Rc<dyn Fn(&T) -> Option<Duration>>>,
    limit_size_to: Option<usize>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T, K>>>| {
        let context = Rc::new_cyclic(|this| Context {
            state: RefCell::new(Subscription {
                downstream_items: ChangeAwareCache::new(),
                eviction_queue: VecDeque::new(),
                expiration_queue: Vec::new(),
                expire_after: expire_after.clone(),
                expire_ats_by_key: HashMap::new(),
                key_selector: key_selector.clone(),
                limit_size_to,
                scheduler: scheduler.clone().unwrap_or_else(SchedulerDefaults::time_based_operations),
                has_initialized: false,
                has_source_completed: false,
                scheduled_expiration: None,
            }),
            observer,
            this: this.clone(),
        });

        let source_subscription = {
            let (next_context, error_context, completed_context) = (context.clone(), context.clone(), context.clone());
            source.subscribe_all(
                move |items: I| {
                    let items: Vec<T> = items.into_iter().collect();
                    let mut items = Some(items);
                    next_context.run(&mut |state, effects| state.on_source_next(items.take().expect("a step runs once"), effects));
                },
                move |error| error_context.on_error(error),
                move || {
                    completed_context.run(&mut |state, effects| {
                        state.has_source_completed = true;
                        state.try_publish_completion(effects);
                    })
                },
            )
        };

        context.run(&mut |state, effects| {
            state.try_publish_downstream_changes(effects);
            state.try_publish_completion(effects);
        });

        Disposable::create(move || {
            source_subscription.dispose();
            let scheduled = context.state.borrow_mut().scheduled_expiration.take();
            if let Some(scheduled) = scheduled {
                scheduled.cancellation.dispose();
            }
        })
    })
}

/// `ToObservableChangeSet.Create(IObservable<TObject>, ..)`: each item as a collection of one.
pub(crate) fn to_observable_change_set_of_items<T: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<T>>,
    key_selector: Rc<dyn Fn(&T) -> K>,
    expire_after: Option<Rc<dyn Fn(&T) -> Option<Duration>>>,
    limit_size_to: Option<usize>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T, K>>> {
    to_observable_change_set(source.select(|item: T| std::iter::once(item)), key_selector, expire_after, limit_size_to, scheduler)
}
