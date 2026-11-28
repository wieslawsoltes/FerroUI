//! What a language without pointers to locals or closures needs beyond the core (#950): getters
//! that return the value, timers with a token, the data of the routed event being handled, and
//! the keyboard focus.

use super::support::*;
use crate::*;
use ferroui_base::input::{InputElement, KeyEventArgs, Pointer, PointerPointProperties, PointerPressedEventArgs};
use ferroui_base::input::{Key, KeyModifiers, PointerType, PointerUpdateKind, RawInputModifiers};
use ferroui_base::interactivity::Interactive;
use ferroui_base::{Point, Visual};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

#[test]
fn getters_with_a_fallback_return_the_value() {
    ui(|| {
        let text = create("TextBlock");
        assert_eq!(ferro_set_f64(text, property_id("FontSize"), 21.5), FERRO_OK);
        assert_eq!(ferro_get_f64_or(text, property_id("FontSize"), -1.0), 21.5);
        let list = create("ListBox");
        assert_eq!(ferro_get_i64_or(list, property_id("SelectedIndex"), 7), -1);
        let button = create("Button");
        assert_eq!(ferro_set_bool(button, property_id("IsEnabled"), false), FERRO_OK);
        assert!(!ferro_get_bool_or(button, property_id("IsEnabled"), true));
        for handle in [text, list, button] {
            assert_eq!(ferro_release(handle), FERRO_OK);
        }
    });
}

#[test]
fn getters_with_a_fallback_return_it_on_a_failure() {
    ui(|| {
        let text = create("TextBlock");
        assert_eq!(ferro_get_f64_or(text, property_id("NoSuchProperty"), -1.0), -1.0);
        assert_eq!(ferro_last_error_code(), FERRO_NOT_FOUND);
        assert_eq!(ferro_get_i64_or(0, property_id("FontSize"), 42), 42);
        assert_eq!(ferro_last_error_code(), FERRO_BAD_HANDLE);
        // Text is not a boolean.
        assert!(ferro_get_bool_or(text, property_id("Text"), true));
        assert_eq!(ferro_last_error_code(), FERRO_TYPE_MISMATCH);
        assert_eq!(ferro_release(text), FERRO_OK);
    });
    // Another thread: the fallback, and the failure on that thread.
    assert_eq!(ferro_get_i64_or(1, property_id("FontSize"), 5), 5);
    assert_eq!(ferro_last_error_code(), FERRO_WRONG_THREAD);
}

static TICKS: AtomicI64 = AtomicI64::new(0);
static ELAPSED: AtomicI64 = AtomicI64::new(0);
static TIMER: AtomicI64 = AtomicI64::new(0);

/// Counts its ticks; the token says after how many it stops itself (0: never).
unsafe extern "C" fn tick(token: i64, elapsed: i64) -> i64 {
    let ticks = TICKS.fetch_add(1, Ordering::SeqCst) + 1;
    ELAPSED.fetch_add(elapsed, Ordering::SeqCst);
    i64::from(token != 0 && ticks >= token)
}

/// Stops its own timer from inside its tick.
unsafe extern "C" fn stops_itself(_token: i64, _elapsed: i64) -> i64 {
    TICKS.fetch_add(1, Ordering::SeqCst);
    assert_eq!(ferro_timer_stop(TIMER.load(Ordering::SeqCst)), FERRO_OK, "{}", last_error());
    0
}

unsafe extern "C" fn nothing(_token: i64) -> i64 {
    0
}

/// Waits a little longer than `ms` and runs the jobs of the UI thread, timers that are due
/// included (a posted job lets the dispatcher look at its timers).
fn run_for(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
    assert_eq!(ferro_post(Some(nothing), 0), FERRO_OK);
    assert_eq!(ferro_run_jobs(), FERRO_OK);
}

/// The timer tests share the counters: one at a time.
static TIMER_TESTS: Mutex<()> = Mutex::new(());

