//! The queues between the render thread and the threads of the bands of the pipelined layers
//! (`pipelined_layer.rs`): the jobs each thread is sent and the answers they send back.
//!
//! A receiver that finds its queue empty spins a few rounds first ([`SPINS`]) and then parks on a
//! condition variable; a sender wakes it only when it parked. A thread that is busy with a band or
//! gets its next job within the spin needs no kernel wakeup, and an idle application parks every
//! thread at once: it burns no processor time. A send takes an uncontended lock and copies the
//! value into a queue that keeps its storage, so it allocates nothing once the queue has grown.
//!
//! The ends behave as those of `std::sync::mpsc`: a receive fails when the queue is empty and
//! every sender is gone, a send fails when the receiver is gone.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

/// How many rounds a receiver spins on an empty queue before it parks.
pub(super) const SPINS: usize = 32;

struct Shared<T> {
    queue: Mutex<State<T>>,
    /// The length of the queue, read without the lock while spinning.
    len: AtomicUsize,
    senders: AtomicUsize,
    receiver_alive: AtomicBool,
    ready: Condvar,
}

struct State<T> {
    items: VecDeque<T>,
    /// The receiver parked: a send has to wake it.
    parked: bool,
}

impl<T> Shared<T> {
    fn lock(&self) -> MutexGuard<'_, State<T>> {
        self.queue.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The sending end; it can be cloned.
pub(super) struct Sender<T>(Arc<Shared<T>>);

/// The receiving end.
pub(super) struct Receiver<T>(Arc<Shared<T>>);

/// A queue and its two ends.
pub(super) fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        queue: Mutex::new(State { items: VecDeque::new(), parked: false }),
        len: AtomicUsize::new(0),
        senders: AtomicUsize::new(1),
        receiver_alive: AtomicBool::new(true),
        ready: Condvar::new(),
    });
    (Sender(shared.clone()), Receiver(shared))
}

impl<T> Sender<T> {
    /// Queues `value`; hands it back when the receiver is gone.
    pub(super) fn send(&self, value: T) -> Result<(), T> {
        if !self.0.receiver_alive.load(Ordering::Acquire) {
            return Err(value);
        }
        let wake = {
            let mut queue = self.0.lock();
            queue.items.push_back(value);
            self.0.len.fetch_add(1, Ordering::Release);
            queue.parked
        };
        if wake {
            self.0.ready.notify_one();
        }
        Ok(())
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.0.senders.fetch_add(1, Ordering::AcqRel);
        Self(self.0.clone())
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.0.senders.fetch_sub(1, Ordering::AcqRel) == 1 {
            // The last sender: a parked receiver learns that nothing more comes. The lock orders this
            // with its check of the senders before it parks.
            let queue = self.0.lock();
            let parked = queue.parked;
            drop(queue);
            if parked {
                self.0.ready.notify_all();
            }
        }
    }
}

impl<T> Receiver<T> {
    /// The next value: spins on an empty queue, then parks until a value comes. `None` when the
    /// queue is empty and every sender is gone.
    pub(super) fn recv(&self) -> Option<T> {
        for _ in 0..SPINS {
            if self.0.len.load(Ordering::Acquire) > 0 || self.0.senders.load(Ordering::Acquire) == 0 {
                break;
            }
            std::hint::spin_loop();
        }
        let mut queue = self.0.lock();
        loop {
            if let Some(value) = queue.items.pop_front() {
                self.0.len.fetch_sub(1, Ordering::Release);
                return Some(value);
            }
            if self.0.senders.load(Ordering::Acquire) == 0 {
                return None;
            }
            queue.parked = true;
            queue = self.0.ready.wait(queue).unwrap_or_else(PoisonError::into_inner);
            queue.parked = false;
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.0.receiver_alive.store(false, Ordering::Release);
        // What is still queued goes with the receiver.
        self.0.lock().items.clear();
        self.0.len.store(0, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn values_arrive_in_order_and_a_parked_receiver_wakes() {
        let (sender, receiver) = channel();
        let consumer = thread::spawn(move || (0..1000).map(|_| receiver.recv().unwrap()).collect::<Vec<u32>>());
        for value in 0..1000 {
            sender.send(value).unwrap();
            if value % 100 == 0 {
                // Long enough for the receiver to spin out and park.
                thread::sleep(Duration::from_millis(5));
            }
        }
        assert_eq!(consumer.join().unwrap(), (0..1000).collect::<Vec<_>>());
    }

    #[test]
    fn a_receive_ends_when_every_sender_is_gone() {
        let (sender, receiver) = channel::<u32>();
        let second = sender.clone();
        sender.send(1).unwrap();
        drop(sender);
        let waiting = thread::spawn(move || (receiver.recv(), receiver.recv()));
        thread::sleep(Duration::from_millis(20));
        // The receiver parked on the empty queue; the last sender going ends its wait.
        drop(second);
        assert_eq!(waiting.join().unwrap(), (Some(1), None));
    }

    #[test]
    fn a_send_fails_when_the_receiver_is_gone() {
        let (sender, receiver) = channel();
        drop(receiver);
        assert_eq!(sender.send(7), Err(7));
    }

    #[test]
    fn a_sender_that_panics_is_gone_for_the_receiver() {
        let (sender, receiver) = channel::<u32>();
        let worker = thread::spawn(move || {
            sender.send(1).unwrap();
            panic!("a worker that fails");
        });
        assert!(worker.join().is_err());
        assert_eq!(receiver.recv(), Some(1));
        assert_eq!(receiver.recv(), None);
    }

    #[test]
    fn a_slow_sender_is_waited_for() {
        let (sender, receiver) = channel();
        let slow = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            sender.send("late").unwrap();
        });
        assert_eq!(receiver.recv(), Some("late"));
        slow.join().unwrap();
    }

    #[test]
    fn an_idle_receiver_parks() {
        let (sender, receiver) = channel::<u32>();
        let shared = sender.0.clone();
        let waiting = thread::spawn(move || receiver.recv());
        // Spinning is over in far less than this; the receiver then waits on the condition variable.
        let mut parked = false;
        for _ in 0..200 {
            if shared.lock().parked {
                parked = true;
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        assert!(parked, "the receiver parks instead of spinning");
        sender.send(3).unwrap();
        assert_eq!(waiting.join().unwrap(), Some(3));
    }
}
