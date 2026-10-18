//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/GroupByUntilTest.cs` of Rx.NET.
//! Adapted: a group is a `Rc<dyn IGroupedObservable<K, V>>`; the overloads are `group_by_until`,
//! `group_by_until_comparer`, `group_by_until_with` and `group_by_until_with_comparer` and their
//! `_capacity` forms, and the `try_` forms for a key selector, element selector, duration selector
//! or comparer that throws. The tests of the region with a capacity are the tests without one, run
//! with the `_capacity` forms: each test calls the scenario it shares with its twin.
//! `GroupByComparer` (of `GroupByTest.cs`), an `IEqualityComparer<string>` that ignores case and
//! throws from `Equals` or `GetHashCode` after a time, is a comparer function (the port compares a
//! new key with each key seen before, with no hash code): it fails as `GetHashCode` would once that
//! time has passed, and as `Equals` would for keys equal but for case (the keys whose hash codes
//! match). The dictionaries the tests keep are `HashMap`s, and the variables that scheduled actions
//! assign are shared slots; `null` keys are `None`; `Unit` is `()`.

use ferroui_reactive::linq::IGroupedObservable;
use ferroui_reactive::{IDisposable, IObservable, IObserver, Notification, Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, HotObservable, MockObserver, ReactiveTest as RT, Recorded, TestScheduler};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

type Source = Rc<dyn IObservable<String>>;
type Group = Rc<dyn IGroupedObservable<String, String>>;
type Groups = Rc<dyn IObservable<Group>>;

/// `_groupByUntilCapacity`.
const GROUP_BY_UNTIL_CAPACITY: usize = 1024;

/// `ushort.MaxValue`, the time after which the default `GroupByComparer` would throw.
const NEVER: i64 = u16::MAX as i64;

/// A variable that scheduled actions assign and read (`var x = default(X);` captured by lambdas).
type Slot<T> = Rc<RefCell<Option<T>>>;

fn slot<T>() -> Slot<T> {
    Rc::new(RefCell::new(None))
}

fn get<T: Clone>(slot: &Slot<T>) -> T {
    slot.borrow().clone().expect("the variable has been assigned")
}

fn set<T>(slot: &Slot<T>, value: T) {
    *slot.borrow_mut() = Some(value);
}

fn s(value: &str) -> String {
    value.to_owned()
}

/// `Reverse(string)`.
fn reverse(value: &str) -> String {
    value.chars().rev().collect()
}

/// `x => x.Trim()`.
fn trim(value: &String) -> String {
    value.trim().to_owned()
}

/// `StringComparison.OrdinalIgnoreCase`: the default `GroupByComparer`, which never throws.
fn ignore_case(x: &String, y: &String) -> bool {
    x.eq_ignore_ascii_case(y)
}

fn increment(counter: &Cell<i32>) -> i32 {
    counter.set(counter.get() + 1);
    counter.get()
}

/// The source most tests share, with `end` at 570 (`OnCompleted<string>(570)` or
/// `OnError<string>(570, ex)`).
fn source(scheduler: &TestScheduler, end: Recorded<Notification<String>>) -> Rc<HotObservable<String>> {
    scheduler.create_hot_observable(vec![
        RT::on_next(90, s("error")),
        RT::on_next(110, s("error")),
        RT::on_next(130, s("error")),
        RT::on_next(220, s("  foo")),
        RT::on_next(240, s(" FoO ")),
        RT::on_next(270, s("baR  ")),
        RT::on_next(310, s("foO ")),
        RT::on_next(350, s(" Baz   ")),
        RT::on_next(360, s("  qux ")),
        RT::on_next(390, s("   bar")),
        RT::on_next(420, s(" BAR  ")),
        RT::on_next(470, s("FOO ")),
        RT::on_next(480, s("baz  ")),
        RT::on_next(510, s(" bAZ ")),
        RT::on_next(530, s("    fOo    ")),
        end,
        RT::on_next(580, s("error")),
        RT::on_completed(600),
        RT::on_error(650, new_error("ex")),
    ])
}

/// `GroupByComparer`: compares strings ignoring case; after `equals_throws_after` the comparison
/// of two keys equal but for case fails with `equals_exception`, and after
/// `get_hash_code_throws_after` every comparison fails with `hash_code_exception`.
#[derive(Clone)]
struct GroupByComparer {
    scheduler: TestScheduler,
    equals_throws_after: i64,
    get_hash_code_throws_after: i64,
    hash_code_exception: ObservableError,
    equals_exception: ObservableError,
}

impl GroupByComparer {
    fn new(scheduler: &TestScheduler) -> Self {
        Self::with(scheduler, NEVER, NEVER)
    }

    fn with(scheduler: &TestScheduler, equals_throws_after: i64, get_hash_code_throws_after: i64) -> Self {
        Self {
            scheduler: scheduler.clone(),
            equals_throws_after,
            get_hash_code_throws_after,
            hash_code_exception: new_error("hash code"),
            equals_exception: new_error("equals"),
        }
    }

    fn equals(&self, x: &String, y: &String) -> Result<bool, ObservableError> {
        if self.scheduler.clock() > self.get_hash_code_throws_after {
            return Err(self.hash_code_exception.clone());
        }
        if !x.eq_ignore_ascii_case(y) {
            // The hash codes differ: upstream's dictionary does not call `Equals`.
            return Ok(false);
        }
        if self.scheduler.clock() > self.equals_throws_after {
            return Err(self.equals_exception.clone());
        }
        Ok(true)
    }

    fn comparer(&self) -> impl Fn(&String, &String) -> Result<bool, ObservableError> + 'static {
        let this = self.clone();
        move |x: &String, y: &String| this.equals(x, y)
    }
}

/// `g => g.Skip(2)`.
fn skip2(g: Group) -> Rc<dyn IObservable<String>> {
    g.skip(2)
}

