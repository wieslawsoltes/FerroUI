//! `Buffer` (`Linq/Observable/Buffer.cs`): buffers by count, by count and skip, by time, by time and
//! shift, by time or count, by a closing selector and by boundaries; and the buffers by openings and
//! closings of `Linq/QueryLanguage.Multiple.cs`, the lists of the windows by openings. A buffer is a
//! `Vec<T>` (upstream's `IList<TSource>`).

use crate::concurrency::{AsyncLock, IScheduler, SchedulerExt};
use crate::disposables::{SerialDisposableValue, SingleAssignmentDisposable, SingleAssignmentDisposableValue};
use crate::internal::{impl_sink_dispose, subscribe_sink, unary, OperatorSink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::time::Duration;

fn out_of_range(parameter: &str) -> ! {
    panic!("Specified argument was out of the range of valid values. (Parameter '{parameter}')")
}

/// The sink of `Buffer` with buffers of `count` values one after another (`Buffer<TSource>.CountExact.ExactSink`).
struct CountExactSink<T> {
    sink: Sink<Vec<T>>,
    count: usize,
    index: Cell<usize>,
    buffer: RefCell<Option<Vec<T>>>,
}

impl<T: 'static> OperatorSink<T, Vec<T>> for CountExactSink<T> {
    fn sink(&self) -> &Sink<Vec<T>> {
        &self.sink
    }
}

impl_sink_dispose!(CountExactSink<T>);

impl<T: 'static> IObserver<T> for CountExactSink<T> {
    fn on_next(&self, value: T) {
        let full = {
            let mut buffer = self.buffer.borrow_mut();
            buffer.get_or_insert_with(Vec::new).push(value);
            let index = self.index.get() + 1;
            if index == self.count {
                self.index.set(0);
                buffer.take()
            } else {
                self.index.set(index);
                None
            }
        };
        if let Some(full) = full {
            self.sink.forward_on_next(full);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.buffer.borrow_mut().take();
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let buffer = self.buffer.borrow_mut().take();
        if let Some(buffer) = buffer {
            self.sink.forward_on_next(buffer);
        }
        self.sink.forward_on_completed();
    }
}

/// The sink of `Buffer` with gaps between buffers, `skip` greater than `count`
/// (`Buffer<TSource>.CountSkip.SkipSink`).
struct CountSkipSink<T> {
    sink: Sink<Vec<T>>,
    count: usize,
    skip: usize,
    index: Cell<usize>,
    buffer: RefCell<Option<Vec<T>>>,
}

impl<T: 'static> OperatorSink<T, Vec<T>> for CountSkipSink<T> {
    fn sink(&self) -> &Sink<Vec<T>> {
        &self.sink
    }
}

impl_sink_dispose!(CountSkipSink<T>);

impl<T: 'static> IObserver<T> for CountSkipSink<T> {
    fn on_next(&self, value: T) {
        let mut index = self.index.get();
        let full = {
            let mut buffer = self.buffer.borrow_mut();
            if index == 0 {
                *buffer = Some(Vec::new());
            }
            if let Some(buffer) = buffer.as_mut() {
                buffer.push(value);
            }
            index += 1;
            if index == self.count {
                buffer.take()
            } else {
                None
            }
        };
        if let Some(full) = full {
            self.sink.forward_on_next(full);
        }
        if index == self.skip {
            self.index.set(0);
        } else {
            self.index.set(index);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.buffer.borrow_mut().take();
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let buffer = self.buffer.borrow_mut().take();
        if let Some(buffer) = buffer {
            self.sink.forward_on_next(buffer);
        }
        self.sink.forward_on_completed();
    }
}

/// The sink of `Buffer` with overlapping buffers, `skip` less than `count`
/// (`Buffer<TSource>.CountOverlap.OverlapSink`).
struct CountOverlapSink<T> {
    sink: Sink<Vec<T>>,
    queue: RefCell<VecDeque<Vec<T>>>,
    count: usize,
    skip: usize,
    n: Cell<usize>,
}

