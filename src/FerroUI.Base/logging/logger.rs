use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError, RwLock};

use super::parametrized_logger::SinkHandle;
use super::{ILogSink, LogEventLevel, ParametrizedLogger};

static SINK: RwLock<Option<Arc<dyn ILogSink + Send + Sync>>> = RwLock::new(None);
static HAS_SINK: AtomicBool = AtomicBool::new(false);

thread_local! {
    static THREAD_SINK: RefCell<Option<Rc<dyn ILogSink>>> = const { RefCell::new(None) };
    static HAS_THREAD_SINK: Cell<bool> = const { Cell::new(false) };
}

/// Logs events.
///
/// The sink is process-wide and therefore has to be thread-safe. In
/// addition, a thread can install a sink of its own with
/// [`set_thread_sink`](Self::set_thread_sink); it takes precedence over the
/// process-wide sink on that thread, does not have to be thread-safe, and is
/// what unit tests (which run in parallel) use to observe log events.
pub struct Logger;

impl Logger {
    /// Gets the application-defined sink for logging.
    pub fn sink() -> Option<Arc<dyn ILogSink + Send + Sync>> {
        if !HAS_SINK.load(Ordering::Acquire) {
            return None;
        }
        SINK.read().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Sets the application-defined sink for logging.
    pub fn set_sink(sink: Option<Arc<dyn ILogSink + Send + Sync>>) {
        let previous = {
            let mut slot = SINK.write().unwrap_or_else(PoisonError::into_inner);
            HAS_SINK.store(sink.is_some(), Ordering::Release);
            std::mem::replace(&mut *slot, sink)
        };
        drop(previous);
    }

    /// Gets the sink of the calling thread, if it has one.
    pub fn thread_sink() -> Option<Rc<dyn ILogSink>> {
        if !HAS_THREAD_SINK.try_with(Cell::get).unwrap_or(false) {
            return None;
        }
        THREAD_SINK.try_with(|sink| sink.borrow().clone()).ok().flatten()
    }

    /// Sets a sink that receives the events logged on the calling thread
    /// instead of the process-wide sink. Returns the previous thread sink.
    pub fn set_thread_sink(sink: Option<Rc<dyn ILogSink>>) -> Option<Rc<dyn ILogSink>> {
        HAS_THREAD_SINK.with(|has| has.set(sink.is_some()));
        THREAD_SINK.with(|slot| std::mem::replace(&mut *slot.borrow_mut(), sink))
    }

    fn current_sink() -> Option<SinkHandle> {
        if let Some(sink) = Self::thread_sink() {
            return Some(SinkHandle::Thread(sink));
        }
        Self::sink().map(SinkHandle::Global)
    }

    /// Checks if given log level is enabled.
    pub fn is_enabled(level: LogEventLevel, area: &str) -> bool {
        match Self::current_sink() {
            Some(sink) => sink.get().is_enabled(level, area),
            None => false,
        }
    }

    /// Returns a parametrized logging sink if given log level is enabled.
    pub fn try_get(level: LogEventLevel, area: &'static str) -> Option<ParametrizedLogger> {
        let sink = Self::current_sink()?;
        if !sink.get().is_enabled(level, area) {
            return None;
        }

        Some(ParametrizedLogger::from_handle(sink, level, area))
    }
}
