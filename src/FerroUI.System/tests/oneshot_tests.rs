//! Tests of `task::oneshot`: the one-shot channel (step A3 of phase C4 of #423,
//! docs/idiomatic-rust-async.md, section 3.3): sending before and after the receiver waits, the
//! cancellation of a dropped sender, a dropped receiver, the waits that block a thread, wakes from
//! other threads, and the shared receiver.

use crate::task::oneshot::{self, Canceled, Receiver};
use crate::task::{block_on, run_pending, spawn_local};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::{Duration, Instant};

/// A waker that counts its wakes.
#[derive(Default)]
struct CountingWaker(AtomicUsize);

impl CountingWaker {
    fn new() -> (Arc<Self>, Waker) {
        let counter = Arc::new(Self::default());
        let waker = Waker::from(counter.clone());
        (counter, waker)
    }

    fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

impl Wake for CountingWaker {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

/// Polls `future` once with `waker`.
fn poll_with<F: Future + Unpin>(future: &mut F, waker: &Waker) -> Poll<F::Output> {
    Pin::new(future).poll(&mut Context::from_waker(waker))
}

/// Runs `test` on a new thread, whose queue no other test sees, and waits for it.
fn on_own_thread(test: impl FnOnce() + Send + 'static) {
    thread::spawn(test).join().unwrap();
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn the_ends_are_send_and_sync_when_the_value_is_send() {
    assert_send_sync::<oneshot::Sender<String>>();
    assert_send_sync::<oneshot::Receiver<String>>();
    assert_send_sync::<oneshot::SharedReceiver<String>>();
    // A value that is `Send` but not `Sync`.
    assert_send_sync::<oneshot::Receiver<std::cell::Cell<i32>>>();
}

#[test]
fn a_value_sent_before_the_first_poll_is_its_output() {
    let (sender, mut receiver) = oneshot::channel();
    assert!(!receiver.is_finished());
    sender.send(7).unwrap();
    assert!(receiver.is_finished());
    let (counter, waker) = CountingWaker::new();
    assert_eq!(poll_with(&mut receiver, &waker), Poll::Ready(Ok(7)));
    assert_eq!(counter.count(), 0);
}

#[test]
fn a_value_sent_after_a_poll_wakes_the_receiver_inside_the_send() {
    let (sender, mut receiver) = oneshot::channel();
    let (counter, waker) = CountingWaker::new();
    assert_eq!(poll_with(&mut receiver, &waker), Poll::Pending);
    assert_eq!(poll_with(&mut receiver, &waker), Poll::Pending);
    assert_eq!(counter.count(), 0);

    sender.send("done").unwrap();
    // The same waker registered twice is woken once.
    assert_eq!(counter.count(), 1);
    assert_eq!(poll_with(&mut receiver, &waker), Poll::Ready(Ok("done")));
}

#[test]
fn a_task_awaiting_the_receiver_resumes_when_the_value_is_sent() {
    on_own_thread(|| {
        let (sender, receiver) = oneshot::channel::<i32>();
        let handle = spawn_local(async move { receiver.await.map(|value| value * 2) });
        assert!(!handle.is_finished());
        // A send on the task's thread runs the task inside the call (the queue of the thread).
        sender.send(21).unwrap();
        assert!(handle.is_finished());
        assert_eq!(block_on(handle).unwrap(), Ok(42));
    });
}

#[test]
fn dropping_the_sender_without_sending_cancels_the_receiver() {
    let (sender, mut receiver) = oneshot::channel::<i32>();
    let (counter, waker) = CountingWaker::new();
    assert_eq!(poll_with(&mut receiver, &waker), Poll::Pending);

    drop(sender);
    assert_eq!(counter.count(), 1);
    assert!(receiver.is_finished());
    assert_eq!(poll_with(&mut receiver, &waker), Poll::Ready(Err(Canceled)));
}

#[test]
fn a_task_awaiting_a_dropped_sender_ends_with_canceled() {
    on_own_thread(|| {
        let (sender, receiver) = oneshot::channel::<i32>();
        let handle = spawn_local(receiver);
        drop(sender);
        assert!(handle.is_finished());
        assert_eq!(block_on(handle).unwrap(), Err(Canceled));
    });
}

#[test]
fn sending_to_a_dropped_receiver_gives_the_value_back() {
    let (sender, receiver) = oneshot::channel();
    assert!(!sender.is_closed());
    drop(receiver);
    assert!(sender.is_closed());
    assert_eq!(sender.send(String::from("lost")), Err(String::from("lost")));
}

#[test]
fn try_recv_tells_empty_sent_taken_and_canceled_apart() {
    let (sender, mut receiver) = oneshot::channel();
    assert_eq!(receiver.try_recv(), Ok(None));
    sender.send(5).unwrap();
    assert_eq!(receiver.try_recv(), Ok(Some(5)));
    // The value is taken once: the channel stays empty for good.
    assert_eq!(receiver.try_recv(), Err(Canceled));

    let (sender, mut receiver) = oneshot::channel::<i32>();
    drop(sender);
    assert_eq!(receiver.try_recv(), Err(Canceled));
}

#[test]
fn a_poll_after_the_value_was_taken_is_canceled() {
    let (sender, mut receiver) = oneshot::channel();
    sender.send(1).unwrap();
    assert_eq!(poll_with(&mut receiver, Waker::noop()), Poll::Ready(Ok(1)));
    assert_eq!(poll_with(&mut receiver, Waker::noop()), Poll::Ready(Err(Canceled)));
}

#[test]
fn a_repoll_with_another_waker_replaces_the_first() {
    let (sender, mut receiver) = oneshot::channel();
    let (first, first_waker) = CountingWaker::new();
    let (second, second_waker) = CountingWaker::new();
    assert_eq!(poll_with(&mut receiver, &first_waker), Poll::Pending);
    assert_eq!(poll_with(&mut receiver, &second_waker), Poll::Pending);

    sender.send(3).unwrap();
    assert_eq!(first.count(), 0);
    assert_eq!(second.count(), 1);
    assert_eq!(poll_with(&mut receiver, &second_waker), Poll::Ready(Ok(3)));
}

#[test]
fn a_send_from_another_thread_wakes_a_task_through_the_queue_of_its_thread() {
    on_own_thread(|| {
        let (sender, receiver) = oneshot::channel::<String>();
        let handle = spawn_local(receiver);
        let sending = thread::spawn(move || sender.send(String::from("from afar")).unwrap());
        sending.join().unwrap();
        // The wake from the other thread waits in the queue of this thread.
        assert!(!handle.is_finished());
        assert!(run_pending());
        assert!(handle.is_finished());
        assert_eq!(block_on(handle).unwrap(), Ok(String::from("from afar")));
    });
}

#[test]
fn block_on_waits_for_a_value_sent_by_another_thread() {
    on_own_thread(|| {
        let (sender, receiver) = oneshot::channel::<i32>();
        let sending = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            sender.send(9).unwrap();
        });
        assert_eq!(block_on(receiver), Ok(9));
        sending.join().unwrap();
    });
}

#[test]
fn a_blocking_receive_returns_the_value_or_canceled() {
    let (sender, receiver) = oneshot::channel::<i32>();
    let sending = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        sender.send(11).unwrap();
    });
    assert_eq!(receiver.blocking_recv(), Ok(11));
    sending.join().unwrap();

    let (sender, receiver) = oneshot::channel::<i32>();
    let dropping = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        drop(sender);
    });
    assert_eq!(receiver.blocking_recv(), Err(Canceled));
    dropping.join().unwrap();
}

