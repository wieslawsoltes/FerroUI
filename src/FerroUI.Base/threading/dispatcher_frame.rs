use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::{CancellationTokenSource, Dispatcher, IControlledDispatcherImpl};

#[derive(Default)]
struct FrameState {
    is_running: bool,
    cancellation_token_source: Option<CancellationTokenSource>,
}

/// Represents an execution loop in the [`Dispatcher`].
///
/// Frames are shared handles: [`set_continue`](Self::set_continue) can be
/// called from any thread.
pub struct DispatcherFrame {
    dispatcher: Arc<Dispatcher>,
    exit_when_requested: bool,
    continue_: AtomicBool,
    state: Mutex<FrameState>,
}

impl DispatcherFrame {
    /// Constructs a new instance of the frame for the current dispatcher.
    /// The frame exits when all frames are requested to exit.
    pub fn new() -> Arc<DispatcherFrame> {
        Self::with_exit_when_requested(true)
    }

    /// Constructs a new instance of the frame for the current dispatcher.
    ///
    /// `exit_when_requested` indicates whether or not this frame will exit
    /// when all frames are requested to exit. Dispatcher frames typically
    /// break down into two categories:
    ///
    /// 1. Long running, general purpose frames, that exit only when told to.
    ///    These frames should exit when requested.
    /// 2. Short running, very specific frames that exit themselves when an
    ///    important criteria is met. These frames may consider not exiting
    ///    when requested in favor of waiting for their important criteria to
    ///    be met. These frames should have a timeout associated with them.
    pub fn with_exit_when_requested(exit_when_requested: bool) -> Arc<DispatcherFrame> {
        let frame = Self::with_dispatcher(&Dispatcher::current_dispatcher(), exit_when_requested);
        frame.dispatcher.verify_access();
        frame
    }

    pub(crate) fn with_dispatcher(dispatcher: &Arc<Dispatcher>, exit_when_requested: bool) -> Arc<DispatcherFrame> {
        Arc::new(DispatcherFrame {
            dispatcher: dispatcher.clone(),
            exit_when_requested,
            continue_: AtomicBool::new(true),
            state: Mutex::new(FrameState::default()),
        })
    }

    /// The dispatcher this frame runs on.
    pub fn dispatcher(&self) -> &Arc<Dispatcher> {
        &self.dispatcher
    }

    fn lock(&self) -> MutexGuard<'_, FrameState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Indicates that this dispatcher frame should continue.
    pub fn continue_(&self) -> bool {
        // This method is free-threaded.

        // First check if this frame wants to continue.
        let mut should_continue = self.continue_.load(Ordering::SeqCst);
        if should_continue {
            // This frame wants to continue, so next check if it will
            // respect the "exit requests" from the dispatcher.
            if self.exit_when_requested {
                let dispatcher = &self.dispatcher;

                // This frame is willing to respect the "exit requests" of
                // the dispatcher, so check them.
                if dispatcher.exit_all_frames_requested() || dispatcher.has_shutdown_started() {
                    should_continue = false;
                }
            }
        }

        should_continue
    }

    /// Sets whether this dispatcher frame should continue. Setting it to
    /// `false` exits the loop the frame is running.
    pub fn set_continue(&self, value: bool) {
        // This method is free-threaded.
        let to_cancel = {
            let state = self.lock();
            self.continue_.store(value, Ordering::SeqCst);
            if !value {
                state.cancellation_token_source.clone()
            } else {
                None
            }
        };
        // Canceled outside of the lock: cancellation runs callbacks.
        if let Some(source) = to_cancel {
            source.cancel();
        }
    }

    pub(crate) fn run(&self, impl_: &dyn IControlledDispatcherImpl) {
        self.dispatcher.verify_access();

        // Since the actual platform run loop is controlled by a cancellation token, we have an
        // outer loop that restarts the platform one in case Continue was set to true after being set to false
        loop {
            let token = {
                let mut state = self.lock();
                if !self.continue_() {
                    return;
                }

                if state.is_running {
                    panic!("This frame is already running");
                }

                let source = CancellationTokenSource::new();
                let token = source.token();
                state.cancellation_token_source = Some(source);
                state.is_running = true;
                token
            };

            let result = catch_unwind(AssertUnwindSafe(|| {
                // Wake up the dispatcher in case it has pending jobs
                self.dispatcher.request_processing();
                impl_.run_loop(token);
            }));

            // finally
            let source = {
                let mut state = self.lock();
                state.is_running = false;
                state.cancellation_token_source.take()
            };
            if let Some(source) = source {
                source.cancel();
            }
            if let Err(payload) = result {
                resume_unwind(payload);
            }
        }
    }

    pub(crate) fn maybe_exit_on_dispatcher_request(&self) {
        if self.exit_when_requested {
            let source = self.lock().cancellation_token_source.clone();
            if let Some(source) = source {
                source.cancel();
            }
        }
    }
}