impl<T: Clone + 'static> OperatorSink<T, Vec<T>> for CountOverlapSink<T> {
    fn sink(&self) -> &Sink<Vec<T>> {
        &self.sink
    }
}

impl_sink_dispose!(CountOverlapSink<T> where T: Clone);

impl<T: Clone + 'static> IObserver<T> for CountOverlapSink<T> {
    fn on_next(&self, value: T) {
        for buffer in self.queue.borrow_mut().iter_mut() {
            buffer.push(value.clone());
        }
        let n = self.n.get();
        // `c = _n - _count + 1`: non-negative once `count` values have arrived.
        if n + 1 >= self.count && (n + 1 - self.count) % self.skip == 0 {
            let buffer = self.queue.borrow_mut().pop_front();
            if let Some(buffer) = buffer {
                if !buffer.is_empty() {
                    self.sink.forward_on_next(buffer);
                }
            }
        }
        let n = n + 1;
        self.n.set(n);
        if n % self.skip == 0 {
            self.queue.borrow_mut().push_back(Vec::new());
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.queue.borrow_mut().clear();
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        loop {
            let buffer = self.queue.borrow_mut().pop_front();
            let Some(buffer) = buffer else {
                break;
            };
            if !buffer.is_empty() {
                self.sink.forward_on_next(buffer);
            }
        }
        self.sink.forward_on_completed();
    }
}

/// `Buffer` by count (`Buffer(IObservable<TSource>, int)`): `CountExact`.
pub(crate) fn buffer_count<T: 'static>(source: Rc<dyn IObservable<T>>, count: usize) -> Rc<dyn IObservable<Vec<T>>> {
    if count == 0 {
        out_of_range("count");
    }
    unary(source, move |observer| Rc::new(CountExactSink { sink: Sink::new(observer), count, index: Cell::new(0), buffer: RefCell::new(None) }))
}

/// `Buffer` by count and skip (`Buffer(IObservable<TSource>, int, int)`): `CountOverlap` when `count`
/// is greater than `skip`, `CountSkip` when it is less, `CountExact` when they are equal.
pub(crate) fn buffer_count_skip<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, count: usize, skip: usize) -> Rc<dyn IObservable<Vec<T>>> {
    if count == 0 {
        out_of_range("count");
    }
    if skip == 0 {
        out_of_range("skip");
    }
    if count > skip {
        unary(source, move |observer| {
            Rc::new(CountOverlapSink { sink: Sink::new(observer), queue: RefCell::new(VecDeque::from([Vec::new()])), count, skip, n: Cell::new(0) })
        })
    } else if count < skip {
        unary(source, move |observer| {
            Rc::new(CountSkipSink { sink: Sink::new(observer), count, skip, index: Cell::new(0), buffer: RefCell::new(None) })
        })
    } else {
        buffer_count(source, count)
    }
}

/// The sink of `Buffer` by time and shift (`Buffer<TSource>.TimeSliding._`): a buffer starts every
/// `time_shift` and lasts `time_span`; one timer at a time runs to the next start or end.
struct TimeSlidingSink<T> {
    sink: Sink<Vec<T>>,
    time_span: Duration,
    time_shift: Duration,
    scheduler: Rc<dyn IScheduler>,
    queue: RefCell<VecDeque<Vec<T>>>,
    timer_serial: SerialDisposableValue,
    total_time: Cell<Duration>,
    next_shift: Cell<Duration>,
    next_span: Cell<Duration>,
}

impl<T: Clone + 'static> TimeSlidingSink<T> {
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        self.total_time.set(Duration::ZERO);
        self.next_shift.set(self.time_shift);
        self.next_span.set(self.time_span);
        self.create_window();
        self.create_timer();
        let upstream = source.subscribe(self.clone());
        self.sink.set_upstream(upstream);
    }

    fn create_window(&self) {
        self.queue.borrow_mut().push_back(Vec::new());
    }

    fn create_timer(self: &Rc<Self>) {
        let timer = SingleAssignmentDisposable::new();
        self.timer_serial.set_disposable(Some(timer.clone()));
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
        if is_span {
            let buffer = self.queue.borrow_mut().pop_front();
            if let Some(buffer) = buffer {
                self.sink.forward_on_next(buffer);
            }
        }
        if is_shift {
            self.create_window();
        }
        self.create_timer();
    }
}

