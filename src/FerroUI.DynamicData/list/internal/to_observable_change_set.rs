//! `ToObservableChangeSet<TObject>` (`List/Internal/ToObservableChangeSet.cs`): the items of a
//! sequence as the changes of a list, which may expire them after a time or drop the oldest beyond
//! a size.

use crate::cache::internal::truncate_to_millisecond;
use crate::kernel::dotnet_binary_search;
use crate::list::{ChangeAwareList, ChangeSet};
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::disposables::{Disposable, SingleAssignmentDisposable};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// An expiration (`Expiration`): equal when both parts are, ordered by the time.
#[derive(Clone, PartialEq)]
struct Expiration {
    expire_at: SystemTime,
    index: usize,
}

struct ScheduledExpiration {
    cancellation: Rc<SingleAssignmentDisposable>,
    expiration: Expiration,
}

struct Subscription<T> {
    downstream_items: ChangeAwareList<T>,
    expire_after: Option<Rc<dyn Fn(&T) -> Option<Duration>>>,
    expiration_queue: Vec<Expiration>,
    limit_size_to: Option<usize>,
    scheduler: Rc<dyn IScheduler>,
    has_initialized: bool,
    has_source_completed: bool,
    scheduled_expiration: Option<ScheduledExpiration>,
}

/// What a step does once the state is released.
struct Effects<T: 'static> {
    to_cancel: Vec<Rc<SingleAssignmentDisposable>>,
    next: Option<ChangeSet<T>>,
    completed: bool,
    unfinished_expiration: Option<(Expiration, Rc<SingleAssignmentDisposable>)>,
}

impl<T: Clone + PartialEq + 'static> Subscription<T> {
    fn new_effects() -> Effects<T> {
        Effects { to_cancel: Vec::new(), next: None, completed: false, unfinished_expiration: None }
    }

    /// `OnScheduledExpirationInvoked`.
    fn on_scheduled_expiration_invoked(&mut self, intended_expiration: &Expiration, effects: &mut Effects<T>) {
        self.scheduled_expiration = None;
        let mut processed_expiration_count = 0;
        let mut i = 0;
        while i < self.expiration_queue.len() {
            let expiration = self.expiration_queue[i].clone();
            if expiration.expire_at > intended_expiration.expire_at {
                break;
            }
            processed_expiration_count += 1;
            self.downstream_items.remove_at(expiration.index);
            for future_expiration in &mut self.expiration_queue[i + 1..] {
                if future_expiration.index > expiration.index {
                    future_expiration.index -= 1;
                }
            }
            i += 1;
        }
        self.expiration_queue.drain(0..processed_expiration_count);
        self.try_begin_scheduling_expiration(effects);
        self.try_publish_downstream_changes(effects);
        self.try_publish_completion(effects);
    }

    /// `OnSourceNext`.
    fn on_source_next(&mut self, upstream_items: Vec<T>, effects: &mut Effects<T>) {
        let now = self.scheduler.now();
        for item in upstream_items {
            if let Some(limit_size_to) = self.limit_size_to {
                while self.downstream_items.count() >= limit_size_to && self.downstream_items.count() > 0 {
                    self.downstream_items.remove_at(0);
                }
                let mut i = 0;
                while i < self.expiration_queue.len() {
                    if self.expiration_queue[i].index == 0 {
                        self.expiration_queue.remove(i);
                        continue;
                    }
                    self.expiration_queue[i].index -= 1;
                    i += 1;
                }
                if let Some(scheduled_expiration) = self.scheduled_expiration.as_mut() {
                    if scheduled_expiration.expiration.index == 0 {
                        effects.to_cancel.push(scheduled_expiration.cancellation.clone());
                        self.scheduled_expiration = None;
                    } else {
                        scheduled_expiration.expiration.index -= 1;
                    }
                }
            }

            let lifetime = self.expire_after.as_ref().and_then(|expire_after| expire_after(&item));
            self.downstream_items.add(item);
            if let Some(lifetime) = lifetime {
                let expire_at = truncate_to_millisecond(now + lifetime);
                let expiration = Expiration { expire_at, index: self.downstream_items.count() - 1 };
                let insertion_index = dotnet_binary_search(&self.expiration_queue, &expiration, |item, value| item.expire_at.cmp(&value.expire_at)).unwrap_or_else(|index| index);
                self.expiration_queue.insert(insertion_index, expiration);
            }
        }
        self.try_begin_scheduling_expiration(effects);
        self.try_publish_downstream_changes(effects);
    }

    /// `TryPublishCompletion`.
    fn try_publish_completion(&mut self, effects: &mut Effects<T>) {
        if self.has_initialized && self.has_source_completed && self.expiration_queue.is_empty() {
            effects.completed = true;
        }
    }

    /// `TryPublishDownstreamChanges`: the first change set is published even when empty.
    fn try_publish_downstream_changes(&mut self, effects: &mut Effects<T>) {
        let downstream_changes = self.downstream_items.capture_changes();
        if downstream_changes.count() != 0 || !self.has_initialized {
            effects.next = Some(downstream_changes);
            self.has_initialized = true;
        }
    }

    /// `TryBeginSchedulingExpiration`.
    fn try_begin_scheduling_expiration(&mut self, effects: &mut Effects<T>) {
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

struct Context<T: 'static> {
    state: RefCell<Subscription<T>>,
    observer: Rc<dyn IObserver<ChangeSet<T>>>,
    this: Weak<Context<T>>,
}

impl<T: Clone + PartialEq + 'static> Context<T> {
    fn run(&self, step: &mut dyn FnMut(&mut Subscription<T>, &mut Effects<T>)) {
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
    }
}

/// `ToObservableChangeSet.Create(IObservable<IEnumerable<TObject>>, ..)`, for any sequence of
/// collections of items.
pub(crate) fn to_observable_change_set<T: Clone + PartialEq + 'static, I: IntoIterator<Item = T> + 'static>(
    source: Rc<dyn IObservable<I>>,
    expire_after: Option<Rc<dyn Fn(&T) -> Option<Duration>>>,
    limit_size_to: Option<usize>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let context = Rc::new_cyclic(|this| Context {
            state: RefCell::new(Subscription {
                downstream_items: ChangeAwareList::new(),
                expire_after: expire_after.clone(),
                expiration_queue: Vec::new(),
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
                move |error| error_context.observer.on_error(error),
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
pub(crate) fn to_observable_change_set_of_items<T: Clone + PartialEq + 'static>(
    source: Rc<dyn IObservable<T>>,
    expire_after: Option<Rc<dyn Fn(&T) -> Option<Duration>>>,
    limit_size_to: Option<usize>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    to_observable_change_set(source.select(|item: T| std::iter::once(item)), expire_after, limit_size_to, scheduler)
}
