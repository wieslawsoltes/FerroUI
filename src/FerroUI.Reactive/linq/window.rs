//! `Window` (`Linq/Observable/Window.cs`, with the window observable of `Linq/Observable/AddRef.cs`):
//! windows by count, by count and skip, by time, by time and shift, by time or count, by a closing
//! selector and by boundaries. A window is a sequence (`Rc<dyn IObservable<T>>`) made of a subject;
//! the subscription to the source lasts while the outer subscription or a subscription to a window
//! does (a `RefCountDisposable`).

use crate::concurrency::{AsyncLock, IScheduler, SchedulerExt};
use crate::disposables::{CompositeDisposable, RefCountDisposable, SerialDisposable, SingleAssignmentDisposable, SingleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use crate::subjects::Subject;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// A window as the handle of a sequence.
pub(crate) type WindowHandle<T> = Rc<dyn IObservable<T>>;

fn out_of_range(parameter: &str) -> ! {
    panic!("Specified argument was out of the range of valid values. (Parameter '{parameter}')")
}

/// The sink of a subscription to a window (`AddRef<TSource>._`): it holds a dependent disposable of
/// the reference count of the window's operator while it lasts.
struct AddRefSink<T> {
    sink: Sink<T>,
    ref_count_disposable: Rc<dyn IDisposable>,
}

impl<T: 'static> IObserver<T> for AddRefSink<T> {
    fn on_next(&self, value: T) {
        self.sink.forward_on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for AddRefSink<T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.ref_count_disposable.dispose();
        }
    }
}

/// A window (`WindowObservable<TSource>`, an `AddRef<TSource>`): the subject of the window, with a
/// reference to the subscription of the operator for each subscription to it.
struct WindowObservable<T> {
    source: Rc<dyn IObservable<T>>,
    ref_count: Rc<RefCountDisposable>,
}

impl<T: 'static> IObservable<T> for WindowObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let ref_count = self.ref_count.clone();
        subscribe_sink(
            observer,
            move |observer| Rc::new(AddRefSink { sink: Sink::new(observer), ref_count_disposable: ref_count.get_disposable() }),
            move |sink: &Rc<AddRefSink<T>>| {
                let upstream = source.subscribe(sink.clone());
                sink.sink.set_upstream(upstream);
            },
        )
    }
}

fn window_observable<T: Clone + 'static>(subject: &Rc<Subject<T>>, ref_count: &Rc<RefCountDisposable>) -> WindowHandle<T> {
    Rc::new(WindowObservable { source: subject.as_observable(), ref_count: ref_count.clone() })
}

/// The observable of the forms of `Window` (the `Producer`s of upstream): each subscription runs
/// `subscribe`.
struct WindowOperator<T> {
    subscribe: Rc<dyn Fn(Rc<dyn IObserver<WindowHandle<T>>>) -> Rc<dyn IDisposable>>,
}

impl<T: 'static> IObservable<WindowHandle<T>> for WindowOperator<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<WindowHandle<T>>>) -> Rc<dyn IDisposable> {
        (self.subscribe)(observer)
    }
}

/// The sink of `Window` by count and skip (`Window<TSource>.Count._`): a window of `count` values
/// starts every `skip` values.
struct CountSink<T> {
    sink: Sink<WindowHandle<T>>,
    queue: RefCell<VecDeque<Rc<Subject<T>>>>,
    m: Rc<SingleAssignmentDisposable>,
    ref_count: Rc<RefCountDisposable>,
    count: usize,
    skip: usize,
    n: Cell<usize>,
}

impl<T: Clone + 'static> CountSink<T> {
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        let first_window = self.create_window();
        self.sink.forward_on_next(first_window);
        self.m.set_disposable(Some(source.subscribe(self.clone())));
        self.sink.set_upstream(self.ref_count.clone());
    }

    fn create_window(&self) -> WindowHandle<T> {
        let subject = Subject::new();
        self.queue.borrow_mut().push_back(subject.clone());
        window_observable(&subject, &self.ref_count)
    }
}

