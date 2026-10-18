//! ferroui-reactive-testing
//!
//! A port of Microsoft.Reactive.Testing (<https://github.com/dotnet/reactive>) for
//! `ferroui-reactive`: the virtual-time [`TestScheduler`], recorded notifications ([`Recorded`]) and
//! subscriptions ([`Subscription`]), hot and cold test observables, the observer that records what
//! it receives, the factories of [`ReactiveTest`] and the assertions of `ReactiveAssert`.
//!
//! A test in the style of upstream:
//!
//! ```
//! use ferroui_reactive::ObservableExt;
//! use ferroui_reactive_testing::{assert_messages, assert_subscriptions, ReactiveTest as RT, TestScheduler};
//!
//! let scheduler = TestScheduler::new();
//! let xs = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 2), RT::on_completed(250)]);
//! let source = xs.as_observable();
//! let res = scheduler.start(move || source.clone());
//! assert_messages!(res.messages(), RT::on_next(210, 2), RT::on_completed::<i32>(250));
//! // A hot observable subscribed directly keeps the subscription until it is disposed.
//! assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
//! ```

use ferroui_reactive::concurrency::{IScheduler, VirtualTimeClock, VirtualTimeScheduler};
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{IDisposable, IObservable, IObserver, Notification, ObservableError};
use std::cell::RefCell;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// The virtual time of the test scheduler: ticks of 100 nanoseconds, as `TimeSpan` counts them.
pub struct TestTime;

impl VirtualTimeClock for TestTime {
    type Absolute = i64;
    type Relative = i64;

    fn add(absolute: i64, relative: i64) -> i64 {
        absolute + relative
    }

    /// `new DateTimeOffset(absolute, TimeSpan.Zero)`, measured from the epoch of the system time.
    fn to_system_time(absolute: i64) -> SystemTime {
        SystemTime::UNIX_EPOCH + ticks_to_duration(absolute.max(0))
    }

    fn to_relative(duration: Duration) -> i64 {
        duration_to_ticks(duration)
    }

    fn subtract(absolute: i64, duration: Duration) -> i64 {
        absolute - duration_to_ticks(duration)
    }

    /// Work scheduled for now or earlier runs one tick later (`TestScheduler.ScheduleAbsolute`).
    fn adjust_due_time(due_time: i64, clock: i64) -> i64 {
        if due_time <= clock {
            clock + 1
        } else {
            due_time
        }
    }
}

/// A duration as ticks of 100 nanoseconds (`TimeSpan.Ticks`).
pub fn duration_to_ticks(duration: Duration) -> i64 {
    (duration.as_nanos() / 100) as i64
}

/// Ticks of 100 nanoseconds as a duration (`TimeSpan.FromTicks`).
pub fn ticks_to_duration(ticks: i64) -> Duration {
    Duration::from_nanos(ticks.max(0) as u64 * 100)
}

/// The time of the test scheduler at `ticks` (`new DateTimeOffset(ticks, TimeSpan.Zero)`).
pub fn ticks_to_time(ticks: i64) -> SystemTime {
    TestTime::to_system_time(ticks)
}

/// A virtual-time scheduler for tests, whose clock counts ticks (`TestScheduler`). It dereferences to
/// the virtual-time scheduler (`clock`, `start`, `advance_to`, `advance_by`, `sleep`,
/// `schedule_absolute`, ...).
#[derive(Clone)]
pub struct TestScheduler {
    inner: Rc<VirtualTimeScheduler<TestTime>>,
}

