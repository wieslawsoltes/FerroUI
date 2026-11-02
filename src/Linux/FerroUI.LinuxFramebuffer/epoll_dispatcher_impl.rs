//! The dispatcher implementation of the platform (the port of
//! `EpollDispatcherImpl.cs`): a loop that waits, with an event poll, on a
//! pipe that other threads write to in order to wake it and on a timer
//! descriptor armed for the next timer.

use std::time::Duration;

/// The value the timer descriptor is armed with for a wait of `wait_for`:
/// seconds (at most 100, after which the loop looks again) and
/// nanoseconds.
///
/// The reference computes the same from a time span. A wait of no time at
/// all would disarm the descriptor (both fields zero) and the loop would
/// sleep until something else woke it; it is one nanosecond here.
pub(crate) fn timer_value(wait_for: Duration) -> (i64, i64) {
    let tv_sec = wait_for.as_secs().min(100) as i64;
    let tv_nsec = wait_for.subsec_nanos() as i64;
    if tv_sec == 0 && tv_nsec == 0 {
        return (0, 1);
    }
    (tv_sec, tv_nsec)
}

#[cfg(target_os = "linux")]
pub use imp::EpollDispatcherImpl;

#[cfg(target_os = "linux")]
mod imp {
    use super::timer_value;
    use ferroui_base::platform::IManagedDispatcherInputProvider;
    use ferroui_base::threading::{
        CancellationToken, DispatcherImplEvent, IControlledDispatcherImpl, IDispatcherImpl,
        IDispatcherImplWithExplicitBackgroundProcessing, IDispatcherImplWithPendingInput, IDispatcherSignal,
    };
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
    use std::thread::{self, ThreadId};
    use std::time::{Duration, Instant};

    mod event_codes {
        pub const TIMER: u64 = 1;
        pub const SIGNAL: u64 = 2;
    }

    #[derive(Default)]
    struct State {
        signaled: bool,
        wakeup_requested: bool,
        background_processing_requested: bool,
    }

    /// The part of the dispatcher implementation other threads reach: the
    /// flags under their lock and the writing end of the wake-up pipe.
    struct Signal {
        lock: Mutex<State>,
        sigwrite: i32,
    }

    impl Signal {
        fn lock(&self) -> MutexGuard<'_, State> {
            self.lock.lock().unwrap_or_else(PoisonError::into_inner)
        }

        /// `Wakeup`, with the lock held by the caller.
        fn wakeup_locked(&self, state: &mut State) {
            if state.wakeup_requested {
                return;
            }
            state.wakeup_requested = true;
            let buf: i32 = 0;
            // SAFETY: writes one byte of a local value to a descriptor this
            // object owns for the life of the process. A full pipe (the
            // descriptor does not block) means a wake-up is already pending.
            unsafe {
                libc::write(self.sigwrite, (&buf as *const i32).cast(), 1);
            }
        }

