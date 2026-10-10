//! The dispatcher platform implementation over the message loop of the UI
//! thread.
//!
//! Work is signalled with a message posted to the message window of the
//! platform, timers are the timer of that window, and the loop is the
//! message loop of the thread.

/// The `wParam` of the message that signals the dispatcher.
pub(crate) const SIGNAL_W: usize = 0xdead_beaf_u32 as i32 as isize as usize;
/// The `lParam` of the message that signals the dispatcher.
pub(crate) const SIGNAL_L: isize = 0x1234_5678;

/// The interval the timer of the message window is set to for a due time:
/// the time left, at least one millisecond and at most what the system
/// accepts.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
pub(crate) fn timer_interval(due_time_in_ms: i64, now: i64) -> u32 {
    (due_time_in_ms - now).max(1).min(i64::from(i32::MAX - 10)) as u32
}

#[cfg(windows)]
pub use imp::Win32DispatcherImpl;

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods::{
        dispatch_message, get_last_error, get_message, kill_timer, msg_wait_for_multiple_objects_ex, post_message,
        set_timer, translate_message, MsgWaitForMultipleObjectsFlags, QueueStatusFlags, WindowsMessage,
    };
    use crate::win32_platform::TIMERID_DISPATCHER;
    use crate::wnd_proc_guard;
    use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
    use ferroui_base::threading::{
        CancellationToken, DispatcherImplEvent, IControlledDispatcherImpl, IDispatcherImpl,
        IDispatcherImplWithPendingInput, IDispatcherSignal,
    };
    use std::sync::Arc;
    use std::thread::{self, ThreadId};
    use std::time::Instant;

    /// The cross-thread wake-up: posts the signal message to the message
    /// window.
    struct Win32Signal {
        message_window: isize,
    }

    impl IDispatcherSignal for Win32Signal {
        fn signal(&self) {
            // Messages from PostMessage are always processed before any user input,
            // so Win32 should call us ASAP
            post_message(self.message_window, WindowsMessage::WM_DISPATCH_WORK_ITEM, SIGNAL_W, SIGNAL_L);
        }
    }

    /// The dispatcher platform implementation of the Windows backend.
    pub struct Win32DispatcherImpl {
        message_window: isize,
        ui_thread: ThreadId,
        clock: Instant,
        signal: Arc<Win32Signal>,
        signaled: DispatcherImplEvent,
        timer: DispatcherImplEvent,
    }

    impl Win32DispatcherImpl {
        /// Creates the implementation for the calling thread, which owns
        /// `message_window`.
        pub fn new(message_window: isize) -> Self {
            Self {
                message_window,
                ui_thread: thread::current().id(),
                clock: Instant::now(),
                signal: Arc::new(Win32Signal { message_window }),
                signaled: DispatcherImplEvent::new(),
                timer: DispatcherImplEvent::new(),
            }
        }

        /// The signal message arrived: raises `signaled`.
        pub fn dispatch_work_item(&self) {
            self.signaled.raise();
        }

        /// The timer of the message window is due: raises `timer`.
        pub fn fire_timer(&self) {
            self.timer.raise();
        }
    }

    impl IDispatcherImpl for Win32DispatcherImpl {
        fn current_thread_is_loop_thread(&self) -> bool {
            self.ui_thread == thread::current().id()
        }

        fn signal(&self) {
            self.signal.signal();
        }

        fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
            self.signal.clone()
        }

        fn signaled(&self) -> &DispatcherImplEvent {
            &self.signaled
        }

        fn timer(&self) -> &DispatcherImplEvent {
            &self.timer
        }

        fn now(&self) -> i64 {
            self.clock.elapsed().as_millis() as i64
        }

        fn update_timer(&self, due_time_in_ms: Option<i64>) {
            match due_time_in_ms {
                None => {
                    kill_timer(self.message_window, TIMERID_DISPATCHER);
                }
                Some(due_time) => {
                    let interval = timer_interval(due_time, self.now());
                    set_timer(self.message_window, TIMERID_DISPATCHER, interval);
                }
            }
        }

        fn as_pending_input(&self) -> Option<&dyn IDispatcherImplWithPendingInput> {
            Some(self)
        }

        fn as_controlled(&self) -> Option<&dyn IControlledDispatcherImpl> {
            Some(self)
        }
    }

    impl IDispatcherImplWithPendingInput for Win32DispatcherImpl {
        fn can_query_pending_input(&self) -> bool {
            true
        }

        fn has_pending_input(&self) -> bool {
            // We need to know if there is any pending input in the Win32
            // queue because we want to only process "background" items
            // after Win32 input has been processed.
            //
            // Win32 provides the GetQueueStatus API -- but it has a major
            // drawback: it only counts "new" input.  This means that
            // sometimes it could return false, even if there really is input
            // that needs to be processed.  This results in very hard to
            // find bugs.
            //
            // Luckily, Win32 also provides the MsgWaitForMultipleObjectsEx
            // API.  While more awkward to use, this API can return queue
            // status information even if the input is "old".  The various
            // flags we use are:
            //
            // QS_INPUT
            // This represents any pending input - such as mouse moves, or
            // key presses.  It also includes the new GenericInput messages.
            //
            // QS_EVENT
            // This is actually a private flag that represents the various
            // events that can be queued in Win32.  Some of these events
            // can cause input, but Win32 doesn't include them in the
            // QS_INPUT flag.  An example is WM_MOUSELEAVE.
            //
            // QS_POSTMESSAGE
            // If there is already a message in the queue, we need to process
            // it before we can process input.
            //
            // MWMO_INPUTAVAILABLE
            // This flag indicates that any input (new or old) is to be
            // reported.
            //
            let result = msg_wait_for_multiple_objects_ex(
                0,
                QueueStatusFlags::QS_INPUT | QueueStatusFlags::QS_EVENT | QueueStatusFlags::QS_POSTMESSAGE,
                MsgWaitForMultipleObjectsFlags::MWMO_INPUTAVAILABLE,
            );
            if result == u32::MAX {
                panic!("MsgWaitForMultipleObjectsEx failed with the error code {}", get_last_error());
            }
            result == 0
        }
    }

    impl IControlledDispatcherImpl for Win32DispatcherImpl {
        fn run_loop(&self, token: CancellationToken) {
            let mut result = 0;
            while !token.is_cancellation_requested() {
                let (r, msg) = get_message();
                result = r;
                if result <= 0 {
                    break;
                }
                translate_message(&msg);
                dispatch_message(&msg);

                // A panic of a window procedure is raised here, where the
                // system has returned to the loop.
                wnd_proc_guard::resume_pending();
            }
            if result < 0 {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::WIN32_PLATFORM) {
                    logger.log(None, &format!("Unmanaged error in run_loop. Error Code: {}", get_last_error()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_signal_parameters_are_those_of_the_reference() {
        // 0xdeadbeaf as a 32-bit signed value, sign-extended to the width
        // of a pointer.
        assert_eq!(SIGNAL_W as isize, 0xdead_beaf_u32 as i32 as isize);
        assert_eq!(SIGNAL_L, 0x1234_5678);
    }

    #[test]
    fn the_timer_interval_is_the_time_left_within_the_bounds_of_the_system() {
        assert_eq!(timer_interval(150, 100), 50);
        // A timer that is due, or overdue, fires as soon as possible.
        assert_eq!(timer_interval(100, 100), 1);
        assert_eq!(timer_interval(10, 100), 1);
        assert_eq!(timer_interval(i64::MAX, 0), (i32::MAX - 10) as u32);
    }
}
