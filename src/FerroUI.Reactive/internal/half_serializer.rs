//! `HalfSerializer` (`Internal/HalfSerializer.cs`): serializes the values of one source with the
//! terminal notifications of others, for the operators whose end may come from a second source
//! (`TakeUntil`, `SkipUntil`, `RetryWhen`, `RepeatWhen`).

use ferroui_system::ObservableError;
use std::cell::{Cell, RefCell};

/// The forwarding of a sink, which the half serializer calls (`ISink<TTarget>`).
pub(crate) trait HalfSerializedSink<T> {
    fn forward_on_next(&self, value: T);
    fn forward_on_error(&self, error: ObservableError);
    fn forward_on_completed(&self);
}

/// The terminal state of a half serializer (`ref Exception? error`): `Terminated` is
/// `ExceptionHelper.Terminated`.
enum Terminal {
    None,
    Error(ObservableError),
    Terminated,
}

/// Serializes the values of one producer with the terminal notifications of others
/// (`HalfSerializer`, with its `wip` and `error` fields): a terminal notification that arrives
/// while a value is being forwarded is forwarded after it, and nothing is forwarded after a
/// terminal notification.
pub(crate) struct HalfSerializer {
    wip: Cell<usize>,
    error: RefCell<Terminal>,
}

impl HalfSerializer {
    pub(crate) const fn new() -> Self {
        Self { wip: Cell::new(0), error: RefCell::new(Terminal::None) }
    }

    /// `HalfSerializer.ForwardOnNext`.
    pub(crate) fn on_next<T, S: HalfSerializedSink<T> + ?Sized>(&self, sink: &S, value: T) {
        if self.wip.get() == 0 {
            self.wip.set(1);
            sink.forward_on_next(value);
            let wip = self.wip.get() - 1;
            self.wip.set(wip);
            if wip != 0 {
                let error = std::mem::replace(&mut *self.error.borrow_mut(), Terminal::Terminated);
                match error {
                    Terminal::Error(error) => sink.forward_on_error(error),
                    _ => sink.forward_on_completed(),
                }
            }
        }
        // Upstream traces a warning for a value that arrives while another one is forwarded.
    }

    /// `HalfSerializer.ForwardOnError`.
    pub(crate) fn on_error<T, S: HalfSerializedSink<T> + ?Sized>(&self, sink: &S, error: ObservableError) {
        if !self.try_set_terminal(Terminal::Error(error.clone())) {
            return;
        }
        let wip = self.wip.get() + 1;
        self.wip.set(wip);
        if wip == 1 {
            *self.error.borrow_mut() = Terminal::Terminated;
            sink.forward_on_error(error);
        }
    }

    /// `HalfSerializer.ForwardOnCompleted`.
    pub(crate) fn on_completed<T, S: HalfSerializedSink<T> + ?Sized>(&self, sink: &S) {
        if !self.try_set_terminal(Terminal::Terminated) {
            return;
        }
        let wip = self.wip.get() + 1;
        self.wip.set(wip);
        if wip == 1 {
            sink.forward_on_completed();
        }
    }

    /// `ExceptionHelper.TrySetException`: sets the terminal state unless one is set.
    fn try_set_terminal(&self, terminal: Terminal) -> bool {
        let mut error = self.error.borrow_mut();
        if !matches!(*error, Terminal::None) {
            return false;
        }
        *error = terminal;
        true
    }
}