#[test]
fn a_receive_with_a_timeout_gives_none_when_nothing_came_in_time() {
    let (sender, mut receiver) = oneshot::channel::<i32>();
    let started = Instant::now();
    assert_eq!(receiver.recv_timeout(Duration::from_millis(10)), Ok(None));
    assert!(started.elapsed() >= Duration::from_millis(10));

    sender.send(4).unwrap();
    assert_eq!(receiver.recv_timeout(Duration::from_millis(10)), Ok(Some(4)));
    assert_eq!(receiver.recv_timeout(Duration::from_millis(10)), Err(Canceled));

    // A timeout too long to be a point in time waits as long as it takes.
    let (sender, mut receiver) = oneshot::channel::<i32>();
    sender.send(6).unwrap();
    assert_eq!(receiver.recv_timeout(Duration::MAX), Ok(Some(6)));
}

#[test]
fn a_shared_receiver_gives_every_waiter_a_copy() {
    let (sender, receiver) = oneshot::channel::<String>();
    let mut first = receiver.shared();
    let mut second = first.clone();
    let (first_counter, first_waker) = CountingWaker::new();
    let (second_counter, second_waker) = CountingWaker::new();
    assert_eq!(poll_with(&mut first, &first_waker), Poll::Pending);
    assert_eq!(poll_with(&mut second, &second_waker), Poll::Pending);
    assert_eq!(first.try_recv(), Ok(None));
    assert!(!sender.is_closed());

    sender.send(String::from("all")).unwrap();
    assert_eq!(first_counter.count(), 1);
    assert_eq!(second_counter.count(), 1);
    assert_eq!(poll_with(&mut first, &first_waker), Poll::Ready(Ok(String::from("all"))));
    assert_eq!(poll_with(&mut second, &second_waker), Poll::Ready(Ok(String::from("all"))));
    // The value stays for later polls and for clones made afterwards.
    assert_eq!(poll_with(&mut first, &first_waker), Poll::Ready(Ok(String::from("all"))));
    let third = second.clone();
    assert_eq!(third.try_recv(), Ok(Some(String::from("all"))));
    assert_eq!(third.blocking_recv(), Ok(String::from("all")));
}

