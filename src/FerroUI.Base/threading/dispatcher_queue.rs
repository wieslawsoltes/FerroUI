use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::dispatcher::DispatcherState;
use super::dispatcher_operation::OperationCore;
use super::{CancellationToken, Dispatcher, DispatcherOperationStatus, DispatcherPriority, IDispatcherImpl};

impl Dispatcher {
    pub(crate) const MAXIMUM_INPUT_STARVATION_TIME_IN_FALLBACK_MODE: i64 = 50;
    pub(crate) const MAXIMUM_INPUT_STARVATION_TIME_IN_EXPLICIT_PROCESSING_EXPLICIT_MODE: i64 = 50;

    pub(crate) fn request_background_processing_locked(&self, st: &mut DispatcherState) {
        // Dispatcher thread only.
        let Some(impl_) = self.local_impl() else {
            return;
        };
        if let Some(background) = impl_.as_explicit_background_processing() {
            if st.explicit_background_processing_requested {
                return;
            }
            st.explicit_background_processing_requested = true;
            background.request_background_processing();
        } else if st.due_time_for_background_processing.is_none() {
            st.due_time_for_background_processing = Some(impl_.now() + 1);
            self.update_os_timer_locked(st);
        }
    }

    pub(crate) fn on_ready_for_explicit_background_processing(&self) {
        self.lock().explicit_background_processing_requested = false;
        self.execute_jobs_core(true);
    }

    /// Force-runs all dispatcher operations ignoring any pending OS events,
    /// use with caution.
    ///
    /// `None` runs every active job.
    pub fn run_jobs(&self, priority: Option<DispatcherPriority>) {
        self.run_jobs_with_cancellation(priority, &CancellationToken::none());
    }

    pub(crate) fn run_jobs_with_cancellation(
        &self,
        priority: Option<DispatcherPriority>,
        cancellation_token: &CancellationToken,
    ) {
        if self.lock().disabled_processing_count > 0 {
            panic!("Cannot perform this operation while dispatcher processing is suspended.");
        }

        let mut priority = priority.unwrap_or(DispatcherPriority::MINIMUM_ACTIVE_VALUE);
        if priority < DispatcherPriority::MINIMUM_ACTIVE_VALUE {
            priority = DispatcherPriority::MINIMUM_ACTIVE_VALUE;
        }
        while !cancellation_token.is_cancellation_requested() {
            let job = self.lock().queue.peek();
            let Some(job) = job else {
                return;
            };
            if job.priority() < priority {
                return;
            }
            self.execute_job(&job);
        }
    }

    pub(crate) fn execute_job(&self, job: &Arc<OperationCore>) {
        let dead_local_operations = {
            let mut st = self.lock();
            if job.status() != DispatcherOperationStatus::Pending {
                return;
            }

            st.queue.remove_item(job);
            job.set_status(DispatcherOperationStatus::Executing);
            std::mem::take(&mut st.dead_local_operations)
        };
        for operation_id in dead_local_operations {
            self.discard_local_operation(operation_id);
        }

        job.execute();
        // The backend might be firing timers with a low priority,
        // so we manually check if our high priority timers are due for execution
        self.promote_timers();
    }

    pub(crate) fn signaled(&self) {
        self.lock().signaled = false;

        self.execute_jobs_core(false);
    }

