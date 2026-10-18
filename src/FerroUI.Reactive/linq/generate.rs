//! `Generate` (`Linq/Observable/Generate.cs`).

use crate::concurrency::{IScheduler, ScheduledAction, SchedulerDefaults, SchedulerExt};
use crate::disposables::{Disposable, MultipleAssignmentDisposableValue, SingleAssignmentDisposable};
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, SystemTime};

type Condition<S> = Rc<dyn Fn(&S) -> Result<bool, ObservableError>>;
type Iterate<S> = Rc<dyn Fn(S) -> Result<S, ObservableError>>;
type ResultSelector<S, R> = Rc<dyn Fn(&S) -> Result<R, ObservableError>>;
type TimeSelector<S, D> = Rc<dyn Fn(&S) -> Result<D, ObservableError>>;

/// The functions of a generated sequence. A function that fails ends the sequence with its error
/// (the `try`/`catch` around the functions upstream).
struct Functions<S, R> {
    condition: Condition<S>,
    iterate: Iterate<S>,
    result_selector: ResultSelector<S, R>,
}

impl<S, R> Clone for Functions<S, R> {
    fn clone(&self) -> Self {
        Self { condition: self.condition.clone(), iterate: self.iterate.clone(), result_selector: self.result_selector.clone() }
    }
}

/// `Generate<TState, TResult>.NoTime`: each value is notified by work of its own on the scheduler,
/// which schedules the next by recursive scheduling.
///
/// Deviation (DEVIATIONS.md, Reactive): the schedulers of the port have no long-running service
/// (`ISchedulerLongRunning`), so the loop of the long-running form has no counterpart; every
/// scheduler takes the recursive form.
struct GenerateNoTime<S, R> {
    initial_state: S,
    functions: Functions<S, R>,
    scheduler: Rc<dyn IScheduler>,
}

/// The sink of `NoTime`.
struct NoTimeSink<S, R> {
    sink: Sink<R>,
    functions: Functions<S, R>,
    state: RefCell<Option<S>>,
    first: Cell<bool>,
}

impl<S: 'static, R: 'static> NoTimeSink<S, R> {
    fn run(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) {
        let upstream = scheduler.schedule_recursive_with(self.clone(), |this: Rc<Self>, recurse| this.loop_rec(recurse));
        self.sink.set_upstream(upstream);
    }

    /// The next state and its value, if the condition holds for it.
    fn step(&self) -> Result<Option<R>, ObservableError> {
        let state = self.state.borrow_mut().take().expect("the state of the sequence is set between its steps");
        let state = if self.first.replace(false) { state } else { (self.functions.iterate)(state)? };
        let has_result = (self.functions.condition)(&state)?;
        let result = if has_result { Some((self.functions.result_selector)(&state)?) } else { None };
        *self.state.borrow_mut() = Some(state);
        Ok(result)
    }

    fn loop_rec(self: Rc<Self>, recurse: &dyn Fn(Rc<Self>)) {
        match self.step() {
            Err(error) => self.sink.forward_on_error(error),
            Ok(Some(result)) => {
                self.sink.forward_on_next(result);
                recurse(self.clone());
            }
            Ok(None) => self.sink.forward_on_completed(),
        }
    }
}

impl<S: 'static, R: 'static> IDisposable for NoTimeSink<S, R> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

impl<S: Clone + 'static, R: 'static> IObservable<R> for GenerateNoTime<S, R> {
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        let state = self.initial_state.clone();
        let functions = self.functions.clone();
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            move |observer| Rc::new(NoTimeSink { sink: Sink::new(observer), functions, state: RefCell::new(Some(state)), first: Cell::new(true) }),
            move |sink: &Rc<NoTimeSink<S, R>>| sink.run(&scheduler),
        )
    }
}

/// A due time of the timed forms: a time relative to now (`TimeSpan`, the form `Relative`) or an
/// absolute one (`DateTimeOffset`, the form `Absolute`).
trait DueTime: 'static {
    fn schedule(scheduler: &Rc<dyn IScheduler>, due_time: Self, action: ScheduledAction) -> Rc<dyn IDisposable>;
}

impl DueTime for Duration {
    fn schedule(scheduler: &Rc<dyn IScheduler>, due_time: Self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        scheduler.schedule_after(due_time, action)
    }
}