impl<T: Clone + 'static> IObserver<T> for TimeSlidingSink<T> {
    fn on_next(&self, value: T) {
        for buffer in self.queue.borrow_mut().iter_mut() {
            buffer.push(value.clone());
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.queue.borrow_mut().clear();
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        loop {
            let buffer = self.queue.borrow_mut().pop_front();
            let Some(buffer) = buffer else {
                break;
            };
            self.sink.forward_on_next(buffer);
        }
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for TimeSlidingSink<T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.timer_serial.dispose();
        }
    }
}

/// The sink of `Buffer` by time (`Buffer<TSource>.TimeHopping._`): a buffer every `time_span`, ended by
/// periodic work.
struct TimeHoppingSink<T> {
    sink: Sink<Vec<T>>,
    list: RefCell<Vec<T>>,
    periodic: SingleAssignmentDisposableValue,
}

impl<T: 'static> TimeHoppingSink<T> {
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>, time_span: Duration, scheduler: &Rc<dyn IScheduler>) {
        let this = self.clone();
        self.periodic.set_disposable(Some(scheduler.schedule_periodic_action(time_span, move || this.tick())));
        let upstream = source.subscribe(self.clone());
        self.sink.set_upstream(upstream);
    }

    fn tick(&self) {
        let list = std::mem::take(&mut *self.list.borrow_mut());
        self.sink.forward_on_next(list);
    }
}

impl<T: 'static> IObserver<T> for TimeHoppingSink<T> {
    fn on_next(&self, value: T) {
        self.list.borrow_mut().push(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.list.borrow_mut().clear();
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let list = std::mem::take(&mut *self.list.borrow_mut());
        self.sink.forward_on_next(list);
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for TimeHoppingSink<T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.periodic.dispose();
        }
    }
}

/// The sink of `Buffer` by time or count (`Buffer<TSource>.Ferry._`): a buffer ends when it holds
/// `count` values or `time_span` after it started, whichever comes first.
struct FerrySink<T> {
    this: Weak<FerrySink<T>>,
    sink: Sink<Vec<T>>,
    count: usize,
    time_span: Duration,
    scheduler: Rc<dyn IScheduler>,
    buffer: RefCell<Vec<T>>,
    timer_serial: SerialDisposableValue,
    n: Cell<usize>,
    window_id: Cell<u64>,
}

impl<T: 'static> FerrySink<T> {
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        self.n.set(0);
        self.window_id.set(0);
        self.create_timer(0);
        let upstream = source.subscribe(self.clone());
        self.sink.set_upstream(upstream);
    }

    fn create_timer(&self, id: u64) {
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let timer = SingleAssignmentDisposable::new();
        self.timer_serial.set_disposable(Some(timer.clone()));
        timer.set_disposable(Some(self.scheduler.schedule_action_after(self.time_span, move || this.tick(id))));
    }

    fn tick(&self, id: u64) {
        if id != self.window_id.get() {
            return;
        }
        self.n.set(0);
        let new_id = self.window_id.get() + 1;
        self.window_id.set(new_id);
        let buffer = std::mem::take(&mut *self.buffer.borrow_mut());
        self.sink.forward_on_next(buffer);
        self.create_timer(new_id);
    }
}

impl<T: 'static> IObserver<T> for FerrySink<T> {
    fn on_next(&self, value: T) {
        let full = {
            let mut buffer = self.buffer.borrow_mut();
            buffer.push(value);
            let n = self.n.get() + 1;
            self.n.set(n);
            if n == self.count {
                self.n.set(0);
                Some(std::mem::take(&mut *buffer))
            } else {
                None
            }
        };
        if let Some(full) = full {
            let new_id = self.window_id.get() + 1;
            self.window_id.set(new_id);
            self.sink.forward_on_next(full);
            self.create_timer(new_id);
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.buffer.borrow_mut().clear();
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let buffer = std::mem::take(&mut *self.buffer.borrow_mut());
        self.sink.forward_on_next(buffer);
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for FerrySink<T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.timer_serial.dispose();
        }
    }
}

