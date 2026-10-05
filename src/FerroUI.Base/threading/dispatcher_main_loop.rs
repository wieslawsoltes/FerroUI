use std::cell::Cell;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::{CancellationToken, Dispatcher, DispatcherFrame, DispatcherPriority, FerroSynchronizationContext};
use crate::reactive::{Disposable, IDisposable};

impl Dispatcher {
    pub(crate) fn exit_all_frames_requested(&self) -> bool {
        self.exit_all_frames_requested.load(Ordering::SeqCst)
    }

    /// Raised when the dispatcher is shutting down.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn shutdown_started(&self, handler: impl Fn(&Arc<Dispatcher>) + 'static) -> Rc<dyn IDisposable> {
        self.verify_access();
        let Some(local) = self.try_local() else {
            return Disposable::empty();
        };
        let token = local.shutdown_started.add(Rc::new(handler));
        let local = Rc::downgrade(&local);
        Disposable::create(move || {
            if let Some(local) = local.upgrade() {
                local.shutdown_started.remove(token);
            }
        })
    }

    /// Raised when the dispatcher is shut down.
    ///
    /// # Panics
    /// Panics when called from a thread other than the dispatcher thread.
    pub fn shutdown_finished(&self, handler: impl Fn(&Arc<Dispatcher>) + 'static) -> Rc<dyn IDisposable> {
        self.verify_access();
        let Some(local) = self.try_local() else {
            return Disposable::empty();
        };
        let token = local.shutdown_finished.add(Rc::new(handler));
        let local = Rc::downgrade(&local);
        Disposable::create(move || {
            if let Some(local) = local.upgrade() {
                local.shutdown_finished.remove(token);
            }
        })
    }

    /// Push an execution frame.
    ///
    /// # Panics
    /// Panics when the platform implementation does not support run loops
    /// (see [`supports_run_loops`](Self::supports_run_loops)), when the
    /// dispatcher has shut down, when processing is disabled and when called
    /// from another thread.
    pub fn push_frame(&self, frame: &Arc<DispatcherFrame>) {
        self.verify_access();
        let impl_ = {
            let impl_ = self.platform_impl();
            let st = self.lock();
            if impl_.as_controlled().is_none() {
                panic!("Operation is not supported on this platform: the dispatcher cannot run nested loops.");
            }

            if st.has_shutdown_finished {
                // Dispatcher thread - no lock needed for read
                panic!("Cannot perform requested operation because the Dispatcher shut down");
            }

            if st.disabled_processing_count > 0 {
                panic!("Cannot perform this operation while dispatcher processing is suspended.");
            }
            impl_
        };

        self.lock().frames.push(frame.clone());
        let result = catch_unwind(AssertUnwindSafe(|| {
            if let Some(controlled) = impl_.as_controlled() {
                let _restore = FerroSynchronizationContext::ensure_with_dispatcher(self, DispatcherPriority::NORMAL);
                frame.run(controlled);
            }
        }));

        // finally
        let no_frames_left = {
            let mut st = self.lock();
            st.frames.pop();
            st.frames.is_empty()
        };
        if no_frames_left {
            if self.has_shutdown_started() {
                self.shutdown_impl();
            } else {
                self.exit_all_frames_requested.store(false, Ordering::SeqCst);
            }
        }
        if let Err(payload) = result {
            resume_unwind(payload);
        }
    }

    /// Runs the dispatcher's main loop until `cancellation_token` is
    /// canceled.
    ///
    /// # Panics
    /// See [`push_frame`](Self::push_frame).
    pub fn main_loop(&self, cancellation_token: &CancellationToken) {
        if !self.supports_run_loops() {
            panic!("Operation is not supported on this platform: the dispatcher cannot run loops.");
        }
        self.verify_access();
        let frame = DispatcherFrame::with_dispatcher(&self.to_arc(), true);
        let to_stop = frame.clone();
        let registration = cancellation_token.register(move || to_stop.set_continue(false));
        let result = catch_unwind(AssertUnwindSafe(|| self.push_frame(&frame)));
        // The frame is done; a long-lived token must not keep it alive.
        registration.dispose();
        if let Err(payload) = result {
            resume_unwind(payload);
        }
    }

    /// Requests that all nested frames exit.
    pub fn exit_all_frames(&self) {
        let frames = self.lock().frames.clone();
        if frames.is_empty() {
            return;
        }
        self.exit_all_frames_requested.store(true, Ordering::SeqCst);
        for frame in frames.iter().rev() {
            frame.maybe_exit_on_dispatcher_request();
        }
    }

    /// Begins the process of shutting down the dispatcher.
    pub fn begin_invoke_shutdown(&self, priority: DispatcherPriority) {
        let dispatcher = self.to_arc();
        self.post(move || dispatcher.start_shutdown_impl(), priority);
    }

    /// Initiates the shutdown process of the dispatcher synchronously.
    pub fn invoke_shutdown(&self) {
        let dispatcher = self.to_arc();
        // The dispatcher cannot be shut down twice; an aborted invoke means
        // it already is.
        let _ = self.invoke_with_priority(move || dispatcher.start_shutdown_impl(), DispatcherPriority::SEND);
    }

    fn start_shutdown_impl(&self) {
        {
            let mut st = self.lock();
            if st.starting_shutdown {
                return;
            }
            // We only need this to prevent reentrancy if the shutdown_started event
            // tries to shut down again.
            st.starting_shutdown = true;
        }

        // Call the shutdown_started event before we actually mark ourselves
        // as shutting down.  This is so the handlers can actually do work
        // when they get this event without panicking.
        if let Some(local) = self.try_local() {
            let dispatcher = self.to_arc();
            for (_, handler) in local.shutdown_started.snapshot().iter() {
                handler(&dispatcher);
            }
        }

        self.has_shutdown_started.store(true, Ordering::SeqCst);

        let has_frames = !self.lock().frames.is_empty();
        if has_frames {
            self.exit_all_frames();
        } else {
            self.shutdown_impl();
        }
    }

    pub(crate) fn shutdown_impl(&self) {
        let local = self.try_local();
        if let Some(local) = &local {
            Self::detach_implementation(local);
        }
        loop {
            let operation = {
                let mut st = self.lock();
                if st.queue.max_priority() != DispatcherPriority::INVALID {
                    st.queue.peek()
                } else {
                    if let Some(local) = &local {
                        local.impl_.borrow().update_timer(None);
                    }
                    st.has_shutdown_finished = true;
                    None
                }
            };

            match operation {
                Some(operation) => {
                    operation.abort();
                }
                None => break,
            }
        }

        // Nothing can resume the futures running on this dispatcher anymore:
        // cancel them, which releases whoever waits for them.
        self.cancel_tasks();

        if let Some(local) = local {
            let dispatcher = self.to_arc();
            for (_, handler) in local.shutdown_finished.snapshot().iter() {
                handler(&dispatcher);
            }
        }
    }

    /// Disable the event processing of the dispatcher.
    ///
    /// This is an advanced method intended to eliminate the chance of
    /// unrelated reentrancy. While processing is disabled no one is allowed
    /// to push a frame and no job processing is permitted. Processing is
    /// enabled again by disposing the returned value.
    pub fn disable_processing(&self) -> DispatcherProcessingDisabled {
        self.verify_access();

        // Turn off processing.
        self.lock().disabled_processing_count += 1;
        DispatcherProcessingDisabled { dispatcher: Cell::new(Some(self.to_arc())) }
    }
}

/// Re-enables dispatcher processing when disposed; see
/// [`Dispatcher::disable_processing`].
pub struct DispatcherProcessingDisabled {
    dispatcher: Cell<Option<Arc<Dispatcher>>>,
}

impl IDisposable for DispatcherProcessingDisabled {
    fn dispose(&self) {
        let Some(dispatcher) = self.dispatcher.take() else {
            return;
        };
        dispatcher.lock().disabled_processing_count -= 1;
    }
}
