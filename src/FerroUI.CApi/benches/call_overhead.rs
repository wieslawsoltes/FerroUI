//! The cost of a call through the C ABI against the same operation in Rust (docs/c-abi.md,
//! sections 8 and 13; #636, "Measures"):
//!
//! - property writes and a read: `ferro_set_f64` of `FontSize`, `ferro_set_str` of `Text` and
//!   `ferro_get_f64` of `FontSize` against `TextBlock::set_font_size`, `set_text` and
//!   `font_size`;
//! - the parts of a call: the check of the thread alone (`ferro_is_ui_thread`), the handle
//!   table (`ferro_retain` and `ferro_release`), a read with a fallback (`ferro_get_f64_or`) and
//!   the reset of the error of the thread (`ferro_clear_error`), which a binding that reports
//!   errors as values makes around a read;
//! - event dispatch: a raise of `Click` to a handler subscribed through the ABI
//!   (`ferro_subscribe`: a C function and a token) against the same raise to a Rust handler, the
//!   whole `ferro_raise` call against the raise in Rust, and a call of the one dispatcher of the
//!   frames (`ferro_frame_dispatch`), which a binding with one trampoline makes for every event;
//! - frames (`ferro_frame_*`): a frame of one text block whose text changes, committed under a
//!   root, against `set_text` of the text block; frames of 100 and of 1000 text blocks of which
//!   one changes and of 100 of which none changes, against the one `set_text` that changes the
//!   same text (what the reconcile costs); and the two halves of the 100-element frame: writing
//!   it with the scalar writer, and committing the same bytes from a buffer of the caller.
//!
//! `cargo bench -p ferroui-capi --no-default-features --bench call_overhead`. No window and no
//! renderer: the thread is the UI thread through `ferro_init`. Each case alternates between two
//! values (a write of the value a property already has is skipped by the property system), runs
//! `ITERATIONS` times per sample, and the median of `SAMPLES` samples is printed in nanoseconds
//! per operation, with the difference of the ABI case to the Rust case.
//!
//! `CALL_OVERHEAD_FILTER=<text>` runs only the cases whose name contains the text, and
//! `CALL_OVERHEAD_SCALE=<n>` multiplies the operations per sample (a long run of one case for a
//! profiler).

use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_capi::*;
use ferroui_controls::{Button, TextBlock};
use std::cell::Cell;
use std::ffi::{c_char, CStr};
use std::hint::black_box;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

const ITERATIONS: u32 = 100_000;
const SAMPLES: usize = 9;

fn scale() -> u32 {
    std::env::var("CALL_OVERHEAD_SCALE").ok().and_then(|value| value.parse().ok()).unwrap_or(1).max(1)
}

/// Whether the case `name` runs (`CALL_OVERHEAD_FILTER`).
fn selected(name: &str) -> bool {
    std::env::var("CALL_OVERHEAD_FILTER").map_or(true, |filter| name.contains(filter.as_str()))
}

/// The median time of one operation in nanoseconds: `operation(i)` runs `ITERATIONS` times per
/// sample, after one sample of warm-up.
fn measure(operation: impl FnMut(u32)) -> f64 {
    measure_n(ITERATIONS, operation)
}

/// [`measure`] with `iterations` operations per sample.
fn measure_n(iterations: u32, mut operation: impl FnMut(u32)) -> f64 {
    let iterations = iterations * scale();
    for i in 0..iterations {
        operation(i);
    }
    let mut samples: Vec<f64> = (0..SAMPLES)
        .map(|_| {
            let start = Instant::now();
            for i in 0..iterations {
                operation(black_box(i));
            }
            start.elapsed().as_nanos() as f64 / f64::from(iterations)
        })
        .collect();
    samples.sort_by(f64::total_cmp);
    samples[SAMPLES / 2]
}

