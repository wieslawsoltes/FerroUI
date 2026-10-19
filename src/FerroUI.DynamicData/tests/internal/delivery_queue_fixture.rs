//! `DeliveryQueueFixture` (`DynamicData.Tests/Internal/DeliveryQueueFixture.cs`).

use super::{panics, same_error, test_error, DelegateObserver, ListObserver};
use crate::internal::DeliveryQueue;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `EnqueueAndDeliver`.
fn enqueue_and_deliver<T: 'static>(queue: &Rc<DeliveryQueue<T>>, item: T) {
    let scope = queue.acquire();
    scope.enqueue_next(item);
}

/// `TriggerDelivery`.
fn trigger_delivery<T: 'static>(queue: &Rc<DeliveryQueue<T>>) {
    let _scope = queue.acquire();
}

#[test]
fn enqueue_and_deliver_delivers_item() {
    let observer = ListObserver::<String>::new();
    let queue = DeliveryQueue::new(observer.clone());

    enqueue_and_deliver(&queue, "A".to_owned());

    assert_eq!(observer.items(), vec!["A"]);
}

#[test]
fn deliver_delivers_items_in_fifo_order() {
    let observer = ListObserver::<&str>::new();
    let queue = DeliveryQueue::new(observer.clone());

    {
        let scope = queue.acquire();
        scope.enqueue_next("A");
        scope.enqueue_next("B");
        scope.enqueue_next("C");
    }

    assert_eq!(observer.items(), vec!["A", "B", "C"]);
}

#[test]
fn deliver_with_empty_queue_is_no_op() {
    let observer = ListObserver::<&str>::new();
    let queue = DeliveryQueue::new(observer.clone());

    trigger_delivery(&queue);

    assert!(observer.items().is_empty());
}

// Not ported: OnlyOneDelivererAtATime - real threads

#[test]
fn second_writer_item_picked_up_by_first_deliverer() {
    let observer = ListObserver::<&str>::new();
    let q: Rc<RefCell<Option<Rc<DeliveryQueue<&'static str>>>>> = Rc::new(RefCell::new(None));
    let (recorder, queue_slot) = (observer.clone(), q.clone());
    let enqueuing_observer = DelegateObserver::<&'static str>::new(move |item: &'static str| {
        ferroui_system::IObserver::on_next(&*recorder, item);
        if recorder.items().len() == 1 {
            let queue = queue_slot.borrow().clone().expect("the queue is set");
            let scope = queue.acquire();
            scope.enqueue_next("B");
        }
    });
    let queue = DeliveryQueue::new(enqueuing_observer);
    *q.borrow_mut() = Some(queue.clone());

    enqueue_and_deliver(&queue, "A");

    assert_eq!(observer.items(), vec!["A", "B"]);
    q.borrow_mut().take();
}

#[test]
fn reentrant_enqueue_does_not_recurse() {
    let call_depth = Rc::new(Cell::new(0));
    let max_depth = Rc::new(Cell::new(0));
    let delivered = Rc::new(RefCell::new(Vec::new()));
    let q: Rc<RefCell<Option<Rc<DeliveryQueue<&'static str>>>>> = Rc::new(RefCell::new(None));
    let (depth, max, log, queue_slot) = (call_depth.clone(), max_depth.clone(), delivered.clone(), q.clone());
    let observer = DelegateObserver::<&'static str>::new(move |item: &'static str| {
        depth.set(depth.get() + 1);
        if depth.get() > max.get() {
            max.set(depth.get());
        }
        log.borrow_mut().push(item);
        if item == "A" {
            let queue = queue_slot.borrow().clone().expect("the queue is set");
            let scope = queue.acquire();
            scope.enqueue_next("B");
        }
        depth.set(depth.get() - 1);
    });
    let queue = DeliveryQueue::new(observer);
    *q.borrow_mut() = Some(queue.clone());

    enqueue_and_deliver(&queue, "A");

    assert_eq!(*delivered.borrow(), vec!["A", "B"]);
    assert_eq!(max_depth.get(), 1, "delivery callback should not recurse");
    q.borrow_mut().take();
}

#[test]
fn exception_in_delivery_resets_delivery_token() {
    let call_count = Rc::new(Cell::new(0));
    let count = call_count.clone();
    let observer = DelegateObserver::<&'static str>::new(move |_: &'static str| {
        count.set(count.get() + 1);
        if count.get() == 1 {
            panic!("boom");
        }
    });
    let queue = DeliveryQueue::new(observer);

    assert!(panics(|| enqueue_and_deliver(&queue, "A")));

    enqueue_and_deliver(&queue, "B");

    assert_eq!(call_count.get(), 2, "delivery should work after exception recovery");
}

#[test]
fn remaining_items_delivered_after_exception_recovery() {
    let delivered = Rc::new(RefCell::new(Vec::new()));
    let should_throw = Rc::new(Cell::new(true));
    let (log, throw) = (delivered.clone(), should_throw.clone());
    let observer = DelegateObserver::<&'static str>::new(move |item: &'static str| {
        if throw.get() && item == "A" {
            panic!("boom");
        }
        log.borrow_mut().push(item);
    });
    let queue = DeliveryQueue::new(observer);

    assert!(panics(|| {
        let scope = queue.acquire();
        scope.enqueue_next("A");
        scope.enqueue_next("B");
    }));

    should_throw.set(false);
    trigger_delivery(&queue);

    assert_eq!(*delivered.borrow(), vec!["B"]);
}

