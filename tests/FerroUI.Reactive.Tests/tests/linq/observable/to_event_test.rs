//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ToEventTest.cs` of Rx.NET.
//! Adapted: the event of `ToEvent` is an `IEventSource<T>`, whose `add` (`OnNext +=`) returns the
//! token that `remove` (`OnNext -=`) takes; `Observable.FromEvent<int>(add, remove)` is
//! `Observable::from_event`; the `using` block is an explicit disposal.

use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{IObserver, Observable, ObservableExt};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn to_event_unit() {
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

    evt.remove(token);

    src.on_next(());
    assert_eq!(2, num.get());
}

#[test]
fn to_event_non_unit() {
    let src = Subject::<i32>::new();
    let evt = src.as_observable().to_event();

    let lst = Rc::new(RefCell::new(Vec::new()));
    let l = lst.clone();
    let hnd: Rc<dyn Fn(i32)> = Rc::new(move |e| l.borrow_mut().push(e));

    let token = evt.add(hnd);

    src.on_next(1);
    src.on_next(2);

    evt.remove(token);

    src.on_next(3);

    assert_eq!(vec![1, 2], *lst.borrow());
}

#[test]
fn to_event_from_event() {
    let src = Subject::<i32>::new();
    let evt = src.as_observable().to_event();

    let (e1, e2) = (evt.clone(), evt.clone());
    let res = Observable::from_event(move |h: Rc<dyn Fn(i32)>| e1.add(h), move |token| {
        e2.remove(token);
    });

    let lst = Rc::new(RefCell::new(Vec::new()));
    let l = lst.clone();
    let d = res.subscribe_next_completed(move |e| l.borrow_mut().push(e), || panic!("no completion is expected"));
    src.on_next(1);
    src.on_next(2);
    d.dispose();

    src.on_next(3);

    assert_eq!(vec![1, 2], *lst.borrow());
}

// Not ported: ToEvent_ArgumentChecks - argument validation