/// `xs.GroupByUntil(keySelector, durationSelector, comparer)`, with `_groupByUntilCapacity` when
/// `capacity` is given.
fn group_by_until_key_comparer(
    xs: Source,
    capacity: Option<usize>,
    key_selector: impl Fn(&String) -> String + 'static,
    duration_selector: impl Fn(Group) -> Rc<dyn IObservable<String>> + 'static,
    comparer: impl Fn(&String, &String) -> bool + 'static,
) -> Groups {
    match capacity {
        None => xs.group_by_until_comparer(key_selector, duration_selector, comparer),
        Some(capacity) => xs.group_by_until_capacity_comparer(key_selector, duration_selector, capacity, comparer),
    }
}

/// `xs.GroupByUntil(keySelector, elementSelector, durationSelector, comparer)`, with
/// `_groupByUntilCapacity` when `capacity` is given.
fn group_by_until_key_element_comparer(
    xs: Source,
    capacity: Option<usize>,
    key_selector: impl Fn(&String) -> String + 'static,
    element_selector: impl Fn(String) -> String + 'static,
    duration_selector: impl Fn(Group) -> Rc<dyn IObservable<String>> + 'static,
    comparer: impl Fn(&String, &String) -> bool + 'static,
) -> Groups {
    match capacity {
        None => xs.group_by_until_with_comparer(key_selector, element_selector, duration_selector, comparer),
        Some(capacity) => xs.group_by_until_with_capacity_comparer(key_selector, element_selector, duration_selector, capacity, comparer),
    }
}

/// [`group_by_until_key_element_comparer`] with selectors and a comparer that throw.
fn try_group_by_until_key_element_comparer(
    xs: Source,
    capacity: Option<usize>,
    key_selector: impl Fn(&String) -> Result<String, ObservableError> + 'static,
    element_selector: impl Fn(String) -> Result<String, ObservableError> + 'static,
    duration_selector: impl Fn(Group) -> Result<Rc<dyn IObservable<String>>, ObservableError> + 'static,
    comparer: impl Fn(&String, &String) -> Result<bool, ObservableError> + 'static,
) -> Groups {
    match capacity {
        None => xs.try_group_by_until_with_comparer(key_selector, element_selector, duration_selector, comparer),
        Some(capacity) => xs.try_group_by_until_with_capacity_comparer(key_selector, element_selector, duration_selector, capacity, comparer),
    }
}

/// `xs.GroupByUntil(keySelector, durationSelector)`, with `_groupByUntilCapacity` when `capacity`
/// is given.
fn group_by_until_key<K: Eq + std::hash::Hash + Clone + 'static, D: 'static>(
    xs: Source,
    capacity: Option<usize>,
    key_selector: impl Fn(&String) -> K + 'static,
    duration_selector: impl Fn(Rc<dyn IGroupedObservable<K, String>>) -> Rc<dyn IObservable<D>> + 'static,
) -> Rc<dyn IObservable<Rc<dyn IGroupedObservable<K, String>>>> {
    match capacity {
        None => xs.group_by_until(key_selector, duration_selector),
        Some(capacity) => xs.group_by_until_capacity(key_selector, duration_selector, capacity),
    }
}

/// `xs.GroupByUntil(keySelector, elementSelector, durationSelector)`, with `_groupByUntilCapacity`
/// when `capacity` is given.
fn group_by_until_key_element(
    xs: Source,
    capacity: Option<usize>,
    key_selector: impl Fn(&String) -> String + 'static,
    element_selector: impl Fn(String) -> String + 'static,
    duration_selector: impl Fn(Group) -> Rc<dyn IObservable<String>> + 'static,
) -> Groups {
    match capacity {
        None => xs.group_by_until_with(key_selector, element_selector, duration_selector),
        Some(capacity) => xs.group_by_until_with_capacity(key_selector, element_selector, duration_selector, capacity),
    }
}

/// The variables of the tests of the groups themselves: `outer`, `outerSubscription`, `inners`,
/// `innerSubscriptions` and `res`.
#[derive(Clone)]
struct Inners {
    scheduler: TestScheduler,
    outer: Slot<Groups>,
    outer_subscription: Slot<Rc<dyn IDisposable>>,
    inners: Rc<RefCell<HashMap<String, Group>>>,
    inner_subscriptions: Rc<RefCell<HashMap<String, Rc<dyn IDisposable>>>>,
    res: Rc<RefCell<HashMap<String, Rc<MockObserver<String>>>>>,
}

