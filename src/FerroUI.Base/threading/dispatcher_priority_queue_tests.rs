use std::any::Any;
use std::sync::Arc;

use super::dispatcher_operation::OperationCore;
use super::dispatcher_priority_queue::DispatcherPriorityQueue;
use super::{Dispatcher, DispatcherPriority};

fn operation(dispatcher: &Arc<Dispatcher>, priority: DispatcherPriority) -> Arc<OperationCore> {
    OperationCore::new_send(dispatcher, priority, false, Box::new(|| Box::new(()) as Box<dyn Any + Send>))
}

fn drain(queue: &mut DispatcherPriorityQueue) -> Vec<Arc<OperationCore>> {
    let mut result = Vec::new();
    while queue.peek().is_some() {
        result.push(queue.dequeue());
    }
    result
}

fn index_of(items: &[Arc<OperationCore>], item: &Arc<OperationCore>) -> usize {
    items.iter().position(|i| Arc::ptr_eq(i, item)).unwrap()
}

#[test]
fn empty_queue_has_invalid_max_priority() {
    let queue = DispatcherPriorityQueue::new();
    assert_eq!(queue.max_priority(), DispatcherPriority::INVALID);
    assert!(queue.peek().is_none());
    assert!(queue.peek_all().is_empty());
}

#[test]
fn dequeues_by_priority_then_arrival_order() {
    let _scope = Dispatcher::unit_test_scope();
    let dispatcher = Dispatcher::current_dispatcher();
    let mut queue = DispatcherPriorityQueue::new();

    let background1 = operation(&dispatcher, DispatcherPriority::BACKGROUND);
    let render1 = operation(&dispatcher, DispatcherPriority::RENDER);
    let background2 = operation(&dispatcher, DispatcherPriority::BACKGROUND);
    let send = operation(&dispatcher, DispatcherPriority::SEND);
    let render2 = operation(&dispatcher, DispatcherPriority::RENDER);
    let inactive = operation(&dispatcher, DispatcherPriority::INACTIVE);
    let all = [&background1, &render1, &background2, &send, &render2, &inactive];
    for item in all {
        queue.enqueue(item.priority(), item.clone());
        assert!(item.is_queued());
    }

    assert_eq!(queue.max_priority(), DispatcherPriority::SEND);
    let sequential = queue.peek_all();
    for (i, item) in all.iter().enumerate() {
        assert_eq!(index_of(&sequential, item), i);
    }

    let order = drain(&mut queue);
    let expected = [&send, &render1, &render2, &background1, &background2, &inactive];
    assert_eq!(order.len(), expected.len());
    for (i, item) in expected.iter().enumerate() {
        assert!(Arc::ptr_eq(&order[i], item), "unexpected item at {i}");
        assert!(!item.is_queued());
    }
    assert_eq!(queue.max_priority(), DispatcherPriority::INVALID);
}

#[test]
fn remove_item_unlinks_from_both_chains() {
    let _scope = Dispatcher::unit_test_scope();
    let dispatcher = Dispatcher::current_dispatcher();
    let mut queue = DispatcherPriorityQueue::new();

    let a = operation(&dispatcher, DispatcherPriority::NORMAL);
    let b = operation(&dispatcher, DispatcherPriority::NORMAL);
    let c = operation(&dispatcher, DispatcherPriority::NORMAL);
    let d = operation(&dispatcher, DispatcherPriority::INPUT);
    for item in [&a, &b, &c, &d] {
        queue.enqueue(item.priority(), item.clone());
    }

    queue.remove_item(&b);
    assert!(!b.is_queued());
    assert_eq!(queue.peek_all().len(), 3);

    queue.remove_item(&a);
    assert!(Arc::ptr_eq(&queue.peek().unwrap(), &c));
    queue.remove_item(&c);
    assert_eq!(queue.max_priority(), DispatcherPriority::INPUT);
    queue.remove_item(&d);
    assert_eq!(queue.max_priority(), DispatcherPriority::INVALID);

    // Slots are reused.
    queue.enqueue(b.priority(), b.clone());
    assert!(Arc::ptr_eq(&queue.dequeue(), &b));
}

#[test]
fn change_item_priority_keeps_sequential_order_in_the_new_chain() {
    let _scope = Dispatcher::unit_test_scope();
    let dispatcher = Dispatcher::current_dispatcher();
    let mut queue = DispatcherPriorityQueue::new();

    let promoted1 = operation(&dispatcher, DispatcherPriority::BACKGROUND);
    let promoted2 = operation(&dispatcher, DispatcherPriority::INPUT);
    let render = operation(&dispatcher, DispatcherPriority::RENDER);
    let late = operation(&dispatcher, DispatcherPriority::BACKGROUND);
    for item in [&promoted1, &promoted2, &render, &late] {
        queue.enqueue(item.priority(), item.clone());
    }

    queue.change_item_priority(&promoted1, DispatcherPriority::RENDER);
    queue.change_item_priority(&promoted2, DispatcherPriority::RENDER);
    // Promoted after `render` in arrival order.
    queue.change_item_priority(&late, DispatcherPriority::RENDER);

    let order = drain(&mut queue);
    let expected = [&promoted1, &promoted2, &render, &late];
    for (i, item) in expected.iter().enumerate() {
        assert!(Arc::ptr_eq(&order[i], item), "unexpected item at {i}");
    }
}

#[test]
fn clear_empties_the_queue() {
    let _scope = Dispatcher::unit_test_scope();
    let dispatcher = Dispatcher::current_dispatcher();
    let mut queue = DispatcherPriorityQueue::new();
    let a = operation(&dispatcher, DispatcherPriority::NORMAL);
    let b = operation(&dispatcher, DispatcherPriority::BACKGROUND);
    queue.enqueue(a.priority(), a.clone());
    queue.enqueue(b.priority(), b.clone());

    let cleared = queue.clear();
    assert_eq!(cleared.len(), 2);
    assert!(!a.is_queued() && !b.is_queued());
    assert!(queue.peek().is_none());
    assert_eq!(queue.max_priority(), DispatcherPriority::INVALID);

    queue.enqueue(a.priority(), a.clone());
    assert!(Arc::ptr_eq(&queue.dequeue(), &a));
}