fn id(lookup: unsafe extern "C" fn(*const c_char, i64) -> i64, name: &CStr) -> i64 {
    // SAFETY: a NUL-terminated name.
    let id = unsafe { lookup(name.as_ptr(), -1) };
    assert!(id > 0, "{name:?}");
    id
}

static ABI_CLICKS: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" fn on_click(token: i64, _sender: FerroHandle, _arg: i64) -> i64 {
    ABI_CLICKS.fetch_add(token as u64, Ordering::Relaxed);
    0
}

static DISPATCHED: AtomicU64 = AtomicU64::new(0);

unsafe extern "C" fn dispatcher(tag: i64, _sender: FerroHandle, _arg: i64) -> i64 {
    DISPATCHED.fetch_add(tag as u64, Ordering::Relaxed);
    0
}

fn report(name: &str, rust: f64, abi: f64) {
    println!("{name:<44} rust {rust:>9.1} ns/op   abi {abi:>9.1} ns/op   overhead {:>8.1} ns", abi - rust);
}

/// A case that has no Rust counterpart.
fn report_abi(name: &str, abi: f64) {
    println!("{name:<44} {:>18}   abi {abi:>9.1} ns/op", "");
}

/// Writes a frame of a stack panel of `count` text blocks with the scalar writer; the first has
/// the text of iteration `i` when `changes`, every other its label.
fn write_texts(text_type: i64, panel_type: i64, text: i64, labels: &[String], values: &[&str; 2], i: u32, changes: bool) {
    ferro_frame_begin();
    ferro_frame_open(panel_type, 0);
    for (n, label) in labels.iter().enumerate() {
        ferro_frame_open(text_type, n as i64 + 1);
        let value = if n == 0 && changes { values[(i & 1) as usize] } else { label.as_str() };
        // SAFETY: a pointer and the length of a Rust string.
        unsafe { ferro_frame_str(text, value.as_ptr().cast(), value.len() as i64) };
        ferro_frame_close();
    }
    ferro_frame_close();
}

/// The records of the frame [`write_texts`] writes, built in Rust in the layout of the model.
fn frame_bytes(text_type: i64, panel_type: i64, text: i64, labels: &[String], first: &str) -> Vec<u8> {
    fn record(buffer: &mut Vec<u8>, op: u32, words: &[i64], tail: &[u8]) {
        let size = 8 + 8 * words.len() + tail.len().div_ceil(8) * 8;
        buffer.extend_from_slice(&op.to_le_bytes());
        buffer.extend_from_slice(&(size as u32).to_le_bytes());
        for word in words {
            buffer.extend_from_slice(&word.to_le_bytes());
        }
        buffer.extend_from_slice(tail);
        buffer.resize(buffer.len() + (tail.len().div_ceil(8) * 8 - tail.len()), 0);
    }
    let mut bytes = Vec::new();
    record(&mut bytes, 1, &[panel_type, 0], &[]);
    for (n, label) in labels.iter().enumerate() {
        record(&mut bytes, 1, &[text_type, n as i64 + 1], &[]);
        let value = if n == 0 { first } else { label.as_str() };
        record(&mut bytes, 3, &[text, value.len() as i64], value.as_bytes());
        record(&mut bytes, 2, &[], &[]);
    }
    record(&mut bytes, 2, &[], &[]);
    bytes
}