impl Inners {
    fn new(scheduler: &TestScheduler) -> Self {
        Self {
            scheduler: scheduler.clone(),
            outer: slot(),
            outer_subscription: slot(),
            inners: Rc::new(RefCell::new(HashMap::new())),
            inner_subscriptions: Rc::new(RefCell::new(HashMap::new())),
            res: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// `scheduler.ScheduleAbsolute(Created, () => outer = create())`.
    fn create(&self, create: impl FnOnce() -> Groups + 'static) {
        let outer = self.outer.clone();
        self.scheduler.schedule_absolute_action(RT::CREATED, move || set(&outer, create()));
    }

    /// The handler of a group: `var result = scheduler.CreateObserver<string>(); inners[group.Key]
    /// = group; res[group.Key] = result; innerSubscriptions[group.Key] = group.Subscribe(result);`,
    /// the subscription made 100 ticks later with `later` (`scheduler.ScheduleRelative(100, ...)`).
    fn on_group(&self, group: Group, later: bool) {
        let result = self.scheduler.create_observer::<String>();
        let key = group.key().clone();
        self.inners.borrow_mut().insert(key.clone(), group.clone());
        self.res.borrow_mut().insert(key.clone(), result.clone());
        if later {
            let inner_subscriptions = self.inner_subscriptions.clone();
            let due = self.scheduler.clock() + 100;
            self.scheduler.schedule_absolute_action(due, move || {
                let subscription = group.subscribe(result);
                inner_subscriptions.borrow_mut().insert(key, subscription);
            });
        } else {
            let subscription = group.subscribe(result);
            self.inner_subscriptions.borrow_mut().insert(key, subscription);
        }
    }

    /// `scheduler.ScheduleAbsolute(Subscribed, () => outerSubscription = outer.Subscribe(group =>
    /// ...))`, with `ex => { }` when `ignore_errors`.
    fn subscribe(&self, later: bool, ignore_errors: bool) {
        let this = self.clone();
        self.scheduler.schedule_absolute_action(RT::SUBSCRIBED, move || {
            let handler = this.clone();
            let on_group = move |group: Group| handler.on_group(group, later);
            let subscription = if ignore_errors { get(&this.outer).subscribe_next_error(on_group, |_| {}) } else { get(&this.outer).subscribe_next(on_group) };
            set(&this.outer_subscription, subscription);
        });
    }

    /// `outer.Subscribe(group => { outerResults.OnNext(group.Key); ... }, outerResults.OnError,
    /// outerResults.OnCompleted)` at `Subscribed`.
    fn subscribe_with_results(&self, outer_results: &Rc<MockObserver<String>>) {
        let this = self.clone();
        let outer_results = outer_results.clone();
        self.scheduler.schedule_absolute_action(RT::SUBSCRIBED, move || {
            let handler = this.clone();
            let (on_next_results, on_error_results, on_completed_results) = (outer_results.clone(), outer_results.clone(), outer_results.clone());
            let subscription = get(&this.outer).subscribe_all(
                move |group: Group| {
                    on_next_results.on_next(group.key().clone());
                    handler.on_group(group, false);
                },
                move |error| on_error_results.on_error(error),
                move || on_completed_results.on_completed(),
            );
            set(&this.outer_subscription, subscription);
        });
    }

    /// `outerSubscription.Dispose(); foreach (var d in innerSubscriptions.Values) d.Dispose();` at
    /// `time`.
    fn dispose_all_at(&self, time: i64) {
        let this = self.clone();
        self.scheduler.schedule_absolute_action(time, move || {
            get(&this.outer_subscription).dispose();
            let subscriptions: Vec<Rc<dyn IDisposable>> = this.inner_subscriptions.borrow().values().cloned().collect();
            for subscription in subscriptions {
                subscription.dispose();
            }
        });
    }

    /// `scheduler.ScheduleAbsolute(time, () => outerSubscription.Dispose())`.
    fn dispose_outer_at(&self, time: i64) {
        let outer_subscription = self.outer_subscription.clone();
        self.scheduler.schedule_absolute_action(time, move || get(&outer_subscription).dispose());
    }

    /// `scheduler.ScheduleAbsolute(time, () => innerSubscriptions[key].Dispose())`.
    fn dispose_inner_at(&self, time: i64, key: &'static str) {
        let inner_subscriptions = self.inner_subscriptions.clone();
        self.scheduler.schedule_absolute_action(time, move || {
            let subscription = inner_subscriptions.borrow()[key].clone();
            subscription.dispose();
        });
    }

    /// `inners.Count`.
    fn count(&self) -> usize {
        self.inners.borrow().len()
    }

    /// `res[key].Messages`.
    fn messages(&self, key: &str) -> Vec<Recorded<Notification<String>>> {
        self.res.borrow()[key].messages()
    }
}

// The region `+ GroupByUntil +`, and with `capacity` the region `+ GroupByUntil w/capacity +`.

fn with_key_comparer(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let key_invoked = Rc::new(Cell::new(0));

    let xs = source(&scheduler, RT::on_completed(570));

    let res = scheduler.start({
        let (xs, key_invoked) = (xs.clone(), key_invoked.clone());
        move || {
            group_by_until_key_comparer(
                xs.as_observable(),
                capacity,
                move |x| {
                    increment(&key_invoked);
                    trim(x)
                },
                skip2,
                ignore_case,
            )
            .select(|g: Group| g.key().clone())
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, s("foo")),
        RT::on_next(270, s("baR")),
        RT::on_next(350, s("Baz")),
        RT::on_next(360, s("qux")),
        RT::on_next(470, s("FOO")),
        RT::on_completed(570)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 570));

    assert_eq!(12, key_invoked.get());
}

/// The scenario of `GroupByUntil_Outer_Complete` and `GroupByUntil_Outer_Error`: the source ends
/// with `end` at 570.
fn outer_end(capacity: Option<usize>, end: Recorded<Notification<String>>) {
    let scheduler = TestScheduler::new();

    let key_invoked = Rc::new(Cell::new(0));
    let ele_invoked = Rc::new(Cell::new(0));

    let xs = source(&scheduler, end.clone());

    let res = scheduler.start({
        let (xs, key_invoked, ele_invoked) = (xs.clone(), key_invoked.clone(), ele_invoked.clone());
        move || {
            group_by_until_key_element_comparer(
                xs.as_observable(),
                capacity,
                move |x| {
                    increment(&key_invoked);
                    trim(x)
                },
                move |x| {
                    increment(&ele_invoked);
                    reverse(&x)
                },
                skip2,
                ignore_case,
            )
            .select(|g: Group| g.key().clone())
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, s("foo")),
        RT::on_next(270, s("baR")),
        RT::on_next(350, s("Baz")),
        RT::on_next(360, s("qux")),
        RT::on_next(470, s("FOO")),
        end
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 570));

    assert_eq!(12, key_invoked.get());
    assert_eq!(12, ele_invoked.get());
}

fn outer_complete(capacity: Option<usize>) {
    outer_end(capacity, RT::on_completed(570));
}

fn outer_error(capacity: Option<usize>) {
    let ex = new_error("ex");
    outer_end(capacity, RT::on_error(570, ex));
}

fn outer_dispose(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let key_invoked = Rc::new(Cell::new(0));
    let ele_invoked = Rc::new(Cell::new(0));

    let xs = source(&scheduler, RT::on_completed(570));

    let res = scheduler.start_disposed(
        {
            let (xs, key_invoked, ele_invoked) = (xs.clone(), key_invoked.clone(), ele_invoked.clone());
            move || {
                group_by_until_key_element_comparer(
                    xs.as_observable(),
                    capacity,
                    move |x| {
                        increment(&key_invoked);
                        trim(x)
                    },
                    move |x| {
                        increment(&ele_invoked);
                        reverse(&x)
                    },
                    skip2,
                    ignore_case,
                )
                .select(|g: Group| g.key().clone())
            }
        },
        355,
    );

    assert_messages!(res.messages(), RT::on_next(220, s("foo")), RT::on_next(270, s("baR")), RT::on_next(350, s("Baz")));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 355));

