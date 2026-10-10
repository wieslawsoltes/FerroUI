//! The dispatcher of the platform on a real run loop: the run loop of the
//! main thread of this process, which only the main thread can run. The
//! test therefore has no harness. It needs Core Foundation and libdispatch
//! and nothing of UIKit, so it runs on the development machine.
//!
//! Not from the reference, which has no tests of its dispatcher.

#[cfg(not(target_vendor = "apple"))]
fn main() {
    println!("dispatcher_main_loop: skipped, the system has no run loop of Core Foundation");
}

#[cfg(target_vendor = "apple")]
fn main() {
    apple::run();
}

#[cfg(target_vendor = "apple")]
mod apple {
    use ferroui_base::threading::{IDispatcherImpl, IDispatcherImplWithExplicitBackgroundProcessing};
    use ferroui_ios::dispatcher_impl::DispatcherImpl;
    use ferroui_ios::interop::kCFRunLoopDefaultMode;
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after_source_handled: u8) -> i32;
    }

    /// Runs the run loop of the main thread until `done` or for two
    /// seconds.
    fn run_until(done: impl Fn() -> bool) -> bool {
        let start = Instant::now();
        while !done() && start.elapsed() < Duration::from_secs(2) {
            // SAFETY: the main thread runs its own run loop in its default
            // mode for a short time.
            unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.01, 0) };
        }
        done()
    }

    fn check(name: &str, passed: bool) {
        println!("test {name} ... {}", if passed { "ok" } else { "FAILED" });
        if !passed {
            std::process::exit(1);
        }
    }

    pub fn run() {
        let dispatcher = DispatcherImpl::instance();
        check("the_dispatcher_is_one_object", Rc::ptr_eq(&dispatcher, &DispatcherImpl::instance()));
        check("the_main_thread_is_the_loop_thread", dispatcher.current_thread_is_loop_thread());

        // A signal of another thread reaches the main thread, once for any
        // number of signals before the loop took its turn.
        let signaled = Rc::new(Cell::new(0u32));
        dispatcher.signaled().add({
            let signaled = signaled.clone();
            Rc::new(move |()| signaled.set(signaled.get() + 1))
        });
        let handle = dispatcher.signal_handle();
        let thread = std::thread::spawn(move || handle.signal());
        check("a_signal_of_another_thread_is_raised_on_the_main_thread", run_until(|| signaled.get() > 0));
        let _ = thread.join();

        dispatcher.signal();
        dispatcher.signal();
        dispatcher.signal();
        check("a_signal_of_the_main_thread_is_raised", run_until(|| signaled.get() > 1));
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(100) {
            // SAFETY: as in `run_until`.
            unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.01, 0) };
        }
        check("signals_before_a_turn_of_the_loop_are_one_signal", signaled.get() == 2);

        // The timer fires at its due time, on the clock of the dispatcher.
        let fired = Rc::new(RefCell::new(None));
        dispatcher.timer().add({
            let fired = fired.clone();
            let dispatcher = dispatcher.clone();
            Rc::new(move |()| {
                fired.borrow_mut().get_or_insert(dispatcher.now());
            })
        });
        let due = dispatcher.now() + 100;
        dispatcher.update_timer(Some(due));
        check("the_timer_fires", run_until(|| fired.borrow().is_some()));
        let at = fired.borrow().unwrap_or_default();
        check("the_timer_fires_at_its_due_time", at >= due - 5 && at < due + 1000);

        // A timer that was cancelled does not fire.
        *fired.borrow_mut() = None;
        dispatcher.update_timer(Some(dispatcher.now() + 100));
        dispatcher.update_timer(None);
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(300) {
            // SAFETY: as in `run_until`.
            unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.01, 0) };
        }
        check("a_cancelled_timer_does_not_fire", fired.borrow().is_none());

        // Background processing gets its turn before the loop waits, once
        // per request.
        let ready = Rc::new(Cell::new(0u32));
        dispatcher.ready_for_background_processing().add({
            let ready = ready.clone();
            Rc::new(move |()| ready.set(ready.get() + 1))
        });
        dispatcher.request_background_processing();
        dispatcher.request_background_processing();
        check("background_processing_gets_its_turn", run_until(|| ready.get() > 0));
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(100) {
            // SAFETY: as in `run_until`.
            unsafe { CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.01, 0) };
        }
        check("one_turn_per_request", ready.get() == 1);

        println!("test result: ok. 10 passed; 0 failed");
    }
}
