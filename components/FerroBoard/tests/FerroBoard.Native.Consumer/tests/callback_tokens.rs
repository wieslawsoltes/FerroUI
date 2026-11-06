//! Native publisher-owned keyboard callbacks and reentrant removal.
use ferroboard_controls::primitives::TouchButton;
use ferroui_controls::{
    AppBuilder, Application,
    automation::{peers::ControlAutomationPeer, provider::IInvokeProvider},
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
use ferroui_system::{events::HandlerList, threading::CancellationToken};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, OnceLock},
};

fn run(action: impl Fn() + Send + Sync + 'static) {
    static SESSION: OnceLock<Arc<HeadlessUnitTestSession>> = OnceLock::new();
    SESSION
        .get_or_init(|| {
            HeadlessUnitTestSession::start_new(|| {
                AppBuilder::configure::<Application>()
                    .use_headless(FerroHeadlessPlatformOptions::default())
            })
        })
        .dispatch(
            move || {
                ferroboard_controls::register_types();
                action();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}

struct DropAction(Option<Box<dyn FnOnce()>>);
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

#[test]
fn tokens_preserve_publisher_ownership_and_explicit_removal() {
    run(|| {
        let button = TouchButton::new();
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        button.click(move |_| count.set(count.get() + 1));
        let count = calls.clone();
        let token: u64 = button.click(move |_| count.set(count.get() + 10));
        let other = TouchButton::new();
        assert!(!other.remove_click(token));
        let long_token = button.long_press(|_| {});
        assert!(!button.remove_click(long_token));
        assert!(!button.remove_long_press(token));
        assert!(button.remove_long_press(long_token));
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        let provider = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        provider.invoke().unwrap();
        assert_eq!(calls.get(), 11);
        assert!(button.remove_click(token));
        assert!(!button.remove_click(token));
        provider.invoke().unwrap();
        assert_eq!(calls.get(), 12);
        let payload = Rc::new(());
        let payload_weak = Rc::downgrade(&payload);
        let _long_token = button.long_press(move |_| {
            std::hint::black_box(&payload);
        });
        drop(provider);
        drop(peer);
        let weak = button.downgrade();
        drop(button);
        assert!(weak.upgrade().is_none());
        assert!(payload_weak.upgrade().is_none());
    });
}

#[test]
fn removal_during_dispatch_keeps_the_current_snapshot_and_defers_new_callbacks() {
    run(|| {
        let button = TouchButton::new();
        let events = Rc::new(RefCell::new(Vec::new()));
        let second = Rc::new(Cell::new(None::<u64>));
        let once = Cell::new(false);
        let weak = button.downgrade();
        let log = events.clone();
        let remove = second.clone();
        let first = button.click(move |_| {
            log.borrow_mut().push("A");
            if !once.replace(true) {
                let button = weak.upgrade().unwrap();
                assert!(button.remove_click(remove.get().unwrap()));
                let nested = log.clone();
                button.click(move |_| nested.borrow_mut().push("C"));
            }
        });
        let log = events.clone();
        second.set(Some(button.click(move |_| log.borrow_mut().push("B"))));
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        let provider = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        provider.invoke().unwrap();
        assert_eq!(*events.borrow(), ["A", "B"]);
        provider.invoke().unwrap();
        assert_eq!(*events.borrow(), ["A", "B", "A", "C"]);
        assert!(button.remove_click(first));
    });
}

#[test]
fn callback_capture_cleanup_reenters_safely_and_panic_does_not_retry_removal() {
    run(|| {
        // Measure the selected SDK directly. A future SDK correction may make
        // this false; keyboard callback behavior below must work either way.
        let raw: Rc<HandlerList<dyn Fn()>> = Rc::new(HandlerList::new());
        let weak = Rc::downgrade(&raw);
        let capture = DropAction(Some(Box::new(move || {
            weak.upgrade().unwrap().add(Rc::new(|| {}));
        })));
        let raw_token = raw.add(Rc::new(move || capture.keep_alive()));
        let raw_panicked =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| raw.remove(raw_token)))
                .is_err();
        println!("pinned_sdk_remove_reentrant_capture_panicked={raw_panicked}");

        let button = TouchButton::new();
        let drops = Rc::new(Cell::new(0));
        let count = drops.clone();
        let weak = button.downgrade();
        let capture = DropAction(Some(Box::new(move || {
            count.set(count.get() + 1);
            let button = weak.upgrade().unwrap();
            button.click(|_| {});
            assert!(!button.remove_click(u64::MAX));
        })));
        let token = button.click(move |_| capture.keep_alive());
        assert!(button.remove_click(token));
        assert!(!button.remove_click(token));
        assert_eq!(drops.get(), 1);

        let count = drops.clone();
        let capture = DropAction(Some(Box::new(move || {
            count.set(count.get() + 1);
            panic!("native callback capture failure");
        })));
        let token = button.click(move |_| capture.keep_alive());
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                button.remove_click(token)
            }))
            .is_err()
        );
        assert!(!button.remove_click(token));
        assert_eq!(drops.get(), 2);
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        button.click(move |_| count.set(count.get() + 1));
        let peer = ControlAutomationPeer::create_peer_for_element(&button);
        peer.get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        assert_eq!(calls.get(), 1);
    });
}