/// `Buffer` by time and shift (`Buffer<TSource>.TimeSliding`).
pub(crate) fn buffer_time_shift<T: Clone + 'static>(
    source: Rc<dyn IObservable<T>>,
    time_span: Duration,
    time_shift: Duration,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<Vec<T>>> {
    Rc::new(BufferObservable {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<Vec<T>>>| {
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
                        timer_serial: SerialDisposableValue::new(),
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

/// `Buffer` by time (`Buffer<TSource>.TimeHopping`).
pub(crate) fn buffer_time<T: 'static>(source: Rc<dyn IObservable<T>>, time_span: Duration, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<Vec<T>>> {
    Rc::new(BufferObservable {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<Vec<T>>>| {
            let (source, scheduler) = (source.clone(), scheduler.clone());
            subscribe_sink(
                observer,
                |observer| Rc::new(TimeHoppingSink { sink: Sink::new(observer), list: RefCell::new(Vec::new()), periodic: SingleAssignmentDisposableValue::new() }),
                move |sink: &Rc<TimeHoppingSink<T>>| sink.run(&source, time_span, &scheduler),
            )
        }),
    })
}

/// `Buffer` by time or count (`Buffer<TSource>.Ferry`). Panics when `count` is zero.
pub(crate) fn buffer_time_count<T: 'static>(
    source: Rc<dyn IObservable<T>>,
    time_span: Duration,
    count: usize,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<Vec<T>>> {
    if count == 0 {
        out_of_range("count");
    }
    Rc::new(BufferObservable {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<Vec<T>>>| {
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
                        buffer: RefCell::new(Vec::new()),
                        timer_serial: SerialDisposableValue::new(),
                        n: Cell::new(0),
                        window_id: Cell::new(0),
                    })
                },
                move |sink: &Rc<FerrySink<T>>| sink.run(&source),
            )
        }),
    })
}

type ClosingSelector<C> = Rc<dyn Fn() -> Result<Rc<dyn IObservable<C>>, ObservableError>>;

/// The sink of `Buffer` by a closing selector (`Buffer<TSource, TBufferClosing>.Selector._`): a buffer
/// ends when the sequence the selector returned for it notifies, and the next one starts.
struct ClosingSelectorSink<T, C> {
    sink: Sink<Vec<T>>,
    buffer: RefCell<Vec<T>>,
    buffer_gate: AsyncLock,
    closing_selector: ClosingSelector<C>,
    closing_serial: SerialDisposableValue,
}

impl<T: 'static, C: 'static> ClosingSelectorSink<T, C> {
    fn run(self: &Rc<Self>, source: &Rc<dyn IObservable<T>>) {
        let upstream = source.subscribe(self.clone());
        self.sink.set_upstream(upstream);
        let this = self.clone();
        self.buffer_gate.wait(move || this.create_buffer_close());
    }

    fn create_buffer_close(self: &Rc<Self>) {
        let buffer_close = match (self.closing_selector)() {
            Ok(buffer_close) => buffer_close,
            Err(error) => {
                self.sink.forward_on_error(error);
                return;
            }
        };
        let closing_observer = Rc::new(BufferClosingObserver { parent: self.clone(), resource: SingleAssignmentDisposableValue::new() });
        self.closing_serial.set_disposable(Some(closing_observer.clone()));
        let subscription = buffer_close.subscribe(closing_observer.clone());
        closing_observer.resource.set_disposable(Some(subscription));
    }

    fn close_buffer(self: &Rc<Self>, closing_subscription: &dyn IDisposable) {
        closing_subscription.dispose();
        let buffer = std::mem::take(&mut *self.buffer.borrow_mut());
        self.sink.forward_on_next(buffer);
        let this = self.clone();
        self.buffer_gate.wait(move || this.create_buffer_close());
    }
}

