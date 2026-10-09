use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, Weak};

use crate::error::Error;
use crate::event_stash::EventStash;
use crate::i_transport::{
    exception_handler, message_handler, ExceptionHandler, HandlerToken, IFerroRemoteTransportConnection, Message,
    MessageHandler,
};
use crate::task::{Task, TaskCompletionSource};

struct SendOperation {
    message: Message,
    tcs: TaskCompletionSource,
}

/// The fields the original guards with `_lock`.
struct Queue {
    send_queue: VecDeque<SendOperation>,
    worker_is_alive: bool,
    // The wrapper was dropped: the worker ends when the queue is empty.
    closed: bool,
}

struct WorkerState {
    lock: Mutex<Queue>,
    // `_signal`: wakes the worker when a message is queued.
    signal: Condvar,
}

/// A connection around another one that keeps the events raised before a
/// handler is subscribed, and that queues the messages to send so that any
/// number of threads may send at once.
pub struct TransportConnectionWrapper {
    conn: Arc<dyn IFerroRemoteTransportConnection>,
    on_message: Arc<EventStash<Message>>,
    on_exception: Arc<EventStash<Error>>,
    worker: Arc<WorkerState>,
}

impl TransportConnectionWrapper {
    pub fn new(conn: Arc<dyn IFerroRemoteTransportConnection>) -> Arc<TransportConnectionWrapper> {
        Arc::new_cyclic(|this: &Weak<TransportConnectionWrapper>| {
            let transport: Weak<dyn IFerroRemoteTransportConnection> = this.clone();
            let on_exception = Arc::new(EventStash::<Error>::new(transport.clone(), None));
            let on_message = {
                let on_exception = on_exception.clone();
                Arc::new(EventStash::<Message>::new(transport, Some(Box::new(move |e| on_exception.fire(e)))))
            };
            {
                let on_exception = on_exception.clone();
                conn.on_exception(exception_handler(move |_, e| on_exception.fire(e.clone())));
            }
            {
                let on_message = on_message.clone();
                conn.on_message(message_handler(move |_, m| on_message.fire(m.clone())));
            }
            TransportConnectionWrapper {
                conn,
                on_message,
                on_exception,
                worker: Arc::new(WorkerState {
                    lock: Mutex::new(Queue { send_queue: VecDeque::new(), worker_is_alive: false, closed: false }),
                    signal: Condvar::new(),
                }),
            }
        })
    }

    // Deviation (DEVIATIONS.md, Remote protocol): the original worker is an
    // `async void` loop that awaits a signal and each send; here it is a
    // thread, started by the first send, that sends one message at a time
    // and ends when the wrapper has been dropped and the queue is empty.
    fn worker(conn: Arc<dyn IFerroRemoteTransportConnection>, state: Arc<WorkerState>) {
        loop {
            let wi = {
                let mut queue = state.lock.lock().unwrap();
                loop {
                    if let Some(wi) = queue.send_queue.pop_front() {
                        break Some(wi);
                    }
                    if queue.closed {
                        break None;
                    }
                    queue = state.signal.wait(queue).unwrap();
                }
            };
            let Some(wi) = wi else {
                return;
            };
            match conn.send(wi.message).wait() {
                Ok(()) => wi.tcs.try_set_result(),
                Err(e) => wi.tcs.try_set_exception(e),
            };
        }
    }
}

impl IFerroRemoteTransportConnection for TransportConnectionWrapper {
    fn dispose(&self) {
        self.conn.dispose()
    }

    /// Queues the message and returns; the task completes when the worker
    /// has sent it. Messages are sent in the order they were queued.
    fn send(&self, data: Message) -> Task {
        let tcs = TaskCompletionSource::new();
        let mut queue = self.worker.lock.lock().unwrap();
        if !queue.worker_is_alive {
            queue.worker_is_alive = true;
            let (conn, state) = (self.conn.clone(), self.worker.clone());
            std::thread::Builder::new()
                .name("remote-protocol-sender".to_string())
                .spawn(move || TransportConnectionWrapper::worker(conn, state))
                .expect("failed to start the sender thread");
        }
        queue.send_queue.push_back(SendOperation { message: data, tcs: tcs.clone() });
        self.worker.signal.notify_one();
        tcs.task()
    }

