use ferroboard_controls::themes::{KeyboardTheme, KeyboardThemes, ThemeSubscription};
use std::{
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

const CHILD_MODE: &str = "FERROBOARD_THEME_SUBSCRIPTION_CHILD";

struct ChildGuard(Child);
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

// A broken registry lock must fail the test without hanging the whole test
// executable or poisoning the registry used by other consumer tests.
fn isolated(name: &str, scenario: &str, run: impl FnOnce()) {
    if std::env::var(CHILD_MODE).as_deref() == Ok(scenario) {
        run();
        return;
    }
    let mut child = ChildGuard(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--nocapture"])
            .env(CHILD_MODE, scenario)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(
                status.success(),
                "theme subscription child failed: {scenario}"
            );
            return;
        }
        assert!(
            Instant::now() < deadline,
            "theme subscription removal deadlocked: {scenario}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

struct DropAction(Option<Box<dyn FnOnce() + Send + Sync>>);
impl DropAction {
    fn keep_alive(&self) {}
}
impl Drop for DropAction {
    fn drop(&mut self) {
        if let Some(action) = self.0.take() {
            action();
        }
    }
}

fn reentrant_capture(events: Arc<Mutex<Vec<&'static str>>>) -> DropAction {
    DropAction(Some(Box::new(move || {
        events.lock().unwrap().push("drop entered");
        assert!(KeyboardThemes::get("floris_day").is_some());
        let nested_events = events.clone();
        let nested = KeyboardThemes::add_changed(move || {
            nested_events.lock().unwrap().push("nested event");
        });
        let theme = KeyboardTheme::new();
        theme.set_id(Some("consumer_reentrant_removal".into()));
        KeyboardThemes::register(theme).unwrap();
        assert!(KeyboardThemes::remove_changed(nested));
        events.lock().unwrap().push("drop finished");
    })))
}

#[test]
fn token_removal_releases_callback_captures_outside_the_registry_lock() {
    isolated(
        "token_removal_releases_callback_captures_outside_the_registry_lock",
        "token",
        || {
            let events = Arc::new(Mutex::new(Vec::new()));
            let capture = reentrant_capture(events.clone());
            let token = KeyboardThemes::add_changed(move || capture.keep_alive());
            assert!(KeyboardThemes::remove_changed(token));
            assert!(!KeyboardThemes::remove_changed(token));
            assert_eq!(
                *events.lock().unwrap(),
                ["drop entered", "nested event", "drop finished"]
            );
        },
    );
}

#[test]
fn explicit_guard_removal_is_reentrant_and_idempotent() {
    isolated(
        "explicit_guard_removal_is_reentrant_and_idempotent",
        "unsubscribe",
        || {
            let events = Arc::new(Mutex::new(Vec::new()));
            let capture = reentrant_capture(events.clone());
            let mut subscription = KeyboardThemes::changed(move || capture.keep_alive());
            assert!(subscription.unsubscribe());
            assert!(!subscription.unsubscribe());
            drop(subscription);
            assert_eq!(
                *events.lock().unwrap(),
                ["drop entered", "nested event", "drop finished"]
            );
        },
    );
}

#[test]
fn callback_capture_panics_propagate_after_removal_and_leave_the_registry_usable() {
    isolated(
        "callback_capture_panics_propagate_after_removal_and_leave_the_registry_usable",
        "panic",
        || {
            let drops = Arc::new(AtomicUsize::new(0));
            let count = drops.clone();
            let capture = DropAction(Some(Box::new(move || {
                assert!(!KeyboardThemes::all().is_empty());
                count.fetch_add(1, Ordering::SeqCst);
                panic!("consumer capture failure");
            })));
            let token = KeyboardThemes::add_changed(move || capture.keep_alive());
            assert!(std::panic::catch_unwind(|| KeyboardThemes::remove_changed(token)).is_err());
            assert_eq!(drops.load(Ordering::SeqCst), 1);
            assert!(!KeyboardThemes::remove_changed(token));
            let capture = reentrant_capture(Arc::new(Mutex::new(Vec::new())));
            let mut subscription = KeyboardThemes::changed(move || capture.keep_alive());
            assert!(subscription.unsubscribe());
        },
    );
}

#[test]
fn dropping_the_native_guard_releases_reentrant_captures_and_stops_notifications() {
    isolated(
        "dropping_the_native_guard_releases_reentrant_captures_and_stops_notifications",
        "drop",
        || {
            let events = Arc::new(Mutex::new(Vec::new()));
            let capture = reentrant_capture(events.clone());
            let subscription: ThemeSubscription =
                KeyboardThemes::changed(move || capture.keep_alive());
            drop(subscription);
            assert_eq!(
                *events.lock().unwrap(),
                ["drop entered", "nested event", "drop finished"]
            );
            let theme = KeyboardTheme::new();
            theme.set_id(Some("consumer_after_guard_drop".into()));
            KeyboardThemes::register(theme).unwrap();
            assert_eq!(events.lock().unwrap().len(), 3);
        },
    );
}

#[test]
fn native_guard_ownership_can_move_to_the_unregistering_thread() {
    isolated(
        "native_guard_ownership_can_move_to_the_unregistering_thread",
        "move-thread",
        || {
            let calls = Arc::new(AtomicUsize::new(0));
            let drops = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            let dropped = drops.clone();
            let capture = DropAction(Some(Box::new(move || {
                dropped.fetch_add(1, Ordering::SeqCst);
            })));
            let subscription = KeyboardThemes::changed(move || {
                capture.keep_alive();
                count.fetch_add(1, Ordering::SeqCst);
            });
            let theme = KeyboardTheme::new();
            theme.set_id(Some("consumer_moved_subscription".into()));
            KeyboardThemes::register(theme.clone()).unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            std::thread::spawn(move || drop(subscription))
                .join()
                .unwrap();
            assert_eq!(drops.load(Ordering::SeqCst), 1);
            KeyboardThemes::register(theme).unwrap();
            assert_eq!(calls.load(Ordering::SeqCst), 1);
        },
    );
}

#[test]
fn native_guard_publishes_removal_before_capture_destructor_panics() {
    isolated(
        "native_guard_publishes_removal_before_capture_destructor_panics",
        "guard-panic",
        || {
            for explicit in [false, true] {
                let drops = Arc::new(AtomicUsize::new(0));
                let count = drops.clone();
                let capture = DropAction(Some(Box::new(move || {
                    assert!(!KeyboardThemes::all().is_empty());
                    count.fetch_add(1, Ordering::SeqCst);
                    panic!("native guard capture failure");
                })));
                let mut subscription = KeyboardThemes::changed(move || capture.keep_alive());
                if explicit {
                    assert!(
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            subscription.unsubscribe()
                        }))
                        .is_err()
                    );
                    assert!(!subscription.unsubscribe());
                    drop(subscription);
                } else {
                    assert!(std::panic::catch_unwind(|| drop(subscription)).is_err());
                }
                assert_eq!(drops.load(Ordering::SeqCst), 1);
                let capture = reentrant_capture(Arc::new(Mutex::new(Vec::new())));
                drop(KeyboardThemes::changed(move || capture.keep_alive()));
            }
        },
    );
}