    assert_eq!(5, key_invoked.get());
    assert_eq!(5, ele_invoked.get());
}

fn outer_key_throw(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let key_invoked = Rc::new(Cell::new(0));
    let ele_invoked = Rc::new(Cell::new(0));
    let ex = new_error("ex");

    let xs = source(&scheduler, RT::on_completed(570));

    let comparer = GroupByComparer::new(&scheduler);

    let res = scheduler.start({
        let (xs, key_invoked, ele_invoked, ex) = (xs.clone(), key_invoked.clone(), ele_invoked.clone(), ex.clone());
        move || {
            try_group_by_until_key_element_comparer(
                xs.as_observable(),
                capacity,
                move |x| {
                    if increment(&key_invoked) == 10 {
                        return Err(ex.clone());
                    }
                    Ok(trim(x))
                },
                move |x| {
                    increment(&ele_invoked);
                    Ok(reverse(&x))
                },
                |g| Ok(skip2(g)),
                comparer.comparer(),
            )
            .select(|g: Group| g.key().clone())
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, s("foo")),
        RT::on_next(270, s("baR")),
        RT::on_next(350, s("Baz")),
        RT::on_next(360, s("qux")),
        RT::on_next(470, s("FOO")),
        RT::on_error(480, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 480));

    assert_eq!(10, key_invoked.get());
    assert_eq!(9, ele_invoked.get());
}

fn outer_ele_throw(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let key_invoked = Rc::new(Cell::new(0));
    let ele_invoked = Rc::new(Cell::new(0));
    let ex = new_error("ex");

    let xs = source(&scheduler, RT::on_completed(570));

    let comparer = GroupByComparer::new(&scheduler);

    let res = scheduler.start({
        let (xs, key_invoked, ele_invoked, ex) = (xs.clone(), key_invoked.clone(), ele_invoked.clone(), ex.clone());
        move || {
            try_group_by_until_key_element_comparer(
                xs.as_observable(),
                capacity,
                move |x| {
                    increment(&key_invoked);
                    Ok(trim(x))
                },
                move |x| {
                    if increment(&ele_invoked) == 10 {
                        return Err(ex.clone());
                    }
                    Ok(reverse(&x))
                },
                |g| Ok(skip2(g)),
                comparer.comparer(),
            )
            .select(|g: Group| g.key().clone())
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, s("foo")),
        RT::on_next(270, s("baR")),
        RT::on_next(350, s("Baz")),
        RT::on_next(360, s("qux")),
        RT::on_next(470, s("FOO")),
        RT::on_error(480, ex.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 480));

    assert_eq!(10, key_invoked.get());
    assert_eq!(10, ele_invoked.get());
}

fn outer_comparer_equals_throw(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let key_invoked = Rc::new(Cell::new(0));
    let ele_invoked = Rc::new(Cell::new(0));

    let xs = source(&scheduler, RT::on_completed(570));

    let comparer = GroupByComparer::with(&scheduler, 250, NEVER);

    let res = scheduler.start({
        let (xs, key_invoked, ele_invoked, comparer) = (xs.clone(), key_invoked.clone(), ele_invoked.clone(), comparer.clone());
        move || {
            try_group_by_until_key_element_comparer(
                xs.as_observable(),
                capacity,
                move |x| {
                    increment(&key_invoked);
                    Ok(trim(x))
                },
                move |x| {
                    increment(&ele_invoked);
                    Ok(reverse(&x))
                },
                |g| Ok(skip2(g)),
                comparer.comparer(),
            )
            .select(|g: Group| g.key().clone())
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, s("foo")),
        RT::on_next(270, s("baR")),
        RT::on_error(310, comparer.equals_exception.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 310));

    assert_eq!(4, key_invoked.get());
    assert_eq!(3, ele_invoked.get());
}

fn outer_comparer_get_hash_code_throw(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let key_invoked = Rc::new(Cell::new(0));
    let ele_invoked = Rc::new(Cell::new(0));

    let xs = source(&scheduler, RT::on_completed(570));

    let comparer = GroupByComparer::with(&scheduler, NEVER, 410);

    let res = scheduler.start({
        let (xs, key_invoked, ele_invoked, comparer) = (xs.clone(), key_invoked.clone(), ele_invoked.clone(), comparer.clone());
        move || {
            try_group_by_until_key_element_comparer(
                xs.as_observable(),
                capacity,
                move |x| {
                    increment(&key_invoked);
                    Ok(trim(x))
                },
                move |x| {
                    increment(&ele_invoked);
                    Ok(reverse(&x))
                },
                |g| Ok(skip2(g)),
                comparer.comparer(),
            )
            .select(|g: Group| g.key().clone())
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, s("foo")),
        RT::on_next(270, s("baR")),
        RT::on_next(350, s("Baz")),
        RT::on_next(360, s("qux")),
        RT::on_error(420, comparer.hash_code_exception.clone())
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));

    assert_eq!(8, key_invoked.get());
    assert_eq!(7, ele_invoked.get());
}

/// `xs.GroupByUntil(x => x.Trim(), x => Reverse(x), g => g.Skip(2), comparer)` with the default
/// `GroupByComparer`.
fn trimmed_reversed(xs: &Rc<HotObservable<String>>, capacity: Option<usize>) -> impl FnOnce() -> Groups + 'static {
    let xs = xs.clone();
    move || group_by_until_key_element_comparer(xs.as_observable(), capacity, trim, |x| reverse(&x), skip2, ignore_case)
}

fn inner_complete(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(570));

    let state = Inners::new(&scheduler);

    state.create(trimmed_reversed(&xs, capacity));
    state.subscribe(true, false);
    state.dispose_all_at(RT::DISPOSED);

    scheduler.run();

    assert_eq!(5, state.count());

    assert_messages!(state.messages("foo"), RT::on_completed(320));

    // Breaking change > v2.2 (upstream's comment): prior to resolving a deadlock, the group would
    // get closed prior to letting the value at 420 (and at 510 for "Baz") through.
    assert_messages!(state.messages("baR"), RT::on_next(390, s("rab   ")), RT::on_next(420, s("  RAB ")), RT::on_completed(420));

    assert_messages!(state.messages("Baz"), RT::on_next(480, s("  zab")), RT::on_next(510, s(" ZAb ")), RT::on_completed(510));

    assert_messages!(state.messages("qux"), RT::on_completed(570));

    assert_messages!(state.messages("FOO"), RT::on_completed(570));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 570));
}

