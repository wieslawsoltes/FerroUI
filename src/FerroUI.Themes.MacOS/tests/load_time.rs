//! A measurement, not a check: the time the WinUI theme takes to load. The
//! theme is built by its compiled markup (`compiled_xaml.rs`): no document
//! is parsed, transformed or interpreted at start-up. The first load also
//! pays for what the process does once (the registration of the types);
//! the second load is the build alone.
//!
//! ```text
//! cargo test --release -p ferroui-themes-macos --lib tests::load_time -- --ignored --nocapture
//! ```

use std::time::Instant;

use super::support::start_application;
use crate::MacOSTheme;

#[test]
#[ignore = "a measurement: prints the time the theme takes to load"]
fn measure_theme_load_time() {
    let _app = start_application();
    let start = Instant::now();
    let first = MacOSTheme::new();
    let cold = start.elapsed();
    let start = Instant::now();
    let second = MacOSTheme::new();
    let warm = start.elapsed();
    println!("MacOSTheme: first load {cold:?}, second load {warm:?}");
    drop((first, second));
}
