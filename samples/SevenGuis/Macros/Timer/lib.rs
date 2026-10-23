//! Timer: a gauge of the elapsed time, its value in seconds, a slider of
//! the duration, and a button that starts the elapsed time again. The time
//! runs up to the duration and follows the slider: a longer duration lets it
//! run on.
//!
//! Written with the `rsx!` macro of `ferroui-view`; `cargo run -p seven-guis-timer-macros`.

use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_view::prelude::*;
use std::time::{Duration, Instant};

/// The timer, ticking on the dispatcher.
#[component]
pub fn Timer() -> Element {
    let elapsed = signal(0.0_f64);
    let duration = signal(15.0_f64);
    let last = std::cell::Cell::new(Instant::now());
    let ticking = DispatcherTimer::run(
        move || {
            let now = Instant::now();
            let step = now.duration_since(last.replace(now)).as_secs_f64();
            if elapsed.is_alive() {
                let limit = duration.get_untracked();
                if elapsed.get_untracked() < limit {
                    elapsed.set((elapsed.get_untracked() + step).min(limit));
                }
            }
            elapsed.is_alive()
        },
        Duration::from_millis(50),
        DispatcherPriority::NORMAL,
    );
    on_cleanup(move || ticking.dispose());
    rsx! { TimerView { elapsed, duration } }
}

/// The user interface of the timer over its two signals: what the tests
/// drive without a clock.
#[component]
pub fn TimerView(elapsed: Signal<f64>, duration: Signal<f64>) -> Element {
    rsx! {
        StackPanel { spacing: 8.0, margin: 16.0, width: 300.0,
            TextBlock { text: "Elapsed time:" }
            ProgressBar { name: "Gauge", minimum: 0.0, maximum: 1.0,
                value: move || if duration.get() > 0.0 { (elapsed.get() / duration.get()).min(1.0) } else { 1.0 } }
            TextBlock { name: "Seconds", text: move || format!("{:.1} s", elapsed.get()) }
            TextBlock { text: "Duration:" }
            Slider { name: "Duration", minimum: 0.0, maximum: 30.0, value: duration.two_way() }
            Button { name: "Reset", on_click: move |_| elapsed.set(0.0), "Reset" }
        }
    }
}