fn inner_complete_all(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(570));

    let state = Inners::new(&scheduler);

    state.create(trimmed_reversed(&xs, capacity));
    state.subscribe(false, false);
    state.dispose_all_at(RT::DISPOSED);

    scheduler.run();

    assert_eq!(5, state.count());

    assert_messages!(
        state.messages("foo"),
        RT::on_next(220, s("oof  ")),
        RT::on_next(240, s(" OoF ")),
        RT::on_next(310, s(" Oof")),
        RT::on_completed(310)
    );

    assert_messages!(
        state.messages("baR"),
        RT::on_next(270, s("  Rab")),
        RT::on_next(390, s("rab   ")),
        RT::on_next(420, s("  RAB ")),
        RT::on_completed(420)
    );

    assert_messages!(
        state.messages("Baz"),
        RT::on_next(350, s("   zaB ")),
        RT::on_next(480, s("  zab")),
        RT::on_next(510, s(" ZAb ")),
        RT::on_completed(510)
    );

    assert_messages!(state.messages("qux"), RT::on_next(360, s(" xuq  ")), RT::on_completed(570));

    assert_messages!(state.messages("FOO"), RT::on_next(470, s(" OOF")), RT::on_next(530, s("    oOf    ")), RT::on_completed(570));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 570));
}

fn inner_error(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let ex1 = new_error("ex1");

    let xs = source(&scheduler, RT::on_error(570, ex1.clone()));

    let state = Inners::new(&scheduler);

    state.create(trimmed_reversed(&xs, capacity));
    state.subscribe(true, true);
    state.dispose_all_at(RT::DISPOSED);

    scheduler.run();

    assert_eq!(5, state.count());

    assert_messages!(state.messages("foo"), RT::on_completed(320));

    // Breaking change > v2.2 (upstream's comment): prior to resolving a deadlock, the group would
    // get closed prior to letting the value at 420 (and at 510 for "Baz") through.
    assert_messages!(state.messages("baR"), RT::on_next(390, s("rab   ")), RT::on_next(420, s("  RAB ")), RT::on_completed(420));

    assert_messages!(state.messages("Baz"), RT::on_next(480, s("  zab")), RT::on_next(510, s(" ZAb ")), RT::on_completed(510));

    assert_messages!(state.messages("qux"), RT::on_error(570, ex1.clone()));

    assert_messages!(state.messages("FOO"), RT::on_error(570, ex1.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 570));
}

fn inner_dispose(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(570));

    let state = Inners::new(&scheduler);

    state.create(trimmed_reversed(&xs, capacity));
    state.subscribe(false, false);
    state.dispose_all_at(400);

    scheduler.run();

    assert_eq!(4, state.count());

    assert_messages!(
        state.messages("foo"),
        RT::on_next(220, s("oof  ")),
        RT::on_next(240, s(" OoF ")),
        RT::on_next(310, s(" Oof")),
        RT::on_completed(310)
    );

    assert_messages!(state.messages("baR"), RT::on_next(270, s("  Rab")), RT::on_next(390, s("rab   ")));

    assert_messages!(state.messages("Baz"), RT::on_next(350, s("   zaB ")));

    assert_messages!(state.messages("qux"), RT::on_next(360, s(" xuq  ")));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 400));
}

fn inner_key_throw(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(570));

    let comparer = GroupByComparer::new(&scheduler);

    let state = Inners::new(&scheduler);

    let key_invoked = Rc::new(Cell::new(0));
    let ex = new_error("ex");

    state.create({
        let (xs, key_invoked, ex) = (xs.clone(), key_invoked.clone(), ex.clone());
        move || {
            try_group_by_until_key_element_comparer(
                xs.as_observable(),
                capacity,
                move |x| {
                    if increment(&key_invoked) == 6 {
                        return Err(ex.clone());
                    }
                    Ok(trim(x))
                },
                |x| Ok(reverse(&x)),
                |g| Ok(skip2(g)),
                comparer.comparer(),
            )
        }
    });
    state.subscribe(false, true);
    state.dispose_all_at(RT::DISPOSED);

    scheduler.run();

    assert_eq!(3, state.count());

    assert_messages!(
        state.messages("foo"),
        RT::on_next(220, s("oof  ")),
        RT::on_next(240, s(" OoF ")),
        RT::on_next(310, s(" Oof")),
        RT::on_completed(310)
    );

    assert_messages!(state.messages("baR"), RT::on_next(270, s("  Rab")), RT::on_error(360, ex.clone()));

    assert_messages!(state.messages("Baz"), RT::on_next(350, s("   zaB ")), RT::on_error(360, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 360));
}