impl DueTime for SystemTime {
    fn schedule(scheduler: &Rc<dyn IScheduler>, due_time: Self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        scheduler.schedule_at(due_time, action)
    }
}

/// `Generate<TState, TResult>.Relative` and `.Absolute`: each value is notified at the time the
/// time selector gives for its state, by work scheduled at that time, which computes the next.
struct GenerateTimed<S, R, D> {
    initial_state: S,
    functions: Functions<S, R>,
    time_selector: TimeSelector<S, D>,
    scheduler: Rc<dyn IScheduler>,
}

/// The sink of `Relative` and `Absolute`.
struct TimedSink<S, R, D> {
    sink: Sink<R>,
    functions: Functions<S, R>,
    time_selector: TimeSelector<S, D>,
    first: Cell<bool>,
    has_result: Cell<bool>,
    result: RefCell<Option<R>>,
    timer_disposable: MultipleAssignmentDisposableValue,
}

impl<S: 'static, R: 'static, D: DueTime> TimedSink<S, R, D> {
    fn run(self: &Rc<Self>, outer_scheduler: &Rc<dyn IScheduler>, initial_state: S) {
        let timer = SingleAssignmentDisposable::new();
        self.timer_disposable.set_disposable(Some(timer.clone()));
        let this = self.clone();
        timer.set_disposable(Some(outer_scheduler.schedule(Box::new(move |scheduler| this.invoke_rec(scheduler, initial_state)))));
    }

    /// Forwards the error and disposes the sink (`ForwardOnError`, with the `Dispose` of the sink).
    fn fail(&self, error: ObservableError) -> Rc<dyn IDisposable> {
        let observer = self.sink.observer();
        observer.on_error(error);
        self.dispose();
        Disposable::empty()
    }

    fn invoke_rec(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>, state: S) -> Rc<dyn IDisposable> {
        if self.has_result.get() {
            let result = self.result.borrow_mut().take();
            if let Some(result) = result {
                self.sink.forward_on_next(result);
            }
        }

        let state = if self.first.replace(false) {
            state
        } else {
            match (self.functions.iterate)(state) {
                Ok(state) => state,
                Err(error) => return self.fail(error),
            }
        };

        let has_result = match (self.functions.condition)(&state) {
            Ok(has_result) => has_result,
            Err(error) => return self.fail(error),
        };
        self.has_result.set(has_result);

        let mut time = None;
        if has_result {
            match (self.functions.result_selector)(&state) {
                Ok(result) => *self.result.borrow_mut() = Some(result),
                Err(error) => return self.fail(error),
            }
            match (self.time_selector)(&state) {
                Ok(due_time) => time = Some(due_time),
                Err(error) => return self.fail(error),
            }
        }

        let Some(time) = time else {
            let observer = self.sink.observer();
            observer.on_completed();
            self.dispose();
            return Disposable::empty();
        };

        let timer = SingleAssignmentDisposable::new();
        self.timer_disposable.set_disposable(Some(timer.clone()));
        let this = self.clone();
        timer.set_disposable(Some(D::schedule(scheduler, time, Box::new(move |scheduler| this.invoke_rec(scheduler, state)))));
        Disposable::empty()
    }
}

impl<S: 'static, R: 'static, D: 'static> IDisposable for TimedSink<S, R, D> {
    fn dispose(&self) {
        self.timer_disposable.dispose();
        self.sink.dispose();
    }
}

impl<S: Clone + 'static, R: 'static, D: DueTime> IObservable<R> for GenerateTimed<S, R, D> {
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        let state = self.initial_state.clone();
        let functions = self.functions.clone();
        let time_selector = self.time_selector.clone();
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new(TimedSink {
                    sink: Sink::new(observer),
                    functions,
                    time_selector,
                    first: Cell::new(true),
                    has_result: Cell::new(false),
                    result: RefCell::new(None),
                    timer_disposable: MultipleAssignmentDisposableValue::new(),
                })
            },
            move |sink: &Rc<TimedSink<S, R, D>>| sink.run(&scheduler, state),
        )
    }
}

