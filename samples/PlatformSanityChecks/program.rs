//! Port of `Program.cs` of `PlatformSanityChecks`: initializes the X11 platform without a window
//! and checks its threading.
//!
//! ```text
//! cargo run -p platform-sanity-checks        # on Linux, with an X server (DISPLAY)
//! ```
//!
//! Each check enters a frame of the main loop of the dispatcher that its timers or its signal
//! end: `Entering <check>`, what the check prints, `Leaving <check>`. A failed check prints the
//! error and the stack and kills the process. The checks, in their order:
//!
//! - `check_signaling`: an action posted from another thread runs on the UI thread and ends the
//!   frame;
//! - `check_timer_ordering`: a timer of one second fires before a timer of two seconds started
//!   before it, and each fires once;
//! - `check_timer_ticking`: a timer of one second ticks five times in 4.5 to 6 seconds.
//!
//! The reference calls `RuntimePlatformServicesInitializer` of a builder that only
//! `AppBuilder.Configure<App>()` made: the initializer is set by
//! `UseStandardRuntimePlatformSubsystem` (which the platforms call) and is null there, so the
//! reference fails at that statement with a null reference. The port selects the standard
//! runtime platform first, which sets the initializer the statement means, and the checks run
//! (`samples/PlatformSanityChecks/GAPS.md`, PSC001).
//!
//! The X11 platform exists on Linux; elsewhere the program says so and exits.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("platform-sanity-checks: the checks run on the X11 platform, which needs Linux");
}

#[cfg(target_os = "linux")]
fn main() {
    checks::main();
}

#[cfg(target_os = "linux")]
mod checks {
    use ferroui_base::animation::TimeSpan;
    use ferroui_base::threading::{CancellationTokenSource, Dispatcher, DispatcherPriority, DispatcherTimer};
    use ferroui_controls::AppBuilder;
    use platform_sanity_checks::App;
    use std::cell::Cell;
    use std::sync::OnceLock;
    use std::thread::ThreadId;
    use std::time::{Duration, Instant};

    static UI_THREAD: OnceLock<ThreadId> = OnceLock::new();

    pub fn main() {
        let _ = UI_THREAD.set(std::thread::current().id());
        // `AppBuilder.Configure<App>().RuntimePlatformServicesInitializer!()`; see the comment of
        // the file for the selection of the standard runtime platform.
        let initializer = AppBuilder::configure::<App>()
            .use_standard_runtime_platform_subsystem()
            .runtime_platform_services_initializer()
            .expect("Object reference not set to an instance of an object.");
        initializer();
        let _app = App::new();

        ferroui_x11::initialize_x11_platform(None);

        check_platform_threading();
    }

    fn check_access() -> bool {
        UI_THREAD.get() == Some(&std::thread::current().id())
    }

    fn verify_access() {
        if !check_access() {
            die("Call from invalid thread");
        }
    }

    /// Prints the error and the stack and kills the process.
    fn die(error: &str) -> ! {
        eprintln!("{error}");
        eprintln!("{}", std::backtrace::Backtrace::force_capture());
        // `Process.GetCurrentProcess().Kill()`: the process ends at once, without unwinding.
        std::process::abort();
    }

    /// Prints `Entering <caller>` now and `Leaving <caller>` when dropped.
    struct Enter(&'static str);

    impl Enter {
        fn new(caller: &'static str) -> Enter {
            println!("Entering {caller}");
            Enter(caller)
        }
    }

    impl Drop for Enter {
        fn drop(&mut self) {
            println!("Leaving {}", self.0);
        }
    }

    /// Runs a frame of the main loop of the UI thread that `cb` arranges to end.
    fn enter_loop(cb: impl FnOnce(&CancellationTokenSource), caller: &'static str) {
        let _enter = Enter::new(caller);
        let cts = CancellationTokenSource::new();
        cb(&cts);
        Dispatcher::ui_thread().main_loop(&cts.token());
        if !cts.is_cancellation_requested() {
            die("Unexpected loop exit");
        }
    }

    fn check_timer_ordering() {
        enter_loop(
            |cts| {
                let first_fired = std::rc::Rc::new(Cell::new(false));
                let second_fired = std::rc::Rc::new(Cell::new(false));
                let cts = cts.clone();
                let (first, second) = (first_fired.clone(), second_fired.clone());
                DispatcherTimer::run(
                    move || {
                        println!("Second tick");
                        verify_access();
                        if !first.get() {
                            die("Invalid timer ordering");
                        }
                        if second.get() {
                            die("Invocation of finished timer");
                        }
                        second.set(true);
                        cts.cancel();
                        false
                    },
                    Duration::from_secs(2),
                    DispatcherPriority::DEFAULT,
                )
                .detach();
                DispatcherTimer::run(
                    move || {
                        println!("First tick");
                        verify_access();
                        if second_fired.get() {
                            die("Invalid timer ordering");
                        }
                        if first_fired.get() {
                            die("Invocation of finished timer");
                        }
                        first_fired.set(true);
                        false
                    },
                    Duration::from_secs(1),
                    DispatcherPriority::DEFAULT,
                )
                .detach();
            },
            "CheckTimerOrdering",
        );
    }

    fn check_timer_ticking() {
        enter_loop(
            |cts| {
                let ticks = Cell::new(0);
                let st = Instant::now();
                let cts = cts.clone();
                DispatcherTimer::run(
                    move || {
                        ticks.set(ticks.get() + 1);
                        println!("Tick {} at {}", ticks.get(), TimeSpan::from(st.elapsed()));
                        if ticks.get() == 5 {
                            let elapsed = st.elapsed().as_secs_f64();
                            if elapsed < 4.5 {
                                die("Timer is too fast");
                            }
                            if elapsed > 6.0 {
                                die("Timer is too slow");
                            }
                            cts.cancel();
                            return false;
                        }

                        true
                    },
                    Duration::from_secs(1),
                    DispatcherPriority::DEFAULT,
                )
                .detach();
            },
            "CheckTimerTicking",
        );
    }

    fn check_signaling() {
        enter_loop(
            |cts| {
                let cts = cts.clone();
                // `ThreadPool.QueueUserWorkItem`: a thread of its own.
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(100));
                    Dispatcher::ui_thread().post(
                        move || {
                            verify_access();
                            cts.cancel();
                        },
                        DispatcherPriority::DEFAULT,
                    );
                });
            },
            "CheckSignaling",
        );
    }

    fn check_platform_threading() {
        check_signaling();
        check_timer_ordering();
        check_timer_ticking();
    }
}