fn inner_ele_throw(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(570));

    let comparer = GroupByComparer::new(&scheduler);

    let state = Inners::new(&scheduler);

    let ele_invoked = Rc::new(Cell::new(0));
    let ex = new_error("ex");

    state.create({
        let (xs, ele_invoked, ex) = (xs.clone(), ele_invoked.clone(), ex.clone());
        move || {
            try_group_by_until_key_element_comparer(
                xs.as_observable(),
                capacity,
                |x| Ok(trim(x)),
                move |x| {
                    if increment(&ele_invoked) == 6 {
                        return Err(ex.clone());
                    }
                    Ok(reverse(&x))
                },
                |g| Ok(skip2(g)),
                comparer.comparer(),
            )
        }
    });
    state.subscribe(false, true);
    state.dispose_all_at(RT::DISPOSED);

    scheduler.run();

    assert_eq!(4, state.count());

    assert_messages!(
        state.messages("foo"),
        RT::on_next(220, s("oof  ")),
        RT::on_next(240, s(" OoF ")),
        RT::on_next(310, s(" Oof")),
        RT::on_completed(310)
    );

    assert_messages!(state.messages("baR"), RT::on_next(270, s("  Rab")), RT::on_error(360, ex.clone()));

    assert_messages!(state.messages("Baz"), RT::on_next(350, s("   zaB ")), RT::on_error(360, ex.clone()));

    assert_messages!(state.messages("qux"), RT::on_error(360, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 360));
}

/// The scenario of `GroupByUntil_Inner_Comparer_EqualsThrow` and
/// `GroupByUntil_Inner_Comparer_GetHashCodeThrow`: the comparer fails at 420 with `error` of
/// `comparer`.
fn inner_comparer_throw(capacity: Option<usize>, comparer: GroupByComparer, error: ObservableError) {
    let scheduler = comparer.scheduler.clone();

    let xs = source(&scheduler, RT::on_completed(570));

    let state = Inners::new(&scheduler);

    state.create({
        let xs = xs.clone();
        move || {
            try_group_by_until_key_element_comparer(
                xs.as_observable(),
                capacity,
                |x| Ok(trim(x)),
                |x| Ok(reverse(&x)),
                |g| Ok(skip2(g)),
                comparer.comparer(),
            )
        }
    });
    state.subscribe(false, true);
    state.dispose_all_at(RT::DISPOSED);

    scheduler.run();

    assert_eq!(4, state.count());

    assert_messages!(
        state.messages("foo"),
        RT::on_next(220, s("oof  ")),
        RT::on_next(240, s(" OoF ")),
        RT::on_next(310, s(" Oof")),
        RT::on_completed(310)
    );

    assert_messages!(state.messages("baR"), RT::on_next(270, s("  Rab")), RT::on_next(390, s("rab   ")), RT::on_error(420, error.clone()));

    assert_messages!(state.messages("Baz"), RT::on_next(350, s("   zaB ")), RT::on_error(420, error.clone()));

    assert_messages!(state.messages("qux"), RT::on_next(360, s(" xuq  ")), RT::on_error(420, error.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

fn inner_comparer_equals_throw(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();
    let comparer = GroupByComparer::with(&scheduler, 400, NEVER);
    let error = comparer.equals_exception.clone();
    inner_comparer_throw(capacity, comparer, error);
}

fn inner_comparer_get_hash_code_throw(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();
    let comparer = GroupByComparer::with(&scheduler, NEVER, 400);
    let error = comparer.hash_code_exception.clone();
    inner_comparer_throw(capacity, comparer, error);
}

fn outer_independence(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(570));

    let state = Inners::new(&scheduler);
    let outer_results = scheduler.create_observer::<String>();

    state.create(trimmed_reversed(&xs, capacity));
    state.subscribe_with_results(&outer_results);
    state.dispose_all_at(RT::DISPOSED);
    state.dispose_outer_at(320);

    scheduler.run();

    assert_eq!(2, state.count());

    assert_messages!(outer_results.messages(), RT::on_next(220, s("foo")), RT::on_next(270, s("baR")));

    assert_messages!(
        state.messages("foo"),
        RT::on_next(220, s("oof  ")),
        RT::on_next(240, s(" OoF ")),
        RT::on_next(310, s(" Oof")),
        RT::on_completed(310)
    );

    assert_messages!(
        state.messages("baR"),
        RT::on_next(270, s("  Rab")),
        RT::on_next(390, s("rab   ")),
        RT::on_next(420, s("  RAB ")),
        RT::on_completed(420)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 420));
}

fn inner_independence(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(570));

    let state = Inners::new(&scheduler);
    let outer_results = scheduler.create_observer::<String>();

    state.create(trimmed_reversed(&xs, capacity));
    state.subscribe_with_results(&outer_results);
    state.dispose_all_at(RT::DISPOSED);
    state.dispose_inner_at(320, "foo");

    scheduler.run();

    assert_eq!(5, state.count());

    assert_messages!(
        state.messages("foo"),
        RT::on_next(220, s("oof  ")),
        RT::on_next(240, s(" OoF ")),
        RT::on_next(310, s(" Oof")),
        RT::on_completed(310)
    );

    assert_messages!(
        state.messages("baR"),
        RT::on_next(270, s("  Rab")),
        RT::on_next(390, s("rab   ")),
        RT::on_next(420, s("  RAB ")),
        RT::on_completed(420)
    );

    assert_messages!(
        state.messages("Baz"),
        RT::on_next(350, s("   zaB ")),
        RT::on_next(480, s("  zab")),
        RT::on_next(510, s(" ZAb ")),
        RT::on_completed(510)
    );

    assert_messages!(state.messages("qux"), RT::on_next(360, s(" xuq  ")), RT::on_completed(570));

    assert_messages!(state.messages("FOO"), RT::on_next(470, s(" OOF")), RT::on_next(530, s("    oOf    ")), RT::on_completed(570));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 570));
}

fn inner_multiple_independence(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = source(&scheduler, RT::on_completed(570));

    let state = Inners::new(&scheduler);
    let outer_results = scheduler.create_observer::<String>();

    state.create(trimmed_reversed(&xs, capacity));
    state.subscribe_with_results(&outer_results);
    state.dispose_all_at(RT::DISPOSED);
    state.dispose_inner_at(320, "foo");
    state.dispose_inner_at(280, "baR");
    state.dispose_inner_at(355, "Baz");
    state.dispose_inner_at(400, "qux");

    scheduler.run();

    assert_eq!(5, state.count());

    assert_messages!(
        state.messages("foo"),
        RT::on_next(220, s("oof  ")),
        RT::on_next(240, s(" OoF ")),
        RT::on_next(310, s(" Oof")),
        RT::on_completed(310)
    );

    assert_messages!(state.messages("baR"), RT::on_next(270, s("  Rab")));

    assert_messages!(state.messages("Baz"), RT::on_next(350, s("   zaB ")));

    assert_messages!(state.messages("qux"), RT::on_next(360, s(" xuq  ")));

    assert_messages!(state.messages("FOO"), RT::on_next(470, s(" OOF")), RT::on_next(530, s("    oOf    ")), RT::on_completed(570));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 570));
}

