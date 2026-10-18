//! `ReplaySubject<T>` (`ReplaySubject.cs`): `ReplayOne`, `ReplayMany`, `ReplayAll` and
//! `ReplayByTime`.

use super::subject::{throw_disposed, SubjectBase};
use crate::concurrency::{CurrentThreadScheduler, IScheduler, IStopwatch, SchedulerExt};
use crate::disposables::Disposable;
use crate::internal::{FastImmediateObserver, IScheduledObserver, ScheduledObserver};
use crate::time_interval::TimeInterval;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// What the subject keeps for replay.
enum Buffer<T> {
    /// The last value (`ReplayOne`).
    One(Option<T>),
    /// The last `n` values (`ReplayMany`), or all of them (`ReplayAll`, `n` = `None`).
    Many { queue: VecDeque<T>, size: Option<usize> },
    /// The values of a window of time, the last `size` of them (`ReplayByTime`), delivered on a
    /// scheduler.
    ByTime {
        queue: VecDeque<TimeInterval<T>>,
        size: Option<usize>,
        window: Option<Duration>,
        scheduler: Rc<dyn IScheduler>,
        stopwatch: Box<dyn IStopwatch>,
    },
}

struct Inner<T> {
    buffer: Buffer<T>,
    observers: Rc<Vec<Rc<dyn IScheduledObserver<T>>>>,
    is_stopped: bool,
    error: Option<ObservableError>,
    is_disposed: bool,
}

/// A subject that keeps notifications and replays them to every observer that subscribes, then
/// hands it the later ones (`ReplaySubject<T>`).
pub struct ReplaySubject<T> {
    this: Weak<ReplaySubject<T>>,
    inner: RefCell<Inner<T>>,
}

impl<T: Clone + 'static> ReplaySubject<T> {
    fn with_buffer(buffer: Buffer<T>) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            inner: RefCell::new(Inner { buffer, observers: Rc::new(Vec::new()), is_stopped: false, error: None, is_disposed: false }),
        })
    }

    /// A subject that replays every value (`ReplaySubject()`).
    pub fn new() -> Rc<Self> {
        Self::with_buffer(Buffer::Many { queue: VecDeque::new(), size: None })
    }

    /// A subject that replays the last `buffer_size` values (`ReplaySubject(int)`).
    pub fn with_buffer_size(buffer_size: usize) -> Rc<Self> {
        if buffer_size == 1 {
            return Self::with_buffer(Buffer::One(None));
        }
        Self::with_buffer(Buffer::Many { queue: VecDeque::new(), size: Some(buffer_size) })
    }

    /// A subject that replays every value, delivering on `scheduler` (`ReplaySubject(IScheduler)`).
    pub fn with_scheduler(scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Self::by_time(None, None, scheduler)
    }

    /// A subject that replays the last `buffer_size` values, delivering on `scheduler`
    /// (`ReplaySubject(int, IScheduler)`).
    pub fn with_buffer_size_and_scheduler(buffer_size: usize, scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Self::by_time(Some(buffer_size), None, scheduler)
    }

    /// A subject that replays the values of the last `window` (`ReplaySubject(TimeSpan)`), on the
    /// scheduler of the current thread.
    pub fn with_window(window: Duration) -> Rc<Self> {
        Self::by_time(None, Some(window), CurrentThreadScheduler::instance())
    }

    /// A subject that replays the values of the last `window`, delivering on `scheduler`
    /// (`ReplaySubject(TimeSpan, IScheduler)`).
    pub fn with_window_and_scheduler(window: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Self::by_time(None, Some(window), scheduler)
    }

    /// A subject that replays the last `buffer_size` values of the last `window`
    /// (`ReplaySubject(int, TimeSpan)`), on the scheduler of the current thread.
    pub fn with_buffer_size_and_window(buffer_size: usize, window: Duration) -> Rc<Self> {
        Self::by_time(Some(buffer_size), Some(window), CurrentThreadScheduler::instance())
    }

    /// A subject that replays the last `buffer_size` values of the last `window`, delivering on
    /// `scheduler` (`ReplaySubject(int, TimeSpan, IScheduler)`).
    pub fn with_buffer_size_window_and_scheduler(buffer_size: usize, window: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Self::by_time(Some(buffer_size), Some(window), scheduler)
    }

    fn by_time(size: Option<usize>, window: Option<Duration>, scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        let stopwatch = scheduler.start_stopwatch();
        Self::with_buffer(Buffer::ByTime { queue: VecDeque::new(), size, window, scheduler, stopwatch })
    }

    pub fn as_observable(self: &Rc<Self>) -> Rc<dyn IObservable<T>> {
        self.clone()
    }

    pub fn as_observer(self: &Rc<Self>) -> Rc<dyn IObserver<T>> {
        self.clone()
    }

    fn next(buffer: &mut Buffer<T>, value: T) {
        match buffer {
            Buffer::One(slot) => *slot = Some(value),
            Buffer::Many { queue, .. } => queue.push_back(value),
            Buffer::ByTime { queue, stopwatch, .. } => queue.push_back(TimeInterval::new(value, stopwatch.elapsed())),
        }
    }

    fn trim(buffer: &mut Buffer<T>) {
        match buffer {
            Buffer::One(_) => {}
            Buffer::Many { queue, size } => {
                if let Some(size) = *size {
                    while queue.len() > size {
                        queue.pop_front();
                    }
                }
            }
            Buffer::ByTime { queue, size, window, stopwatch, .. } => {
                if let Some(size) = *size {
                    while queue.len() > size {
                        queue.pop_front();
                    }
                }
                if let Some(window) = *window {
                    let now = stopwatch.elapsed();
                    while queue.front().is_some_and(|item| now.saturating_sub(item.interval) > window) {
                        queue.pop_front();
                    }
                }
            }
        }
    }

    fn replay(buffer: &Buffer<T>) -> Vec<T> {
        match buffer {
            Buffer::One(slot) => slot.iter().cloned().collect(),
            Buffer::Many { queue, .. } => queue.iter().cloned().collect(),
            Buffer::ByTime { queue, .. } => queue.iter().map(|item| item.value.clone()).collect(),
        }
    }

    fn create_scheduled_observer(buffer: &Buffer<T>, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IScheduledObserver<T>> {
        match buffer {
            Buffer::ByTime { scheduler, .. } => ScheduledObserver::new(scheduler.clone(), observer),
            _ => FastImmediateObserver::new(observer),
        }
    }

    fn unsubscribe(&self, observer: &Rc<dyn IScheduledObserver<T>>) {
        let mut inner = self.inner.borrow_mut();
        if inner.is_disposed {
            return;
        }
        if let Some(index) = inner.observers.iter().position(|o| Rc::ptr_eq(o, observer)) {
            let mut next: Vec<_> = inner.observers.iter().cloned().collect();
            next.remove(index);
            inner.observers = Rc::new(next);
        }
    }
}

