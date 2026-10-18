//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/AutoConnectTest.cs` of Rx.NET.
//! Adapted: the blocking `ToList().First()` is a subscription to `to_list` that records the list,
//! as the sequences end during the subscription (they run on the current thread); the default
//! arguments of `AutoConnect` are the methods `auto_connect`, `auto_connect_min` and
//! `auto_connect_with`.

use ferroui_reactive::linq::ConnectableObservableExt;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{IDisposable, IObservable, IObserver, Observable, ObservableExt};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `source.ToList().First()`.
fn to_list_first(source: &Rc<dyn IObservable<i32>>) -> Vec<i32> {
    let result: Rc<RefCell<Option<Vec<i32>>>> = Rc::new(RefCell::new(None));
    {
        let result = result.clone();
        source.to_list().subscribe_next(move |list| {
            let mut result = result.borrow_mut();
            if result.is_none() {
                *result = Some(list);
            }
        });
    }
    let list = result.borrow_mut().take().expect("the sequence has an element");
    list
}

#[test]
fn auto_connect_basic() {
    let called = Rc::new(Cell::new(0));

    let source = {
        let called = called.clone();
        Observable::defer(move || {
            called.set(called.get() + 1);
            Observable::range(1, 5)
        })
        .replay()
        .auto_connect()
    };

    assert_eq!(0, called.get());

    let list = to_list_first(&source);

    assert_eq!(1, called.get());
    assert_eq!(vec![1, 2, 3, 4, 5], list);

    let list = to_list_first(&source);

    assert_eq!(1, called.get());
    assert_eq!(vec![1, 2, 3, 4, 5], list);
}

#[test]
fn auto_connect_immediately() {
    let called = Rc::new(Cell::new(0));

    let source = {
        let called = called.clone();
        Observable::defer(move || {
            called.set(called.get() + 1);
            Observable::range(1, 5)
        })
        .replay()
        .auto_connect_min(0)
    };

    assert_eq!(1, called.get());

    let list = to_list_first(&source);

    assert_eq!(1, called.get());
    assert_eq!(vec![1, 2, 3, 4, 5], list);

    let list = to_list_first(&source);

    assert_eq!(1, called.get());
    assert_eq!(vec![1, 2, 3, 4, 5], list);
}

#[test]
fn auto_connect_two_consumers() {
    let called = Rc::new(Cell::new(0));

    let source = {
        let called = called.clone();
        Observable::defer(move || {
            called.set(called.get() + 1);
            Observable::range(1, 5)
        })
        .replay()
        .auto_connect_min(2)
    };

    assert_eq!(0, called.get());

    let list0 = Rc::new(RefCell::new(Vec::new()));

    {
        let list0 = list0.clone();
        source.subscribe_next(move |v| list0.borrow_mut().push(v));
    }

    assert_eq!(0, called.get());
    assert_eq!(0, list0.borrow().len());

    let list = to_list_first(&source);

    assert_eq!(1, called.get());
    assert_eq!(vec![1, 2, 3, 4, 5], list);

    assert_eq!(vec![1, 2, 3, 4, 5], *list0.borrow());

    let list = to_list_first(&source);

    assert_eq!(1, called.get());
    assert_eq!(vec![1, 2, 3, 4, 5], list);
}

#[test]
fn auto_connect_dispose() {
    let subject = Subject::<i32>::new();

    let disposable: Rc<RefCell<Option<Rc<dyn IDisposable>>>> = Rc::new(RefCell::new(None));

    let source = {
        let disposable = disposable.clone();
        subject.as_observable().replay().auto_connect_with(1, move |d| *disposable.borrow_mut() = Some(d))
    };

    assert!(disposable.borrow().is_none());

    let list = Rc::new(RefCell::new(Vec::new()));

    {
        let list = list.clone();
        source.subscribe_next(move |v| list.borrow_mut().push(v));
    }

    assert!(disposable.borrow().is_some());

    subject.on_next(1);
    subject.on_next(2);
    subject.on_next(3);

    let connection = disposable.borrow().clone().expect("the connection is handed over");
    connection.dispose();

    subject.on_next(4);
    subject.on_next(5);

    assert_eq!(vec![1, 2, 3], *list.borrow());
}