/// The source of the tests of a group that escapes its handler, with `end` at 570.
fn escape_source(scheduler: &TestScheduler, end: Recorded<Notification<String>>) -> Rc<HotObservable<String>> {
    scheduler.create_hot_observable(vec![
        RT::on_next(220, s("  foo")),
        RT::on_next(240, s(" FoO ")),
        RT::on_next(310, s("foO ")),
        RT::on_next(470, s("FOO ")),
        RT::on_next(530, s("    fOo    ")),
        end,
    ])
}

/// The scenario of the tests of a group that escapes its handler: the last group is subscribed to
/// at 600; the subscription to the groups is disposed at `outer_disposed`.
fn inner_escape(
    scheduler: &TestScheduler,
    capacity: Option<usize>,
    xs: &Rc<HotObservable<String>>,
    ignore_errors: bool,
    outer_disposed: i64,
) -> Rc<MockObserver<String>> {
    let outer: Slot<Groups> = slot();
    let outer_subscription: Slot<Rc<dyn IDisposable>> = slot();
    let inner: Slot<Group> = slot();
    let inner_subscription: Slot<Rc<dyn IDisposable>> = slot();
    let res = scheduler.create_observer::<String>();

    {
        let (xs, outer) = (xs.clone(), outer.clone());
        scheduler.schedule_absolute_action(RT::CREATED, move || set(&outer, group_by_until_key(xs.as_observable(), capacity, trim, skip2)));
    }

    {
        let (outer, outer_subscription, inner) = (outer.clone(), outer_subscription.clone(), inner.clone());
        scheduler.schedule_absolute_action(RT::SUBSCRIBED, move || {
            let inner = inner.clone();
            let on_group = move |group: Group| set(&inner, group);
            let subscription = if ignore_errors { get(&outer).subscribe_next_error(on_group, |_| {}) } else { get(&outer).subscribe_next(on_group) };
            set(&outer_subscription, subscription);
        });
    }

    if outer_disposed != RT::DISPOSED {
        let outer_subscription = outer_subscription.clone();
        scheduler.schedule_absolute_action(outer_disposed, move || get(&outer_subscription).dispose());
    }

    {
        let (inner, inner_subscription, res) = (inner.clone(), inner_subscription.clone(), res.clone());
        scheduler.schedule_absolute_action(600, move || set(&inner_subscription, get(&inner).subscribe(res)));
    }

    {
        let (outer_subscription, inner_subscription) = (outer_subscription.clone(), inner_subscription.clone());
        scheduler.schedule_absolute_action(RT::DISPOSED, move || {
            if outer_disposed == RT::DISPOSED {
                get(&outer_subscription).dispose();
            }
            get(&inner_subscription).dispose();
        });
    }

    scheduler.run();

    res
}

fn inner_escape_complete(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = escape_source(&scheduler, RT::on_completed(570));

    let res = inner_escape(&scheduler, capacity, &xs, false, RT::DISPOSED);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 570));

    assert_messages!(res.messages(), RT::on_completed(600));
}

fn inner_escape_error(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = escape_source(&scheduler, RT::on_error(570, ex.clone()));

    let res = inner_escape(&scheduler, capacity, &xs, true, RT::DISPOSED);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 570));

    assert_messages!(res.messages(), RT::on_error(600, ex.clone()));
}

fn inner_escape_dispose(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = escape_source(&scheduler, RT::on_error(570, new_error("ex")));

    let res = inner_escape(&scheduler, capacity, &xs, false, 290);

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 290));

    assert_messages!(res.messages());
}

fn default(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let key_invoked = Rc::new(Cell::new(0));
    let ele_invoked = Rc::new(Cell::new(0));

    let xs = source(&scheduler, RT::on_completed(570));

    let res = scheduler.start({
        let (xs, key_invoked, ele_invoked) = (xs.clone(), key_invoked.clone(), ele_invoked.clone());
        move || {
            group_by_until_key_element(
                xs.as_observable(),
                capacity,
                move |x| {
                    increment(&key_invoked);
                    x.trim().to_lowercase()
                },
                move |x| {
                    increment(&ele_invoked);
                    reverse(&x)
                },
                skip2,
            )
            .select(|g: Group| g.key().clone())
        }
    });

    assert_messages!(
        res.messages(),
        RT::on_next(220, s("foo")),
        RT::on_next(270, s("bar")),
        RT::on_next(350, s("baz")),
        RT::on_next(360, s("qux")),
        RT::on_next(470, s("foo")),
        RT::on_completed(570)
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 570));

    assert_eq!(12, key_invoked.get());
    assert_eq!(12, ele_invoked.get());
}

fn duration_selector_throws(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![RT::on_next(210, s("foo"))]);

    let ex = new_error("ex");

    let res = scheduler.start({
        let (xs, ex) = (xs.clone(), ex.clone());
        move || {
            let key_selector = |x: &String| Ok(x.clone());
            let duration_selector = move |_: Group| Err::<Rc<dyn IObservable<String>>, ObservableError>(ex.clone());
            match capacity {
                None => xs.as_observable().try_group_by_until(key_selector, duration_selector),
                Some(capacity) => xs.as_observable().try_group_by_until_capacity(key_selector, duration_selector, capacity),
            }
        }
    });

    // `OnError<IGroupedObservable<string, string>>(210, ex)`: a group has no equality, so the
    // messages are compared by hand.
    let messages = res.messages();
    assert_eq!(1, messages.len());
    assert_eq!(210, messages[0].time);
    assert!(matches!(&messages[0].value, Notification::OnError(error) if Rc::ptr_eq(error, &ex)));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 210));
}

