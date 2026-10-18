//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/SkipWhileTest.cs` of Rx.NET.
//! The operators are applied to the handle of the test observable (`as_observable`); a predicate that throws is the `try_` form returning `Err`.

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn skip_while_complete_before() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_completed::<i32>(330),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_completed::<i32>(600),
    ]);

    let invoked = Rc::new(Cell::new(0));

    let res = {
        let xs = xs.clone();
        let invoked = invoked.clone();
        scheduler.start(move || xs.as_observable().skip_while(move |x: &i32| {
            invoked.set(invoked.get() + 1);
            is_prime(*x)
        }))
    };

    assert_messages!(res.messages(),
        RT::on_completed::<i32>(330),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 330),
    );

    assert_eq!(invoked.get(), 4);
}

#[test]
fn skip_while_complete_after() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_completed::<i32>(600),
    ]);

    let invoked = Rc::new(Cell::new(0));

    let res = {
        let xs = xs.clone();
        let invoked = invoked.clone();
        scheduler.start(move || xs.as_observable().skip_while(move |x: &i32| {
            invoked.set(invoked.get() + 1);
            is_prime(*x)
        }))
    };

    assert_messages!(res.messages(),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_completed::<i32>(600),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 600),
    );

    assert_eq!(invoked.get(), 6);
}

#[test]
fn skip_while_error_before() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_error::<i32>(270, ex.clone()),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
    ]);

    let invoked = Rc::new(Cell::new(0));

    let res = {
        let xs = xs.clone();
        let invoked = invoked.clone();
        scheduler.start(move || xs.as_observable().skip_while(move |x: &i32| {
            invoked.set(invoked.get() + 1);
            is_prime(*x)
        }))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(270, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 270),
    );

    assert_eq!(invoked.get(), 2);
}

#[test]
fn skip_while_error_after() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_error::<i32>(600, ex.clone()),
    ]);

    let invoked = Rc::new(Cell::new(0));

    let res = {
        let xs = xs.clone();
        let invoked = invoked.clone();
        scheduler.start(move || xs.as_observable().skip_while(move |x: &i32| {
            invoked.set(invoked.get() + 1);
            is_prime(*x)
        }))
    };

    assert_messages!(res.messages(),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_error::<i32>(600, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 600),
    );

    assert_eq!(invoked.get(), 6);
}

#[test]
fn skip_while_dispose_before() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_completed::<i32>(600),
    ]);

    let invoked = Rc::new(Cell::new(0));

    let res = {
        let xs = xs.clone();
        let invoked = invoked.clone();
        scheduler.start_disposed(move || xs.as_observable().skip_while(move |x: &i32| {
            invoked.set(invoked.get() + 1);
            is_prime(*x)
        }), 300)
    };

    assert_messages!(res.messages());

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 300),
    );

    assert_eq!(invoked.get(), 3);
}

#[test]
fn skip_while_dispose_after() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_completed::<i32>(600),
    ]);

    let invoked = Rc::new(Cell::new(0));

    let res = {
        let xs = xs.clone();
        let invoked = invoked.clone();
        scheduler.start_disposed(move || xs.as_observable().skip_while(move |x: &i32| {
            invoked.set(invoked.get() + 1);
            is_prime(*x)
        }), 470)
    };

    assert_messages!(res.messages(),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 470),
    );

    assert_eq!(invoked.get(), 6);
}

#[test]
fn skip_while_zero() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(205, 100),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_completed::<i32>(600),
    ]);

    let invoked = Rc::new(Cell::new(0));

    let res = {
        let xs = xs.clone();
        let invoked = invoked.clone();
        scheduler.start(move || xs.as_observable().skip_while(move |x: &i32| {
            invoked.set(invoked.get() + 1);
            is_prime(*x)
        }))
    };

    assert_messages!(res.messages(),
        RT::on_next(205, 100),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_completed::<i32>(600),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 600),
    );

    assert_eq!(invoked.get(), 1);
}

#[test]
fn skip_while_throw() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_completed::<i32>(600),
    ]);

    let invoked = Rc::new(Cell::new(0));
    let ex = new_error("ex");

    let res = {
        let xs = xs.clone();
        let invoked = invoked.clone();
        let ex = ex.clone();
        scheduler.start(move || xs.as_observable().try_skip_while(move |x: &i32| {
            invoked.set(invoked.get() + 1);
            if invoked.get() == 3 {
                return Err(ex.clone());
            }
            Ok(is_prime(*x))
        }))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(290, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 290),
    );

    assert_eq!(invoked.get(), 3);
}

#[test]
fn skip_while_index() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(205, 100),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_completed::<i32>(600),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_while_with_index(|_, i| i < 5))
    };

    assert_messages!(res.messages(),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_next(410, 17),
        RT::on_next(450, 8),
        RT::on_next(500, 23),
        RT::on_completed::<i32>(600),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 600),
    );
}

#[test]
fn skip_while_index_throw() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(205, 100),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_error::<i32>(400, ex.clone()),
    ]);

    let res = {
        let xs = xs.clone();
        scheduler.start(move || xs.as_observable().skip_while_with_index(|_, i| i < 5))
    };

    assert_messages!(res.messages(),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_error::<i32>(400, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 400),
    );
}

#[test]
fn skip_while_index_selector_throws() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(90, -1),
        RT::on_next(110, -1),
        RT::on_next(205, 100),
        RT::on_next(210, 2),
        RT::on_next(260, 5),
        RT::on_next(290, 13),
        RT::on_next(320, 3),
        RT::on_next(350, 7),
        RT::on_next(390, 4),
        RT::on_completed::<i32>(400),
    ]);

    let res = {
        let xs = xs.clone();
        let ex = ex.clone();
        scheduler.start(move || xs.as_observable().try_skip_while_with_index(move |_, i| if i < 5 { Ok(true) } else { Err(ex.clone()) }))
    };

    assert_messages!(res.messages(),
        RT::on_error::<i32>(350, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(),
        RT::subscribe(200, 350),
    );
}

fn is_prime(i: i32) -> bool {
    if i <= 1 {
        return false;
    }
    let max = (i as f64).sqrt() as i32;
    for j in 2..=max {
        if i % j == 0 {
            return false;
        }
    }
    true
}

// Not ported: SkipWhile_ArgumentChecking - argument validation