fn functions<S: 'static, R: 'static>(
    condition: impl Fn(&S) -> Result<bool, ObservableError> + 'static,
    iterate: impl Fn(S) -> Result<S, ObservableError> + 'static,
    result_selector: impl Fn(&S) -> Result<R, ObservableError> + 'static,
) -> Functions<S, R> {
    Functions { condition: Rc::new(condition), iterate: Rc::new(iterate), result_selector: Rc::new(result_selector) }
}

impl crate::linq::Observable {
    /// The values of a loop of states: from `initial_state`, while `condition` holds for the state,
    /// the value `result_selector` gives for it, then the next state `iterate` gives; on the
    /// scheduler of iteration, the current-thread one
    /// (`Generate<TState, TResult>(TState, Func<TState, bool>, Func<TState, TState>, Func<TState, TResult>)`).
    pub fn generate<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> bool + 'static,
        iterate: impl Fn(S) -> S + 'static,
        result_selector: impl Fn(&S) -> R + 'static,
    ) -> Rc<dyn IObservable<R>> {
        Self::generate_on(initial_state, condition, iterate, result_selector, SchedulerDefaults::iteration())
    }

    /// [`generate`](Self::generate), each value notified by work of its own on `scheduler`
    /// (`Generate<TState, TResult>(TState, Func<TState, bool>, Func<TState, TState>, Func<TState, TResult>, IScheduler)`).
    pub fn generate_on<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> bool + 'static,
        iterate: impl Fn(S) -> S + 'static,
        result_selector: impl Fn(&S) -> R + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<R>> {
        Self::try_generate_on(
            initial_state,
            move |state| Ok(condition(state)),
            move |state| Ok(iterate(state)),
            move |state| Ok(result_selector(state)),
            scheduler,
        )
    }

    /// [`generate`](Self::generate) with functions that may fail; a failure ends the sequence with
    /// its error (`Generate` with functions that throw).
    pub fn try_generate<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> Result<bool, ObservableError> + 'static,
        iterate: impl Fn(S) -> Result<S, ObservableError> + 'static,
        result_selector: impl Fn(&S) -> Result<R, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<R>> {
        Self::try_generate_on(initial_state, condition, iterate, result_selector, SchedulerDefaults::iteration())
    }

    /// [`generate_on`](Self::generate_on) with functions that may fail; a failure ends the sequence
    /// with its error (`Generate` with functions that throw).
    pub fn try_generate_on<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> Result<bool, ObservableError> + 'static,
        iterate: impl Fn(S) -> Result<S, ObservableError> + 'static,
        result_selector: impl Fn(&S) -> Result<R, ObservableError> + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<R>> {
        Rc::new(GenerateNoTime { initial_state, functions: functions(condition, iterate, result_selector), scheduler })
    }

    /// [`generate`](Self::generate), each value notified after the time `time_selector` gives for
    /// its state, on the scheduler of time-based operations
    /// (`Generate<TState, TResult>(TState, Func<TState, bool>, Func<TState, TState>, Func<TState, TResult>, Func<TState, TimeSpan>)`).
    pub fn generate_after<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> bool + 'static,
        iterate: impl Fn(S) -> S + 'static,
        result_selector: impl Fn(&S) -> R + 'static,
        time_selector: impl Fn(&S) -> Duration + 'static,
    ) -> Rc<dyn IObservable<R>> {
        Self::generate_after_on(initial_state, condition, iterate, result_selector, time_selector, SchedulerDefaults::time_based_operations())
    }

    /// [`generate_after`](Self::generate_after) on `scheduler`
    /// (`Generate<TState, TResult>(TState, Func<TState, bool>, Func<TState, TState>, Func<TState, TResult>, Func<TState, TimeSpan>, IScheduler)`).
    pub fn generate_after_on<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> bool + 'static,
        iterate: impl Fn(S) -> S + 'static,
        result_selector: impl Fn(&S) -> R + 'static,
        time_selector: impl Fn(&S) -> Duration + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<R>> {
        Self::try_generate_after_on(
            initial_state,
            move |state| Ok(condition(state)),
            move |state| Ok(iterate(state)),
            move |state| Ok(result_selector(state)),
            move |state| Ok(time_selector(state)),
            scheduler,
        )
    }

    /// [`generate_after`](Self::generate_after) with functions that may fail; a failure ends the
    /// sequence with its error (`Generate` with a `TimeSpan` selector and functions that throw).
    pub fn try_generate_after<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> Result<bool, ObservableError> + 'static,
        iterate: impl Fn(S) -> Result<S, ObservableError> + 'static,
        result_selector: impl Fn(&S) -> Result<R, ObservableError> + 'static,
        time_selector: impl Fn(&S) -> Result<Duration, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<R>> {
        Self::try_generate_after_on(initial_state, condition, iterate, result_selector, time_selector, SchedulerDefaults::time_based_operations())
    }

    /// [`generate_after_on`](Self::generate_after_on) with functions that may fail; a failure ends
    /// the sequence with its error (`Generate` with a `TimeSpan` selector and functions that throw).
    pub fn try_generate_after_on<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> Result<bool, ObservableError> + 'static,
        iterate: impl Fn(S) -> Result<S, ObservableError> + 'static,
        result_selector: impl Fn(&S) -> Result<R, ObservableError> + 'static,
        time_selector: impl Fn(&S) -> Result<Duration, ObservableError> + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<R>> {
        let time_selector: TimeSelector<S, Duration> = Rc::new(time_selector);
        Rc::new(GenerateTimed { initial_state, functions: functions(condition, iterate, result_selector), time_selector, scheduler })
    }

    /// [`generate`](Self::generate), each value notified at the time `time_selector` gives for its
    /// state, on the scheduler of time-based operations
    /// (`Generate<TState, TResult>(TState, Func<TState, bool>, Func<TState, TState>, Func<TState, TResult>, Func<TState, DateTimeOffset>)`).
    pub fn generate_at<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> bool + 'static,
        iterate: impl Fn(S) -> S + 'static,
        result_selector: impl Fn(&S) -> R + 'static,
        time_selector: impl Fn(&S) -> SystemTime + 'static,
    ) -> Rc<dyn IObservable<R>> {
        Self::generate_at_on(initial_state, condition, iterate, result_selector, time_selector, SchedulerDefaults::time_based_operations())
    }

    /// [`generate_at`](Self::generate_at) on `scheduler`
    /// (`Generate<TState, TResult>(TState, Func<TState, bool>, Func<TState, TState>, Func<TState, TResult>, Func<TState, DateTimeOffset>, IScheduler)`).
    pub fn generate_at_on<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> bool + 'static,
        iterate: impl Fn(S) -> S + 'static,
        result_selector: impl Fn(&S) -> R + 'static,
        time_selector: impl Fn(&S) -> SystemTime + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<R>> {
        Self::try_generate_at_on(
            initial_state,
            move |state| Ok(condition(state)),
            move |state| Ok(iterate(state)),
            move |state| Ok(result_selector(state)),
            move |state| Ok(time_selector(state)),
            scheduler,
        )
    }

    /// [`generate_at`](Self::generate_at) with functions that may fail; a failure ends the sequence
    /// with its error (`Generate` with a `DateTimeOffset` selector and functions that throw).
    pub fn try_generate_at<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> Result<bool, ObservableError> + 'static,
        iterate: impl Fn(S) -> Result<S, ObservableError> + 'static,
        result_selector: impl Fn(&S) -> Result<R, ObservableError> + 'static,
        time_selector: impl Fn(&S) -> Result<SystemTime, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<R>> {
        Self::try_generate_at_on(initial_state, condition, iterate, result_selector, time_selector, SchedulerDefaults::time_based_operations())
    }

    /// [`generate_at_on`](Self::generate_at_on) with functions that may fail; a failure ends the
    /// sequence with its error (`Generate` with a `DateTimeOffset` selector and functions that
    /// throw).
    pub fn try_generate_at_on<S: Clone + 'static, R: 'static>(
        initial_state: S,
        condition: impl Fn(&S) -> Result<bool, ObservableError> + 'static,
        iterate: impl Fn(S) -> Result<S, ObservableError> + 'static,
        result_selector: impl Fn(&S) -> Result<R, ObservableError> + 'static,
        time_selector: impl Fn(&S) -> Result<SystemTime, ObservableError> + 'static,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<R>> {
        let time_selector: TimeSelector<S, SystemTime> = Rc::new(time_selector);
        Rc::new(GenerateTimed { initial_state, functions: functions(condition, iterate, result_selector), time_selector, scheduler })
    }
}