impl<T: 'static, C: 'static> IObserver<T> for ClosingSelectorSink<T, C> {
    fn on_next(&self, value: T) {
        self.buffer.borrow_mut().push(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.buffer.borrow_mut().clear();
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let buffer = std::mem::take(&mut *self.buffer.borrow_mut());
        self.sink.forward_on_next(buffer);
        self.sink.forward_on_completed();
    }
}

impl<T: 'static, C: 'static> IDisposable for ClosingSelectorSink<T, C> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.closing_serial.dispose();
        }
    }
}

/// The observer of the closing sequence of a buffer (`BufferClosingObserver`, a `SafeObserver`).
struct BufferClosingObserver<T, C> {
    parent: Rc<ClosingSelectorSink<T, C>>,
    resource: SingleAssignmentDisposableValue,
}

impl<T: 'static, C: 'static> IObserver<C> for BufferClosingObserver<T, C> {
    fn on_next(&self, _value: C) {
        self.parent.close_buffer(self);
    }

    fn on_error(&self, error: ObservableError) {
        IObserver::on_error(&*self.parent, error);
    }

    fn on_completed(&self) {
        self.parent.close_buffer(self);
    }
}

impl<T, C> IDisposable for BufferClosingObserver<T, C> {
    fn dispose(&self) {
        self.resource.dispose();
    }
}

/// `Buffer` by a closing selector (`Buffer<TSource, TBufferClosing>.Selector`).
pub(crate) fn buffer_closing<T: 'static, C: 'static>(source: Rc<dyn IObservable<T>>, closing_selector: ClosingSelector<C>) -> Rc<dyn IObservable<Vec<T>>> {
    Rc::new(BufferObservable {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<Vec<T>>>| {
            let (source, closing_selector) = (source.clone(), closing_selector.clone());
            subscribe_sink(
                observer,
                |observer| {
                    Rc::new(ClosingSelectorSink {
                        sink: Sink::new(observer),
                        buffer: RefCell::new(Vec::new()),
                        buffer_gate: AsyncLock::new(),
                        closing_selector,
                        closing_serial: SerialDisposableValue::new(),
                    })
                },
                move |sink: &Rc<ClosingSelectorSink<T, C>>| sink.run(&source),
            )
        }),
    })
}

/// The sink of `Buffer` by boundaries (`Buffer<TSource, TBufferBoundary>.Boundaries._`): each value of
/// the boundaries ends a buffer and starts the next one.
struct BoundariesSink<T> {
    sink: Sink<Vec<T>>,
    buffer: RefCell<Vec<T>>,
    boundaries_disposable: SingleAssignmentDisposableValue,
}

impl<T: 'static> IObserver<T> for BoundariesSink<T> {
    fn on_next(&self, value: T) {
        self.buffer.borrow_mut().push(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.buffer.borrow_mut().clear();
        self.sink.forward_on_error(error);
    }

    fn on_completed(&self) {
        let buffer = std::mem::take(&mut *self.buffer.borrow_mut());
        self.sink.forward_on_next(buffer);
        self.sink.forward_on_completed();
    }
}

impl<T: 'static> IDisposable for BoundariesSink<T> {
    fn dispose(&self) {
        if self.sink.dispose() {
            self.boundaries_disposable.dispose();
        }
    }
}

/// The observer of the boundaries (`Boundaries._.BufferClosingObserver`).
struct BoundaryObserver<T> {
    parent: Rc<BoundariesSink<T>>,
}

impl<T: 'static, B> IObserver<B> for BoundaryObserver<T> {
    fn on_next(&self, _value: B) {
        let buffer = std::mem::take(&mut *self.parent.buffer.borrow_mut());
        self.parent.sink.forward_on_next(buffer);
    }

    fn on_error(&self, error: ObservableError) {
        self.parent.on_error(error);
    }

    fn on_completed(&self) {
        self.parent.on_completed();
    }
}

