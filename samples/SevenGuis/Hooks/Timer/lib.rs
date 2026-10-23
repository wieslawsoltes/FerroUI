//! Timer: a gauge of the elapsed time, the elapsed seconds, a slider for the
//! duration and a button that resets the elapsed time.
//!
//! Written as components that run again, with their state in hooks; `cargo run -p seven-guis-timer-hooks`.

use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_view::prelude::*;
use std::time::{Duration, Instant};

/// The timer, ticking on the dispatcher. The clock is an effect of the
/// component: started once its controls are in place, stopped when it is
/// taken out.
#[component(rerender)]
pub fn Timer() -> Element {
    let elapsed = use_signal(|| 0.0_f64);
    let duration = use_signal(|| 15.0_f64);
    use_effect_with((), move |_| {
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
    });
    rsx! { TimerView { elapsed, duration } }
}

/// The user interface of the timer over its two signals: what the tests
/// drive without a clock. It runs again on every tick.
#[component(rerender)]
pub fn TimerView(elapsed: Signal<f64>, duration: Signal<f64>) -> Element {
    let (seconds, limit) = (elapsed.get(), duration.get());
    let fraction = if limit > 0.0 { (seconds / limit).min(1.0) } else { 1.0 };
    rsx! {
        StackPanel { spacing: 8.0, margin: 16.0, width: 300.0,
            TextBlock { text: "Elapsed time:" }
            ProgressBar { name: "Gauge", minimum: 0.0, maximum: 1.0, value: fraction }
            TextBlock { name: "Seconds", text: format!("{seconds:.1} s") }
            TextBlock { text: "Duration:" }
            Slider { name: "Duration", minimum: 0.0, maximum: 30.0, value: duration.two_way() }
            Button { name: "Reset", on_click: move |_| elapsed.set(0.0), "Reset" }
        }
    }
}
