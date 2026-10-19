//! `SharedDeliveryQueueFixture` (`DynamicData.Tests/Internal/SharedDeliveryQueueFixture.cs`).
//!
//! Upstream parks a delivery on one thread and enqueues from another. On one thread, a scope that
//! is held open stands for the enqueue of another thread: what it enqueues takes its place in the
//! order received, and is delivered by the drain that runs (or the next one), not inline. The tests
//! that rest on that are ported so; the values of upstream's randomizer are constants.

use super::{same_error, test_error};
use crate::internal::{DeliverySubQueue, SharedDeliveryQueue, SubQueueScopedAccess};
use ferroui_system::{IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An observer of values with a function, which records the error and the completion
/// (`TestObserver<T>`).
struct TestObserver<T> {
    on_next: Box<dyn Fn(T)>,
    error: RefCell<Option<ObservableError>>,
    is_completed: Cell<bool>,
}

impl<T: 'static> TestObserver<T> {
    fn new(on_next: impl Fn(T) + 'static) -> Rc<Self> {
        Rc::new(Self { on_next: Box::new(on_next), error: RefCell::new(None), is_completed: Cell::new(false) })
    }
}

impl<T> IObserver<T> for TestObserver<T> {
    fn on_next(&self, value: T) {
        (self.on_next)(value);
    }

    fn on_error(&self, error: ObservableError) {
        *self.error.borrow_mut() = Some(error);
    }

    fn on_completed(&self) {
        self.is_completed.set(true);
    }
}

type Log = Rc<RefCell<Vec<String>>>;

fn record(delivered: &Log, entry: String) {
    delivered.borrow_mut().push(entry);
}

#[test]
fn single_source_delivers_items() {
    let queue = SharedDeliveryQueue::new();
    let delivered = Rc::new(RefCell::new(Vec::new()));
    let log = delivered.clone();
    let observer = TestObserver::new(move |value: i32| log.borrow_mut().push(value));
    let sub = queue.create_queue(observer);

    {
        let scope = sub.acquire();
        scope.enqueue_next(1);
        scope.enqueue_next(2);
        scope.enqueue_next(3);
    }

    assert_eq!(*delivered.borrow(), vec![1, 2, 3]);
}

#[test]
fn multiple_sources_serialize_delivery() {
    let queue = SharedDeliveryQueue::new();
    let delivered: Log = Rc::new(RefCell::new(Vec::new()));
    let (log1, log2) = (delivered.clone(), delivered.clone());
    let sub1 = queue.create_queue(TestObserver::new(move |i: i32| record(&log1, format!("int:{i}"))));
    let sub2 = queue.create_queue(TestObserver::new(move |s: String| record(&log2, format!("str:{s}"))));

    {
        let scope1 = sub1.acquire();
        scope1.enqueue_next(1);
    }
    {
        let scope2 = sub2.acquire();
        scope2.enqueue_next("hello".to_owned());
    }

    assert_eq!(*delivered.borrow(), vec!["int:1", "str:hello"]);
}

#[test]
fn error_terminates_all_sub_queues() {
    let queue = SharedDeliveryQueue::new();
    let delivered1 = Rc::new(RefCell::new(Vec::new()));
    let delivered2 = Rc::new(RefCell::new(Vec::<String>::new()));
    let (log1, log2) = (delivered1.clone(), delivered2.clone());
    let obs1 = TestObserver::new(move |value: i32| log1.borrow_mut().push(value));
    let obs2 = TestObserver::new(move |value: String| log2.borrow_mut().push(value));
    let sub1 = queue.create_queue(obs1.clone());
    let sub2 = queue.create_queue(obs2);
    let error = test_error("boom");

    {
        let scope1 = sub1.acquire();
        scope1.enqueue_next(1);
        scope1.enqueue_error(error.clone());
    }

    assert!(queue.is_terminated());

    // Further enqueues are ignored.
    {
        let scope2 = sub2.acquire();
        scope2.enqueue_next("ignored".to_owned());
    }

    assert_eq!(*delivered1.borrow(), vec![1]);
    assert!(same_error(&obs1.error.borrow(), &error));
    assert!(delivered2.borrow().is_empty());
}