impl Default for TestScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for TestScheduler {
    type Target = VirtualTimeScheduler<TestTime>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl TestScheduler {
    /// A scheduler with its clock at zero.
    pub fn new() -> Self {
        Self { inner: VirtualTimeScheduler::new(0) }
    }

    /// The scheduler as the handle operators take.
    pub fn scheduler(&self) -> Rc<dyn IScheduler> {
        self.inner.clone()
    }

    /// Schedules an action at an absolute virtual time.
    pub fn schedule_absolute_action(&self, due_time: i64, action: impl FnOnce() + 'static) -> Rc<dyn IDisposable> {
        self.inner.schedule_absolute(
            due_time,
            Box::new(move |_| {
                action();
                Disposable::empty()
            }),
        )
    }

    /// Creates the sequence at `created`, subscribes an observer at `subscribed` and disposes the
    /// subscription at `disposed`, runs the scheduler, and returns the observer with what it
    /// recorded (`Start(Func<IObservable<T>>, long, long, long)`).
    pub fn start_with<T: Clone + 'static>(
        &self,
        create: impl FnOnce() -> Rc<dyn IObservable<T>> + 'static,
        created: i64,
        subscribed: i64,
        disposed: i64,
    ) -> Rc<MockObserver<T>> {
        assert!(subscribed >= created, "Specified argument was out of the range of valid values. (Parameter 'subscribed')");
        assert!(disposed >= created && disposed >= subscribed, "Specified argument was out of the range of valid values. (Parameter 'disposed')");
        let observer = self.create_observer::<T>();
        let source: Rc<RefCell<Option<Rc<dyn IObservable<T>>>>> = Rc::new(RefCell::new(None));
        let subscription: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));
        {
            let source = source.clone();
            self.schedule_absolute_action(created, move || *source.borrow_mut() = Some(create()));
        }
        {
            let (source, subscription, observer) = (source.clone(), subscription.clone(), observer.clone());
            self.schedule_absolute_action(subscribed, move || {
                let source = source.borrow().clone().expect("the sequence was created");
                *subscription.borrow_mut() = Some(source.subscribe(observer));
            });
        }
        {
            let subscription = subscription.clone();
            self.schedule_absolute_action(disposed, move || {
                let subscription = subscription.borrow().clone();
                if let Some(subscription) = subscription {
                    subscription.dispose();
                }
            });
        }
        self.inner.start();
        observer
    }

    /// [`start_with`](Self::start_with) at the default times, created at 100, subscribed at 200 and
    /// disposed at `disposed` (`Start(Func<IObservable<T>>, long)`).
    pub fn start_disposed<T: Clone + 'static>(&self, create: impl FnOnce() -> Rc<dyn IObservable<T>> + 'static, disposed: i64) -> Rc<MockObserver<T>> {
        self.start_with(create, ReactiveTest::CREATED, ReactiveTest::SUBSCRIBED, disposed)
    }

    /// [`start_with`](Self::start_with) at the default times: created at 100, subscribed at 200,
    /// disposed at 1000 (`Start(Func<IObservable<T>>)`).
    pub fn start<T: Clone + 'static>(&self, create: impl FnOnce() -> Rc<dyn IObservable<T>> + 'static) -> Rc<MockObserver<T>> {
        self.start_with(create, ReactiveTest::CREATED, ReactiveTest::SUBSCRIBED, ReactiveTest::DISPOSED)
    }

    /// Runs the queued work (`VirtualTimeScheduler.Start()`).
    pub fn run(&self) {
        self.inner.start();
    }

    /// A sequence that notifies its messages at their absolute times to the observers subscribed
    /// then (`CreateHotObservable`).
    pub fn create_hot_observable<T: Clone + 'static>(&self, messages: Vec<Recorded<Notification<T>>>) -> Rc<HotObservable<T>> {
        HotObservable::new(self.clone(), messages)
    }

    /// A sequence that notifies its messages at their times relative to each subscription
    /// (`CreateColdObservable`).
    pub fn create_cold_observable<T: Clone + 'static>(&self, messages: Vec<Recorded<Notification<T>>>) -> Rc<ColdObservable<T>> {
        Rc::new(ColdObservable { scheduler: self.clone(), messages, subscriptions: Rc::new(RefCell::new(Vec::new())) })
    }

    /// An observer that records what it receives with the time (`CreateObserver`).
    pub fn create_observer<T: 'static>(&self) -> Rc<MockObserver<T>> {
        Rc::new(MockObserver { scheduler: self.clone(), messages: RefCell::new(Vec::new()) })
    }
}

/// A value with the virtual time it was recorded at (`Recorded<T>`).
#[derive(Clone, PartialEq)]
pub struct Recorded<T> {
    /// The virtual time (`Time`).
    pub time: i64,
    /// The value (`Value`).
    pub value: T,
}

