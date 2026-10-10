//! The dispatcher implementation of the platform (the port of
//! `X11PlatformThreading.cs`): a loop that waits on the connection to the
//! server, on a pipe that other threads write to in order to wake it, and
//! for the next timer.

use super::{IX11PlatformDispatcher, X11EventDispatcher};
#[cfg(target_os = "linux")]
use crate::x11_exception::X11Exception;
use crate::x11_platform::FerroX11Platform;
use ferroui_base::threading::{
    CancellationToken, DispatcherImplEvent, IControlledDispatcherImpl, IDispatcherImpl,
    IDispatcherImplWithPendingInput, IDispatcherSignal,
};
use std::cell::Cell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};
use std::time::Instant;

#[derive(Default)]
struct SignalState {
    signaled: bool,
    wakeup_requested: bool,
}

/// The part of the dispatcher implementation other threads reach: the
/// flags under their lock and the writing end of the wake-up pipe.
struct Signal {
    lock: Mutex<SignalState>,
    sigwrite: i32,
}

impl Signal {
    /// `Wakeup`, with the lock held by the caller.
    fn wakeup_locked(&self, state: &mut SignalState) {
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
        let mut state = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        self.wakeup_locked(&mut state);
    }
}

impl IDispatcherSignal for Signal {
    fn signal(&self) {
        let mut state = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        if state.signaled {
            return;
        }
        state.signaled = true;
        self.wakeup_locked(&mut state);
    }
}

/// What the loop waits on: the connection and the reading end of the
/// pipe.
struct Waiter {
    /// The descriptor of the event poll that holds both (Linux).
    #[cfg(target_os = "linux")]
    epoll: i32,
    #[cfg(not(target_os = "linux"))]
    x11_fd: i32,
    sigread: i32,
}

#[cfg(target_os = "linux")]
mod event_codes {
    pub const X11: u64 = 1;
    pub const SIGNAL: u64 = 2;
}

impl Waiter {
    #[cfg(target_os = "linux")]
    fn new(x11_fd: i32, sigread: i32) -> Self {
        // SAFETY: plain system calls; the event structures are local and
        // valid for the calls.
        unsafe {
            let epoll = libc::epoll_create1(0);
            if epoll == -1 {
                X11Exception::new("epoll_create1 failed").throw();
            }

            let mut ev = libc::epoll_event { events: libc::EPOLLIN as u32, u64: event_codes::X11 };
            if libc::epoll_ctl(epoll, libc::EPOLL_CTL_ADD, x11_fd, &mut ev) == -1 {
                X11Exception::new("Unable to attach X11 connection handle to epoll").throw();
            }

            let mut ev = libc::epoll_event { events: libc::EPOLLIN as u32, u64: event_codes::SIGNAL };
            if libc::epoll_ctl(epoll, libc::EPOLL_CTL_ADD, sigread, &mut ev) == -1 {
                X11Exception::new("Unable to attach signal pipe to epoll").throw();
            }
            Self { epoll, sigread }
        }
    }

    // Addition (DEVIATIONS.md, X11 platform): the reference runs on Linux
    // only, where it waits with an event poll. On the other systems with
    // an X server the same two descriptors are waited on with `poll`.
    #[cfg(not(target_os = "linux"))]
    fn new(x11_fd: i32, sigread: i32) -> Self {
        Self { x11_fd, sigread }
    }

    /// Waits until one of the descriptors can be read or the timeout (in
    /// milliseconds, negative for none) passes.
    fn wait(&self, timeout: i32) {
        #[cfg(target_os = "linux")]
        // SAFETY: `ev` is a valid place for one event.
        unsafe {
            let mut ev = libc::epoll_event { events: 0, u64: 0 };
            libc::epoll_wait(self.epoll, &mut ev, 1, timeout);
        }
        #[cfg(not(target_os = "linux"))]
        // SAFETY: `fds` holds two valid entries for the call.
        unsafe {
            let mut fds = [
                libc::pollfd { fd: self.x11_fd, events: libc::POLLIN, revents: 0 },
                libc::pollfd { fd: self.sigread, events: libc::POLLIN, revents: 0 },
            ];
            libc::poll(fds.as_mut_ptr(), 2, timeout);
        }
    }