    pub(crate) fn execute_jobs_core(&self, from_explicit_background_processing_callback: bool) {
        let mut background_job_execution_started_at: Option<i64> = None;
        loop {
            let (job, maximum_input_starvation_time) = {
                let st = self.lock();
                (st.queue.peek(), st.maximum_input_starvation_time)
            };
            let Some(impl_) = self.local_impl() else {
                return;
            };

            let Some(job) = job else {
                return;
            };
            if job.priority() < DispatcherPriority::MINIMUM_ACTIVE_VALUE {
                return;
            }

            let pending_input = impl_.as_pending_input().filter(|pending| pending.can_query_pending_input());

            // We don't stop for executing jobs queued with >Input priority
            if job.priority() > DispatcherPriority::INPUT {
                self.execute_job(&job);
            }
            // If platform supports pending input query, ask the platform if we can continue running low priority jobs
            else if let Some(pending_input) = pending_input {
                if !pending_input.has_pending_input() {
                    self.execute_job(&job);
                } else {
                    self.request_background_processing_locked(&mut self.lock());
                    return;
                }
            }
            // We can't ask if the implementation has pending input, so we should let it to call us back
            // Once it thinks that input is handled
            else if impl_.as_explicit_background_processing().is_some()
                && !from_explicit_background_processing_callback
            {
                self.request_background_processing_locked(&mut self.lock());
                return;
            }
            // We can't check if there is pending input, but still need to enforce interactivity
            // so we stop processing background jobs after some timeout and start a timer to continue later
            else {
                let started_at = *background_job_execution_started_at.get_or_insert_with(|| impl_.now());

                if impl_.now() - started_at > maximum_input_starvation_time {
                    self.request_background_processing_locked(&mut self.lock());
                    return;
                } else {
                    self.execute_job(&job);
                }
            }

            // Yield after each job once shutdown has started, so frames that were asked to exit
            // can unwind before the next job runs; shutdown_impl then aborts whatever is left
            if self.has_shutdown_started() {
                // Dispatcher thread
                let has_shutdown_finished = self.lock().has_shutdown_finished;
                if !has_shutdown_finished && self.has_jobs_with_priority(DispatcherPriority::MINIMUM_ACTIVE_VALUE) {
                    self.request_processing();
                }
                return;
            }
        }
    }

    pub(crate) fn request_processing(&self) -> bool {
        self.request_processing_locked(&mut self.lock())
    }

    pub(crate) fn request_processing_locked(&self, st: &mut DispatcherState) -> bool {
        if !self.check_access() {
            Self::request_foreground_processing_locked(st, None);
            return true;
        }

        let impl_ = self.local_impl();
        if st.queue.max_priority() <= DispatcherPriority::INPUT {
            let can_run_now = impl_
                .as_ref()
                .and_then(|impl_| impl_.as_pending_input())
                .is_some_and(|pending| pending.can_query_pending_input() && !pending.has_pending_input());
            if can_run_now {
                Self::request_foreground_processing_locked(st, impl_.as_deref());
            } else {
                self.request_background_processing_locked(st);
            }
        } else {
            Self::request_foreground_processing_locked(st, impl_.as_deref());
        }
        true
    }

    /// `impl_` is the platform implementation when called on the dispatcher
    /// thread; other threads go through the thread-safe signal.
    fn request_foreground_processing_locked(st: &mut DispatcherState, impl_: Option<&dyn IDispatcherImpl>) {
        if !st.signaled {
            st.signaled = true;
            match impl_ {
                Some(impl_) => impl_.signal(),
                None => st.signal.signal(),
            }
        }
    }

    pub(crate) fn abort(&self, operation: &Arc<OperationCore>) -> bool {
        let mut st = self.lock();
        if operation.status() != DispatcherOperationStatus::Pending {
            return false;
        }
        st.queue.remove_item(operation);
        operation.set_status(DispatcherOperationStatus::Aborted);
        true
    }

    /// Returns whether or not the priority was set.
    pub(crate) fn set_priority(&self, operation: &Arc<OperationCore>, priority: DispatcherPriority) -> bool {
        let mut notify = false;

        let mut st = self.lock();
        if operation.is_queued() {
            st.queue.change_item_priority(operation, priority);
            notify = true;

            // Make sure we will wake up to process this operation.
            self.request_processing_locked(&mut st);
        }
        notify
    }

    /// Whether a job with at least the given priority is queued.
    pub fn has_jobs_with_priority(&self, priority: DispatcherPriority) -> bool {
        self.lock().queue.max_priority() >= priority
    }

    /// Gets all pending jobs, in arrival order, without removing them.
    ///
    /// Only use between unit tests!
    #[allow(dead_code)]
    pub(crate) fn get_jobs(&self) -> Vec<Arc<OperationCore>> {
        self.lock().queue.peek_all()
    }

    /// Clears all pending jobs.
    ///
    /// Only use between unit tests!
    #[allow(dead_code)]
    pub(crate) fn clear_jobs(&self) {
        let jobs = self.lock().queue.clear();
        // Released outside of the lock.
        drop(jobs);
    }

    pub(crate) fn has_shutdown_started(&self) -> bool {
        self.has_shutdown_started.load(Ordering::SeqCst)
    }
}