impl<T> Recorded<T> {
    pub fn new(time: i64, value: T) -> Self {
        Self { time, value }
    }
}

impl<T: fmt::Debug> fmt::Debug for Recorded<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@{}", self.value, self.time)
    }
}

/// A subscription to a test observable: the virtual times it was made and disposed
/// (`Subscription`).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Subscription {
    /// The time of the subscription (`Subscribe`).
    pub subscribe: i64,
    /// The time of its disposal; [`Subscription::INFINITE`] while it lasts (`Unsubscribe`).
    pub unsubscribe: i64,
}

impl Subscription {
    /// The time of a subscription that has not been disposed (`Infinite`).
    pub const INFINITE: i64 = i64::MAX;

    pub fn new(subscribe: i64, unsubscribe: i64) -> Self {
        Self { subscribe, unsubscribe }
    }

    pub fn starting(subscribe: i64) -> Self {
        Self { subscribe, unsubscribe: Self::INFINITE }
    }
}

impl fmt::Debug for Subscription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.unsubscribe == Self::INFINITE {
            write!(f, "({}, Infinite)", self.subscribe)
        } else {
            write!(f, "({}, {})", self.subscribe, self.unsubscribe)
        }
    }
}

/// An error made by a test, which compares by identity (`new Exception()` of upstream's tests).
#[derive(Debug)]
pub struct TestException(pub String);

impl fmt::Display for TestException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TestException {}

/// A new error for a test (`new Exception(message)`).
pub fn new_error(message: &str) -> ObservableError {
    Rc::new(TestException(message.to_owned()))
}

/// A notification a test expects: one notification, or a notification that satisfies a predicate
/// (the predicate forms of `ReactiveTest.OnNext` and `OnError`).
#[derive(Clone)]
pub enum Expected<T> {
    /// The notification itself.
    Exact(Notification<T>),
    /// A value for which the predicate is true (`OnNext<T>(long, Func<T, bool>)`).
    OnNextWhere(Rc<dyn Fn(&T) -> bool>),
    /// An error for which the predicate is true (`OnError<T>(long, Func<Exception, bool>)`).
    OnErrorWhere(Rc<dyn Fn(&ObservableError) -> bool>),
}

impl<T: PartialEq> Expected<T> {
    /// Whether the notification is the expected one.
    pub fn matches(&self, notification: &Notification<T>) -> bool {
        match (self, notification) {
            (Self::Exact(expected), actual) => expected == actual,
            (Self::OnNextWhere(predicate), Notification::OnNext(value)) => predicate(value),
            (Self::OnErrorWhere(predicate), Notification::OnError(error)) => predicate(error),
            _ => false,
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Expected<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(notification) => notification.fmt(f),
            Self::OnNextWhere(_) => f.write_str("OnNext(<predicate>)"),
            Self::OnErrorWhere(_) => f.write_str("OnError(<predicate>)"),
        }
    }
}

impl<T> From<Recorded<Notification<T>>> for Recorded<Expected<T>> {
    fn from(recorded: Recorded<Notification<T>>) -> Self {
        Recorded::new(recorded.time, Expected::Exact(recorded.value))
    }
}

/// The factories of recorded notifications and subscriptions, and the default times of
/// [`TestScheduler::start`] (`ReactiveTest`).
pub struct ReactiveTest;

impl ReactiveTest {
    /// The default time the sequence is created at (`Created`).
    pub const CREATED: i64 = 100;
    /// The default time the observer subscribes at (`Subscribed`).
    pub const SUBSCRIBED: i64 = 200;
    /// The default time the subscription is disposed at (`Disposed`).
    pub const DISPOSED: i64 = 1000;

    /// A value at `ticks` (`OnNext<T>(long, T)`).
    pub fn on_next<T>(ticks: i64, value: T) -> Recorded<Notification<T>> {
        Recorded::new(ticks, Notification::OnNext(value))
    }

    /// A value at `ticks` for which `predicate` is true (`OnNext<T>(long, Func<T, bool>)`).
    pub fn on_next_where<T>(ticks: i64, predicate: impl Fn(&T) -> bool + 'static) -> Recorded<Expected<T>> {
        Recorded::new(ticks, Expected::OnNextWhere(Rc::new(predicate)))
    }

