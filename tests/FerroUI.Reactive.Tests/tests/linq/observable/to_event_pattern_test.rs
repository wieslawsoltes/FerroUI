//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ToEventPatternTest.cs` of Rx.NET.
//! Adapted: the events of `ToEvent` and `ToEventPattern` are an `IEventSource<T>` and an
//! `IEventPatternSource<S, A>`, whose `add` (`OnNext +=`) returns the token that `remove`
//! (`OnNext -=`) takes; a handler added twice has two tokens, and `-=` of upstream removes the
//! last subscription of the handler, the last token. The table of subscriptions that upstream reads
//! through reflection (`GetSubscriptionTable`) is checked through `remove`, which is false for a
//! token without a subscription. The sender `object` (compared by reference) is an `Rc<Object>`,
//! or the `Sender` identity of the object that raises a FerroUI event; `EventArgs<T>` is
//! `ValueEventArgs<T>`. An error of the source panics where upstream rethrows it.

use ferroui_reactive::linq::EventPattern;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{IObserver, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_throws, new_error};
use ferroui_system::events::Event;
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// An `object` of the tests, compared by reference.
struct Object;

/// `EventArgs<T>`: arguments with a value.
#[derive(Clone, Debug, PartialEq)]
struct ValueEventArgs<T> {
    value: T,
}

/// The identity of the object that raises an event (the sender of upstream, compared by
/// reference).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sender(usize);

impl Sender {
    fn of<T>(object: &T) -> Self {
        Self(object as *const T as usize)
    }
}

/// `EventSrc`: the event `E` of a sender and `EventArgs<string>`.
#[derive(Default)]
struct EventSrc {
    e: Event<(Sender, ValueEventArgs<String>)>,
}

impl EventSrc {
    fn on(&self, s: &str) {
        self.e.raise(&(Sender::of(self), ValueEventArgs { value: s.to_owned() }));
    }
}

type Pattern = EventPattern<Rc<Object>, ValueEventArgs<i32>>;

#[test]
fn to_event_pattern_i_event() {
    let src = Subject::<Pattern>::new();
    let evt = src.as_observable().to_event_pattern();

    let snd = Rc::new(Object);

    let lst = Rc::new(RefCell::new(Vec::new()));
    let (s, l) = (snd.clone(), lst.clone());
    let hnd: Rc<dyn Fn(Rc<Object>, ValueEventArgs<i32>)> = Rc::new(move |sender, e| {
        assert!(Rc::ptr_eq(&s, &sender));
        l.borrow_mut().push(e.value);
    });

    let token = evt.add(hnd);

    src.on_next(EventPattern::new(snd.clone(), ValueEventArgs { value: 42 }));
    src.on_next(EventPattern::new(snd.clone(), ValueEventArgs { value: 43 }));

    evt.remove(token);

    src.on_next(EventPattern::new(snd.clone(), ValueEventArgs { value: 44 }));

    assert_eq!(vec![42, 43], *lst.borrow());
}

#[test]
fn to_event_pattern_i_event_fails() {
    let src = Subject::<Pattern>::new();
    let evt = src.as_observable().to_event_pattern();

    let snd = Rc::new(Object);

    let lst = Rc::new(RefCell::new(Vec::new()));
    let (s, l) = (snd.clone(), lst.clone());
    let hnd: Rc<dyn Fn(Rc<Object>, ValueEventArgs<i32>)> = Rc::new(move |sender, e| {
        assert!(Rc::ptr_eq(&s, &sender));
        l.borrow_mut().push(e.value);
    });

    let _token = evt.add(hnd);

    src.on_next(EventPattern::new(snd.clone(), ValueEventArgs { value: 42 }));
    src.on_next(EventPattern::new(snd.clone(), ValueEventArgs { value: 43 }));

    let ex = new_error("ex");

    let source = src.clone();
    assert_throws(move || source.on_error(ex));

    assert_eq!(vec![42, 43], *lst.borrow());
}

#[test]
fn to_event_pattern_i_event_completes() {
    let src = Subject::<Pattern>::new();
    let evt = src.as_observable().to_event_pattern();

    let snd = Rc::new(Object);

    let lst = Rc::new(RefCell::new(Vec::new()));
    let (s, l) = (snd.clone(), lst.clone());
    let hnd: Rc<dyn Fn(Rc<Object>, ValueEventArgs<i32>)> = Rc::new(move |sender, e| {
        assert!(Rc::ptr_eq(&s, &sender));
        l.borrow_mut().push(e.value);
    });

    let _token = evt.add(hnd);

    src.on_next(EventPattern::new(snd.clone(), ValueEventArgs { value: 42 }));
    src.on_next(EventPattern::new(snd.clone(), ValueEventArgs { value: 43 }));

    src.on_completed();

    assert_eq!(vec![42, 43], *lst.borrow());
}

#[test]
fn from_event_pattern_to_event_pattern() {
    let src = Rc::new(EventSrc::default());
    let (s1, s2) = (src.clone(), src.clone());
    let evt = Observable::from_event_pattern_conversion(
        |h: Rc<dyn Fn(Sender, ValueEventArgs<String>)>| h,
        move |h: Rc<dyn Fn(Sender, ValueEventArgs<String>)>| {
            s1.e.add(Rc::new(move |(sender, e): &(Sender, ValueEventArgs<String>)| h(*sender, e.clone())))
        },
        move |token| {
            s2.e.remove(token);
        },
    );

    let res = evt.to_event_pattern();

    let lst = Rc::new(RefCell::new(Vec::new()));
    let (sender, l) = (Sender::of(&*src), lst.clone());
    let hnd: Rc<dyn Fn(Sender, ValueEventArgs<String>)> = Rc::new(move |s, e| {
        assert_eq!(sender, s);
        l.borrow_mut().push(e.value);
    });

    src.on("bar");

    let token = res.add(hnd);

    src.on("foo");
    src.on("baz");

    res.remove(token);

    src.on("qux");

    assert_eq!(vec!["foo".to_owned(), "baz".to_owned()], *lst.borrow());
}