/// `Buffer` by boundaries (`Buffer<TSource, TBufferBoundary>.Boundaries`).
pub(crate) fn buffer_boundaries<T: 'static, B: 'static>(source: Rc<dyn IObservable<T>>, boundaries: Rc<dyn IObservable<B>>) -> Rc<dyn IObservable<Vec<T>>> {
    Rc::new(BufferObservable {
        subscribe: Rc::new(move |observer: Rc<dyn IObserver<Vec<T>>>| {
            let (source, boundaries) = (source.clone(), boundaries.clone());
            subscribe_sink(
                observer,
                |observer| Rc::new(BoundariesSink { sink: Sink::new(observer), buffer: RefCell::new(Vec::new()), boundaries_disposable: SingleAssignmentDisposableValue::new() }),
                move |sink: &Rc<BoundariesSink<T>>| {
                    let upstream = source.subscribe(sink.clone());
                    sink.sink.set_upstream(upstream);
                    let boundary_observer: Rc<dyn IObserver<B>> = Rc::new(BoundaryObserver { parent: sink.clone() });
                    sink.boundaries_disposable.set_disposable(Some(boundaries.subscribe(boundary_observer)));
                },
            )
        }),
    })
}

/// The observable of the forms of `Buffer` whose sinks are not observers of the source alone (the
/// `Producer`s of upstream): each subscription runs `subscribe`.
struct BufferObservable<T> {
    subscribe: Rc<dyn Fn(Rc<dyn IObserver<Vec<T>>>) -> Rc<dyn IDisposable>>,
}

impl<T: 'static> IObservable<Vec<T>> for BufferObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<Vec<T>>>) -> Rc<dyn IDisposable> {
        (self.subscribe)(observer)
    }
}

/// `Buffer` by openings and closings (`QueryLanguage.Buffer<TSource, TBufferOpening, TBufferClosing>`,
/// `Linq/QueryLanguage.Multiple.cs`): upstream has no operator of its own for it, but
/// `source.Window(bufferOpenings, bufferClosingSelector).SelectMany(ToList)`, the list of each window
/// by openings and closings.
pub(crate) fn buffer_openings<T: Clone + 'static, O: 'static, C: 'static>(
    source: Rc<dyn IObservable<T>>,
    buffer_openings: Rc<dyn IObservable<O>>,
    buffer_closing_selector: impl Fn(&O) -> Result<Rc<dyn IObservable<C>>, ObservableError> + 'static,
) -> Rc<dyn IObservable<Vec<T>>> {
    crate::linq::select_many::select_many(
        crate::linq::window::window_openings(source, buffer_openings, buffer_closing_selector),
        |window: Rc<dyn IObservable<T>>, _| Ok(crate::linq::to_list::to_list(window)),
    )
}