    /// The end of the sequence at `ticks` (`OnCompleted<T>(long)`).
    pub fn on_completed<T>(ticks: i64) -> Recorded<Notification<T>> {
        Recorded::new(ticks, Notification::OnCompleted)
    }

    /// An error at `ticks` (`OnError<T>(long, Exception)`).
    pub fn on_error<T>(ticks: i64, error: ObservableError) -> Recorded<Notification<T>> {
        Recorded::new(ticks, Notification::OnError(error))
    }

    /// An error at `ticks` for which `predicate` is true (`OnError<T>(long, Func<Exception, bool>)`).
    pub fn on_error_where<T>(ticks: i64, predicate: impl Fn(&ObservableError) -> bool + 'static) -> Recorded<Expected<T>> {
        Recorded::new(ticks, Expected::OnErrorWhere(Rc::new(predicate)))
    }

    /// A subscription from `start` to `end` (`Subscribe(long, long)`).
    pub fn subscribe(start: i64, end: i64) -> Subscription {
        Subscription::new(start, end)
    }

    /// A subscription from `start` that has not been disposed (`Subscribe(long)`).
    pub fn subscribe_from(start: i64) -> Subscription {
        Subscription::starting(start)
    }
}

/// Asserts that the recorded notifications are the expected ones, in order (`AssertEqual` on the
/// messages of an observer or observable).
pub fn assert_recorded_eq<T: PartialEq + fmt::Debug>(actual: &[Recorded<Notification<T>>], expected: &[Recorded<Expected<T>>]) {
    let matches = actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(a, e)| a.time == e.time && e.value.matches(&a.value));
    if !matches {
        panic!("the recorded notifications differ\n  expected: {expected:?}\n    actual: {actual:?}");
    }
}

/// Asserts that the recorded notifications are the expected ones; each expected one is a
/// `Recorded<Notification<T>>` or a `Recorded<Expected<T>>` (`res.Messages.AssertEqual(...)`).
#[macro_export]
macro_rules! assert_messages {
    ($actual:expr $(, $expected:expr)* $(,)?) => {{
        let expected = ::std::vec![$($crate::Recorded::<$crate::Expected<_>>::from($expected)),*];
        $crate::assert_recorded_eq(&$actual, &expected);
    }};
}

/// Asserts that the subscriptions are the expected ones, in order (`xs.Subscriptions.AssertEqual(...)`).
#[macro_export]
macro_rules! assert_subscriptions {
    ($actual:expr $(, $expected:expr)* $(,)?) => {{
        let expected: ::std::vec::Vec<$crate::Subscription> = ::std::vec![$($expected),*];
        assert_eq!($actual, expected, "the subscriptions differ");
    }};
}

/// Asserts that `action` panics (`ReactiveAssert.Throws`): the form of an exception thrown to the
/// caller in the port.
pub fn assert_throws(action: impl FnOnce()) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action));
    assert!(result.is_err(), "expected the action to panic");
}

/// An observer that records what it receives with the virtual time (`MockObserver<T>`,
/// `ITestableObserver<T>`).
pub struct MockObserver<T> {
    scheduler: TestScheduler,
    messages: RefCell<Vec<Recorded<Notification<T>>>>,
}

impl<T: Clone> MockObserver<T> {
    /// What the observer received (`Messages`).
    pub fn messages(&self) -> Vec<Recorded<Notification<T>>> {
        self.messages.borrow().clone()
    }
}

impl<T> IObserver<T> for MockObserver<T> {
    fn on_next(&self, value: T) {
        let time = self.scheduler.clock();
        self.messages.borrow_mut().push(Recorded::new(time, Notification::OnNext(value)));
    }

    fn on_error(&self, error: ObservableError) {
        let time = self.scheduler.clock();
        self.messages.borrow_mut().push(Recorded::new(time, Notification::OnError(error)));
    }

    fn on_completed(&self) {
        let time = self.scheduler.clock();
        self.messages.borrow_mut().push(Recorded::new(time, Notification::OnCompleted));
    }
}

