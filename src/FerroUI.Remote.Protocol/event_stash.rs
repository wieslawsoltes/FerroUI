use std::sync::{Mutex, Weak};

use crate::error::{catch_handler, Error};
use crate::i_transport::{Delegate, Handler, HandlerToken, IFerroRemoteTransportConnection};

struct State<T> {
    stash: Option<Vec<T>>,
    delegate: Delegate<T>,
}

/// An event that keeps what is fired while it has no handler and hands it to
/// the first handler that is added.
pub(crate) struct EventStash<T> {
    transport: Weak<dyn IFerroRemoteTransportConnection>,
    exception_handler: Option<Box<dyn Fn(Error) + Send + Sync>>,
    // `lock (this)`.
    state: Mutex<State<T>>,
}

impl<T> EventStash<T> {
    pub(crate) fn new(
        transport: Weak<dyn IFerroRemoteTransportConnection>,
        exception_handler: Option<Box<dyn Fn(Error) + Send + Sync>>,
    ) -> EventStash<T> {
        EventStash { transport, exception_handler, state: Mutex::new(State { stash: None, delegate: Delegate::new() }) }
    }

    pub(crate) fn add(&self, handler: Handler<T>) -> HandlerToken {
        let token;
        let stash;
        {
            let mut state = self.state.lock().unwrap();
            let needs_replay = state.delegate.is_null();
            token = state.delegate.add(handler);
            if !needs_replay {
                return token;
            }

            stash = match state.stash.take() {
                Some(stash) => stash,
                None => return token,
            };
        }
        // The transport is the connection this stash belongs to, which is
        // alive while one of its members is being called.
        let Some(transport) = self.transport.upgrade() else {
            return token;
        };
        for m in stash {
            let delegate = self.state.lock().unwrap().delegate.snapshot();
            if let Some(exception_handler) = &self.exception_handler {
                // `try { _delegate?.Invoke(_transport, m); } catch (Exception e) { _exceptionHandler(e); }`:
                // an exception of a handler is a panic of one here.
                let result = catch_handler(|| {
                    for handler in &delegate {
                        handler(&*transport, &m);
                    }
                });
                if let Err(e) = result {
                    exception_handler(e);
                }
            } else {
                for handler in &delegate {
                    handler(&*transport, &m);
                }
            }
        }
        token
    }

    pub(crate) fn remove(&self, token: HandlerToken) {
        self.state.lock().unwrap().delegate.remove(token);
    }

    /// `Fire(transport, ev)`. The original ignores the transport it is
    /// given and passes the one of the stash to the handlers, so the port
    /// takes the event only. The original also reads the delegate outside of
    /// its lock; here the test and the stashing are one step.
    pub(crate) fn fire(&self, ev: T) {
        let delegate;
        {
            let mut state = self.state.lock().unwrap();
            if state.delegate.is_null() {
                state.stash.get_or_insert_with(Vec::new).push(ev);
                return;
            }
            delegate = state.delegate.snapshot();
        }
        // Without its connection there is nobody to tell.
        let Some(transport) = self.transport.upgrade() else {
            return;
        };
        for handler in &delegate {
            handler(&*transport, &ev);
        }
    }
}