    /// Drain the signaled pipe
    fn drain(&self) {
        let mut buf: i32 = 0;
        // SAFETY: reads at most four bytes into a local value from a
        // descriptor that does not block.
        while unsafe { libc::read(self.sigread, (&mut buf as *mut i32).cast(), 4) } > 0 {}
    }
}

/// Makes the pipe other threads wake the loop with: both ends do not
/// block.
fn create_signal_pipe() -> (i32, i32) {
    let mut fds = [0i32; 2];
    // SAFETY: `fds` has room for the two descriptors.
    unsafe {
        #[cfg(target_os = "linux")]
        libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK);
        #[cfg(not(target_os = "linux"))]
        {
            libc::pipe(fds.as_mut_ptr());
            for fd in fds {
                let flags = libc::fcntl(fd, libc::F_GETFL);
                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }
    }
    (fds[0], fds[1])
}

/// The timeout of the wait before the next timer (`None`: no timer, wait
/// without a limit; otherwise at least a millisecond).
pub(crate) fn wait_timeout(next_timer: Option<i64>, now: i64) -> i32 {
    match next_timer {
        None => -1,
        Some(next_timer) => (next_timer - now).max(1).min(i32::MAX as i64) as i32,
    }
}

/// The dispatcher implementation of the X11 platform.
pub struct X11PlatformThreading {
    platform: Weak<FerroX11Platform>,
    main_thread: ThreadId,
    signal: Arc<Signal>,
    waiter: Waiter,
    next_timer: Cell<Option<i64>>,
    clock: Instant,
    x11_events: Rc<X11EventDispatcher>,
    signaled: DispatcherImplEvent,
    timer: DispatcherImplEvent,
}

impl X11PlatformThreading {
    pub fn new(platform: &Rc<FerroX11Platform>) -> Rc<Self> {
        let x11_events = X11EventDispatcher::new(platform);
        let (sigread, sigwrite) = create_signal_pipe();
        let waiter = Waiter::new(x11_events.fd(), sigread);
        Rc::new(Self {
            platform: Rc::downgrade(platform),
            main_thread: thread::current().id(),
            signal: Arc::new(Signal { lock: Mutex::new(SignalState::default()), sigwrite }),
            waiter,
            next_timer: Cell::new(None),
            clock: Instant::now(),
            x11_events,
            signaled: DispatcherImplEvent::new(),
            timer: DispatcherImplEvent::new(),
        })
    }

    fn check_signaled(&self) {
        {
            let mut state = self.signal.lock.lock().unwrap_or_else(PoisonError::into_inner);
            if !state.signaled {
                return;
            }
            state.signaled = false;
        }

        self.signaled.raise();
    }

    fn elapsed_milliseconds(&self) -> i64 {
        self.clock.elapsed().as_millis() as i64
    }
}

impl IDispatcherImpl for X11PlatformThreading {
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
        self.elapsed_milliseconds()
    }

    fn update_timer(&self, due_time_in_ms: Option<i64>) {
        self.next_timer.set(due_time_in_ms);
        if due_time_in_ms.is_some() {
            self.signal.wakeup();
        }
    }

    fn as_pending_input(&self) -> Option<&dyn IDispatcherImplWithPendingInput> {
        Some(self)
    }

    fn as_controlled(&self) -> Option<&dyn IControlledDispatcherImpl> {
        Some(self)
    }
}

impl IDispatcherImplWithPendingInput for X11PlatformThreading {
    fn can_query_pending_input(&self) -> bool {
        true
    }

