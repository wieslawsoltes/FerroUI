//! Port of `LayoutQueueTests.cs`.

use super::layout_queue::LayoutQueue;
use std::collections::VecDeque;

fn s(value: &str) -> String {
    value.to_owned()
}

fn items(target: &LayoutQueue<String>) -> Vec<String> {
    target.into_iter().cloned().collect()
}

#[test]
fn should_enqueue() {
    let mut target = LayoutQueue::<String>::new(|_| true);
    let mut ref_queue = VecDeque::new();
    let items_ = ["1", "2", "3"];

    for item in items_ {
        target.enqueue(s(item));
        ref_queue.push_back(s(item));
    }

    assert_eq!(ref_queue.into_iter().collect::<Vec<_>>(), items(&target));
}

#[test]
fn should_dequeue() {
    let mut target = LayoutQueue::<String>::new(|_| true);
    let mut ref_queue = VecDeque::new();
    let items_ = ["1", "2", "3"];

    for item in items_ {
        target.enqueue(s(item));
        ref_queue.push_back(s(item));
    }

    while let Some(expected) = ref_queue.pop_front() {
        assert_eq!(expected, target.dequeue());
    }
}

#[test]
fn should_enqueue_unique_elements() {
    let mut target = LayoutQueue::<String>::new(|_| true);
    let items_ = ["1", "2", "3", "1"];

    for item in items_ {
        target.enqueue(s(item));
    }

    assert_eq!(3, target.count());
    assert_eq!(items_.iter().take(3).map(|x| s(x)).collect::<Vec<_>>(), items(&target));
}

#[test]
fn shouldnt_enqueue_more_than_limit_in_loop() {
    let mut target = LayoutQueue::<String>::new(|_| true);

    //1
    target.enqueue(s("Foo"));

    assert_eq!(1, target.count());

    target.begin_loop(3);

    target.dequeue();

    //2
    target.enqueue(s("Foo"));

    target.dequeue();

    //3
    target.enqueue(s("Foo"));

    assert_eq!(1, target.count());

    target.dequeue();

    //4 more than limit shouldn't be added
    target.enqueue(s("Foo"));

    assert_eq!(0, target.count());
}

#[test]
fn shouldnt_count_unique_enqueue_for_limit_in_loop() {
    let mut target = LayoutQueue::<String>::new(|_| true);

    //1
    target.enqueue(s("Foo"));

    assert_eq!(1, target.count());

    target.begin_loop(3);

    target.dequeue();

    //2
    target.enqueue(s("Foo"));
    target.enqueue(s("Foo"));

    target.dequeue();

    //3
    target.enqueue(s("Foo"));
    target.enqueue(s("Foo"));

    assert_eq!(1, target.count());

    target.dequeue();

    //4 more than limit shouldn't be added
    target.enqueue(s("Foo"));

    assert_eq!(0, target.count());
}

#[test]
fn should_enqueue_when_condition_true_after_loop_when_limit_met() {
    let mut target = LayoutQueue::<String>::new(|_| true);

    //1
    target.enqueue(s("Foo"));

    assert_eq!(1, target.count());

    target.begin_loop(3);

    target.dequeue();

    //2
    target.enqueue(s("Foo"));

    target.dequeue();

    //3
    target.enqueue(s("Foo"));

    assert_eq!(1, target.count());

    target.dequeue();

    //4 more than limit shouldn't be added to queue
    target.enqueue(s("Foo"));

    assert_eq!(0, target.count());

    target.end_loop();

    //after loop should be added once
    assert_eq!(1, target.count());
    assert_eq!(Some(&s("Foo")), target.into_iter().next());
}

#[test]
fn shouldnt_enqueue_when_condition_false_after_loop_when_limit_met() {
    let mut target = LayoutQueue::<String>::new(|_| false);

    //1
    target.enqueue(s("Foo"));

    assert_eq!(1, target.count());

    target.begin_loop(3);

    target.dequeue();

    //2
    target.enqueue(s("Foo"));

    target.dequeue();

    //3
    target.enqueue(s("Foo"));

    assert_eq!(1, target.count());

    target.dequeue();

    //4 more than limit shouldn't be added
    target.enqueue(s("Foo"));

    assert_eq!(0, target.count());

    target.end_loop();

    assert_eq!(0, target.count());
}