#[test]
fn a_timer_ticks_with_its_token_and_the_elapsed_time_until_it_is_stopped() {
    let _one = TIMER_TESTS.lock().unwrap_or_else(|error| error.into_inner());
    ui(|| {
        TICKS.store(0, Ordering::SeqCst);
        ELAPSED.store(0, Ordering::SeqCst);
        let timer = ferro_timer_start(5, Some(tick), 0);
        assert!(timer > 0, "{}", last_error());
        for _ in 0..20 {
            if TICKS.load(Ordering::SeqCst) >= 2 {
                break;
            }
            run_for(10);
        }
        let ticks = TICKS.load(Ordering::SeqCst);
        assert!(ticks >= 2, "the timer ticked {ticks} times");
        assert!(ELAPSED.load(Ordering::SeqCst) >= 5_000, "the elapsed time is in microseconds");
        assert_eq!(ferro_timer_stop(timer), FERRO_OK);
        assert_eq!(ferro_timer_stop(timer), FERRO_NOT_FOUND);
        let ticks = TICKS.load(Ordering::SeqCst);
        run_for(20);
        assert_eq!(TICKS.load(Ordering::SeqCst), ticks, "a stopped timer does not tick");
    });
}

#[test]
fn a_timer_stops_when_its_function_says_so() {
    let _one = TIMER_TESTS.lock().unwrap_or_else(|error| error.into_inner());
    ui(|| {
        TICKS.store(0, Ordering::SeqCst);
        let timer = ferro_timer_start(2, Some(tick), 1);
        assert!(timer > 0, "{}", last_error());
        for _ in 0..20 {
            run_for(10);
        }
        assert_eq!(TICKS.load(Ordering::SeqCst), 1, "the function asked for one tick");
        assert_eq!(ferro_timer_stop(timer), FERRO_NOT_FOUND, "the timer is gone after its last tick");

        TICKS.store(0, Ordering::SeqCst);
        let timer = ferro_timer_start(2, Some(stops_itself), 0);
        TIMER.store(timer, Ordering::SeqCst);
        for _ in 0..20 {
            run_for(10);
        }
        assert_eq!(TICKS.load(Ordering::SeqCst), 1, "the timer stopped itself in its tick");
    });
}

#[test]
fn timers_check_their_arguments_and_the_thread() {
    ui(|| {
        assert_eq!(ferro_timer_start(10, None, 0), i64::from(FERRO_NULL_POINTER));
        assert_eq!(ferro_timer_start(0, Some(tick), 0), i64::from(FERRO_INVALID_ARGUMENT));
        assert_eq!(ferro_timer_start(-5, Some(tick), 0), i64::from(FERRO_INVALID_ARGUMENT));
        assert_eq!(ferro_timer_stop(123_456), FERRO_NOT_FOUND);
    });
    assert_eq!(ferro_timer_start(10, Some(tick), 0), i64::from(FERRO_WRONG_THREAD));
}

/// What the handlers of the event data tests saw: x, y, button and key.
static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn seen() -> std::sync::MutexGuard<'static, Vec<String>> {
    SEEN.lock().unwrap_or_else(|error| error.into_inner())
}

unsafe extern "C" fn pressed(_token: i64, _sender: FerroHandle, _arg: i64) -> i64 {
    let (x, y) = (ferro_event_x(0), ferro_event_y(0));
    let button = ferro_event_button();
    let key = ferro_event_key();
    let key_code = ferro_last_error_code();
    seen().push(format!("{x} {y} {button} {} {key_code}", key.is_null()));
    0
}

unsafe extern "C" fn key_down(_token: i64, _sender: FerroHandle, _arg: i64) -> i64 {
    let key = ferro_event_key();
    assert!(!key.is_null(), "{}", last_error());
    // SAFETY: a NUL-terminated string of the library.
    let key = unsafe { std::ffi::CStr::from_ptr(key) }.to_string_lossy().into_owned();
    let x = ferro_event_x(0);
    seen().push(format!("{key} {} {}", x.is_nan(), ferro_event_button()));
    0
}

/// The interactive of a handle.
fn interactive(handle: FerroHandle) -> ferroui_base::Ref<Interactive> {
    object_of(handle).and_then(|object| object.cast::<Interactive>()).expect("an interactive")
}

