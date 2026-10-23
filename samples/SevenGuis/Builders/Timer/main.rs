//! The task in a window of its own.

fn main() {
    seven_guis_model::run("builders-timer", "Timer", 360.0, 300.0, seven_guis_timer_builders::timer);
}