#[test]
fn a_waker_that_several_clones_registered_is_woken_once() {
    let (sender, receiver) = oneshot::channel::<i32>();
    let mut first = receiver.shared();
    let mut second = first.clone();
    let (counter, waker) = CountingWaker::new();
    assert_eq!(poll_with(&mut first, &waker), Poll::Pending);
    assert_eq!(poll_with(&mut second, &waker), Poll::Pending);
    sender.send(8).unwrap();
    assert_eq!(counter.count(), 1);
    assert_eq!(poll_with(&mut first, &waker), Poll::Ready(Ok(8)));
    assert_eq!(poll_with(&mut second, &waker), Poll::Ready(Ok(8)));
}

#[test]
fn a_shared_receiver_is_canceled_with_its_sender_and_closes_it_with_its_last_clone() {
    let (sender, receiver) = oneshot::channel::<i32>();
    let first = receiver.shared();
    let second = first.clone();
    drop(first);
    assert!(!sender.is_closed());
    drop(second);
    assert!(sender.is_closed());
    assert_eq!(sender.send(1), Err(1));

    let (sender, receiver) = oneshot::channel::<i32>();
    let mut shared = receiver.shared();
    let (counter, waker) = CountingWaker::new();
    assert_eq!(poll_with(&mut shared, &waker), Poll::Pending);
    drop(sender);
    assert_eq!(counter.count(), 1);
    assert!(shared.is_finished());
    assert_eq!(poll_with(&mut shared, &waker), Poll::Ready(Err(Canceled)));
    assert_eq!(shared.clone().try_recv(), Err(Canceled));
    assert_eq!(shared.recv_timeout(Duration::from_millis(1)), Err(Canceled));
}

#[test]
fn shared_waiters_are_woken_in_the_order_they_waited_and_a_dropped_one_is_not() {
    let (sender, receiver) = oneshot::channel::<i32>();
    let shared = receiver.shared();
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    struct Ordered(usize, Arc<std::sync::Mutex<Vec<usize>>>);

    impl Wake for Ordered {
        fn wake(self: Arc<Self>) {
            self.1.lock().unwrap().push(self.0);
        }
    }

    let mut waiters: Vec<_> = (0..3).map(|_| shared.clone()).collect();
    for (index, waiter) in waiters.iter_mut().enumerate() {
        let waker = Waker::from(Arc::new(Ordered(index, order.clone())));
        assert_eq!(poll_with(waiter, &waker), Poll::Pending);
    }
    // The middle one is dropped while it waits.
    drop(waiters.remove(1));

    sender.send(0).unwrap();
    assert_eq!(*order.lock().unwrap(), [0, 2]);
}

#[test]
fn several_tasks_await_clones_of_a_shared_receiver() {
    on_own_thread(|| {
        let (sender, receiver) = oneshot::channel::<i32>();
        let shared = receiver.shared();
        let handles: Vec<_> = (1..=3)
            .map(|factor| {
                let shared = shared.clone();
                spawn_local(async move { shared.await.map(|value| value * factor) })
            })
            .collect();
        drop(shared);
        let sending = thread::spawn(move || sender.send(10).unwrap());
        sending.join().unwrap();
        run_pending();
        let outcomes: Vec<_> = handles.into_iter().map(|handle| block_on(handle).unwrap()).collect();
        assert_eq!(outcomes, [Ok(10), Ok(20), Ok(30)]);
    });
}

#[test]
fn a_receiver_made_shared_after_it_gave_its_value_is_canceled() {
    let (sender, mut receiver): (_, Receiver<i32>) = oneshot::channel();
    sender.send(2).unwrap();
    assert_eq!(receiver.try_recv(), Ok(Some(2)));
    assert_eq!(receiver.shared().try_recv(), Err(Canceled));
}

#[test]
fn canceled_is_an_error_with_a_message() {
    let error: Box<dyn std::error::Error + Send + Sync> = Box::new(Canceled);
    assert_eq!(error.to_string(), "the operation was canceled");
    assert_eq!(format!("{:?}", Canceled), "Canceled");
}
