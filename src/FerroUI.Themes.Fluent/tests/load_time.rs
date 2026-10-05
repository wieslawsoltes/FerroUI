//! A measurement, not a check: the time the Fluent theme takes to load, which
//! today is parsing, transforming and interpreting its documents through the
//! run-time loader at start-up (the work the ahead-of-time XAML compiler
//! removes). The first load includes the parse and the transform; the
//! second reuses the transformed documents the loader caches and only
//! interprets them. Not a test of upstream.
//!
//! ```text
//! cargo test --release -p ferroui-themes-fluent --lib tests::load_time -- --ignored --nocapture
//! ```

use std::time::Instant;

use super::support::start_application;
use crate::FluentTheme;

#[test]
#[ignore = "a measurement: prints the time the theme takes to load"]
fn measure_theme_load_time() {
    let _app = start_application();
    let start = Instant::now();
    let first = FluentTheme::new();
    let cold = start.elapsed();
    let start = Instant::now();
    let second = FluentTheme::new();
    let warm = start.elapsed();
    println!("FluentTheme: first load {cold:?}, second load {warm:?}");
    drop((first, second));
}