impl<T: Clone + 'static> IObserver<T> for CountSink<T> {
    fn on_next(&self, value: T) {
        let windows: Vec<Rc<Subject<T>>> = self.queue.borrow().iter().cloned().collect();
        for window in windows {
            window.on_next(value.clone());
        }
        let n = self.n.get();
        // `c = _n - _count + 1`: non-negative once `count` values have arrived.
        if n + 1 >= self.count && (n + 1 - self.count) % self.skip == 0 {
            let window = self.queue.borrow_mut().pop_front();
            if let Some(window) = window {
                window.on_completed();
            }
        }
        let n = n + 1;
        self.n.set(n);
        if n % self.skip == 0 {
            let new_window = self.create_window();
            self.sink.forward_on_next(new_window);
        }
    }

    fn on_error(&self, error: ObservableError) {
        loop {
            let window = self.queue.borrow_mut().pop_front();
            let Some(window) = window else {
                break;
            };
            window.on_error(error.clone());
        }
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        loop {
            let window = self.queue.borrow_mut().pop_front();
            let Some(window) = window else {
                break;
            };
            window.on_completed();
        }
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for CountSink<T> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// `Window` by count and skip (`Window<TSource>.Count`). Panics when `count` or `skip` is zero.
pub(crate) fn window_count<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, count: usize, skip: usize) -> Rc<dyn IObservable<WindowHandle<T>>> {
    if count == 0 {
        out_of_range("count");
    }
    if skip == 0 {
        out_of_range("skip");
    }
    Rc::new(WindowOperator {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<WindowHandle<T>>>| {
            let source = source.clone();
            subscribe_sink(
                observer,
                |observer| {
                    let m = SingleAssignmentDisposable::new();
                    let ref_count = RefCountDisposable::new(m.clone());
                    Rc::new(CountSink { sink: Sink::new(observer), queue: RefCell::new(VecDeque::new()), m, ref_count, count, skip, n: Cell::new(0) })
                },
                move |sink: &Rc<CountSink<T>>| sink.run(&source),
            )
        }),
    })
}

/// The sink of `Window` by time and shift (`Window<TSource>.TimeSliding._`): a window starts every
/// `time_shift` and lasts `time_span`; one timer at a time runs to the next start or end.
struct TimeSlidingSink<T> {
    sink: Sink<WindowHandle<T>>,
    time_span: Duration,
    time_shift: Duration,
    scheduler: Rc<dyn IScheduler>,
    queue: RefCell<VecDeque<Rc<Subject<T>>>>,
    timer: Rc<SerialDisposable>,
    ref_count: RefCell<Option<Rc<RefCountDisposable>>>,
    total_time: Cell<Duration>,
    next_shift: Cell<Duration>,
    next_span: Cell<Duration>,
}

