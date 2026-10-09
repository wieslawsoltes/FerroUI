//! The contract of a connection that carries the messages of the protocol.
//!
//! # Threading contract
//!
//! The original is asynchronous: `Send` returns a `Task`, the streams are
//! read and written with `async` calls and the events are raised on threads
//! of the thread pool. This library has no asynchronous runtime and gives
//! the following contract instead (DEVIATIONS.md, Remote protocol):
//!
//! - A connection is shared between threads (`Arc<dyn
//!   IFerroRemoteTransportConnection>`, `Send + Sync`) and every member may
//!   be called from any thread.
//! - Each connection that reads has one reader thread. The message event and
//!   the exception event of a failed read are raised on it, one event at a
//!   time and in the order of arrival; the next message is not read before
//!   the handlers of the previous one have returned. The exception event of
//!   a failed write is raised on the thread that wrote. A handler that is
//!   added to a wrapped connection while events are stashed receives them on
//!   the thread that adds it.
//! - Handlers are therefore `Send + Sync` and must not block for long. Code
//!   that belongs to another thread (the UI thread) posts to it from the
//!   handler, as the callers in the original do.
//! - [`IFerroRemoteTransportConnection::send`] returns a [`Task`]. A stream
//!   connection writes on the calling thread and returns the task completed;
//!   a wrapped connection queues the message for its sender thread and
//!   returns at once. [`Task::wait`] blocks the calling thread.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::error::Error;
use crate::metsys_bson::BsonObject;
use crate::task::Task;

/// A message of the protocol: `object` in the signatures of the original.
/// The class of a received message is found with
/// `message.downcast_ref::<MeasureViewportMessage>()`.
pub type Message = Arc<dyn BsonObject>;

/// `Action<IFerroRemoteTransportConnection, T>`: a handler of an event of a
/// connection.
pub type Handler<T> = Arc<dyn Fn(&dyn IFerroRemoteTransportConnection, &T) + Send + Sync>;

/// A handler of the message event.
pub type MessageHandler = Handler<Message>;

/// A handler of the exception event.
pub type ExceptionHandler = Handler<Error>;

/// Makes a handler of the message event from a closure.
pub fn message_handler(
    handler: impl Fn(&dyn IFerroRemoteTransportConnection, &Message) + Send + Sync + 'static,
) -> MessageHandler {
    Arc::new(handler)
}

/// Makes a handler of the exception event from a closure.
pub fn exception_handler(
    handler: impl Fn(&dyn IFerroRemoteTransportConnection, &Error) + Send + Sync + 'static,
) -> ExceptionHandler {
    Arc::new(handler)
}

/// What adding a handler returns and removing it takes: the identity of a
/// delegate in `event -= handler`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HandlerToken(u64);

impl HandlerToken {
    fn next() -> HandlerToken {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        HandlerToken(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// A multicast delegate: the handlers of an event in the order they were
/// added. Public for the connections that are implemented outside this
/// library (the ones of the designer support), which keep their handlers in
/// one as the connections here do.
pub struct Delegate<T> {
    handlers: Vec<(HandlerToken, Handler<T>)>,
}

impl<T> Delegate<T> {
    pub fn new() -> Delegate<T> {
        Delegate { handlers: Vec::new() }
    }

    /// `_delegate == null`.
    pub fn is_null(&self) -> bool {
        self.handlers.is_empty()
    }

    /// `_delegate += handler`.
    pub fn add(&mut self, handler: Handler<T>) -> HandlerToken {
        let token = HandlerToken::next();
        self.handlers.push((token, handler));
        token
    }

    /// `_delegate -= handler`.
    pub fn remove(&mut self, token: HandlerToken) {
        self.handlers.retain(|(t, _)| *t != token);
    }

    /// The handlers as they are now, to invoke them outside a lock.
    pub fn snapshot(&self) -> Vec<Handler<T>> {
        self.handlers.iter().map(|(_, handler)| handler.clone()).collect()
    }
}

impl<T> Default for Delegate<T> {
    fn default() -> Self {
        Delegate::new()
    }
}

/// A connection to the other end of the protocol.
pub trait IFerroRemoteTransportConnection: Send + Sync {
    /// `IDisposable.Dispose`: closes the connection. No exception event
    /// follows from the closing itself.
    fn dispose(&self);
    /// Sends a message. The task completes when the message was written (or
    /// dropped because the connection is broken) and fails when the message
    /// cannot be sent as it is: its class is not known to the resolver, or
    /// it cannot be serialized.
    fn send(&self, data: Message) -> Task;
    /// `OnMessage += handler`.
    fn on_message(&self, handler: MessageHandler) -> HandlerToken;
    /// `OnMessage -= handler`.
    fn remove_on_message(&self, token: HandlerToken);
    /// `OnException += handler`.
    fn on_exception(&self, handler: ExceptionHandler) -> HandlerToken;
    /// `OnException -= handler`.
    fn remove_on_exception(&self, token: HandlerToken);
    fn start(&self);
}