    fn on_message(&self, handler: MessageHandler) -> HandlerToken {
        self.on_message.add(handler)
    }

    fn remove_on_message(&self, token: HandlerToken) {
        self.on_message.remove(token)
    }

    fn on_exception(&self, handler: ExceptionHandler) -> HandlerToken {
        self.on_exception.add(handler)
    }

    fn remove_on_exception(&self, token: HandlerToken) {
        self.on_exception.remove(token)
    }

    fn start(&self) {
        self.conn.start()
    }
}

impl Drop for TransportConnectionWrapper {
    fn drop(&mut self) {
        self.worker.lock.lock().unwrap().closed = true;
        self.worker.signal.notify_all();
    }
}

// Tests of the port: the upstream project tests the wrapper through the
// socket transport only (`tests/remote_protocol_tests.rs`).
#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    use super::*;
    use crate::i_transport::Delegate;
    use crate::viewport_messages::MeasureViewportMessage;

    const TIMEOUT: Duration = Duration::from_secs(5);

    /// A connection that records what is sent and raises what the test tells
    /// it to.
    struct FakeConnection {
        sent: Mutex<Vec<Message>>,
        on_message: Mutex<Delegate<Message>>,
        on_exception: Mutex<Delegate<Error>>,
        disposed: AtomicUsize,
        started: AtomicUsize,
        fail_sends: bool,
    }

    impl FakeConnection {
        fn new(fail_sends: bool) -> Arc<FakeConnection> {
            Arc::new(FakeConnection {
                sent: Mutex::new(Vec::new()),
                on_message: Mutex::new(Delegate::new()),
                on_exception: Mutex::new(Delegate::new()),
                disposed: AtomicUsize::new(0),
                started: AtomicUsize::new(0),
                fail_sends,
            })
        }

        fn raise_message(&self, message: Message) {
            let handlers = self.on_message.lock().unwrap().snapshot();
            for handler in &handlers {
                handler(self, &message);
            }
        }

        fn raise_exception(&self, e: Error) {
            let handlers = self.on_exception.lock().unwrap().snapshot();
            for handler in &handlers {
                handler(self, &e);
            }
        }
    }

    impl IFerroRemoteTransportConnection for FakeConnection {
        fn dispose(&self) {
            self.disposed.fetch_add(1, Ordering::SeqCst);
        }

        fn send(&self, data: Message) -> Task {
            if self.fail_sends {
                return Task::from_result(Err(Error::InvalidOperation("refused".to_string())));
            }
            self.sent.lock().unwrap().push(data);
            Task::completed()
        }

        fn on_message(&self, handler: MessageHandler) -> HandlerToken {
            self.on_message.lock().unwrap().add(handler)
        }

        fn remove_on_message(&self, token: HandlerToken) {
            self.on_message.lock().unwrap().remove(token);
        }

        fn on_exception(&self, handler: ExceptionHandler) -> HandlerToken {
            self.on_exception.lock().unwrap().add(handler)
        }

        fn remove_on_exception(&self, token: HandlerToken) {
            self.on_exception.lock().unwrap().remove(token);
        }

        fn start(&self) {
            self.started.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn measure(width: f64) -> Message {
        Arc::new(MeasureViewportMessage { width, height: 0.0 })
    }

    fn width_of(message: &Message) -> f64 {
        message.downcast_ref::<MeasureViewportMessage>().unwrap().width
    }

    #[test]
    fn events_raised_before_a_handler_is_added_are_replayed_to_the_first_handler_in_order() {
        let inner = FakeConnection::new(false);
        let wrapper = TransportConnectionWrapper::new(inner.clone());
        inner.raise_message(measure(1.0));
        inner.raise_message(measure(2.0));
        inner.raise_exception(Error::EndOfStream);

        let (sender, received) = channel();
        wrapper.on_message(message_handler(move |_, m| {
            let _ = sender.send(width_of(m));
        }));
        assert_eq!(received.try_recv(), Ok(1.0));
        assert_eq!(received.try_recv(), Ok(2.0));
        assert!(received.try_recv().is_err());

        // A second handler gets no replay; both get what is raised from now on.
        let (second_sender, second) = channel();
        wrapper.on_message(message_handler(move |_, m| {
            let _ = second_sender.send(width_of(m));
        }));
        assert!(second.try_recv().is_err());
        inner.raise_message(measure(3.0));
        assert_eq!(received.try_recv(), Ok(3.0));
        assert_eq!(second.try_recv(), Ok(3.0));

        let (error_sender, errors) = channel();
        wrapper.on_exception(exception_handler(move |_, e| {
            let _ = error_sender.send(e.clone());
        }));
        assert!(matches!(errors.try_recv(), Ok(Error::EndOfStream)));
        assert!(errors.try_recv().is_err());
    }

    #[test]
    fn handlers_receive_the_wrapper_as_the_connection() {
        let inner = FakeConnection::new(false);
        let wrapper = TransportConnectionWrapper::new(inner.clone());
        // A handler answers on the connection it is given: the answer goes
        // through the queue of the wrapper to the inner connection.
        wrapper.on_message(message_handler(|connection, m| {
            connection.send(measure(width_of(m) + 1.0));
        }));
        inner.raise_message(measure(1.0));
        let deadline = std::time::Instant::now() + TIMEOUT;
        while inner.sent.lock().unwrap().is_empty() {
            assert!(std::time::Instant::now() < deadline, "the answer was not sent");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(width_of(&inner.sent.lock().unwrap()[0]), 2.0);
    }

    #[test]
    fn a_removed_handler_is_not_called() {
        let inner = FakeConnection::new(false);
        let wrapper = TransportConnectionWrapper::new(inner.clone());
        let (sender, received) = channel();
        let token = wrapper.on_message(message_handler(move |_, m| {
            let _ = sender.send(width_of(m));
        }));
        inner.raise_message(measure(1.0));
        wrapper.remove_on_message(token);
        // Without a handler the event is stashed again.
        inner.raise_message(measure(2.0));
        assert_eq!(received.try_recv(), Ok(1.0));
        assert!(received.try_recv().is_err());
    }

    #[test]
    fn a_panic_of_a_handler_during_the_replay_is_raised_as_an_exception() {
        let inner = FakeConnection::new(false);
        let wrapper = TransportConnectionWrapper::new(inner.clone());
        inner.raise_message(measure(1.0));
        let (error_sender, errors) = channel();
        wrapper.on_exception(exception_handler(move |_, e| {
            let _ = error_sender.send(e.clone());
        }));
        wrapper.on_message(message_handler(|_, _| panic!("handler failed")));
        match errors.try_recv() {
            Ok(Error::Handler(message)) => assert_eq!(message, "handler failed"),
            other => panic!("expected the panic of the handler, got {:?}", other),
        }
    }

    #[test]
    fn sends_are_queued_and_sent_in_order() {
        let inner = FakeConnection::new(false);
        let wrapper = TransportConnectionWrapper::new(inner.clone());
        let tasks: Vec<Task> = (0..500).map(|i| wrapper.send(measure(i as f64))).collect();
        for task in &tasks {
            assert!(matches!(task.wait_timeout(TIMEOUT), Some(Ok(()))));
        }
        let sent = inner.sent.lock().unwrap();
        assert_eq!(sent.len(), 500);
        for (i, message) in sent.iter().enumerate() {
            assert_eq!(width_of(message), i as f64);
        }
    }

    #[test]
    fn a_failed_send_fails_its_task() {
        let inner = FakeConnection::new(true);
        let wrapper = TransportConnectionWrapper::new(inner);
        match wrapper.send(measure(1.0)).wait_timeout(TIMEOUT) {
            Some(Err(Error::InvalidOperation(message))) => assert_eq!(message, "refused"),
            other => panic!("expected the failure of the send, got {:?}", other),
        }
    }

    #[test]
    fn dispose_and_start_go_to_the_inner_connection() {
        let inner = FakeConnection::new(false);
        let wrapper = TransportConnectionWrapper::new(inner.clone());
        wrapper.start();
        wrapper.dispose();
        assert_eq!(inner.started.load(Ordering::SeqCst), 1);
        assert_eq!(inner.disposed.load(Ordering::SeqCst), 1);
    }
}