impl<T: Clone + 'static> TimeSlidingSink<T> {
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        self.total_time.set(Duration::ZERO);
        self.next_shift.set(self.time_shift);
        self.next_span.set(self.time_span);
        let group = CompositeDisposable::with_capacity(2);
        group.add(self.timer.clone());
        let ref_count = RefCountDisposable::new(group.clone());
        *self.ref_count.borrow_mut() = Some(ref_count.clone());
        self.create_window();
        self.create_timer();
        group.add(source.subscribe(self.clone()));
        self.sink.set_upstream(ref_count);
    }

    fn create_window(&self) {
        let subject = Subject::new();
        self.queue.borrow_mut().push_back(subject.clone());
        let ref_count = self.ref_count.borrow().clone().expect("the reference count is set as the sink runs");
        self.sink.forward_on_next(window_observable(&subject, &ref_count));
    }

    fn create_timer(self: &Rc<Self>) {
        let timer = SingleAssignmentDisposable::new();
        self.timer.set_disposable(Some(timer.clone()));
        let (next_span, next_shift) = (self.next_span.get(), self.next_shift.get());
        let (is_span, is_shift) = if next_span == next_shift {
            (true, true)
        } else if next_span < next_shift {
            (true, false)
        } else {
            (false, true)
        };
        let new_total_time = if is_span { next_span } else { next_shift };
        let due = new_total_time.saturating_sub(self.total_time.get());
        self.total_time.set(new_total_time);
        if is_span {
            self.next_span.set(next_span + self.time_shift);
        }
        if is_shift {
            self.next_shift.set(next_shift + self.time_shift);
        }
        let this = self.clone();
        timer.set_disposable(Some(self.scheduler.schedule_action_after(due, move || this.tick(is_span, is_shift))));
    }

    fn tick(self: &Rc<Self>, is_span: bool, is_shift: bool) {
        // The window ends before the next one starts (the order of upstream since v2, the same for
        // every form of `Window` and `Buffer`).
        if is_span {
            let window = self.queue.borrow_mut().pop_front();
            if let Some(window) = window {
                window.on_completed();
            }
        }
        if is_shift {
            self.create_window();
        }
        self.create_timer();
    }

    fn windows(&self) -> Vec<Rc<Subject<T>>> {
        self.queue.borrow().iter().cloned().collect()
    }
}

impl<T: Clone + 'static> IObserver<T> for TimeSlidingSink<T> {
    fn on_next(&self, value: T) {
        for window in self.windows() {
            window.on_next(value.clone());
        }
    }

    fn on_error(&self, error: ObservableError) {
        for window in self.windows() {
            window.on_error(error.clone());
        }
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        for window in self.windows() {
            window.on_completed();
        }
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for TimeSlidingSink<T> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// `Window` by time and shift (`Window<TSource>.TimeSliding`).
pub(crate) fn window_time_shift<T: Clone + 'static>(
    source: Rc<dyn IObservable<T>>,
    time_span: Duration,
    time_shift: Duration,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<WindowHandle<T>>> {
    Rc::new(WindowOperator {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<WindowHandle<T>>>| {
            let (source, scheduler) = (source.clone(), scheduler.clone());
            subscribe_sink(
                observer,
                |observer| {
                    Rc::new(TimeSlidingSink {
                        sink: Sink::new(observer),
                        time_span,
                        time_shift,
                        scheduler,
                        queue: RefCell::new(VecDeque::new()),
                        timer: SerialDisposable::new(),
                        ref_count: RefCell::new(None),
                        total_time: Cell::new(Duration::ZERO),
                        next_shift: Cell::new(Duration::ZERO),
                        next_span: Cell::new(Duration::ZERO),
                    })
                },
                move |sink: &Rc<TimeSlidingSink<T>>| sink.run(&source),
            )
        }),
    })
}

/// The sink of `Window` by time (`Window<TSource>.TimeHopping._`): a window every `time_span`, ended
/// by periodic work.
struct TimeHoppingSink<T> {
    sink: Sink<WindowHandle<T>>,
    subject: RefCell<Rc<Subject<T>>>,
    ref_count: RefCell<Option<Rc<RefCountDisposable>>>,
}

impl<T: Clone + 'static> TimeHoppingSink<T> {
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>, time_span: Duration, scheduler: &Rc<dyn IScheduler>) {
        let group = CompositeDisposable::with_capacity(2);
        let ref_count = RefCountDisposable::new(group.clone());
        *self.ref_count.borrow_mut() = Some(ref_count.clone());
        self.next_window();
        let this = self.clone();
        group.add(scheduler.schedule_periodic_action(time_span, move || this.tick()));
        group.add(source.subscribe(self.clone()));
        self.sink.set_upstream(ref_count);
    }

    fn tick(&self) {
        let subject = self.subject.borrow().clone();
        subject.on_completed();
        *self.subject.borrow_mut() = Subject::new();
        self.next_window();
    }

    fn next_window(&self) {
        let subject = self.subject.borrow().clone();
        let ref_count = self.ref_count.borrow().clone().expect("the reference count is set as the sink runs");
        self.sink.forward_on_next(window_observable(&subject, &ref_count));
    }

    fn current(&self) -> Rc<Subject<T>> {
        self.subject.borrow().clone()
    }
}

