//! `DelayableNotificationSignal<T>` (ReactiveUI.Primitives, `Signals/DelayableNotificationSignal{T}.cs`):
//! the subject behind the change notifications of a reactive object, which forwards a value at once
//! unless the notifications are delayed, and then keeps it until [`flush`](DelayableNotificationSignal::flush).
//!
//! It comes from ReactiveUI.Primitives, which both flavours of ReactiveUI share; it is ported here,
//! beside its only user ([`ExtensionState`](super::extension_state::ExtensionState)).

use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Forwards notifications at once unless delayed; while delayed it buffers them, and
/// [`flush`](Self::flush) emits the buffered batch with its duplicates removed.
pub(crate) struct DelayableNotificationSignal<T> {
    /// Whether the notifications are delayed now (`_isDelayed`).
    is_delayed: Box<dyn Fn() -> bool>,
    /// Removes the duplicates of a buffered batch before it is emitted (`_flushDistinct`).
    flush_distinct: Box<dyn Fn(Vec<T>) -> Vec<T>>,
    /// The observers (`_broadcaster`).
    broadcaster: Rc<Subject<T>>,
    /// The notifications kept while delayed; `None` until the first one after a flush (`_buffer`).
    buffer: RefCell<Option<Vec<T>>>,
    /// Whether the signal has ended (`_stopped`).
    stopped: Cell<bool>,
}

impl<T: Clone + 'static> DelayableNotificationSignal<T> {
    /// A signal whose delay is `is_delayed` and whose batches `flush_distinct` reduces.
    pub(crate) fn new(is_delayed: impl Fn() -> bool + 'static, flush_distinct: impl Fn(Vec<T>) -> Vec<T> + 'static) -> Rc<Self> {
        Rc::new(Self {
            is_delayed: Box::new(is_delayed),
            flush_distinct: Box::new(flush_distinct),
            broadcaster: Subject::new(),
            buffer: RefCell::new(None),
            stopped: Cell::new(false),
        })
    }

    /// Whether observers are subscribed (`HasObservers`).
    #[allow(dead_code)]
    pub(crate) fn has_observers(&self) -> bool {
        self.broadcaster.has_observers() && !self.stopped.get()
    }

    /// Emits the buffered notifications as one batch without duplicates; called when a delay
    /// starts or ends (`Flush`).
    pub(crate) fn flush(&self) {
        if self.stopped.get() {
            return;
        }
        let Some(buffered) = self.buffer.borrow_mut().take() else {
            return;
        };
        for item in (self.flush_distinct)(buffered) {
            self.broadcaster.on_next(item);
        }
    }
}

impl<T: Clone + 'static> IObserver<T> for DelayableNotificationSignal<T> {
    fn on_next(&self, value: T) {
        let delayed = (self.is_delayed)();
        if self.stopped.get() {
            return;
        }
        if delayed {
            self.buffer.borrow_mut().get_or_insert_with(Vec::new).push(value);
            return;
        }
        self.broadcaster.on_next(value);
    }

    fn on_error(&self, error: ObservableError) {
        if self.stopped.replace(true) {
            return;
        }
        self.broadcaster.on_error(error);
    }

    fn on_completed(&self) {
        if self.stopped.replace(true) {
            return;
        }
        self.broadcaster.on_completed();
    }
}

impl<T: Clone + 'static> IObservable<T> for DelayableNotificationSignal<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        // The subject keeps the end and replays it to a late observer, as upstream does.
        self.broadcaster.subscribe(observer)
    }
}