    fn has_pending_input(&self) -> bool {
        self.platform.upgrade().is_some_and(|platform| platform.event_grouper_dispatch_queue().has_jobs())
            || self.x11_events.is_pending()
    }
}

impl IControlledDispatcherImpl for X11PlatformThreading {
    fn run_loop(&self, cancellation_token: CancellationToken) {
        let Some(platform) = self.platform.upgrade() else {
            return;
        };
        let queue = platform.event_grouper_dispatch_queue().clone();
        while !cancellation_token.is_cancellation_requested() {
            let mut now = self.elapsed_milliseconds();
            if self.next_timer.get().is_some_and(|next_timer| now > next_timer) {
                self.timer.raise();
            }

            if cancellation_token.is_cancellation_requested() {
                return;
            }

            //Flush whatever requests were made to XServer
            self.x11_events.flush();
            if !self.x11_events.is_pending() {
                now = self.elapsed_milliseconds();
                if self.next_timer.get().is_some_and(|next_timer| next_timer < now) {
                    continue;
                }

                self.waiter.wait(wait_timeout(self.next_timer.get(), now));

                self.waiter.drain();

                self.signal.lock.lock().unwrap_or_else(PoisonError::into_inner).wakeup_requested = false;
            }

            if cancellation_token.is_cancellation_requested() {
                return;
            }
            self.check_signaled();
            self.x11_events.dispatch_x11_events(&cancellation_token);
            while queue.has_jobs() {
                self.check_signaled();
                queue.dispatch_next();
            }
        }
    }
}

impl IX11PlatformDispatcher for X11PlatformThreading {
    fn event_dispatcher(&self) -> &Rc<X11EventDispatcher> {
        &self.x11_events
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_wait_ends_at_the_next_timer_and_never_at_once() {
        assert_eq!(wait_timeout(None, 1_000), -1);
        assert_eq!(wait_timeout(Some(1_050), 1_000), 50);
        // A timer that is due is waited for one millisecond, as the reference does.
        assert_eq!(wait_timeout(Some(1_000), 1_000), 1);
        assert_eq!(wait_timeout(Some(i64::MAX), 0), i32::MAX);
    }

    #[test]
    fn a_signal_wakes_the_loop_once_until_it_is_taken() {
        let (sigread, sigwrite) = create_signal_pipe();
        let signal = Signal { lock: Mutex::new(SignalState::default()), sigwrite };
        let pending = |fd: i32| {
            let mut fds = [libc::pollfd { fd, events: libc::POLLIN, revents: 0 }];
            // SAFETY: one valid entry, no wait.
            unsafe { libc::poll(fds.as_mut_ptr(), 1, 0) > 0 }
        };

        assert!(!pending(sigread));
        signal.signal();
        signal.signal();
        signal.wakeup();
        assert!(pending(sigread));
        {
            let state = signal.lock.lock().unwrap();
            assert!(state.signaled && state.wakeup_requested);
        }

        // One byte was written for the three calls.
        let mut buf = [0u8; 8];
        // SAFETY: reads into a local buffer of the length that is passed.
        let read = unsafe { libc::read(sigread, buf.as_mut_ptr().cast(), buf.len()) };
        assert_eq!(read, 1);
        assert!(!pending(sigread));

        // SAFETY: closes the two descriptors this test opened.
        unsafe {
            libc::close(sigread);
            libc::close(sigwrite);
        }
    }

    #[test]
    fn the_pipe_does_not_block() {
        let (sigread, sigwrite) = create_signal_pipe();
        let mut buf = 0i32;
        // SAFETY: reads into a local value; the descriptor does not block,
        // so an empty pipe answers at once.
        let read = unsafe { libc::read(sigread, (&mut buf as *mut i32).cast(), 4) };
        assert_eq!(read, -1);
        // SAFETY: closes the two descriptors this test opened.
        unsafe {
            libc::close(sigread);
            libc::close(sigwrite);
        }
    }
}