impl<T: Clone + 'static> IObserver<T> for TimeHoppingSink<T> {
    fn on_next(&self, value: T) {
        self.current().on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.current().on_error(error.clone());
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.current().on_completed();
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for TimeHoppingSink<T> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// `Window` by time (`Window<TSource>.TimeHopping`).
pub(crate) fn window_time<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, time_span: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<WindowHandle<T>>> {
    Rc::new(WindowOperator {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<WindowHandle<T>>>| {
            let (source, scheduler) = (source.clone(), scheduler.clone());
            subscribe_sink(
                observer,
                |observer| Rc::new(TimeHoppingSink { sink: Sink::new(observer), subject: RefCell::new(Subject::new()), ref_count: RefCell::new(None) }),
                move |sink: &Rc<TimeHoppingSink<T>>| sink.run(&source, time_span, &scheduler),
            )
        }),
    })
}

/// The sink of `Window` by time or count (`Window<TSource>.Ferry._`): a window ends when it has had
/// `count` values or `time_span` after it started, whichever comes first.
struct FerrySink<T> {
    this: Weak<FerrySink<T>>,
    sink: Sink<WindowHandle<T>>,
    count: usize,
    time_span: Duration,
    scheduler: Rc<dyn IScheduler>,
    subject: RefCell<Rc<Subject<T>>>,
    timer: Rc<SerialDisposable>,
    n: Cell<usize>,
    ref_count: RefCell<Option<Rc<RefCountDisposable>>>,
}

impl<T: Clone + 'static> FerrySink<T> {
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        let group = CompositeDisposable::with_capacity(2);
        group.add(self.timer.clone());
        let ref_count = RefCountDisposable::new(group.clone());
        *self.ref_count.borrow_mut() = Some(ref_count.clone());
        self.next_window();
        self.create_timer(self.current());
        group.add(source.subscribe(self.clone()));
        self.sink.set_upstream(ref_count);
    }

    fn current(&self) -> Rc<Subject<T>> {
        self.subject.borrow().clone()
    }

    fn create_timer(&self, window: Rc<Subject<T>>) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let timer = SingleAssignmentDisposable::new();
        self.timer.set_disposable(Some(timer.clone()));
        timer.set_disposable(Some(self.scheduler.schedule_action_after(self.time_span, move || this.tick(window))));
    }

    fn next_window(&self) {
        let subject = self.current();
        let ref_count = self.ref_count.borrow().clone().expect("the reference count is set as the sink runs");
        self.sink.forward_on_next(window_observable(&subject, &ref_count));
    }

    fn tick(&self, window: Rc<Subject<T>>) {
        let current = self.current();
        if !Rc::ptr_eq(&window, &current) {
            return;
        }
        self.n.set(0);
        let new_window = Subject::new();
        current.on_completed();
        *self.subject.borrow_mut() = new_window.clone();
        self.next_window();
        self.create_timer(new_window);
    }
}

