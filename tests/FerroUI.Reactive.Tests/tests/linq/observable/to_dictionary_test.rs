//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ToDictionaryTest.cs` of Rx.NET.
//! Adapted: the dictionary is a `HashMap`; `EqualityComparer<int>.Default` is the `Eq` and `Hash`
//! of the key; selectors that throw are the `try_` forms returning `Err(ex)`; the blocking
//! `First()` of the tests of the default overloads is a helper that takes the first value the
//! sequence gives while it is subscribed to. A duplicate key ends with
//! `SequenceError::DuplicateKey`.

use ferroui_reactive::{IObservable, Observable, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::RefCell;
use std::collections::HashMap;
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
fn to_dictionary_completed() {
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
        move || xs.as_observable().to_dictionary_with(|x| x * 2, |x| x * 4)
    });

    assert_messages!(
        res.messages(),
        RT::on_next_where(660, |d: &HashMap<i32, i32>| d.len() == 4 && d[&4] == 8 && d[&6] == 12 && d[&8] == 16 && d[&10] == 20),
        RT::on_completed::<HashMap<i32, i32>>(660),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 660));
}

#[test]
fn to_dictionary_error() {
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
        move || xs.as_observable().to_dictionary_with(|x| x * 2, |x| x * 4)
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<HashMap<i32, i32>>(660, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 660));
}

#[test]
fn to_dictionary_key_selector_throws() {
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
        move || xs.as_observable().try_to_dictionary_with(move |x| if *x < 4 { Ok(x * 2) } else { Err(ex.clone()) }, |x| Ok(x * 4))
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<HashMap<i32, i32>>(440, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 440));
}

#[test]
fn to_dictionary_element_selector_throws() {
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
        move || xs.as_observable().try_to_dictionary_with(|x| Ok(x * 2), move |x| if x < 4 { Ok(x * 4) } else { Err(ex.clone()) })
    });

    assert_messages!(
        res.messages(),
        RT::on_error::<HashMap<i32, i32>>(440, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 440));
}

#[test]
fn to_dictionary_disposed() {
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
        move || xs.as_observable().to_dictionary_with(|x| x * 2, |x| x * 4)
    });

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 1000));
}

#[test]
fn to_dictionary_multiple_add() {
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
        move || xs.as_observable().to_dictionary_with(|x| x % 2, |x| x * 2)
    });

    assert_messages!(
        res.messages(),
        RT::on_error_where::<HashMap<i32, i32>>(440, |_| true),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 440));
}

#[test]
fn to_dictionary_default() {
    let d1 = first(Observable::range(1, 10).to_dictionary(|x| x.to_string()));
    assert_eq!(d1["7"], 7);

    let d2 = first(Observable::range(1, 10).to_dictionary_with(|x| x.to_string(), |x| x * 2));
    assert_eq!(d2["9"], 18);

    let d3 = first(Observable::range(1, 10).to_dictionary(|x| x.to_string()));
    assert_eq!(d3["7"], 7);
}

// Not ported: ToDictionary_ArgumentChecking - argument validation