impl<T: Clone + 'static> IObserver<T> for ReplaySubject<T> {
    fn on_next(&self, value: T) {
        let observers = {
            let mut inner = self.inner.borrow_mut();
            if inner.is_disposed {
                throw_disposed();
            }
            if inner.is_stopped {
                return;
            }
            Self::next(&mut inner.buffer, value.clone());
            Self::trim(&mut inner.buffer);
            inner.observers.clone()
        };
        for observer in observers.iter() {
            observer.on_next(value.clone());
        }
        for observer in observers.iter() {
            observer.ensure_active();
        }
    }

    fn on_error(&self, error: ObservableError) {
        let observers = {
            let mut inner = self.inner.borrow_mut();
            if inner.is_disposed {
                throw_disposed();
            }
            if inner.is_stopped {
                return;
            }
            inner.is_stopped = true;
            inner.error = Some(error.clone());
            Self::trim(&mut inner.buffer);
            std::mem::replace(&mut inner.observers, Rc::new(Vec::new()))
        };
        for observer in observers.iter() {
            observer.on_error(error.clone());
        }
        for observer in observers.iter() {
            observer.ensure_active();
        }
    }

    fn on_completed(&self) {
        let observers = {
            let mut inner = self.inner.borrow_mut();
            if inner.is_disposed {
                throw_disposed();
            }
            if inner.is_stopped {
                return;
            }
            inner.is_stopped = true;
            Self::trim(&mut inner.buffer);
            std::mem::replace(&mut inner.observers, Rc::new(Vec::new()))
        };
        for observer in observers.iter() {
            observer.on_completed();
        }
        for observer in observers.iter() {
            observer.ensure_active();
        }
    }
}

impl<T: Clone + 'static> IObservable<T> for ReplaySubject<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (scheduled, values, error, is_stopped) = {
            let mut inner = self.inner.borrow_mut();
            if inner.is_disposed {
                throw_disposed();
            }
            let scheduled = Self::create_scheduled_observer(&inner.buffer, observer);
            // The buffer is trimmed at every subscription, also after the end (upstream keeps the
            // behaviour of v1: a window of time empties after the end too).
            Self::trim(&mut inner.buffer);
            let values = Self::replay(&inner.buffer);
            (scheduled, values, inner.error.clone(), inner.is_stopped)
        };
        let mut n = values.len();
        for value in values {
            scheduled.on_next(value);
        }
        if let Some(error) = error {
            n += 1;
            scheduled.on_error(error);
        } else if is_stopped {
            n += 1;
            scheduled.on_completed();
        }
        let subscription: Rc<dyn IDisposable> = if !is_stopped {
            {
                let mut inner = self.inner.borrow_mut();
                let mut next: Vec<_> = inner.observers.iter().cloned().collect();
                next.push(scheduled.clone());
                inner.observers = Rc::new(next);
            }
            let subject = self.this.clone();
            let entry = RefCell::new(Some(scheduled.clone()));
            Disposable::create(move || {
                let observer = entry.borrow_mut().take();
                if let Some(observer) = observer {
                    observer.dispose();
                    if let Some(subject) = subject.upgrade() {
                        subject.unsubscribe(&observer);
                    }
                }
            })
        } else {
            Disposable::empty()
        };
        scheduled.ensure_active_n(n);
        subscription
    }
}

impl<T: Clone + 'static> IDisposable for ReplaySubject<T> {
    fn dispose(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.is_disposed = true;
        inner.observers = Rc::new(Vec::new());
        match &mut inner.buffer {
            Buffer::One(slot) => *slot = None,
            Buffer::Many { queue, .. } => queue.clear(),
            Buffer::ByTime { queue, .. } => queue.clear(),
        }
    }
}

impl<T: Clone + 'static> SubjectBase<T> for ReplaySubject<T> {
    fn has_observers(&self) -> bool {
        let inner = self.inner.borrow();
        !inner.is_disposed && !inner.observers.is_empty()
    }

    fn is_disposed(&self) -> bool {
        self.inner.borrow().is_disposed
    }
}