/// The methods of `Buffer` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! buffer_methods {
    () => {
        /// Buffers of `count` values, one after another; the last one may be shorter
        /// (`Buffer(IObservable<TSource>, int)`). Panics when `count` is zero.
        fn buffer(&self, count: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::linq::buffer::buffer_count(self.observable(), count)
        }

        /// Buffers of `count` values, one started every `skip` values
        /// (`Buffer(IObservable<TSource>, int, int)`). Panics when `count` or `skip` is zero.
        fn buffer_with_skip(&self, count: usize, skip: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            T: Clone,
        {
            crate::linq::buffer::buffer_count_skip(self.observable(), count, skip)
        }

        /// Buffers of the values of each `time_span`, on the default scheduler of time-based
        /// operations (`Buffer(IObservable<TSource>, TimeSpan)`).
        fn buffer_time(&self, time_span: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::linq::buffer::buffer_time(self.observable(), time_span, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// [`buffer_time`](Self::buffer_time) on `scheduler` (`Buffer(IObservable<TSource>, TimeSpan, IScheduler)`).
        fn buffer_time_on(
            &self,
            time_span: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::linq::buffer::buffer_time(self.observable(), time_span, scheduler)
        }

        /// Buffers that last `time_span`, one started every `time_shift`, on the default scheduler of
        /// time-based operations (`Buffer(IObservable<TSource>, TimeSpan, TimeSpan)`).
        fn buffer_time_shift(&self, time_span: std::time::Duration, time_shift: std::time::Duration) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            T: Clone,
        {
            crate::linq::buffer::buffer_time_shift(self.observable(), time_span, time_shift, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// [`buffer_time_shift`](Self::buffer_time_shift) on `scheduler`
        /// (`Buffer(IObservable<TSource>, TimeSpan, TimeSpan, IScheduler)`).
        fn buffer_time_shift_on(
            &self,
            time_span: std::time::Duration,
            time_shift: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            T: Clone,
        {
            crate::linq::buffer::buffer_time_shift(self.observable(), time_span, time_shift, scheduler)
        }

        /// Buffers that end when they hold `count` values or `time_span` after they started, on the
        /// default scheduler of time-based operations (`Buffer(IObservable<TSource>, TimeSpan, int)`).
        /// Panics when `count` is zero.
        fn buffer_time_count(&self, time_span: std::time::Duration, count: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::linq::buffer::buffer_time_count(self.observable(), time_span, count, crate::concurrency::SchedulerDefaults::time_based_operations())
        }

        /// [`buffer_time_count`](Self::buffer_time_count) on `scheduler`
        /// (`Buffer(IObservable<TSource>, TimeSpan, int, IScheduler)`).
        fn buffer_time_count_on(
            &self,
            time_span: std::time::Duration,
            count: usize,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::linq::buffer::buffer_time_count(self.observable(), time_span, count, scheduler)
        }

        /// Buffers that end when the sequence `closing_selector` returns for each notifies
        /// (`Buffer<TSource, TBufferClosing>(IObservable<TSource>, Func<IObservable<TBufferClosing>>)`).
        fn buffer_closing<C: 'static>(
            &self,
            closing_selector: impl Fn() -> std::rc::Rc<dyn ferroui_system::IObservable<C>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::linq::buffer::buffer_closing(self.observable(), std::rc::Rc::new(move || Ok(closing_selector())))
        }

        /// [`buffer_closing`](Self::buffer_closing) with a selector that may fail; a failure ends the
        /// sequence with its error (`Buffer` with a closing selector that throws).
        fn try_buffer_closing<C: 'static>(
            &self,
            closing_selector: impl Fn() -> Result<std::rc::Rc<dyn ferroui_system::IObservable<C>>, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::linq::buffer::buffer_closing(self.observable(), std::rc::Rc::new(closing_selector))
        }

        /// Buffers that each value of `boundaries` ends
        /// (`Buffer<TSource, TBufferBoundary>(IObservable<TSource>, IObservable<TBufferBoundary>)`).
        fn buffer_boundaries<B: 'static>(&self, boundaries: std::rc::Rc<dyn ferroui_system::IObservable<B>>) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>> {
            crate::linq::buffer::buffer_boundaries(self.observable(), boundaries)
        }

        /// Buffers that each value of `buffer_openings` starts, each ending when the sequence
        /// `buffer_closing_selector` returns for its opening produces or ends
        /// (`Buffer<TSource, TBufferOpening, TBufferClosing>(IObservable<TSource>,
        /// IObservable<TBufferOpening>, Func<TBufferOpening, IObservable<TBufferClosing>>)`).
        fn buffer_openings<O: 'static, C: 'static>(
            &self,
            buffer_openings: std::rc::Rc<dyn ferroui_system::IObservable<O>>,
            buffer_closing_selector: impl Fn(&O) -> std::rc::Rc<dyn ferroui_system::IObservable<C>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            T: Clone,
        {
            crate::linq::buffer::buffer_openings(self.observable(), buffer_openings, move |opening: &O| Ok(buffer_closing_selector(opening)))
        }

        /// [`buffer_openings`](Self::buffer_openings) with a closing selector that may fail; a failure
        /// ends the sequence with its error (`Buffer` by openings with a closing selector that
        /// throws).
        fn try_buffer_openings<O: 'static, C: 'static>(
            &self,
            buffer_openings: std::rc::Rc<dyn ferroui_system::IObservable<O>>,
            buffer_closing_selector: impl Fn(&O) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<C>>, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<Vec<T>>>
        where
            T: Clone,
        {
            crate::linq::buffer::buffer_openings(self.observable(), buffer_openings, buffer_closing_selector)
        }
    };
}
pub(crate) use buffer_methods;