        fn wakeup(&self) {
            let mut state = self.lock();
            self.wakeup_locked(&mut state);
        }
    }

    impl IDispatcherSignal for Signal {
        fn signal(&self) {
            let mut state = self.lock();
            if state.signaled {
                return;
            }
            state.signaled = true;
            self.wakeup_locked(&mut state);
        }
    }

    /// The dispatcher implementation of the framebuffer platform.
    pub struct EpollDispatcherImpl {
        input_provider: Box<dyn IManagedDispatcherInputProvider>,
        main_thread: ThreadId,
        sigread: i32,
        timerfd: i32,
        signal: Arc<Signal>,
        next_timer: Cell<Option<Duration>>,
        epoll: i32,
        clock: Instant,
        signaled: DispatcherImplEvent,
        timer: DispatcherImplEvent,
        ready_for_background_processing: DispatcherImplEvent,
    }

    impl EpollDispatcherImpl {
        /// # Panics
        /// Panics when the event poll cannot be created or a descriptor
        /// cannot be attached to it.
        pub fn new(input_provider: Box<dyn IManagedDispatcherInputProvider>) -> Rc<Self> {
            // SAFETY: plain system calls; the event structures and the
            // array of descriptors are local and valid for the calls.
            let (epoll, sigread, sigwrite, timerfd) = unsafe {
                let epoll = libc::epoll_create1(libc::EPOLL_CLOEXEC);
                if epoll == -1 {
                    panic!("epoll_create1 failed");
                }

                let mut fds = [0i32; 2];
                libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC);
                let (sigread, sigwrite) = (fds[0], fds[1]);

                let mut ev = libc::epoll_event { events: libc::EPOLLIN as u32, u64: event_codes::SIGNAL };
                if libc::epoll_ctl(epoll, libc::EPOLL_CTL_ADD, sigread, &mut ev) == -1 {
                    panic!("Unable to attach signal pipe to epoll");
                }

                let timerfd = libc::timerfd_create(libc::CLOCK_MONOTONIC, libc::O_NONBLOCK | libc::O_CLOEXEC);
                ev.u64 = event_codes::TIMER;
                if libc::epoll_ctl(epoll, libc::EPOLL_CTL_ADD, timerfd, &mut ev) == -1 {
                    panic!("Unable to attach timer fd to epoll");
                }
                (epoll, sigread, sigwrite, timerfd)
            };

            Rc::new(Self {
                input_provider,
                main_thread: thread::current().id(),
                sigread,
                timerfd,
                signal: Arc::new(Signal { lock: Mutex::new(State::default()), sigwrite }),
                next_timer: Cell::new(None),
                epoll,
                clock: Instant::now(),
                signaled: DispatcherImplEvent::new(),
                timer: DispatcherImplEvent::new(),
                ready_for_background_processing: DispatcherImplEvent::new(),
            })
        }

        fn check_signaled(&self) -> bool {
            {
                let mut state = self.signal.lock();
                if !state.signaled {
                    return false;
                }
                state.signaled = false;
            }

            self.signaled.raise();
            true
        }

        fn set_timer(&self, tv_sec: i64, tv_nsec: i64) {
            // SAFETY: an all-zero `itimerspec` is valid; the structure is
            // local and the old value is not asked for.
            unsafe {
                let mut timer: libc::itimerspec = std::mem::zeroed();
                timer.it_value.tv_sec = tv_sec as _;
                timer.it_value.tv_nsec = tv_nsec as _;
                libc::timerfd_settime(self.timerfd, 0, &timer, std::ptr::null_mut());
            }
        }

        fn drain(fd: i32) {
            let mut buf: i64 = 0;
            // SAFETY: reads at most eight bytes into a local value from a
            // descriptor that does not block.
            while unsafe { libc::read(fd, (&mut buf as *mut i64).cast(), 8) } > 0 {}
        }
    }

    impl IDispatcherImpl for EpollDispatcherImpl {
        fn current_thread_is_loop_thread(&self) -> bool {
            thread::current().id() == self.main_thread
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
            self.next_timer.set(due_time_in_ms.map(|ms| Duration::from_millis(ms.max(0) as u64)));
            if due_time_in_ms.is_some() {
                self.signal.wakeup();
            }
        }

        fn as_pending_input(&self) -> Option<&dyn IDispatcherImplWithPendingInput> {
            Some(self)
        }

        fn as_explicit_background_processing(&self) -> Option<&dyn IDispatcherImplWithExplicitBackgroundProcessing> {
            Some(self)
        }

        fn as_controlled(&self) -> Option<&dyn IControlledDispatcherImpl> {
            Some(self)
        }
    }

    impl IDispatcherImplWithPendingInput for EpollDispatcherImpl {
        fn can_query_pending_input(&self) -> bool {
            true
        }

        fn has_pending_input(&self) -> bool {
            self.input_provider.has_input()
        }
    }

    impl IDispatcherImplWithExplicitBackgroundProcessing for EpollDispatcherImpl {
        fn ready_for_background_processing(&self) -> &DispatcherImplEvent {
            &self.ready_for_background_processing
        }

        fn request_background_processing(&self) {
            let mut state = self.signal.lock();
            state.background_processing_requested = true;
            self.signal.wakeup_locked(&mut state);
        }
    }

    impl IControlledDispatcherImpl for EpollDispatcherImpl {
        fn run_loop(&self, cancellation_token: CancellationToken) {
            while !cancellation_token.is_cancelled() {
                let now = self.clock.elapsed();
                if self.next_timer.get().is_some_and(|next_timer| now > next_timer) {
                    self.timer.raise();
                    continue;
                }

                if self.check_signaled() {
                    continue;
                }

                if self.input_provider.has_input() {
                    self.input_provider.dispatch_next_input_event();
                    continue;
                }

                let trigger_background_processing =
                    std::mem::take(&mut self.signal.lock().background_processing_requested);

                if trigger_background_processing {
                    self.ready_for_background_processing.raise();
                    continue;
                }

                match self.next_timer.get() {
                    Some(next_timer) => {
                        let Some(wait_for) = next_timer.checked_sub(now) else {
                            continue;
                        };
                        let (tv_sec, tv_nsec) = timer_value(wait_for);
                        self.set_timer(tv_sec, tv_nsec);
                    }
                    None => self.set_timer(0, 0),
                }

                // SAFETY: `ev` is a valid place for one event.
                unsafe {
                    let mut ev = libc::epoll_event { events: 0, u64: 0 };
                    libc::epoll_wait(self.epoll, &mut ev, 1, -1);
                }

                // Drain the signaled pipe
                Self::drain(self.sigread);

                // Drain timer fd
                Self::drain(self.timerfd);

                self.signal.lock().wakeup_requested = false;
            }
        }
    }

    #[cfg(test)]
    mod tests {
        // Not from the reference, which has no tests for this file. They
        // run the loop on a real event poll, on Linux only.
        use super::*;
        use ferroui_base::threading::CancellationToken;
        use std::cell::RefCell;
        use std::collections::VecDeque;

        #[derive(Default)]
        struct Input {
            events: Rc<RefCell<VecDeque<Box<dyn FnOnce()>>>>,
        }

        impl IManagedDispatcherInputProvider for Input {
            fn has_input(&self) -> bool {
                !self.events.borrow().is_empty()
            }

            fn dispatch_next_input_event(&self) {
                let next = self.events.borrow_mut().pop_front();
                if let Some(next) = next {
                    next();
                }
            }
        }

        #[test]
        fn the_loop_raises_the_timer_the_signal_input_and_background_processing_and_ends_when_cancelled() {
            let input = Input::default();
            let events = input.events.clone();
            let dispatcher = EpollDispatcherImpl::new(Box::new(input));
            let log = Rc::new(RefCell::new(Vec::<&'static str>::new()));
            let cts = CancellationToken::new();

            {
                let log = log.clone();
                let weak = Rc::downgrade(&dispatcher);
                dispatcher.timer().add(Rc::new(move |()| {
                    log.borrow_mut().push("timer");
                    let dispatcher = weak.upgrade().unwrap();
                    dispatcher.update_timer(None);
                    dispatcher.request_background_processing();
                }));
            }
            {
                let log = log.clone();
                dispatcher.signaled().add(Rc::new(move |()| log.borrow_mut().push("signaled")));
            }
            {
                let log = log.clone();
                let cts = cts.clone();
                dispatcher.ready_for_background_processing().add(Rc::new(move |()| {
                    log.borrow_mut().push("background");
                    cts.cancel();
                }));
            }
            {
                let log = log.clone();
                events.borrow_mut().push_back(Box::new(move || log.borrow_mut().push("input")));
            }

            // The signal comes from another thread, as a post does.
            let handle = dispatcher.signal_handle();
            thread::spawn(move || handle.signal()).join().unwrap();
            dispatcher.update_timer(Some(dispatcher.now() + 30));
            assert!(dispatcher.has_pending_input());

            let started = Instant::now();
            dispatcher.run_loop(cts.clone());

            assert_eq!(*log.borrow(), ["signaled", "input", "timer", "background"]);
            assert!(started.elapsed() >= Duration::from_millis(25), "the timer was raised early");
            assert!(started.elapsed() < Duration::from_secs(5));
            assert!(!dispatcher.has_pending_input());
        }

        #[test]
        fn a_cancelled_token_ends_a_waiting_loop_when_the_loop_is_woken() {
            let dispatcher = EpollDispatcherImpl::new(Box::new(Input::default()));
            let cts = CancellationToken::new();
            let handle = dispatcher.signal_handle();
            let worker = {
                let cts = cts.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(30));
                    cts.cancel();
                    handle.signal();
                })
            };
            dispatcher.run_loop(cts.clone());
            worker.join().unwrap();
            assert!(dispatcher.current_thread_is_loop_thread());
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_timer_is_armed_for_the_wait_with_the_seconds_capped() {
        assert_eq!(timer_value(Duration::from_millis(1_500)), (1, 500_000_000));
        assert_eq!(timer_value(Duration::from_micros(250)), (0, 250_000));
        assert_eq!(timer_value(Duration::from_secs(3_600)), (100, 0));
        // Both fields zero would disarm the timer.
        assert_eq!(timer_value(Duration::ZERO), (0, 1));
    }
}