#[test]
fn completion_does_not_terminate_parent() {
    let queue = SharedDeliveryQueue::new();
    let delivered2 = Rc::new(RefCell::new(Vec::<String>::new()));
    let log2 = delivered2.clone();
    let obs1 = TestObserver::new(|_: i32| {});
    let obs2 = TestObserver::new(move |value: String| log2.borrow_mut().push(value));
    let sub1 = queue.create_queue(obs1.clone());
    let sub2 = queue.create_queue(obs2);

    {
        let scope1 = sub1.acquire();
        scope1.enqueue_next(1);
        scope1.enqueue_completed();
    }

    assert!(!queue.is_terminated(), "completion of one sub-queue should not terminate parent");
    assert!(obs1.is_completed.get());

    // The other sub-queue still works.
    {
        let scope2 = sub2.acquire();
        scope2.enqueue_next("still alive".to_owned());
    }

    assert_eq!(*delivered2.borrow(), vec!["still alive"]);
}

#[test]
fn dispose_terminates_and_waits() {
    let queue = SharedDeliveryQueue::new();
    let sub = queue.create_queue(TestObserver::new(|_: i32| {}));

    {
        let scope = sub.acquire();
        scope.enqueue_next(1);
    }

    queue.dispose();

    assert!(queue.is_terminated());
}

// Not ported: ConcurrentMultiSourceDelivery - real threads

#[test]
fn receipt_order_is_preserved_across_sub_queues() {
    let queue = SharedDeliveryQueue::new();
    let delivered: Log = Rc::new(RefCell::new(Vec::new()));
    let (log1, log2) = (delivered.clone(), delivered.clone());
    let sub1 = queue.create_queue(TestObserver::new(move |i: i32| record(&log1, format!("int:{i}"))));
    let sub2 = queue.create_queue(TestObserver::new(move |s: String| record(&log2, format!("str:{s}"))));

    // The held scopes queue the notifications rather than deliver them.
    let first = sub1.acquire();
    first.enqueue_next(1);
    let second = sub1.acquire();
    second.enqueue_next(2);
    let third = sub2.acquire();
    third.enqueue_next("hello".to_owned());
    drop(third);
    drop(second);
    drop(first);

    assert_eq!(
        *delivered.borrow(),
        vec!["int:1", "int:2", "str:hello"],
        "delivery should follow the order the notifications were received, not the order the sub-queues were created"
    );
}

#[test]
fn interleaved_sub_queues_deliver_in_receipt_order() {
    let queue = SharedDeliveryQueue::new();
    let delivered: Log = Rc::new(RefCell::new(Vec::new()));
    let (log1, log2) = (delivered.clone(), delivered.clone());
    let sub1 = queue.create_queue(TestObserver::new(move |i: i32| record(&log1, format!("int:{i}"))));
    let sub2 = queue.create_queue(TestObserver::new(move |s: String| record(&log2, format!("str:{s}"))));

    let scopes1: Vec<SubQueueScopedAccess<i32>> = Vec::new();
    let mut scopes1 = scopes1;
    let mut scopes2: Vec<SubQueueScopedAccess<String>> = Vec::new();
    let scope = sub1.acquire();
    scope.enqueue_next(0);
    scopes1.push(scope);
    let scope = sub2.acquire();
    scope.enqueue_next("a".to_owned());
    scopes2.push(scope);
    let scope = sub1.acquire();
    scope.enqueue_next(2);
    scopes1.push(scope);
    let scope = sub2.acquire();
    scope.enqueue_next("b".to_owned());
    scopes2.push(scope);
    let scope = sub1.acquire();
    scope.enqueue_next(4);
    scopes1.push(scope);
    drop(scopes2);
    drop(scopes1);

    assert_eq!(*delivered.borrow(), vec!["int:0", "str:a", "int:2", "str:b", "int:4"]);
}

#[test]
fn disposed_sub_queue_does_not_deliver_queued_items() {
    let queue = SharedDeliveryQueue::new();
    let delivered: Log = Rc::new(RefCell::new(Vec::new()));
    let (log1, log2) = (delivered.clone(), delivered.clone());
    let sub1 = queue.create_queue(TestObserver::new(move |i: i32| record(&log1, format!("int:{i}"))));
    let sub2 = queue.create_queue(TestObserver::new(move |s: String| record(&log2, format!("str:{s}"))));

    let first = sub1.acquire();
    first.enqueue_next(0);
    let second = sub2.acquire();
    second.enqueue_next("dropped".to_owned());
    sub2.dispose();
    drop(second);
    drop(first);

    assert_eq!(*delivered.borrow(), vec!["int:0"], "a disposed sub-queue should not deliver what it had queued");
}