impl<T: Clone + 'static> IObserver<T> for FerrySink<T> {
    fn on_next(&self, value: T) {
        let current = self.current();
        current.on_next(value);
        let n = self.n.get() + 1;
        self.n.set(n);
        if n == self.count {
            self.n.set(0);
            let new_window = Subject::new();
            current.on_completed();
            *self.subject.borrow_mut() = new_window.clone();
            self.next_window();
            self.create_timer(new_window);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.current().on_error(error.clone());
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.current().on_completed();
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for FerrySink<T> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// `Window` by time or count (`Window<TSource>.Ferry`). Panics when `count` is zero.
pub(crate) fn window_time_count<T: Clone + 'static>(
    source: Rc<dyn IObservable<T>>,
    time_span: Duration,
    count: usize,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<WindowHandle<T>>> {
    if count == 0 {
        out_of_range("count");
    }
    Rc::new(WindowOperator {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<WindowHandle<T>>>| {
            let (source, scheduler) = (source.clone(), scheduler.clone());
            subscribe_sink(
                observer,
                |observer| {
                    Rc::new_cyclic(|this| FerrySink {
                        this: this.clone(),
                        sink: Sink::new(observer),
                        count,
                        time_span,
                        scheduler,
                        subject: RefCell::new(Subject::new()),
                        timer: SerialDisposable::new(),
                        n: Cell::new(0),
                        ref_count: RefCell::new(None),
                    })
                },
                move |sink: &Rc<FerrySink<T>>| sink.run(&source),
            )
        }),
    })
}

type ClosingSelector<C> = Rc<dyn Fn() -> Result<Rc<dyn IObservable<C>>, ObservableError>>;

/// The sink of `Window` by a closing selector (`Window<TSource, TWindowClosing>.Selector._`): a window
/// ends when the sequence the selector returned for it notifies, and the next one starts.
struct ClosingSelectorSink<T, C> {
    sink: Sink<WindowHandle<T>>,
    window: RefCell<Rc<Subject<T>>>,
    window_gate: AsyncLock,
    m: Rc<SerialDisposable>,
    closing_selector: ClosingSelector<C>,
    ref_count: RefCell<Option<Rc<RefCountDisposable>>>,
}

impl<T: Clone + 'static, C: 'static> ClosingSelectorSink<T, C> {
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        let group = CompositeDisposable::with_capacity(2);
        group.add(self.m.clone());
        let ref_count = RefCountDisposable::new(group.clone());
        *self.ref_count.borrow_mut() = Some(ref_count.clone());
        self.next_window();
        group.add(source.subscribe(self.clone()));
        let this = self.clone();
        self.window_gate.wait(move || this.create_window_close());
        self.sink.set_upstream(ref_count);
    }

    fn current(&self) -> Rc<Subject<T>> {
        self.window.borrow().clone()
    }

    fn next_window(&self) {
        let window = self.current();
        let ref_count = self.ref_count.borrow().clone().expect("the reference count is set as the sink runs");
        self.sink.forward_on_next(window_observable(&window, &ref_count));
    }

    fn create_window_close(self: &Rc<Self>) {
        let window_close = match (self.closing_selector)() {
            Ok(window_close) => window_close,
            Err(error) => {
                self.sink.forward_on_error(error);
                return;
            }
        };
        let observer = Rc::new(WindowClosingObserver { parent: self.clone(), resource: SingleAssignmentDisposableValue::new() });
        self.m.set_disposable(Some(observer.clone()));
        let subscription = window_close.subscribe(observer.clone());
        observer.resource.set_disposable(Some(subscription));
    }

    fn close_window(self: &Rc<Self>, closing_subscription: &dyn IDisposable) {
        closing_subscription.dispose();
        self.current().on_completed();
        *self.window.borrow_mut() = Subject::new();
        self.next_window();
        let this = self.clone();
        self.window_gate.wait(move || this.create_window_close());
    }
}

impl<T: Clone + 'static, C: 'static> IObserver<T> for ClosingSelectorSink<T, C> {
    fn on_next(&self, value: T) {
        self.current().on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.current().on_error(error.clone());
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.current().on_completed();
        self.sink.forward_on_completed();
    }
}

impl<T: 'static, C: 'static> IDisposable for ClosingSelectorSink<T, C> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// The observer of the closing sequence of a window (`WindowClosingObserver`, a `SafeObserver`).
struct WindowClosingObserver<T, C> {
    parent: Rc<ClosingSelectorSink<T, C>>,
    resource: SingleAssignmentDisposableValue,
}

impl<T: Clone + 'static, C: 'static> IObserver<C> for WindowClosingObserver<T, C> {
    fn on_next(&self, _value: C) {
        self.parent.close_window(self);
    }

    fn on_error(&self, error: ObservableError) {
        IObserver::on_error(&*self.parent, error);
    }

