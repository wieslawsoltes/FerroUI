//! Events: subscriptions with a token or with user data, routes, handled events, reentrant calls.

use super::support::*;
use crate::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

static CLICKS: AtomicI64 = AtomicI64::new(0);
static SENDER: AtomicU64 = AtomicU64::new(0);

/// Counts the clicks into the token's tens and writes the count into the content of the button,
/// through the ABI, from inside the handler.
unsafe extern "C" fn clicked(token: i64, sender: FerroHandle, arg: i64) -> i64 {
    assert_eq!(arg, 0);
    let clicks = CLICKS.fetch_add(token, Ordering::SeqCst) + token;
    SENDER.store(sender, Ordering::SeqCst);
    let text = format!("Clicked {clicks}");
    assert_eq!(set_str(sender, "Content", &text), FERRO_OK, "{}", last_error());
    0
}

#[test]
fn a_handler_is_called_with_its_token_and_the_sender() {
    ui(|| {
        let button = create("Button");
        let click = event_id("Click");
        let subscription = ferro_subscribe(button, click, Some(clicked), 1);
        assert!(subscription > 0, "{}", last_error());
        assert_eq!(ferro_raise(button, click), FERRO_OK, "{}", last_error());
        assert_eq!(ferro_raise(button, click), FERRO_OK);
        assert_eq!(CLICKS.load(Ordering::SeqCst), 2);
        assert_eq!(SENDER.load(Ordering::SeqCst), button);
        assert_eq!(get_str(button, "Content").as_deref(), Ok("Clicked 2"));
        assert_eq!(ferro_unsubscribe(subscription), FERRO_OK);
        assert_eq!(ferro_unsubscribe(subscription), FERRO_NOT_FOUND);
        assert_eq!(ferro_raise(button, click), FERRO_OK);
        assert_eq!(CLICKS.load(Ordering::SeqCst), 2, "the handler is removed");
        assert_eq!(ferro_release(button), FERRO_OK);
    });
}

static BUBBLED: AtomicI64 = AtomicI64::new(0);

unsafe extern "C" fn bubbled(token: i64, _sender: FerroHandle, _arg: i64) -> i64 {
    BUBBLED.fetch_add(1, Ordering::SeqCst);
    // A token of 1 handles the event.
    token
}

#[test]
fn a_routed_event_bubbles_until_a_handler_handles_it() {
    ui(|| {
        let panel = create("StackPanel");
        let button = create("Button");
        assert_eq!(ferro_add_child(panel, button), FERRO_OK);
        let click = event_id("Click");
        // The panel has no event Click: it names the one of the button.
        assert_eq!(ferro_subscribe(panel, click, Some(bubbled), 0), i64::from(FERRO_NOT_FOUND));
        let on_panel = ferro_subscribe(panel, event_id("Button.Click"), Some(bubbled), 0);
        assert!(on_panel > 0, "{}", last_error());
        // Not handled on the button: the panel sees it.
        let on_button = ferro_subscribe(button, click, Some(bubbled), 0);
        assert_eq!(ferro_raise(button, click), FERRO_OK);
        assert_eq!(BUBBLED.load(Ordering::SeqCst), 2);
        // Handled on the button: the panel does not.
        assert_eq!(ferro_unsubscribe(on_button), FERRO_OK);
        let on_button = ferro_subscribe(button, click, Some(bubbled), 1);
        assert_eq!(ferro_raise(button, click), FERRO_OK);
        assert_eq!(BUBBLED.load(Ordering::SeqCst), 3);
        for subscription in [on_button, on_panel] {
            assert_eq!(ferro_unsubscribe(subscription), FERRO_OK);
        }
        assert_eq!(ferro_release(button), FERRO_OK);
        assert_eq!(ferro_release(panel), FERRO_OK);
    });
}

static DATA_CALLS: AtomicI64 = AtomicI64::new(0);
static RELEASED: AtomicI64 = AtomicI64::new(0);

unsafe extern "C" fn with_data(user_data: *mut c_void, _sender: FerroHandle, _arg: i64) -> i64 {
    // SAFETY: the test passes a pointer to a static `AtomicI64`.
    let counter = unsafe { &*(user_data as *const AtomicI64) };
    counter.fetch_add(1, Ordering::SeqCst);
    0
}

unsafe extern "C" fn release(user_data: *mut c_void) {
    assert_eq!(user_data as *const AtomicI64, &DATA_CALLS as *const AtomicI64);
    RELEASED.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn user_data_is_released_with_its_handler() {
    ui(|| {
        let button = create("Button");
        let click = event_id("Click");
        let data = &DATA_CALLS as *const AtomicI64 as *mut c_void;
        let subscription = ferro_subscribe_data(button, click, Some(with_data), data, Some(release));
        assert!(subscription > 0, "{}", last_error());
        assert_eq!(ferro_raise(button, click), FERRO_OK);
        assert_eq!(DATA_CALLS.load(Ordering::SeqCst), 1);
        assert_eq!(RELEASED.load(Ordering::SeqCst), 0);
        assert_eq!(ferro_unsubscribe(subscription), FERRO_OK);
        assert_eq!(RELEASED.load(Ordering::SeqCst), 1);
        // A failed subscription releases the user data at once.
        let failed = ferro_subscribe_data(button, event_id("NoSuchEvent"), Some(with_data), data, Some(release));
        assert_eq!(failed, i64::from(FERRO_NOT_FOUND));
        assert_eq!(RELEASED.load(Ordering::SeqCst), 2);
        let failed = ferro_subscribe_data(button, click, None, data, Some(release));
        assert_eq!(failed, i64::from(FERRO_NULL_POINTER));
        assert_eq!(RELEASED.load(Ordering::SeqCst), 3);
        assert_eq!(ferro_release(button), FERRO_OK);
    });
}

#[test]
fn events_that_are_not_found_or_cannot_be_raised_are_errors() {
    ui(|| {
        let button = create("Button");
        assert_eq!(ferro_subscribe(button, event_id("NoSuchEvent"), Some(clicked), 0), i64::from(FERRO_NOT_FOUND));
        assert_eq!(ferro_subscribe(button, event_id("Click"), None, 0), i64::from(FERRO_NULL_POINTER));
        assert_eq!(ferro_subscribe(0, event_id("Click"), Some(clicked), 0), i64::from(FERRO_BAD_HANDLE));
        // The arguments of a pointer event carry the pointer.
        assert_eq!(ferro_raise(button, event_id("PointerPressed")), FERRO_NOT_SUPPORTED);
        assert_eq!(ferro_release(button), FERRO_OK);
    });
}
