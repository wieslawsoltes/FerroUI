//! The cost of a call through the C ABI against the same operation in Rust (docs/c-abi.md,
//! section 8; #636, "Measures"):
//!
//! - property writes and a read: `ferro_set_f64` of `FontSize`, `ferro_set_str` of `Text` and
//!   `ferro_get_f64` of `FontSize` against `TextBlock::set_font_size`, `set_text` and
//!   `font_size`;
//! - event dispatch: a raise of `Click` to a handler subscribed through the ABI
//!   (`ferro_subscribe`: a C function and a token) against the same raise to a Rust handler, and
//!   the whole `ferro_raise` call against the raise in Rust.
//!
//! `cargo bench -p ferroui-capi --no-default-features --bench call_overhead`. No window and no
//! renderer: the thread is the UI thread through `ferro_init`. Each case alternates between two
//! values (a write of the value a property already has is skipped by the property system), runs
//! `ITERATIONS` times per sample, and the median of `SAMPLES` samples is printed in nanoseconds
//! per operation, with the difference of the ABI case to the Rust case.

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

/// The median time of one operation in nanoseconds: `operation(i)` runs `ITERATIONS` times per
/// sample, after one sample of warm-up.
fn measure(mut operation: impl FnMut(u32)) -> f64 {
    for i in 0..ITERATIONS {
        operation(i);
    }
    let mut samples: Vec<f64> = (0..SAMPLES)
        .map(|_| {
            let start = Instant::now();
            for i in 0..ITERATIONS {
                operation(black_box(i));
            }
            start.elapsed().as_nanos() as f64 / f64::from(ITERATIONS)
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

fn report(name: &str, rust: f64, abi: f64) {
    println!("{name:<40} rust {rust:>8.1} ns/op   abi {abi:>8.1} ns/op   overhead {:>7.1} ns", abi - rust);
}

fn main() {
    assert_eq!(ferro_init(), FERRO_OK);
    let text_type = id(ferro_type_id, c"TextBlock");
    let button_type = id(ferro_type_id, c"Button");
    let font_size = id(ferro_property_id, c"FontSize");
    let text = id(ferro_property_id, c"Text");
    let click = id(ferro_event_id, c"Click");
    println!("call_overhead: {ITERATIONS} operations per sample, the median of {SAMPLES} samples");

    // Property writes and a read, each side on a TextBlock of its own.
    let rust_text = TextBlock::new();
    let handle = ferro_create(text_type);
    assert_ne!(handle, 0);
    let rust = measure(|i| rust_text.set_font_size(if i & 1 == 0 { 12.0 } else { 13.0 }));
    let abi = measure(|i| {
        let code = ferro_set_f64(handle, font_size, if i & 1 == 0 { 12.0 } else { 13.0 });
        debug_assert_eq!(code, FERRO_OK);
    });
    report("write FontSize (double)", rust, abi);

    let values: [&CStr; 2] = [c"Hello, World!", c"Clicked 1 time"];
    let texts: [&str; 2] = ["Hello, World!", "Clicked 1 time"];
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

    let rust = measure(|_| {
        black_box(rust_text.font_size());
    });
    let abi = measure(|_| {
        let mut value = 0.0;
        // SAFETY: a pointer to a local.
        let code = unsafe { ferro_get_f64(handle, font_size, &mut value) };
        debug_assert_eq!(code, FERRO_OK);
        black_box(value);
    });
    report("read FontSize (double)", rust, abi);

    // Event dispatch: a raise of Click on a button with one handler, a Rust closure on one side
    // and a C function with a token, subscribed through the ABI, on the other.
    let rust_button = Button::new();
    let rust_clicks = Rc::new(Cell::new(0u64));
    let counter = rust_clicks.clone();
    rust_button.add_handler(Button::click_event(), move |_, _| counter.set(counter.get() + 1));
    let rust = measure(|_| rust_button.raise_event(&RoutedEventArgs::with_event(Button::click_event())));

    let button = ferro_create(button_type);
    assert_ne!(button, 0);
    assert!(ferro_subscribe(button, click, Some(on_click), 1) > 0);
    let abi_button = object_of(button).and_then(|object| object.cast::<Button>()).expect("a button");
    let abi = measure(|_| abi_button.raise_event(&RoutedEventArgs::with_event(Button::click_event())));
    report("dispatch Click to one handler", rust, abi);

    let abi = measure(|_| {
        let code = ferro_raise(button, click);
        debug_assert_eq!(code, FERRO_OK);
    });
    report("raise Click (ferro_raise, whole call)", rust, abi);

    assert_eq!(rust_clicks.get(), ABI_CLICKS.load(Ordering::Relaxed) / 2, "every raise reached its handler");
}
