//! The rules of the ABI: version, IDs, strings, handles, statuses and threads.

use super::support::*;
use crate::*;
use std::sync::atomic::{AtomicI64, Ordering};

#[test]
fn the_abi_has_a_version() {
    assert_eq!(ferro_abi_version(), FERRO_ABI_VERSION);
    assert_eq!(FERRO_ABI_VERSION, 1);
}

#[test]
fn a_type_is_found_by_its_name_or_by_its_namespace_and_name() {
    let button = type_id("Button");
    assert!(button > 0, "{}", last_error());
    assert_eq!(type_id("Button"), button, "one ID per type");
    assert_eq!(type_id("FerroUI.Controls.Button"), button);
    assert_ne!(type_id("TextBlock"), button);
    assert_eq!(type_id("NoSuchControl"), i64::from(FERRO_NOT_FOUND));
    assert_eq!(ferro_last_error_code(), FERRO_NOT_FOUND);
    assert!(last_error().contains("NoSuchControl"), "{}", last_error());
}

#[test]
fn a_negative_length_is_a_nul_terminated_name() {
    let by_length = type_id("StackPanel");
    // SAFETY: a NUL-terminated literal.
    let terminated = unsafe { ferro_type_id(c"StackPanel".as_ptr(), -1) };
    assert_eq!(terminated, by_length);
    // A length shorter than the text takes that many bytes.
    // SAFETY: the literal has at least five bytes.
    let prefix = unsafe { ferro_property_id(c"Textual".as_ptr(), 4) };
    assert_eq!(prefix, property_id("Text"));
    // SAFETY: as above.
    let terminated = unsafe { ferro_property_id(c"Text".as_ptr(), -1) };
    assert_eq!(terminated, property_id("Text"));
}

#[test]
fn names_that_are_not_names_are_rejected() {
    // SAFETY: NULL with a length of 0 is the empty string; NULL with a length is an error.
    unsafe {
        assert_eq!(ferro_property_id(std::ptr::null(), 0), i64::from(FERRO_INVALID_ARGUMENT));
        assert_eq!(ferro_property_id(std::ptr::null(), 3), i64::from(FERRO_NULL_POINTER));
        assert_eq!(ferro_event_id(std::ptr::null(), -1), i64::from(FERRO_NULL_POINTER));
        let not_utf8 = [0x66u8, 0xff, 0x6f];
        assert_eq!(ferro_property_id(not_utf8.as_ptr().cast(), 3), i64::from(FERRO_INVALID_UTF8));
    }
    assert_eq!(property_id("   "), i64::from(FERRO_INVALID_ARGUMENT));
}

#[test]
fn property_and_event_ids_are_interned_names() {
    let text = property_id("Text");
    assert!(text > 0);
    assert_eq!(property_id("Text"), text);
    assert_ne!(property_id("FontSize"), text);
    let click = event_id("Click");
    assert!(click > 0);
    assert_eq!(event_id("Click"), click);
}

#[test]
fn ids_are_looked_up_on_any_thread() {
    let here = property_id("Margin");
    let there = std::thread::spawn(|| (property_id("Margin"), type_id("Border"))).join().unwrap();
    assert_eq!(there.0, here);
    assert_eq!(there.1, type_id("Border"));
}

#[test]
fn an_object_is_created_and_its_handle_counts_references() {
    ui(|| {
        let button = create("Button");
        assert_eq!(ferro_type_of(button), type_id("Button"));
        assert_eq!(ferro_retain(button), FERRO_OK);
        assert_eq!(ferro_release(button), FERRO_OK);
        assert_eq!(ferro_type_of(button), type_id("Button"), "one reference is left");
        assert_eq!(ferro_release(button), FERRO_OK);
        // The last reference is gone: the handle is stale, and a new object never gets it.
        assert_eq!(ferro_release(button), FERRO_BAD_HANDLE);
        assert_eq!(ferro_retain(button), FERRO_BAD_HANDLE);
        let other = create("Button");
        assert_ne!(other, button, "a freed slot comes back with another generation");
        assert_eq!(ferro_release(other), FERRO_OK);
    });
}

#[test]
fn bad_handles_are_checked_errors() {
    ui(|| {
        let text = property_id("Text");
        assert_eq!(ferro_release(0), FERRO_BAD_HANDLE);
        assert_eq!(ferro_set_f64(0, property_id("FontSize"), 12.0), FERRO_BAD_HANDLE);
        assert_eq!(set_str(0x7777_0000_0001, "Text", "x"), FERRO_BAD_HANDLE);
        assert_eq!(ferro_type_of(0x1234_5678_9abc), i64::from(FERRO_BAD_HANDLE));
        assert_eq!(get_str(0xffff_ffff, "Text"), Err(FERRO_BAD_HANDLE));
        assert_eq!(ferro_clear(0, text), FERRO_BAD_HANDLE);
        assert!(last_error().contains("handle"), "{}", last_error());
    });
}