    fn on_completed(&self) {
        self.parent.close_window(self);
    }
}

impl<T, C> IDisposable for WindowClosingObserver<T, C> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// `Window` by a closing selector (`Window<TSource, TWindowClosing>.Selector`).
pub(crate) fn window_closing<T: Clone + 'static, C: 'static>(source: Rc<dyn IObservable<T>>, closing_selector: ClosingSelector<C>) -> Rc<dyn IObservable<WindowHandle<T>>> {
    Rc::new(WindowOperator {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<WindowHandle<T>>>| {
            let (source, closing_selector) = (source.clone(), closing_selector.clone());
            subscribe_sink(
                observer,
                |observer| {
                    Rc::new(ClosingSelectorSink {
                        sink: Sink::new(observer),
                        window: RefCell::new(Subject::new()),
                        window_gate: AsyncLock::new(),
                        m: SerialDisposable::new(),
                        closing_selector,
                        ref_count: RefCell::new(None),
                    })
                },
                move |sink: &Rc<ClosingSelectorSink<T, C>>| sink.run(&source),
            )
        }),
    })
}

/// The sink of `Window` by boundaries (`Window<TSource, TWindowBoundary>.Boundaries._`): each value of
/// the boundaries ends a window and starts the next one.
struct BoundariesSink<T> {
    sink: Sink<WindowHandle<T>>,
    window: RefCell<Rc<Subject<T>>>,
    ref_count: RefCell<Option<Rc<RefCountDisposable>>>,
}

impl<T: Clone + 'static> BoundariesSink<T> {
    fn current(&self) -> Rc<Subject<T>> {
        self.window.borrow().clone()
    }

    fn next_window(&self) {
        let window = self.current();
        let ref_count = self.ref_count.borrow().clone().expect("the reference count is set as the sink runs");
        self.sink.forward_on_next(window_observable(&window, &ref_count));
    }
}

impl<T: Clone + 'static> IObserver<T> for BoundariesSink<T> {
    fn on_next(&self, value: T) {
        self.current().on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.current().on_error(error.clone());
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        self.current().on_completed();
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for BoundariesSink<T> {
    fn dispose(&self) {
        self.sink.dispose();
    }
}

/// The observer of the boundaries (`Boundaries._.WindowClosingObserver`).
struct BoundaryObserver<T> {
    parent: Rc<BoundariesSink<T>>,
}

impl<T: Clone + 'static, B> IObserver<B> for BoundaryObserver<T> {
    fn on_next(&self, _value: B) {
        let parent = &self.parent;
        parent.current().on_completed();
        *parent.window.borrow_mut() = Subject::new();
        parent.next_window();
    }

    fn on_error(&self, error: ObservableError) {
        IObserver::on_error(&*self.parent, error);
    }

    fn on_completed(&self) {
        IObserver::on_completed(&*self.parent);
    }
}

/// `Window` by boundaries (`Window<TSource, TWindowBoundary>.Boundaries`).
pub(crate) fn window_boundaries<T: Clone + 'static, B: 'static>(
    source: Rc<dyn IObservable<T>>,
    boundaries: Rc<dyn IObservable<B>>,
) -> Rc<dyn IObservable<WindowHandle<T>>> {
    Rc::new(WindowOperator {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<WindowHandle<T>>>| {
            let (source, boundaries) = (source.clone(), boundaries.clone());
            subscribe_sink(
                observer,
                |observer| Rc::new(BoundariesSink { sink: Sink::new(observer), window: RefCell::new(Subject::new()), ref_count: RefCell::new(None) }),
                move |sink: &Rc<BoundariesSink<T>>| {
                    let group = CompositeDisposable::with_capacity(2);
                    let ref_count = RefCountDisposable::new(group.clone());
                    *sink.ref_count.borrow_mut() = Some(ref_count.clone());
                    sink.next_window();
                    group.add(source.subscribe(sink.clone()));
                    let boundary_observer: Rc<dyn IObserver<B>> = Rc::new(BoundaryObserver { parent: sink.clone() });
                    group.add(boundaries.subscribe(boundary_observer));
                    sink.sink.set_upstream(ref_count);
                },
            )
        }),
    })
}

