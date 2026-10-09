//! A connection for the tests of this crate: it records what is sent and
//! raises what a test tells it to, on the thread of the test. (Not from
//! upstream.)

use ferroui_remote_protocol::{
    Delegate, Error, ExceptionHandler, HandlerToken, IFerroRemoteTransportConnection, Message, MessageHandler, Task,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub(crate) struct TestConnection {
    sent: Mutex<Vec<Message>>,
    on_message: Mutex<Delegate<Message>>,
    on_exception: Mutex<Delegate<Error>>,
    message_handlers: AtomicUsize,
    exception_handlers: AtomicUsize,
    disposed: AtomicUsize,
    started: AtomicUsize,
}

impl TestConnection {
    pub(crate) fn new() -> Arc<TestConnection> {
        Arc::new(TestConnection {
            sent: Mutex::new(Vec::new()),
            on_message: Mutex::new(Delegate::new()),
            on_exception: Mutex::new(Delegate::new()),
            message_handlers: AtomicUsize::new(0),
            exception_handlers: AtomicUsize::new(0),
            disposed: AtomicUsize::new(0),
            started: AtomicUsize::new(0),
        })
    }

    pub(crate) fn raise_message(&self, message: Message) {
        let handlers = self.on_message.lock().unwrap().snapshot();
        for handler in &handlers {
            handler(self, &message);
        }
    }

    pub(crate) fn raise_exception(&self, e: Error) {
        let handlers = self.on_exception.lock().unwrap().snapshot();
        for handler in &handlers {
            handler(self, &e);
        }
    }

    /// What was sent, in order.
    pub(crate) fn sent(&self) -> Vec<Message> {
        self.sent.lock().unwrap().clone()
    }

    /// What was sent of the class `T`, in order.
    pub(crate) fn sent_of<T: Clone + 'static>(&self) -> Vec<T> {
        self.sent().iter().filter_map(|message| message.downcast_ref::<T>().cloned()).collect()
    }

    pub(crate) fn message_handlers(&self) -> usize {
        self.message_handlers.load(Ordering::SeqCst)
    }

    pub(crate) fn exception_handlers(&self) -> usize {
        self.exception_handlers.load(Ordering::SeqCst)
    }

    pub(crate) fn disposed(&self) -> usize {
        self.disposed.load(Ordering::SeqCst)
    }

    pub(crate) fn started(&self) -> usize {
        self.started.load(Ordering::SeqCst)
    }
}

impl IFerroRemoteTransportConnection for TestConnection {
    fn dispose(&self) {
        self.disposed.fetch_add(1, Ordering::SeqCst);
    }

    fn send(&self, data: Message) -> Task {
        self.sent.lock().unwrap().push(data);
        Task::completed()
    }

    fn on_message(&self, handler: MessageHandler) -> HandlerToken {
        self.message_handlers.fetch_add(1, Ordering::SeqCst);
        self.on_message.lock().unwrap().add(handler)
    }

    fn remove_on_message(&self, token: HandlerToken) {
        self.message_handlers.fetch_sub(1, Ordering::SeqCst);
        self.on_message.lock().unwrap().remove(token);
    }

    fn on_exception(&self, handler: ExceptionHandler) -> HandlerToken {
        self.exception_handlers.fetch_add(1, Ordering::SeqCst);
        self.on_exception.lock().unwrap().add(handler)
    }

    fn remove_on_exception(&self, token: HandlerToken) {
        self.exception_handlers.fetch_sub(1, Ordering::SeqCst);
        self.on_exception.lock().unwrap().remove(token);
    }

    fn start(&self) {
        self.started.fetch_add(1, Ordering::SeqCst);
    }
}