/// A sequence of recorded notifications at absolute times, shared by its observers
/// (`HotObservable<T>`, `ITestableObservable<T>`).
pub struct HotObservable<T> {
    scheduler: TestScheduler,
    messages: Vec<Recorded<Notification<T>>>,
    observers: Rc<RefCell<Vec<Rc<dyn IObserver<T>>>>>,
    subscriptions: Rc<RefCell<Vec<Subscription>>>,
}

impl<T: Clone + 'static> HotObservable<T> {
    fn new(scheduler: TestScheduler, messages: Vec<Recorded<Notification<T>>>) -> Rc<Self> {
        let observers: Rc<RefCell<Vec<Rc<dyn IObserver<T>>>>> = Rc::new(RefCell::new(Vec::new()));
        for message in &messages {
            let observers = observers.clone();
            let notification = message.value.clone();
            scheduler.schedule_absolute_action(message.time, move || {
                let current = observers.borrow().clone();
                for observer in current {
                    notification.clone().accept(&*observer);
                }
            });
        }
        Rc::new(Self { scheduler, messages, observers, subscriptions: Rc::new(RefCell::new(Vec::new())) })
    }

    /// The subscriptions made to the sequence (`Subscriptions`).
    pub fn subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions.borrow().clone()
    }

    /// The notifications of the sequence (`Messages`).
    pub fn messages(&self) -> Vec<Recorded<Notification<T>>> {
        self.messages.clone()
    }

    /// The sequence as the handle operators take.
    pub fn as_observable(self: &Rc<Self>) -> Rc<dyn IObservable<T>> {
        self.clone()
    }
}

impl<T: Clone + 'static> IObservable<T> for HotObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.observers.borrow_mut().push(observer.clone());
        let index = {
            let mut subscriptions = self.subscriptions.borrow_mut();
            subscriptions.push(Subscription::starting(self.scheduler.clock()));
            subscriptions.len() - 1
        };
        let (observers, subscriptions, scheduler) = (self.observers.clone(), self.subscriptions.clone(), self.scheduler.clone());
        Disposable::create(move || {
            {
                let mut observers = observers.borrow_mut();
                if let Some(position) = observers.iter().position(|o| Rc::ptr_eq(o, &observer)) {
                    observers.remove(position);
                }
            }
            let mut subscriptions = subscriptions.borrow_mut();
            let subscribe = subscriptions[index].subscribe;
            subscriptions[index] = Subscription::new(subscribe, scheduler.clock());
        })
    }
}

/// A sequence of recorded notifications at times relative to each subscription
/// (`ColdObservable<T>`, `ITestableObservable<T>`).
pub struct ColdObservable<T> {
    scheduler: TestScheduler,
    messages: Vec<Recorded<Notification<T>>>,
    subscriptions: Rc<RefCell<Vec<Subscription>>>,
}

impl<T: Clone + 'static> ColdObservable<T> {
    /// The subscriptions made to the sequence (`Subscriptions`).
    pub fn subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions.borrow().clone()
    }

    /// The notifications of the sequence (`Messages`).
    pub fn messages(&self) -> Vec<Recorded<Notification<T>>> {
        self.messages.clone()
    }

    /// The sequence as the handle operators take.
    pub fn as_observable(self: &Rc<Self>) -> Rc<dyn IObservable<T>> {
        self.clone()
    }
}

impl<T: Clone + 'static> IObservable<T> for ColdObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let index = {
            let mut subscriptions = self.subscriptions.borrow_mut();
            subscriptions.push(Subscription::starting(self.scheduler.clock()));
            subscriptions.len() - 1
        };
        let group = CompositeDisposable::new();
        for message in &self.messages {
            let notification = message.value.clone();
            let target = observer.clone();
            group.add(self.scheduler.schedule_relative(
                message.time,
                Box::new(move |_| {
                    notification.accept(&*target);
                    Disposable::empty()
                }),
            ));
        }
        let (subscriptions, scheduler) = (self.subscriptions.clone(), self.scheduler.clone());
        Disposable::create(move || {
            {
                let mut subscriptions = subscriptions.borrow_mut();
                let subscribe = subscriptions[index].subscribe;
                subscriptions[index] = Subscription::new(subscribe, scheduler.clock());
            }
            group.dispose();
        })
    }
}
