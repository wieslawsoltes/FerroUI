//! Opt-in native catalog animation measurements. No screenshots/readbacks during a flight.
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::Window;
use std::time::Duration;

fn later(ms: u64, action: impl Fn() + 'static) {
    DispatcherTimer::run_once(
        action,
        Duration::from_millis(ms),
        DispatcherPriority::NORMAL,
    )
    .detach();
}

pub fn install() {
    Window::window_opened_event()
        .add_class_handler::<Window>(|window, _| {
            let weak = window.to_ref().downgrade();
            later(1000, || super::startup_probe::checkpoint("startup_idle"));
            if std::env::var_os("FERROUI_ANIMATION_PROBE_IDLE").is_some() {
                later(3000, move || {
                    super::startup_probe::checkpoint("idle_end");
                    if let Some(window) = weak.upgrade() {
                        window.close();
                    }
                });
                return;
            }
            later(1500, || {
                assert!(control_catalog::select_page("Connected animations"))
            });
            later(3000, || {
                let configuration = std::env::var("FERROUI_ANIMATION_PROBE_CONFIGURATION")
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0);
                assert!(control_catalog::pages::smoke_set_configuration(
                    configuration
                ));
                super::startup_probe::checkpoint("page_ready");
            });
            let rounds = std::env::var("FERROUI_ANIMATION_PROBE_ROUNDS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(6u64);
            for index in 0..rounds {
                let start = 3500 + index * 2500;
                later(start, move || {
                    eprintln!("ANIMATION_BEGIN {index}");
                    super::startup_probe::checkpoint(&format!("flight_{index}_begin"));
                    assert!(control_catalog::pages::smoke_open_card(0));
                    super::startup_probe::checkpoint(&format!("flight_{index}_prepared"));
                });
                later(start + 1000, move || {
                    super::startup_probe::checkpoint(&format!("flight_{index}_end"));
                    eprintln!("ANIMATION_END {index}");
                });
                later(start + 1200, move || {
                    eprintln!("RETURN_BEGIN {index}");
                    assert!(control_catalog::pages::smoke_close_card());
                });
                later(start + 2200, move || eprintln!("RETURN_END {index}"));
            }
            later(3500 + rounds * 2500, move || {
                super::startup_probe::checkpoint("animations_end");
                if let Some(window) = weak.upgrade() {
                    window.close();
                }
            });
        })
        .detach();
}
