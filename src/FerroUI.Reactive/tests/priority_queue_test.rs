//! Ported from `Tests.System.Reactive/Tests/Internal/PriorityQueueTest.cs` of Rx.NET. The queue of
//! the port keeps a key and a value per item; these tests use the value as its own key.

use crate::internal::PriorityQueue;

fn enqueue(q: &mut PriorityQueue<i32, i32>, value: i32) {
    q.push(value, value);
}

fn dequeue(q: &mut PriorityQueue<i32, i32>) -> i32 {
    q.pop().expect("a non-empty queue").1
}

fn peek(q: &PriorityQueue<i32, i32>) -> i32 {
    *q.peek().expect("a non-empty queue").1
}

#[test]
fn enqueue_dequeue() {
    let mut q = PriorityQueue::new();
    for i in 0..16 {
        assert_eq!(0, q.len());
        enqueue(&mut q, i);
        assert_eq!(1, q.len());
        assert_eq!(i, peek(&q));
        assert_eq!(1, q.len());
        assert_eq!(i, dequeue(&mut q));
        assert_eq!(0, q.len());
    }
}

#[test]
fn enqueue_all_dequeue_all() {
    let mut q = PriorityQueue::new();
    for i in 0..33 {
        enqueue(&mut q, i);
        assert_eq!(i as usize + 1, q.len());
    }
    assert_eq!(33, q.len());
    for i in 0..33 {
        assert_eq!(33 - i as usize, q.len());
        assert_eq!(i, peek(&q));
        assert_eq!(i, dequeue(&mut q));
    }
    assert_eq!(0, q.len());
}

#[test]
fn reverse_enqueue_all_dequeue_all() {
    let mut q = PriorityQueue::new();
    for i in (0..=32).rev() {
        enqueue(&mut q, i);
        assert_eq!(33 - i as usize, q.len());
    }
    assert_eq!(33, q.len());
    for i in 0..33 {
        assert_eq!(33 - i as usize, q.len());
        assert_eq!(i, peek(&q));
        assert_eq!(i, dequeue(&mut q));
    }
    assert_eq!(0, q.len());
}

#[test]
fn remove_from_middle() {
    let mut q = PriorityQueue::new();
    for i in 0..33 {
        enqueue(&mut q, i);
    }
    assert!(q.remove_where(|v| *v == 16));
    for i in 0..16 {
        assert_eq!(i, dequeue(&mut q));
    }
    for i in 16..32 {
        assert_eq!(i + 1, dequeue(&mut q));
    }
}

#[test]
fn repro_329() {
    let mut queue = PriorityQueue::new();
    enqueue(&mut queue, 2);
    enqueue(&mut queue, 1);
    enqueue(&mut queue, 5);
    enqueue(&mut queue, 2);
    assert_eq!(1, dequeue(&mut queue));
    assert_eq!(2, dequeue(&mut queue));
    assert_eq!(2, dequeue(&mut queue));
    assert_eq!(5, dequeue(&mut queue));
}

/// Items of equal priority leave in the order they were added, which the schedulers rely on.
#[test]
fn equal_priorities_keep_their_order() {
    let mut q: PriorityQueue<i32, &str> = PriorityQueue::new();
    q.push(1, "a");
    q.push(0, "x");
    q.push(1, "b");
    q.push(1, "c");
    assert_eq!(Some((0, "x")), q.pop());
    assert_eq!(Some((1, "a")), q.pop());
    assert_eq!(Some((1, "b")), q.pop());
    assert_eq!(Some((1, "c")), q.pop());
    assert!(q.is_empty());
}