/// The methods of `Window` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! window_methods {
    () => {
        /// Windows of `count` values, one after another (`Window(IObservable<TSource>, int)`). Panics
        /// when `count` is zero.
        fn window(&self, count: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_count(self.observable(), count, count)
        }

        /// Windows of `count` values, one started every `skip` values
        /// (`Window(IObservable<TSource>, int, int)`). Panics when `count` or `skip` is zero.
        fn window_with_skip(&self, count: usize, skip: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_count(self.observable(), count, skip)
        }

        /// Windows of the values of each `time_span`, on the default scheduler of time-based
        /// operations (`Window(IObservable<TSource>, TimeSpan)`).
        fn window_time(&self, time_span: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_time(self.observable(), time_span, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// [`window_time`](Self::window_time) on `scheduler` (`Window(IObservable<TSource>, TimeSpan, IScheduler)`).
        fn window_time_on(
            &self,
            time_span: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_time(self.observable(), time_span, scheduler)
        }

        /// Windows that last `time_span`, one started every `time_shift`, on the default scheduler of
        /// time-based operations (`Window(IObservable<TSource>, TimeSpan, TimeSpan)`).
        fn window_time_shift(
            &self,
            time_span: std::time::Duration,
            time_shift: std::time::Duration,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_time_shift(self.observable(), time_span, time_shift, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// [`window_time_shift`](Self::window_time_shift) on `scheduler`
        /// (`Window(IObservable<TSource>, TimeSpan, TimeSpan, IScheduler)`).
        fn window_time_shift_on(
            &self,
            time_span: std::time::Duration,
            time_shift: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_time_shift(self.observable(), time_span, time_shift, scheduler)
        }

        /// Windows that end when they have had `count` values or `time_span` after they started, on
        /// the default scheduler of time-based operations (`Window(IObservable<TSource>, TimeSpan, int)`).
        /// Panics when `count` is zero.
        fn window_time_count(
            &self,
            time_span: std::time::Duration,
            count: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_time_count(self.observable(), time_span, count, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// [`window_time_count`](Self::window_time_count) on `scheduler`
        /// (`Window(IObservable<TSource>, TimeSpan, int, IScheduler)`).
        fn window_time_count_on(
            &self,
            time_span: std::time::Duration,
            count: usize,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_time_count(self.observable(), time_span, count, scheduler)
        }

        /// Windows that end when the sequence `closing_selector` returns for each notifies
        /// (`Window<TSource, TWindowClosing>(IObservable<TSource>, Func<IObservable<TWindowClosing>>)`).
        fn window_closing<C: 'static>(
            &self,
            closing_selector: impl Fn() -> std::rc::Rc<dyn ferroui_system::IObservable<C>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_closing(self.observable(), std::rc::Rc::new(move || Ok(closing_selector())))
        }

        /// [`window_closing`](Self::window_closing) with a selector that may fail; a failure ends the
        /// sequence with its error (`Window` with a closing selector that throws).
        fn try_window_closing<C: 'static>(
            &self,
            closing_selector: impl Fn() -> Result<std::rc::Rc<dyn ferroui_system::IObservable<C>>, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_closing(self.observable(), std::rc::Rc::new(closing_selector))
        }

        /// Windows that each value of `boundaries` ends
        /// (`Window<TSource, TWindowBoundary>(IObservable<TSource>, IObservable<TWindowBoundary>)`).
        fn window_boundaries<B: 'static>(
            &self,
            boundaries: std::rc::Rc<dyn ferroui_system::IObservable<B>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<std::rc::Rc<dyn ferroui_system::IObservable<T>>>>
        where
            T: Clone,
        {
            crate::linq::window::window_boundaries(self.observable(), boundaries)
        }
    };
}
pub(crate) use window_methods;