/// A slot for a sub-queue that an observer created before it uses.
type Slot<T> = Rc<RefCell<Option<Rc<DeliverySubQueue<T>>>>>;

fn sub_queue<T: 'static>(slot: &Slot<T>) -> Rc<DeliverySubQueue<T>> {
    slot.borrow().clone().expect("the sub-queue is set")
}

#[test]
fn reentrant_notification_is_delivered_before_notifications_queued_by_other_threads() {
    let (first, second, child_value) = (17, 42, "word".to_owned());
    let queue = SharedDeliveryQueue::new();
    let delivered: Log = Rc::new(RefCell::new(Vec::new()));
    let is_first = Rc::new(Cell::new(true));
    let child: Slot<String> = Rc::new(RefCell::new(None));
    let parent: Slot<i32> = Rc::new(RefCell::new(None));
    let (log, flag, child_slot, parent_slot, value) = (delivered.clone(), is_first.clone(), child.clone(), parent.clone(), child_value.clone());
    let parent_queue = queue.create_queue(TestObserver::new(move |current: i32| {
        record(&log, format!("parent:{current}:start"));
        if flag.replace(false) {
            // The notification of "another thread", queued while this one is delivered.
            let held = sub_queue(&parent_slot).acquire();
            held.enqueue_next(second);
            drop(held);
            sub_queue(&child_slot).on_next(value.clone());
        }
        record(&log, format!("parent:{current}:end"));
    }));
    *parent.borrow_mut() = Some(parent_queue.clone());
    let log = delivered.clone();
    *child.borrow_mut() = Some(queue.create_queue(TestObserver::new(move |current: String| record(&log, format!("child:{current}")))));

    parent_queue.on_next(first);

    assert_eq!(
        *delivered.borrow(),
        vec![format!("parent:{first}:start"), format!("child:{child_value}"), format!("parent:{first}:end"), format!("parent:{second}:start"), format!("parent:{second}:end")],
        "a notification raised during a delivery is delivered inline, and one queued by another thread waits for that delivery to finish"
    );
    parent.borrow_mut().take();
    child.borrow_mut().take();
}

#[test]
fn reentrant_notification_for_the_delivering_source_waits_for_its_turn() {
    let (first, second, third) = (17, 42, 99);
    let queue = SharedDeliveryQueue::new();
    let delivered: Log = Rc::new(RefCell::new(Vec::new()));
    let is_first = Rc::new(Cell::new(true));
    let source: Slot<i32> = Rc::new(RefCell::new(None));
    let (log, flag, source_slot) = (delivered.clone(), is_first.clone(), source.clone());
    let source_queue = queue.create_queue(TestObserver::new(move |current: i32| {
        record(&log, format!("{current}:start"));
        if flag.replace(false) {
            // The notification of "another thread", queued while this one is delivered.
            let held = sub_queue(&source_slot).acquire();
            held.enqueue_next(second);
            drop(held);
            sub_queue(&source_slot).on_next(third);
        }
        record(&log, format!("{current}:end"));
    }));
    *source.borrow_mut() = Some(source_queue.clone());

    source_queue.on_next(first);

    assert_eq!(
        *delivered.borrow(),
        vec![
            format!("{first}:start"),
            format!("{first}:end"),
            format!("{second}:start"),
            format!("{second}:end"),
            format!("{third}:start"),
            format!("{third}:end")
        ],
        "an observer is never re-entered, so its own reentrant notification is delivered in the order it was received"
    );
    source.borrow_mut().take();
}