fn null_keys_error(capacity: Option<usize>) {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(220, s("bar")),
        RT::on_next(240, s("foo")),
        RT::on_next(310, s("qux")),
        RT::on_next(470, s("baz")),
        RT::on_error(500, ex.clone()),
    ]);

    let null_group = scheduler.create_observer::<String>();
    let err: Slot<ObservableError> = slot();

    {
        let (xs, null_group, err) = (xs.clone(), null_group.clone(), err.clone());
        scheduler.schedule_absolute_action(200, move || {
            group_by_until_key(
                xs.as_observable(),
                capacity,
                |x: &String| if x.starts_with('b') { None } else { Some(x.to_uppercase()) },
                |_| Observable::never::<()>(),
            )
            .where_(|g| g.key().is_none())
            .subscribe_next_error(
                move |g| {
                    g.subscribe(null_group.clone());
                },
                move |ex_| set(&err, ex_),
            );
        });
    }
    scheduler.run();

    assert!(Rc::ptr_eq(&ex, &get(&err)));

    assert_messages!(null_group.messages(), RT::on_next(220, s("bar")), RT::on_next(470, s("baz")), RT::on_error(500, ex.clone()));

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn group_by_until_with_key_comparer() {
    with_key_comparer(None);
}

#[test]
fn group_by_until_outer_complete() {
    outer_complete(None);
}

#[test]
fn group_by_until_outer_error() {
    outer_error(None);
}

#[test]
fn group_by_until_outer_dispose() {
    outer_dispose(None);
}

#[test]
fn group_by_until_outer_key_throw() {
    outer_key_throw(None);
}

#[test]
fn group_by_until_outer_ele_throw() {
    outer_ele_throw(None);
}

#[test]
fn group_by_until_outer_comparer_equals_throw() {
    outer_comparer_equals_throw(None);
}

#[test]
fn group_by_until_outer_comparer_get_hash_code_throw() {
    outer_comparer_get_hash_code_throw(None);
}

#[test]
fn group_by_until_inner_complete() {
    inner_complete(None);
}

#[test]
fn group_by_until_inner_complete_all() {
    inner_complete_all(None);
}

#[test]
fn group_by_until_inner_error() {
    inner_error(None);
}

#[test]
fn group_by_until_inner_dispose() {
    inner_dispose(None);
}

#[test]
fn group_by_until_inner_key_throw() {
    inner_key_throw(None);
}

#[test]
fn group_by_until_inner_ele_throw() {
    inner_ele_throw(None);
}

#[test]
fn group_by_until_inner_comparer_equals_throw() {
    inner_comparer_equals_throw(None);
}

#[test]
fn group_by_until_inner_comparer_get_hash_code_throw() {
    inner_comparer_get_hash_code_throw(None);
}

#[test]
fn group_by_until_outer_independence() {
    outer_independence(None);
}

#[test]
fn group_by_until_inner_independence() {
    inner_independence(None);
}

#[test]
fn group_by_until_inner_multiple_independence() {
    inner_multiple_independence(None);
}

#[test]
fn group_by_until_inner_escape_complete() {
    inner_escape_complete(None);
}

#[test]
fn group_by_until_inner_escape_error() {
    inner_escape_error(None);
}

#[test]
fn group_by_until_inner_escape_dispose() {
    inner_escape_dispose(None);
}

#[test]
fn group_by_until_default() {
    default(None);
}

#[test]
fn group_by_until_duration_selector_throws() {
    duration_selector_throws(None);
}

#[test]
fn group_by_until_null_keys_error() {
    null_keys_error(None);
}

#[test]
fn group_by_until_capacity_with_key_comparer() {
    with_key_comparer(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_outer_complete() {
    outer_complete(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_outer_error() {
    outer_error(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_outer_dispose() {
    outer_dispose(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_outer_key_throw() {
    outer_key_throw(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_outer_ele_throw() {
    outer_ele_throw(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_outer_comparer_equals_throw() {
    outer_comparer_equals_throw(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_outer_comparer_get_hash_code_throw() {
    outer_comparer_get_hash_code_throw(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_complete() {
    inner_complete(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_complete_all() {
    inner_complete_all(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_error() {
    inner_error(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_dispose() {
    inner_dispose(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_key_throw() {
    inner_key_throw(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_ele_throw() {
    inner_ele_throw(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_comparer_equals_throw() {
    inner_comparer_equals_throw(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_comparer_get_hash_code_throw() {
    inner_comparer_get_hash_code_throw(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_outer_independence() {
    outer_independence(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_independence() {
    inner_independence(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_multiple_independence() {
    inner_multiple_independence(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_escape_complete() {
    inner_escape_complete(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_escape_error() {
    inner_escape_error(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_inner_escape_dispose() {
    inner_escape_dispose(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_default() {
    default(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_duration_selector_throws() {
    duration_selector_throws(Some(GROUP_BY_UNTIL_CAPACITY));
}

#[test]
fn group_by_until_capacity_null_keys_error() {
    null_keys_error(Some(GROUP_BY_UNTIL_CAPACITY));
}

// Not ported: GroupByUntil_ArgumentChecking - argument validation
// Not ported: GroupByUntil_NullKeys_Simple_Never - needs operator SelectMany (phase 3, later)
// Not ported: GroupByUntil_NullKeys_Simple_Expire1 - needs operator SelectMany (phase 3, later)
// Not ported: GroupByUntil_NullKeys_Simple_Expire2 - needs operator SelectMany (phase 3, later)
// Not ported: GroupByUntil_Capacity_ArgumentChecking - argument validation (and a negative capacity, which `usize` rules out)
// Not ported: GroupByUntil_Capacity_NullKeys_Simple_Never - needs operator SelectMany (phase 3, later)
// Not ported: GroupByUntil_Capacity_NullKeys_Simple_Expire1 - needs operator SelectMany (phase 3, later)
// Not ported: GroupByUntil_Capacity_NullKeys_Simple_Expire2 - needs operator SelectMany (phase 3, later)