#[test]
fn to_event_duplicate_handlers() {
    let src = Subject::<()>::new();
    let evt = src.as_observable().to_event();

    let num = Rc::new(Cell::new(0));
    let n = num.clone();
    let hnd: Rc<dyn Fn(())> = Rc::new(move |_| n.set(n.get() + 1));

    let first = evt.add(hnd.clone());

    assert_eq!(0, num.get());

    src.on_next(());
    assert_eq!(1, num.get());

    let second = evt.add(hnd.clone());

    src.on_next(());
    assert_eq!(3, num.get());

    evt.remove(second);

    src.on_next(());
    assert_eq!(4, num.get());

    evt.remove(first);

    src.on_next(());
    assert_eq!(4, num.get());
}

#[test]
fn to_event_source_completes() {
    let src = Subject::<()>::new();
    let evt = src.as_observable().to_event();

    let num = Rc::new(Cell::new(0));
    let n = num.clone();
    let hnd: Rc<dyn Fn(())> = Rc::new(move |_| n.set(n.get() + 1));

    let token = evt.add(hnd);

    assert_eq!(0, num.get());

    src.on_next(());
    assert_eq!(1, num.get());

    src.on_next(());
    assert_eq!(2, num.get());

    src.on_completed();
    assert_eq!(2, num.get());

    // The table of subscriptions is empty.
    assert!(!evt.remove(token));
}

#[test]
fn to_event_source_fails() {
    let src = Subject::<()>::new();
    let evt = src.as_observable().to_event();

    let num = Rc::new(Cell::new(0));
    let n = num.clone();
    let hnd: Rc<dyn Fn(())> = Rc::new(move |_| n.set(n.get() + 1));

    let token = evt.add(hnd);

    assert_eq!(0, num.get());

    src.on_next(());
    assert_eq!(1, num.get());

    src.on_next(());
    assert_eq!(2, num.get());

    let ex = new_error("ex");

    let source = src.clone();
    assert_throws(move || source.on_error(ex));

    // The table of subscriptions is empty.
    assert!(!evt.remove(token));
}

#[test]
fn to_event_done_immediately() {
    let src = Observable::empty::<()>();
    let evt = src.to_event();

    let num = Rc::new(Cell::new(0));
    let n = num.clone();
    let hnd: Rc<dyn Fn(())> = Rc::new(move |_| n.set(n.get() + 1));

    for _ in 0..2 {
        let token = evt.add(hnd.clone());

        assert_eq!(0, num.get());

        // The table of subscriptions is empty.
        assert!(!evt.remove(token));
    }
}

#[test]
fn to_event_unbalanced_handlers() {
    let src = Subject::<()>::new();
    let evt = src.as_observable().to_event();

    let num = Rc::new(Cell::new(0));
    let n = num.clone();
    let hnd: Rc<dyn Fn(())> = Rc::new(move |_| n.set(n.get() + 1));

    let first = evt.add(hnd.clone());
    assert_eq!(0, num.get());

    evt.remove(first);
    assert_eq!(0, num.get());

    evt.remove(first);
    assert_eq!(0, num.get());

    let second = evt.add(hnd.clone());
    assert_eq!(0, num.get());

    src.on_next(());
    assert_eq!(1, num.get());

    src.on_next(());
    assert_eq!(2, num.get());

    evt.remove(second);
    assert_eq!(2, num.get());

    src.on_next(());
    assert_eq!(2, num.get());
}

fn hash_of<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// `EventArgs.Empty`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct EventArgs;

#[test]
fn event_pattern_equality() {
    let e1 = EventPattern::new("Bart", EventArgs);
    let e2 = EventPattern::new("Bart", EventArgs);

    assert!(e1.eq(&e1));
    assert!(e1.eq(&e2));
    assert!(e2.eq(&e1));
    assert!(e1 == e2);
    assert!(!(e1 != e2));
    assert!(hash_of(&e1) == hash_of(&e2));

    // `e1.Equals(null)`, `e1.Equals("xy")` and `e1 == null` compare with values of other types,
    // which the types of Rust rule out.
}

/// `MyEventArgs`: arguments compared by reference.
#[derive(Clone, Debug)]
struct MyEventArgs(Rc<()>);

impl MyEventArgs {
    fn new() -> Self {
        Self(Rc::new(()))
    }
}

impl PartialEq for MyEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for MyEventArgs {}

impl Hash for MyEventArgs {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.0).hash(state);
    }
}

#[test]
fn event_pattern_inequality() {
    let a1 = MyEventArgs::new();
    let a2 = MyEventArgs::new();

    let e1 = EventPattern::new("Bart", a1.clone());
    let e2 = EventPattern::new("John", a1.clone());
    let e3 = EventPattern::new("Bart", a2.clone());

    assert!(!e1.eq(&e2));
    assert!(!e2.eq(&e1));
    assert!(!(e1 == e2));
    assert!(e1 != e2);
    assert!(hash_of(&e1) != hash_of(&e2));

    assert!(!e1.eq(&e3));
    assert!(!e3.eq(&e1));
    assert!(!(e1 == e3));
    assert!(e1 != e3);
    assert!(hash_of(&e1) != hash_of(&e3));
}

// Not ported: ToEventPattern_ArgumentChecking - argument validation
// Not ported (in part): EventPattern_Equality - the comparisons with `null` and with a string (`Equals(null)`, `Equals("xy")`, `== null`): the types of Rust rule them out
