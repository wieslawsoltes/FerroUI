//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/CastTest.cs` of Rx.NET.
//! Adapted: the sequences of `object` are sequences of `Rc<dyn Any>`, and `Cast<B>` gives `Rc<B>`.
//! Rust has no class inheritance and a value casts to the type it was created with, so the classes
//! of upstream that derive from `B` (`D : B`) are `B` values that remember their class: `D::new(1)`
//! is a `B` of class `D`, and equality compares the class, as upstream's compares `GetType()`.
//! The classes are shared with `of_type_test`, as upstream's are defined in this file.

use ferroui_reactive::linq::InvalidCastException;
use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};
use std::any::Any;
use std::rc::Rc;

/// A value of any type, as upstream's `object`.
pub(crate) fn obj<X: 'static>(value: X) -> Rc<dyn Any> {
    Rc::new(value)
}

/// `class A`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct A {
    id: i32,
}

impl A {
    pub(crate) fn new(id: i32) -> Self {
        Self { id }
    }
}

/// `class B : A`, and the classes that derive from it (`D`), which the class names.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct B {
    id: i32,
    class: &'static str,
}

impl B {
    pub(crate) fn new(id: i32) -> Self {
        Self { id, class: "B" }
    }
}

/// `class C : A`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct C {
    id: i32,
}

impl C {
    pub(crate) fn new(id: i32) -> Self {
        Self { id }
    }
}

/// `class D : B`: a `B` of class `D`.
pub(crate) struct D;

impl D {
    pub(crate) fn new(id: i32) -> B {
        B { id, class: "D" }
    }
}

/// `class E`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct E {
    id: i32,
}

impl E {
    pub(crate) fn new(id: i32) -> Self {
        Self { id }
    }
}

#[test]
fn cast_complete() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, obj(B::new(0))),
        RT::on_next(220, obj(D::new(1))),
        RT::on_next(240, obj(B::new(2))),
        RT::on_next(270, obj(D::new(3))),
        RT::on_completed(300),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().cast::<B>());

    assert_messages!(
        res.messages(),
        RT::on_next(210, Rc::new(B::new(0))),
        RT::on_next(220, Rc::new(D::new(1))),
        RT::on_next(240, Rc::new(B::new(2))),
        RT::on_next(270, Rc::new(D::new(3))),
        RT::on_completed(300),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn cast_error() {
    let scheduler = TestScheduler::new();

    let ex = new_error("ex");

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, obj(B::new(0))),
        RT::on_next(220, obj(D::new(1))),
        RT::on_next(240, obj(B::new(2))),
        RT::on_next(270, obj(D::new(3))),
        RT::on_error(300, ex.clone()),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().cast::<B>());

    assert_messages!(
        res.messages(),
        RT::on_next(210, Rc::new(B::new(0))),
        RT::on_next(220, Rc::new(D::new(1))),
        RT::on_next(240, Rc::new(B::new(2))),
        RT::on_next(270, Rc::new(D::new(3))),
        RT::on_error(300, ex.clone()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 300));
}

#[test]
fn cast_dispose() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, obj(B::new(0))),
        RT::on_next(220, obj(D::new(1))),
        RT::on_next(240, obj(B::new(2))),
        RT::on_next(270, obj(D::new(3))),
        RT::on_completed(300),
    ]);

    let source = xs.clone();
    let res = scheduler.start_disposed(move || source.as_observable().cast::<B>(), 250);

    assert_messages!(
        res.messages(),
        RT::on_next(210, Rc::new(B::new(0))),
        RT::on_next(220, Rc::new(D::new(1))),
        RT::on_next(240, Rc::new(B::new(2))),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

#[test]
fn cast_not_valid() {
    let scheduler = TestScheduler::new();

    let xs = scheduler.create_hot_observable(vec![
        RT::on_next(210, obj(B::new(0))),
        RT::on_next(220, obj(D::new(1))),
        RT::on_next(240, obj(B::new(2))),
        RT::on_next(250, obj(A::new(-1))),
        RT::on_next(270, obj(D::new(3))),
        RT::on_completed(300),
    ]);

    let source = xs.clone();
    let res = scheduler.start(move || source.as_observable().cast::<B>());

    assert_messages!(
        res.messages(),
        RT::on_next(210, Rc::new(B::new(0))),
        RT::on_next(220, Rc::new(D::new(1))),
        RT::on_next(240, Rc::new(B::new(2))),
        RT::on_error_where(250, |e| e.is::<InvalidCastException>()),
    );

    assert_subscriptions!(xs.subscriptions(), RT::subscribe(200, 250));
}

// Not ported: Cast_ArgumentChecking - argument validation
