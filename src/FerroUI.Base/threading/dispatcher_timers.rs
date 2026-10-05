use std::rc::Rc;

use super::dispatcher::DispatcherState;
use super::{Dispatcher, DispatcherPriority, DispatcherTimer};

impl Dispatcher {
    /// The current time of the platform implementation, in milliseconds.
    ///
    /// Dispatcher thread only; a dispatcher that has been reset reports 0.
    pub fn now(&self) -> i64 {
        self.verify_access();
        self.local_impl().map_or(0, |impl_| impl_.now())
    }

    pub(crate) fn update_os_timer(&self) {
        self.update_os_timer_locked(&mut self.lock());
    }

    pub(crate) fn update_os_timer_locked(&self, st: &mut DispatcherState) {
        self.verify_access();
        let next_due_time = match (st.due_time_for_timers, st.due_time_for_background_processing) {
            (Some(timers), Some(background)) => Some(timers.min(background)),
            (timers, background) => timers.or(background),
        };
        if st.os_timer_set_to == next_due_time {
            return;
        }

        st.os_timer_set_to = next_due_time;
        if let Some(impl_) = self.local_impl() {
            impl_.update_timer(next_due_time);
        }
    }

    pub(crate) fn reschedule_timers(&self) {
        if !self.check_access() {
            let dispatcher = self.to_arc();
            self.post(move || dispatcher.reschedule_timers(), DispatcherPriority::SEND);
            return;
        }

        let local = self.try_local();
        let mut st = self.lock();
        if !st.has_shutdown_finished {
            let old_due_time_found = st.due_time_found;
            let old_due_time_in_ticks = st.due_time_in_ms;
            st.due_time_found = false;
            st.due_time_in_ms = 0;

            if let Some(local) = &local {
                // We could do better if we sorted the list of timers.
                for timer in local.timers.borrow().iter() {
                    if !st.due_time_found || timer.due_time_in_ms().wrapping_sub(st.due_time_in_ms) < 0 {
                        st.due_time_found = true;
                        st.due_time_in_ms = timer.due_time_in_ms();
                    }
                }
            }

            if st.due_time_found {
                if st.due_time_for_timers.is_none()
                    || !old_due_time_found
                    || old_due_time_in_ticks != st.due_time_in_ms
                {
                    st.due_time_for_timers = Some(st.due_time_in_ms);
                    self.update_os_timer_locked(&mut st);
                }
            } else if old_due_time_found {
                st.due_time_for_timers = None;
                self.update_os_timer_locked(&mut st);
            }
        }
    }

    pub(crate) fn add_timer(&self, timer: &Rc<DispatcherTimer>) {
        if let Some(local) = self.try_local() {
            if !self.lock().has_shutdown_finished {
                local.timers.borrow_mut().push(timer.clone());
                local.timers_version.set(local.timers_version.get() + 1);
            }
        }

        self.reschedule_timers();
    }

    pub(crate) fn remove_timer(&self, timer: &DispatcherTimer) {
        if let Some(local) = self.try_local() {
            if !self.lock().has_shutdown_finished {
                let removed = {
                    let mut timers = local.timers.borrow_mut();
                    timers.iter().position(|t| std::ptr::eq(Rc::as_ptr(t), timer)).map(|index| timers.remove(index))
                };
                local.timers_version.set(local.timers_version.get() + 1);
                drop(removed);
            }
        }

        self.reschedule_timers();
    }

    pub(crate) fn on_os_timer(&self) {
        let need_to_promote_timers;
        let need_to_process_queue;
        {
            let Some(impl_) = self.local_impl() else {
                return;
            };
            let mut st = self.lock();
            impl_.update_timer(None);
            st.os_timer_set_to = None;
            let now = impl_.now();
            need_to_promote_timers = st.due_time_for_timers.is_some_and(|due| due <= now);
            if need_to_promote_timers {
                st.due_time_for_timers = None;
            }
            need_to_process_queue = st.due_time_for_background_processing.is_some_and(|due| due <= now);
            if need_to_process_queue {
                st.due_time_for_background_processing = None;
            }
        }

        if need_to_promote_timers {
            self.promote_timers();
        }
        if need_to_process_queue {
            self.execute_jobs_core(false);
        }
        self.update_os_timer();
    }

    pub(crate) fn promote_timers(&self) {
        let Some(local) = self.try_local() else {
            return;
        };
        if local.timers.borrow().is_empty() {
            // Nothing can be due and nothing has to be rescheduled.
            let st = self.lock();
            if !st.due_time_found {
                return;
            }
        }

        let current_time_in_ticks = self.now();

        let (has_due_timers, mut timers_version) = {
            let st = self.lock();
            let due = !st.has_shutdown_finished
                && st.due_time_found
                && st.due_time_in_ms.wrapping_sub(current_time_in_ticks) <= 0;
            (due, local.timers_version.get())
        };

        if has_due_timers {
            let mut i_timer = 0usize;

            loop {
                let timer = {
                    let mut timers = local.timers.borrow_mut();
                    let mut timer = None;

                    // If the timers collection changed while we are in the middle of
                    // looking for timers, start over.
                    if timers_version != local.timers_version.get() {
                        timers_version = local.timers_version.get();
                        i_timer = 0;
                    }

                    while i_timer < timers.len() {
                        // WARNING: this is vulnerable to wrapping
                        if timers[i_timer].due_time_in_ms().wrapping_sub(current_time_in_ticks) <= 0 {
                            // Remove this timer from our list.
                            // Do not increment the index.
                            timer = Some(timers.remove(i_timer));
                            break;
                        } else {
                            i_timer += 1;
                        }
                    }
                    timer
                };

                // Now that we are outside of the borrow, promote the timer.
                match timer {
                    Some(timer) => timer.promote(),
                    None => break,
                }
            }
        }

        // finally
        self.reschedule_timers();
    }

    #[allow(dead_code)]
    pub(crate) fn snapshot_timers_for_unit_tests() -> Vec<Rc<DispatcherTimer>> {
        Dispatcher::ui_thread().try_local().map(|local| local.timers.borrow().clone()).unwrap_or_default()
    }
}

/// Test support for the other crates of the workspace: the reference gives
/// its test assemblies access to the internal timer queue.
impl Dispatcher {
    /// The timers currently scheduled on the dispatcher of this thread.
    #[doc(hidden)]
    pub fn timers_for_unit_tests() -> Vec<Rc<DispatcherTimer>> {
        Self::snapshot_timers_for_unit_tests()
    }

    /// Fires a scheduled timer now, whatever its due time.
    #[doc(hidden)]
    pub fn force_fire_timer_for_unit_tests(timer: &Rc<DispatcherTimer>) {
        timer.promote();
        timer.dispatcher().remove_timer(timer);
        timer.dispatcher().run_jobs(None);
    }
}