fn main() {
    assert_eq!(ferro_init(), FERRO_OK);
    let text_type = id(ferro_type_id, c"TextBlock");
    let button_type = id(ferro_type_id, c"Button");
    let font_size = id(ferro_property_id, c"FontSize");
    let text = id(ferro_property_id, c"Text");
    let click = id(ferro_event_id, c"Click");
    println!("call_overhead: {} operations per sample, the median of {SAMPLES} samples", ITERATIONS * scale());

    // Property writes and a read, each side on a TextBlock of its own.
    let rust_text = TextBlock::new();
    let handle = ferro_create(text_type);
    assert_ne!(handle, 0);
    if selected("write FontSize (double)") {
        let rust = measure(|i| rust_text.set_font_size(if i & 1 == 0 { 12.0 } else { 13.0 }));
        let abi = measure(|i| {
            let code = ferro_set_f64(handle, font_size, if i & 1 == 0 { 12.0 } else { 13.0 });
            debug_assert_eq!(code, FERRO_OK);
        });
        report("write FontSize (double)", rust, abi);
    }

    let values: [&CStr; 2] = [c"Hello, World!", c"Clicked 1 time"];
    let texts: [&str; 2] = ["Hello, World!", "Clicked 1 time"];
    if selected("write Text") {
        let rust = measure(|i| rust_text.set_text(Some(texts[(i & 1) as usize])));
        let abi = measure(|i| {
            // SAFETY: NUL-terminated literals.
            let code = unsafe { ferro_set_str(handle, text, values[(i & 1) as usize].as_ptr(), -1) };
            debug_assert_eq!(code, FERRO_OK);
        });
        report("write Text (NUL-terminated string)", rust, abi);
        let abi = measure(|i| {
            let value = texts[(i & 1) as usize];
            // SAFETY: a pointer and the length of a Rust string.
            let code = unsafe { ferro_set_str(handle, text, value.as_ptr().cast(), value.len() as i64) };
            debug_assert_eq!(code, FERRO_OK);
        });
        report("write Text (string with a length)", rust, abi);
    }

    let rust_read = || {
        measure(|_| {
            black_box(rust_text.font_size());
        })
    };
    if selected("read FontSize (double)") {
        let rust = rust_read();
        let abi = measure(|_| {
            let mut value = 0.0;
            // SAFETY: a pointer to a local.
            let code = unsafe { ferro_get_f64(handle, font_size, &mut value) };
            debug_assert_eq!(code, FERRO_OK);
            black_box(value);
        });
        report("read FontSize (double)", rust, abi);
    }
    if selected("read FontSize with a fallback") {
        let rust = rust_read();
        let abi = measure(|_| {
            black_box(ferro_get_f64_or(handle, font_size, -1.0));
        });
        report("read FontSize with a fallback", rust, abi);
    }

    // The parts of a call.
    if selected("check of the thread") {
        report_abi("check of the thread (ferro_is_ui_thread)", measure(|_| assert!(black_box(ferro_is_ui_thread()))));
    }
    if selected("handle table") {
        let abi = measure(|_| {
            let code = ferro_retain(black_box(handle));
            debug_assert_eq!(code, FERRO_OK);
            let code = ferro_release(black_box(handle));
            debug_assert_eq!(code, FERRO_OK);
        });
        report_abi("handle table (ferro_retain + ferro_release)", abi);
    }
    if selected("clear the error") {
        report_abi("clear the error (ferro_clear_error)", measure(|_| ferro_clear_error()));
    }

    // Event dispatch: a raise of Click on a button with one handler, a Rust closure on one side
    // and a C function with a token, subscribed through the ABI, on the other.
    let rust_button = Button::new();
    let rust_clicks = Rc::new(Cell::new(0u64));
    let counter = rust_clicks.clone();
    rust_button.add_handler(Button::click_event(), move |_, _| counter.set(counter.get() + 1));
    let rust_raise = || measure(|_| rust_button.raise_event(&RoutedEventArgs::with_event(Button::click_event())));

    let button = ferro_create(button_type);
    assert_ne!(button, 0);
    assert!(ferro_subscribe(button, click, Some(on_click), 1) > 0);
    let abi_button = object_of(button).and_then(|object| object.cast::<Button>()).expect("a button");
    if selected("dispatch Click") {
        let rust = rust_raise();
        let abi = measure(|_| abi_button.raise_event(&RoutedEventArgs::with_event(Button::click_event())));
        report("dispatch Click to one handler", rust, abi);
        assert_eq!(rust_clicks.get(), ABI_CLICKS.load(Ordering::Relaxed), "every raise reached its handler");
    }
    if selected("raise Click") {
        let rust = rust_raise();
        let abi = measure(|_| {
            let code = ferro_raise(button, click);
            debug_assert_eq!(code, FERRO_OK);
        });
        report("raise Click (ferro_raise, whole call)", rust, abi);
    }
    assert_eq!(ferro_frame_dispatcher(Some(dispatcher)), FERRO_OK);
    if selected("frame dispatcher") {
        report_abi("call the frame dispatcher (ferro_frame_dispatch)", measure(|i| {
            black_box(ferro_frame_dispatch(i64::from(i & 1), button, 0));
        }));
    }

    // Frames: the scalar writer and a commit under a root, against the write it comes down to.
    let border_type = id(ferro_type_id, c"Border");
    let panel_type = id(ferro_type_id, c"StackPanel");
    let root = ferro_create(border_type);
    let rust_one = || measure(|i| rust_text.set_text(Some(texts[(i & 1) as usize])));
    if selected("frame: one text block") {
        let one = measure(|i| {
            ferro_frame_begin();
            ferro_frame_open(text_type, 1);
            let value = texts[(i & 1) as usize];
            // SAFETY: a pointer and the length of a Rust string.
            unsafe { ferro_frame_str(text, value.as_ptr().cast(), value.len() as i64) };
            ferro_frame_close();
            let code = ferro_frame_commit(root);
            debug_assert_eq!(code, FERRO_OK);
        });
        report("frame: one text block, its text changes", rust_one(), one);
        assert_eq!(ferro_frame_clear(root), FERRO_OK);
    }

    for count in [100usize, 1000] {
        let labels: Vec<String> = (0..count).map(|n| format!("Item {n}")).collect();
        let name = format!("frame: {count} text blocks, one changes");
        if selected(&name) {
            let many = measure_n(ITERATIONS / count as u32, |i| {
                write_texts(text_type, panel_type, text, &labels, &texts, i, true);
                let code = ferro_frame_commit(root);
                debug_assert_eq!(code, FERRO_OK);
            });
            report(&name, rust_one(), many);
            assert_eq!(ferro_frame_clear(root), FERRO_OK);
        }
        if count != 100 {
            continue;
        }
        if selected("frame: 100 text blocks, none changes") {
            let none = measure_n(ITERATIONS / 100, |i| {
                write_texts(text_type, panel_type, text, &labels, &texts, i, false);
                let code = ferro_frame_commit(root);
                debug_assert_eq!(code, FERRO_OK);
            });
            report_abi("frame: 100 text blocks, none changes", none);
            assert_eq!(ferro_frame_clear(root), FERRO_OK);
        }
        if selected("frame: 100 text blocks, the writer only") {
            let writer = measure_n(ITERATIONS / 100, |i| {
                write_texts(text_type, panel_type, text, &labels, &texts, i, true);
            });
            report_abi("frame: 100 text blocks, the writer only", writer);
            ferro_frame_begin();
        }
        if selected("frame: 100 text blocks, commit of bytes") {
            let frames = [
                frame_bytes(text_type, panel_type, text, &labels, texts[0]),
                frame_bytes(text_type, panel_type, text, &labels, texts[1]),
            ];
            let bytes = measure_n(ITERATIONS / 100, |i| {
                let frame = &frames[(i & 1) as usize];
                // SAFETY: a buffer of `frame.len()` bytes.
                let code = unsafe { ferro_frame_commit_bytes(root, frame.as_ptr(), frame.len() as i64) };
                debug_assert_eq!(code, FERRO_OK);
            });
            report("frame: 100 text blocks, commit of bytes", rust_one(), bytes);
            assert_eq!(ferro_frame_clear(root), FERRO_OK);
        }
    }
    assert!(DISPATCHED.load(Ordering::Relaxed) > 0 || !selected("frame dispatcher"));
}
