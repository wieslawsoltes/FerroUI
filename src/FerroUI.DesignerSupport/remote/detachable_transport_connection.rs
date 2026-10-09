use ferroui_remote_protocol::{
    message_handler, Delegate, Error, ExceptionHandler, HandlerToken, IFerroRemoteTransportConnection, Message,
    MessageHandler, Task,
};
use std::sync::{Arc, Mutex, PoisonError, Weak};

/// A connection around another one that can be cut off from it: after
/// `dispose` the messages of the inner connection no longer reach the
/// handlers of this one, and what is sent goes nowhere. The previewer gives
/// one to each window, so that the window it replaces stops listening to the
/// connection of the session.
pub(crate) struct DetachableTransportConnection {
    inner: Mutex<Option<Arc<dyn IFerroRemoteTransportConnection>>>,
    // The handler added to the inner connection (`FireOnMessage` as a
    // delegate, which `Dispose` removes).
    inner_token: HandlerToken,
    on_message: Mutex<Delegate<Message>>,
    // `OnException { add {} remove {} }`: the handlers are accepted and
    // never called.
    on_exception: Mutex<Delegate<Error>>,
}

impl DetachableTransportConnection {
    pub(crate) fn new(inner: Arc<dyn IFerroRemoteTransportConnection>) -> Arc<DetachableTransportConnection> {
        Arc::new_cyclic(|this: &Weak<DetachableTransportConnection>| {
            let this = this.clone();
            let inner_token = inner.on_message(message_handler(move |transport, obj| {
                if let Some(this) = this.upgrade() {
                    this.fire_on_message(transport, obj);
                }
            }));
            DetachableTransportConnection {
                inner: Mutex::new(Some(inner)),
                inner_token,
                on_message: Mutex::new(Delegate::new()),
                on_exception: Mutex::new(Delegate::new()),
            }
        })
    }

    fn inner(&self) -> Option<Arc<dyn IFerroRemoteTransportConnection>> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    pub(crate) fn fire_on_message(&self, transport: &dyn IFerroRemoteTransportConnection, obj: &Message) {
        let handlers = self.on_message.lock().unwrap_or_else(PoisonError::into_inner).snapshot();
        for handler in &handlers {
            handler(transport, obj);
        }
    }
}

impl IFerroRemoteTransportConnection for DetachableTransportConnection {
    fn dispose(&self) {
        let inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(inner) = inner {
            inner.remove_on_message(self.inner_token);
        }
    }

    /// The original returns no task at all once the connection is detached;
    /// here the task of a message that goes nowhere is a completed one.
    fn send(&self, data: Message) -> Task {
        match self.inner() {
            Some(inner) => inner.send(data),
            None => Task::completed(),
        }
    }

    fn on_message(&self, handler: MessageHandler) -> HandlerToken {
        self.on_message.lock().unwrap_or_else(PoisonError::into_inner).add(handler)
    }

    fn remove_on_message(&self, token: HandlerToken) {
        self.on_message.lock().unwrap_or_else(PoisonError::into_inner).remove(token);
    }

    fn on_exception(&self, handler: ExceptionHandler) -> HandlerToken {
        self.on_exception.lock().unwrap_or_else(PoisonError::into_inner).add(handler)
    }

    fn remove_on_exception(&self, token: HandlerToken) {
        self.on_exception.lock().unwrap_or_else(PoisonError::into_inner).remove(token);
    }

    fn start(&self) {
        if let Some(inner) = self.inner() {
            inner.start();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use crate::remote::test_connection::TestConnection;
    use ferroui_remote_protocol::viewport::MeasureViewportMessage;
    use ferroui_remote_protocol::exception_handler;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn measure(width: f64) -> Message {
        Arc::new(MeasureViewportMessage { width, height: 0.0 })
    }

    #[test]
    fn messages_and_sends_pass_until_the_connection_is_detached() {
        let inner = TestConnection::new();
        let detachable = DetachableTransportConnection::new(inner.clone());
        let received = Arc::new(AtomicUsize::new(0));
        let counter = received.clone();
        let token = detachable.on_message(message_handler(move |_, _| {
            counter.fetch_add(1, Ordering::SeqCst);
        }));

        inner.raise_message(measure(1.0));
        assert_eq!(1, received.load(Ordering::SeqCst));
        assert!(detachable.send(measure(2.0)).is_completed());
        assert_eq!(1, inner.sent().len());
        detachable.start();
        assert_eq!(1, inner.started());

        // What is fired by hand reaches the handlers too (the messages the
        // previewer replays to a new window).
        detachable.fire_on_message(&*detachable, &measure(3.0));
        assert_eq!(2, received.load(Ordering::SeqCst));

        detachable.dispose();
        assert_eq!(0, inner.message_handlers());
        inner.raise_message(measure(4.0));
        assert_eq!(2, received.load(Ordering::SeqCst));
        assert!(detachable.send(measure(5.0)).is_completed());
        assert_eq!(1, inner.sent().len());
        detachable.start();
        assert_eq!(1, inner.started());
        // The inner connection is not disposed, and a second dispose does nothing.
        detachable.dispose();
        assert_eq!(0, inner.disposed());

        detachable.remove_on_message(token);
        detachable.fire_on_message(&*detachable, &measure(6.0));
        assert_eq!(2, received.load(Ordering::SeqCst));
    }

    #[test]
    fn exception_handlers_are_accepted_and_never_called() {
        let inner = TestConnection::new();
        let detachable = DetachableTransportConnection::new(inner.clone());
        let token = detachable.on_exception(exception_handler(|_, _| panic!("never called")));
        inner.raise_exception(Error::EndOfStream);
        assert_eq!(0, inner.exception_handlers());
        detachable.remove_on_exception(token);
    }
}