#[test]
fn inline_delivery_keeps_the_sources_earlier_notifications_first() {
    let (first, second) = (17, 42);
    let (queued_child_value, reentrant_child_value) = ("queued".to_owned(), "reentrant".to_owned());
    let queue = SharedDeliveryQueue::new();
    let delivered: Log = Rc::new(RefCell::new(Vec::new()));
    let is_first = Rc::new(Cell::new(true));
    let child: Slot<String> = Rc::new(RefCell::new(None));
    let parent: Slot<i32> = Rc::new(RefCell::new(None));
    let (log, flag, child_slot, parent_slot) = (delivered.clone(), is_first.clone(), child.clone(), parent.clone());
    let (queued, reentrant) = (queued_child_value.clone(), reentrant_child_value.clone());
    let parent_queue = queue.create_queue(TestObserver::new(move |current: i32| {
        record(&log, format!("parent:{current}:start"));
        if flag.replace(false) {
            // The notifications of "another thread": the parent's is queued; the child's is held.
            let held_parent = sub_queue(&parent_slot).acquire();
            held_parent.enqueue_next(second);
            drop(held_parent);
            let held_child = sub_queue(&child_slot).acquire();
            held_child.enqueue_next(queued.clone());
            sub_queue(&child_slot).on_next(reentrant.clone());
            drop(held_child);
        }
        record(&log, format!("parent:{current}:end"));
    }));
    *parent.borrow_mut() = Some(parent_queue.clone());
    let log = delivered.clone();
    *child.borrow_mut() = Some(queue.create_queue(TestObserver::new(move |current: String| record(&log, format!("child:{current}")))));

    parent_queue.on_next(first);

    assert_eq!(
        *delivered.borrow(),
        vec![
            format!("parent:{first}:start"),
            format!("child:{queued_child_value}"),
            format!("child:{reentrant_child_value}"),
            format!("parent:{first}:end"),
            format!("parent:{second}:start"),
            format!("parent:{second}:end")
        ],
        "a source's notifications stay in the order it raised them, even when the later one is delivered inline"
    );
    parent.borrow_mut().take();
    child.borrow_mut().take();
}

#[test]
fn inline_delivery_does_not_let_later_notifications_jump_the_queue() {
    let (first, second, third) = (17, 42, 99);
    let (queued_child_value, reentrant_child_value, later_child_value) = ("queued".to_owned(), "reentrant".to_owned(), "later".to_owned());
    let queue = SharedDeliveryQueue::new();
    let delivered: Log = Rc::new(RefCell::new(Vec::new()));
    let is_first = Rc::new(Cell::new(true));
    let child: Slot<String> = Rc::new(RefCell::new(None));
    let parent: Slot<i32> = Rc::new(RefCell::new(None));
    // The hold of "another thread" on the child that is still in progress after the delivery.
    let held_later: Rc<RefCell<Option<SubQueueScopedAccess<String>>>> = Rc::new(RefCell::new(None));
    let (log, flag, child_slot, parent_slot, later_slot) = (delivered.clone(), is_first.clone(), child.clone(), parent.clone(), held_later.clone());
    let (queued, reentrant, later) = (queued_child_value.clone(), reentrant_child_value.clone(), later_child_value.clone());
    let parent_queue = queue.create_queue(TestObserver::new(move |current: i32| {
        record(&log, format!("parent:{current}:start"));
        if flag.replace(false) {
            // The child's first two notifications are delivered inline, leaving their places in the
            // order received behind; its next one waits for its own place, after the parent's third.
            let held = sub_queue(&parent_slot).acquire();
            held.enqueue_next(second);
            drop(held);
            let held_child = sub_queue(&child_slot).acquire();
            held_child.enqueue_next(queued.clone());
            let held = sub_queue(&parent_slot).acquire();
            held.enqueue_next(third);
            drop(held);
            sub_queue(&child_slot).on_next(reentrant.clone());
            drop(held_child);
            let held = sub_queue(&child_slot).acquire();
            held.enqueue_next(later.clone());
            *later_slot.borrow_mut() = Some(held);
        }
        record(&log, format!("parent:{current}:end"));
    }));
    *parent.borrow_mut() = Some(parent_queue.clone());
    let log = delivered.clone();
    *child.borrow_mut() = Some(queue.create_queue(TestObserver::new(move |current: String| record(&log, format!("child:{current}")))));

    parent_queue.on_next(first);
    held_later.borrow_mut().take();

    assert_eq!(
        *delivered.borrow(),
        vec![
            format!("parent:{first}:start"),
            format!("child:{queued_child_value}"),
            format!("child:{reentrant_child_value}"),
            format!("parent:{first}:end"),
            format!("parent:{second}:start"),
            format!("parent:{second}:end"),
            format!("parent:{third}:start"),
            format!("parent:{third}:end"),
            format!("child:{later_child_value}"),
        ],
        "notifications delivered inline must not hand their places in the receipt order to later ones"
    );
    parent.borrow_mut().take();
    child.borrow_mut().take();
}