#[test]
fn terminal_completed_stops_delivery() {
    let observer = ListObserver::<&str>::new();
    let queue = DeliveryQueue::new(observer.clone());

    {
        let scope = queue.acquire();
        scope.enqueue_next("A");
        scope.enqueue_completed();
        // Ignored after the terminal notification.
        scope.enqueue_next("B");
    }

    assert_eq!(observer.items(), vec!["A"]);
    assert!(observer.is_completed());
    assert!(queue.is_terminated());
}

#[test]
fn terminal_error_stops_delivery() {
    let observer = ListObserver::<&str>::new();
    let queue = DeliveryQueue::new(observer.clone());
    let error = test_error("test");

    {
        let scope = queue.acquire();
        scope.enqueue_next("A");
        scope.enqueue_error(error.clone());
        // Ignored after the terminal notification.
        scope.enqueue_next("B");
    }

    assert_eq!(observer.items(), vec!["A"]);
    assert!(same_error(&observer.error(), &error));
    assert!(queue.is_terminated());
}

#[test]
fn enqueue_after_termination_is_ignored() {
    let observer = ListObserver::<&str>::new();
    let queue = DeliveryQueue::new(observer.clone());

    {
        let scope = queue.acquire();
        scope.enqueue_completed();
    }

    enqueue_and_deliver(&queue, "AFTER");

    assert!(observer.items().is_empty());
}

#[test]
fn is_terminated_is_false_initially() {
    let observer = ListObserver::<&str>::new();
    let queue = DeliveryQueue::new(observer);
    assert!(!queue.is_terminated());
}

// Not ported: ConcurrentEnqueueAllItemsDelivered - real threads
// Not ported: ConcurrentEnqueueNoDuplicates - real threads
// Not ported: ConcurrentEnqueuePreservesPerThreadOrdering - real threads

#[test]
fn dispose_terminates_queue() {
    let observer = ListObserver::<&str>::new();
    let queue = DeliveryQueue::new(observer.clone());

    enqueue_and_deliver(&queue, "A");
    queue.dispose();

    assert!(queue.is_terminated());

    // Further enqueues are ignored.
    enqueue_and_deliver(&queue, "B");
    assert_eq!(observer.items(), vec!["A"]);
}

#[test]
fn dispose_clears_pending_items() {
    let observer = ListObserver::<&str>::new();
    let delivery_count = Rc::new(Cell::new(0));
    let q: Rc<RefCell<Option<Rc<DeliveryQueue<&'static str>>>>> = Rc::new(RefCell::new(None));
    let (recorder, count, queue_slot) = (observer.clone(), delivery_count.clone(), q.clone());
    let blocking_observer = DelegateObserver::<&'static str>::new(move |item: &'static str| {
        ferroui_system::IObserver::on_next(&*recorder, item);
        count.set(count.get() + 1);
        if count.get() == 1 {
            // While the first item is delivered, enqueue more, then terminate.
            let queue = queue_slot.borrow().clone().expect("the queue is set");
            {
                let scope = queue.acquire();
                scope.enqueue_next("B");
                scope.enqueue_next("C");
            }
            // Re-entrant: must not wait.
            queue.dispose();
        }
    });
    let queue = DeliveryQueue::new(blocking_observer);
    *q.borrow_mut() = Some(queue.clone());

    enqueue_and_deliver(&queue, "A");

    // Only "A" is delivered: "B" and "C" were cleared by the disposal.
    assert_eq!(observer.items(), vec!["A"]);
    assert!(queue.is_terminated());
    q.borrow_mut().take();
}

#[test]
fn dispose_from_drain_thread_does_not_deadlock() {
    let q: Rc<RefCell<Option<Rc<DeliveryQueue<&'static str>>>>> = Rc::new(RefCell::new(None));
    let queue_slot = q.clone();
    let terminating_observer = DelegateObserver::<&'static str>::new(move |_: &'static str| {
        // Called during the delivery: the disposal must return at once.
        let queue = queue_slot.borrow().clone().expect("the queue is set");
        queue.dispose();
    });
    let queue = DeliveryQueue::new(terminating_observer);
    *q.borrow_mut() = Some(queue.clone());

    enqueue_and_deliver(&queue, "A");

    assert!(queue.is_terminated(), "Dispose from the delivery should not deadlock");
    q.borrow_mut().take();
}

// Not ported: DisposeWaitsForInFlightDelivery - real threads

#[test]
fn terminal_items_delivered_before_termination() {
    let observer = ListObserver::<&str>::new();
    let queue = DeliveryQueue::new(observer.clone());

    {
        let scope = queue.acquire();
        scope.enqueue_next("A");
        scope.enqueue_next("B");
        scope.enqueue_completed();
        // Ignored after the terminal notification.
        scope.enqueue_next("C");
    }

    assert_eq!(observer.items(), vec!["A", "B"]);
    assert!(observer.is_completed());
    assert!(queue.is_terminated());
}

#[test]
fn error_terminates_and_clears_pending() {
    let observer = ListObserver::<&str>::new();
    let queue = DeliveryQueue::new(observer.clone());
    let error = test_error("test");

    {
        let scope = queue.acquire();
        scope.enqueue_next("A");
        scope.enqueue_error(error.clone());
        // Ignored.
        scope.enqueue_next("B");
    }

    assert_eq!(observer.items(), vec!["A"]);
    assert!(same_error(&observer.error(), &error));
    assert!(queue.is_terminated());
}
