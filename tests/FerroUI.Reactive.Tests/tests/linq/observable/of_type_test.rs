//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/OfTypeTest.cs` of Rx.NET.
//! Adapted: the sequences of `object` are sequences of `Rc<dyn Any>`, and `OfType<B>` gives
//! `Rc<B>`; the classes are those of `cast_test` (a `D` is a `B` of class `D`, as Rust has no class
//! inheritance).

use super::cast_test::{obj, A, B, C, D, E};
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::any::Any;
use std::rc::Rc;

/// The values of upstream's tests, ending at 350 with `end`.
fn values(end: ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<Rc<dyn Any>>>) -> Vec<ferroui_reactive_testing::Recorded<ferroui_reactive::Notification<Rc<dyn Any>>>> {
    vec![
        RT::on_next(210, obj(B::new(0))),
        RT::on_next(220, obj(A::new(1))),
        RT::on_next(230, obj(E::new(2))),
        RT::on_next(240, obj(D::new(3))),
        RT::on_next(250, obj(C::new(4))),
        RT::on_next(260, obj(B::new(5))),
        RT::on_next(270, obj(B::new(6))),
        RT::on_next(280, obj(D::new(7))),
        RT::on_next(290, obj(A::new(8))),
        RT::on_next(300, obj(E::new(9))),
        RT::on_next(310, obj(3)),
        RT::on_next(320, obj("foo")),
        RT::on_next(330, obj(true)),
        RT::on_next(340, obj(B::new(10))),
        end,
    ]
}

#[test]
fn of_type_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(values(RT::on_completed(350)));

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().of_type::<B>());

    assert_messages!(
        res.messages(),
        RT::on_next(210, Rc::new(B::new(0))),
        RT::on_next(240, Rc::new(D::new(3))),
        RT::on_next(260, Rc::new(B::new(5))),
        RT::on_next(270, Rc::new(B::new(6))),
        RT::on_next(280, Rc::new(D::new(7))),
        RT::on_next(340, Rc::new(B::new(10))),
        RT::on_completed(350),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 350));
}

#[test]
fn of_type_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(values(RT::on_error(350, ex.clone())));

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().of_type::<B>());

    assert_messages!(
        res.messages(),
        RT::on_next(210, Rc::new(B::new(0))),
        RT::on_next(240, Rc::new(D::new(3))),
        RT::on_next(260, Rc::new(B::new(5))),
        RT::on_next(270, Rc::new(B::new(6))),
        RT::on_next(280, Rc::new(D::new(7))),
        RT::on_next(340, Rc::new(B::new(10))),
        RT::on_error(350, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 350));
}

#[test]
fn of_type_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(values(RT::on_error(350, new_error("ex"))));

    let source = xs.clone();
    let res = scheduler.start_disposed(move || source.as_observable().of_type::<B>(), 275);

    assert_messages!(
        res.messages(),
        RT::on_next(210, Rc::new(B::new(0))),
        RT::on_next(240, Rc::new(D::new(3))),
        RT::on_next(260, Rc::new(B::new(5))),
        RT::on_next(270, Rc::new(B::new(6))),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 275));
}

// Not ported: OfType_ArgumentChecking - argument validation
// Not ported: OfType_NullableSourceOfTypeNonNull - needs a source of nullable `A?` whose values of the derived classes `B`, `C` and `D` are `A`s; Rust has no null references or class inheritance, and `of_type` matches the exact type of a value
