//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ToLookupTest.cs` of Rx.NET.
//! Adapted: the lookup is `Lookup<K, E>`, whose indexer is `get`; `EqualityComparer<int>.Default`
//! is the `Eq` and `Hash` of the key; selectors that throw are the `try_` forms returning
//! `Err(ex)`; the blocking `First()` of the tests of the default overloads is a helper that takes
//! the first value the sequence gives while it is subscribed to. The non-generic enumerators of
//! `ToLookup_Groups_2` are the same iterators as the generic ones.

use ferroui_reactive::linq::Lookup;
use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::rc::Rc;

/// The first value of a sequence that gives its values while it is subscribed to, as
/// `Observable.Range` on the current-thread scheduler does (the blocking `First()`).
fn first<T: 'static>(source: Rc<dyn IObservable<T>>) -> T {
    let slot: Rc<RefCell<Option<T>>> = Rc::new(RefCell::new(None));
    let target = slot.clone();
    source.subscribe_next(move |value| {
        target.borrow_mut().get_or_insert(value);
    });
    let value = slot.borrow_mut().take();
    value.expect("the sequence has a value")
}

#[test]
fn to_lookup_completed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(220, 2),
        RT::on_next(330, 3),
        RT::on_next(440, 4),
        RT::on_next(550, 5),
        RT::on_completed::<i32>(660),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().to_lookup_with(|x| x % 2, |x| x * 2)
    });

    assert_messages!(
        res.messages(),
        RT::on_next_where(660, |d: &Lookup<i32, i32>| d.count() == 2 && d.get(&0) == [4, 8] && d.get(&1) == [6, 10]),
        RT::on_completed::<Lookup<i32, i32>>(660),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 660));
}

#[test]
fn to_lookup_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(220, 2),
        RT::on_next(330, 3),
        RT::on_next(440, 4),
        RT::on_next(550, 5),
        RT::on_error::<i32>(660, ex.clone()),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().to_lookup_with(|x| x % 2, |x| x * 2)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Lookup<i32, i32>>(660, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 660));
}

#[test]
fn to_lookup_key_selector_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(220, 2),
        RT::on_next(330, 3),
        RT::on_next(440, 4),
        RT::on_next(550, 5),
        RT::on_completed::<i32>(600),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_to_lookup_with(move |x| if *x < 4 { Ok(x * 2) } else { Err(ex.clone()) }, |x| Ok(x * 4))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Lookup<i32, i32>>(440, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 440));
}

#[test]
fn to_lookup_element_selector_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(220, 2),
        RT::on_next(330, 3),
        RT::on_next(440, 4),
        RT::on_next(550, 5),
        RT::on_completed::<i32>(600),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        let ex = ex.clone();
        move || xs.as_observable().try_to_lookup_with(|x| Ok(x * 2), move |x| if x < 4 { Ok(x * 4) } else { Err(ex.clone()) })
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<Lookup<i32, i32>>(440, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 440));
}

#[test]
fn to_lookup_disposed() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(110, 1),
        RT::on_next(220, 2),
        RT::on_next(330, 3),
        RT::on_next(440, 4),
        RT::on_next(550, 5),
    ]);

    let res = scheduler.start({
        let xs = xs.clone();
        move || xs.as_observable().to_lookup_with(|x| x % 2, |x| x * 2)
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn to_lookup_default() {
    let d1 = first(Observable::range(1, 10).to_lookup(|x| (x % 2).to_string()));
    assert_eq!(d1.get("0"), [2, 4, 6, 8, 10]);

    let d2 = first(Observable::range(1, 10).to_lookup_with(|x| (x % 2).to_string(), |x| x * 2));
    assert_eq!(d2.get("1"), [2, 6, 10, 14, 18]);

    let d3 = first(Observable::range(1, 10).to_lookup(|x| (x % 2).to_string()));
    assert_eq!(d3.get("0"), [2, 4, 6, 8, 10]);
}

#[test]
fn to_lookup_contains() {
    let d1 = first(Observable::range(1, 10).to_lookup(|x| (x % 2).to_string()));
    assert!(d1.contains("1"));
    assert!(!d1.contains("2"));
}

#[test]
fn to_lookup_groups() {
    let d1 = first(Observable::range(1, 10).to_lookup(|x| (x % 2).to_string()));

    for g in &d1 {
        if g.key() == "0" {
            assert_eq!(g.elements(), [2, 4, 6, 8, 10]);
        } else if g.key() == "1" {
            assert_eq!(g.elements(), [1, 3, 5, 7, 9]);
        } else {
            panic!("Unknown group.");
        }
    }
}

#[test]
fn to_lookup_groups_2() {
    let d1 = first(Observable::range(1, 10).to_lookup(|x| (x % 2).to_string()));

    for g in d1.iter() {
        if g.key() == "0" {
            let mut l = Vec::new();
            for v in g {
                l.push(*v);
            }

            assert_eq!(l, [2, 4, 6, 8, 10]);
        } else if g.key() == "1" {
            let mut l = Vec::new();
            for v in g {
                l.push(*v);
            }

            assert_eq!(l, [1, 3, 5, 7, 9]);
        } else {
            panic!("Unknown group.");
        }
    }
}

#[test]
fn to_lookup_indexer_for_invalid_key() {
    let d1 = first(Observable::range(1, 10).to_lookup(|x| (x % 2).to_string()));

    let values = d1.get("2");
    assert!(values.is_empty());
}

// Not ported: ToLookup_ArgumentChecking - argument validation
// Not ported: ToLookup_Hides_Internal_List - the elements of a key are a shared slice (`&[E]`), which the type system keeps from being modified; there is nothing to check at run time
