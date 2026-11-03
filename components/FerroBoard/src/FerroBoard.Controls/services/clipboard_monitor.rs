//! Sampled native clipboard changes; the pinned platform API has no change event.
use ferroui_base::{
    input::{LocalBoxFuture, platform::ClipboardError},
    threading::{Dispatcher, DispatcherPriority, DispatcherTimer},
};
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll, Waker},
    time::Duration,
};
type Read = LocalBoxFuture<Result<Option<String>, ClipboardError>>;
struct State {
    timer: Rc<DispatcherTimer>,
    reader: Box<dyn Fn() -> Read>,
    changed: Box<dyn Fn(String)>,
    enabled: Cell<bool>,
    generation: Cell<u64>,
    reading: Cell<bool>,
    pending: RefCell<Option<Read>>,
    wake: RefCell<Option<Waker>>,
    primed: Cell<bool>,
    last_text: RefCell<Option<String>>,
    last_error: RefCell<Option<ClipboardError>>,
}
/// UI-thread clipboard sampling with a 500 ms interval and at most one pending read.
///
/// The first successful read establishes a baseline. Empty/nontext contents never emit a change.
/// Disable this during private/password/incognito input or when history is disabled. Re-enabling
/// establishes a new baseline so text copied while disabled is not recorded later. Drop stops the
/// timer and drops any pending read. Sampling can miss transient and repeated identical copies.
pub struct ClipboardMonitor {
    state: Rc<State>,
}
impl ClipboardMonitor {
    pub fn new(reader: impl Fn() -> Read + 'static, changed: impl Fn(String) + 'static) -> Self {
        let timer = DispatcherTimer::with_interval(
            Duration::from_millis(500),
            DispatcherPriority::BACKGROUND,
            &Dispatcher::current_dispatcher(),
        );
        let state = Rc::new(State {
            timer,
            reader: Box::new(reader),
            changed: Box::new(changed),
            enabled: Cell::new(false),
            generation: Cell::new(0),
            reading: Cell::new(false),
            pending: RefCell::new(None),
            wake: RefCell::new(None),
            primed: Cell::new(false),
            last_text: RefCell::new(None),
            last_error: RefCell::new(None),
        });
        let weak = Rc::downgrade(&state);
        state.timer.tick(move |_| {
            if let Some(state) = weak.upgrade() {
                State::refresh(&state);
            }
        });
        Self { state }
    }
    pub fn set_enabled(&self, enabled: bool) {
        if self.state.enabled.replace(enabled) == enabled {
            return;
        }
        if enabled {
            self.state.timer.start();
            State::refresh(&self.state);
        } else {
            self.state.stop();
        }
    }
    pub fn is_enabled(&self) -> bool {
        self.state.enabled.get()
    }
    /// Requests a sample now; also useful when a native activation/change notification is available.
    pub fn refresh(&self) {
        State::refresh(&self.state);
    }
    pub fn last_error(&self) -> Option<ClipboardError> {
        self.state.last_error.borrow().clone()
    }
}
impl Drop for ClipboardMonitor {
    fn drop(&mut self) {
        self.state.enabled.set(false);
        self.state.stop();
    }
}
impl State {
    fn stop(&self) {
        self.timer.stop();
        self.generation.set(self.generation.get().wrapping_add(1));
        self.reading.set(false);
        self.primed.set(false);
        self.last_text.borrow_mut().take();
        let pending = self.pending.borrow_mut().take();
        let wake = self.wake.borrow_mut().take();
        drop(pending);
        if let Some(wake) = wake {
            wake.wake();
        }
    }
    fn refresh(state: &Rc<Self>) {
        if !state.enabled.get() || state.reading.replace(true) {
            return;
        }
        let generation = state.generation.get();
        let read = (state.reader)();
        // A native reader may synchronously reenter the owner and disable monitoring.
        if !state.enabled.get() || generation != state.generation.get() {
            return;
        }
        *state.pending.borrow_mut() = Some(read);
        let weak = Rc::downgrade(state);
        drop(
            Dispatcher::current_dispatcher().invoke_async_task_local_with_priority(
                move || ReadSample {
                    state: weak,
                    generation,
                },
                DispatcherPriority::BACKGROUND,
            ),
        );
    }
    fn accept(&self, result: Result<Option<String>, ClipboardError>) {
        match result {
            Ok(text) => {
                self.last_error.borrow_mut().take();
                let previous = self.last_text.replace(text.clone());
                let was_primed = self.primed.replace(true);
                if was_primed
                    && previous != text
                    && let Some(text) = text.filter(|text| !text.is_empty())
                {
                    (self.changed)(text);
                }
            }
            Err(error) => {
                *self.last_error.borrow_mut() = Some(error);
                // Access can return after private/background activity. Do not replay that gap.
                self.primed.set(false);
                self.last_text.borrow_mut().take();
            }
        }
    }
}
struct ReadSample {
    state: Weak<State>,
    generation: u64,
}
impl Future for ReadSample {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let Some(state) = self.state.upgrade() else {
            return Poll::Ready(());
        };
        if !state.enabled.get() || self.generation != state.generation.get() {
            return Poll::Ready(());
        }
        *state.wake.borrow_mut() = Some(cx.waker().clone());
        let Some(mut read) = state.pending.borrow_mut().take() else {
            return Poll::Ready(());
        };
        let result = read.as_mut().poll(cx);
        if !state.enabled.get() || self.generation != state.generation.get() {
            return Poll::Ready(());
        }
        match result {
            Poll::Pending => {
                *state.pending.borrow_mut() = Some(read);
                Poll::Pending
            }
            Poll::Ready(result) => {
                state.wake.borrow_mut().take();
                state.reading.set(false);
                state.accept(result);
                Poll::Ready(())
            }
        }
    }
}