#[test]
fn types_without_a_default_constructor_are_not_created() {
    ui(|| {
        // A static class.
        assert_eq!(ferro_create(type_id("KeyboardNavigation")), 0);
        assert_eq!(ferro_last_error_code(), FERRO_NOT_SUPPORTED);
        assert_eq!(ferro_create(0), 0);
        assert_eq!(ferro_last_error_code(), FERRO_NOT_FOUND);
        assert_eq!(ferro_create(1 << 40), 0);
        assert_eq!(ferro_last_error_code(), FERRO_NOT_FOUND);
    });
}

#[test]
fn ui_calls_from_another_thread_are_refused() {
    let button = ui(|| create("Button"));
    let codes = std::thread::spawn(move || {
        (
            ferro_init(),
            ferro_is_ui_thread(),
            ferro_retain(button),
            ferro_release(button),
            ferro_create(type_id("Button")),
            ferro_last_error_code(),
            ferro_set_f64(button, property_id("FontSize"), 3.0),
            ferro_run_jobs(),
            ferro_app_run(std::ptr::null(), Some(start_never), 0),
        )
    })
    .join()
    .unwrap();
    assert_eq!(codes.0, FERRO_WRONG_THREAD);
    assert!(!codes.1);
    assert_eq!(codes.2, FERRO_WRONG_THREAD);
    assert_eq!(codes.3, FERRO_WRONG_THREAD);
    assert_eq!(codes.4, 0);
    assert_eq!(codes.5, FERRO_WRONG_THREAD);
    assert_eq!(codes.6, FERRO_WRONG_THREAD);
    assert_eq!(codes.7, FERRO_WRONG_THREAD);
    assert_eq!(codes.8, FERRO_WRONG_THREAD);
    ui(move || {
        assert!(ferro_is_ui_thread());
        assert_eq!(ferro_init(), FERRO_OK, "ferro_init again on the UI thread does nothing");
        assert_eq!(ferro_release(button), FERRO_OK);
    });
}

unsafe extern "C" fn start_never(_token: i64) -> i64 {
    panic!("the start function must not be called");
}

static POSTED: AtomicI64 = AtomicI64::new(0);

unsafe extern "C" fn posted(token: i64) -> i64 {
    assert!(ferro_is_ui_thread(), "posted work runs on the UI thread");
    POSTED.fetch_add(token, Ordering::SeqCst);
    0
}

unsafe extern "C" fn posted_data(user_data: *mut std::ffi::c_void) {
    assert!(ferro_is_ui_thread());
    // SAFETY: the test posts a pointer to an `AtomicI64` that outlives the work.
    unsafe { &*(user_data as *const AtomicI64) }.fetch_add(1, Ordering::SeqCst);
}

#[test]
fn work_is_posted_to_the_ui_thread_from_any_thread() {
    static DATA: AtomicI64 = AtomicI64::new(0);
    ui(|| ());
    std::thread::spawn(|| {
        assert_eq!(ferro_post(Some(posted), 40), FERRO_OK);
        assert_eq!(ferro_post(Some(posted), 2), FERRO_OK);
        assert_eq!(ferro_post_data(Some(posted_data), &DATA as *const AtomicI64 as *mut _), FERRO_OK);
        assert_eq!(ferro_post(None, 0), FERRO_NULL_POINTER);
    })
    .join()
    .unwrap();
    ui(|| assert_eq!(ferro_run_jobs(), FERRO_OK));
    assert_eq!(POSTED.load(Ordering::SeqCst), 42);
    assert_eq!(DATA.load(Ordering::SeqCst), 1);
}

#[test]
fn the_application_takes_no_options_yet() {
    let code = ui(|| ferro_app_run(1 as *const std::ffi::c_void, Some(start_never), 0));
    assert_eq!(code, FERRO_INVALID_ARGUMENT);
    let code = ui(|| ferro_app_run(std::ptr::null(), None, 0));
    assert_eq!(code, FERRO_NULL_POINTER);
}

#[cfg(not(feature = "desktop"))]
#[test]
fn without_the_desktop_platform_the_application_does_not_run() {
    let code = ui(|| ferro_app_run(std::ptr::null(), Some(start_never), 0));
    assert_eq!(code, FERRO_NOT_SUPPORTED);
}

#[test]
fn only_a_window_is_shown_or_closed() {
    ui(|| {
        let button = create("Button");
        assert_eq!(ferro_show(button), FERRO_TYPE_MISMATCH);
        assert_eq!(ferro_close(button), FERRO_TYPE_MISMATCH);
        assert_eq!(ferro_release(button), FERRO_OK);
    });
}

#[test]
fn the_last_error_is_per_thread() {
    std::thread::spawn(|| {
        assert_eq!(ferro_last_error_code(), FERRO_OK);
        assert_eq!(last_error(), "");
        assert_eq!(type_id("Nothing"), i64::from(FERRO_NOT_FOUND));
        assert_eq!(ferro_last_error_code(), FERRO_NOT_FOUND);
        // A successful call leaves the last error as it is.
        assert!(type_id("Button") > 0);
        assert_eq!(ferro_last_error_code(), FERRO_NOT_FOUND);
    })
    .join()
    .unwrap();
}