#[test]
fn a_handler_reads_the_position_and_the_button_of_a_pointer_event() {
    ui(|| {
        seen().clear();
        let canvas = create("Canvas");
        let subscription = ferro_subscribe(canvas, event_id("PointerPressed"), Some(pressed), 0);
        assert!(subscription > 0, "{}", last_error());
        let target = interactive(canvas);
        let root: ferroui_base::Ref<Visual> = object_of(canvas).and_then(|o| o.cast::<Visual>()).expect("a visual");
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let args = PointerPressedEventArgs::new(
            object_of(canvas).expect("the canvas"),
            pointer,
            &root,
            Point::new(30.5, 40.0),
            1,
            PointerPointProperties::new(RawInputModifiers::RIGHT_MOUSE_BUTTON, PointerUpdateKind::RightButtonPressed),
            KeyModifiers::NONE,
            1,
        );
        target.raise_event(&args);
        assert_eq!(seen().as_slice(), ["30.5 40 2 true -12"]);
        assert_eq!(ferro_unsubscribe(subscription), FERRO_OK);
        assert_eq!(ferro_release(canvas), FERRO_OK);
    });
}

#[test]
fn a_handler_reads_the_key_of_a_key_event() {
    ui(|| {
        seen().clear();
        let text_box = create("TextBox");
        let subscription = ferro_subscribe(text_box, event_id("KeyDown"), Some(key_down), 0);
        assert!(subscription > 0, "{}", last_error());
        let mut args = KeyEventArgs::new();
        args.key = Key::Enter;
        // `Enter` is the second name of the member `Return`.
        args.set_routed_event(Some(&InputElement::key_down_event()));
        interactive(text_box).raise_event(&args);
        // A key event has no position and no button.
        assert_eq!(seen().as_slice(), [format!("Return true {}", FERRO_NOT_SUPPORTED)]);
        assert_eq!(ferro_unsubscribe(subscription), FERRO_OK);
        assert_eq!(ferro_release(text_box), FERRO_OK);
    });
}

unsafe extern "C" fn clicked(_token: i64, _sender: FerroHandle, _arg: i64) -> i64 {
    let x = ferro_event_x(0);
    seen().push(format!("{} {}", x.is_nan(), ferro_last_error_code()));
    0
}

#[test]
fn event_data_is_an_error_outside_a_handler_and_for_an_event_without_it() {
    ui(|| {
        assert!(ferro_event_x(0).is_nan());
        assert_eq!(ferro_last_error_code(), FERRO_INVALID_STATE);
        assert_eq!(ferro_event_button(), i64::from(FERRO_INVALID_STATE));
        assert!(ferro_event_key().is_null());
        seen().clear();
        let button = create("Button");
        let click = event_id("Click");
        assert!(ferro_subscribe(button, click, Some(clicked), 0) > 0);
        assert_eq!(ferro_raise(button, click), FERRO_OK);
        assert_eq!(seen().as_slice(), [format!("true {FERRO_NOT_SUPPORTED}")]);
        // After the handler there is no current event again.
        assert!(ferro_event_y(0).is_nan());
        assert_eq!(ferro_last_error_code(), FERRO_INVALID_STATE);
        assert_eq!(ferro_release(button), FERRO_OK);
    });
    assert!(ferro_event_x(0).is_nan());
    assert_eq!(ferro_last_error_code(), FERRO_WRONG_THREAD);
}

#[test]
fn focus_needs_an_input_element_that_can_take_it() {
    ui(|| {
        // Not in a window: it cannot take the focus.
        let text_box = create("TextBox");
        assert_eq!(ferro_focus(text_box), FERRO_INVALID_STATE);
        let row = create("RowDefinition");
        assert_eq!(ferro_focus(row), FERRO_TYPE_MISMATCH);
        assert_eq!(ferro_focus(0), FERRO_BAD_HANDLE);
        for handle in [text_box, row] {
            assert_eq!(ferro_release(handle), FERRO_OK);
        }
    });
}
