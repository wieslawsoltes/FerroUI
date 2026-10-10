//! The styling benchmarks.
//!
//! Two upstream files are not here, because every benchmark of them brackets
//! what it attaches with the styling pass of the value store of the target
//! (`GetValueStore().BeginStyling()` and `EndStyling()`), and the value store
//! of an object is not reachable from outside the base crate: `Style_Apply`
//! and `Style_ClassSelector`.

use crate::harness::Registry;
use ferroui_controls::testing::TestServices;
use ferroui_themes_simple::SimpleTheme;

pub mod apply_styling;
pub mod control_theme_change;
pub mod resource_benchmarks;
pub mod selector_benchmark;
pub mod style_activation;
pub mod style_apply_detach_complex;
pub mod style_non_active;

/// The services of a styled window as the upstream test library has them:
/// with the Simple theme as the theme of the application. The preset of the
/// controls crate has a theme of its own for the top-level classes only
/// (that crate is not built on the themes), and the benchmarks that start
/// from these services style a text box.
pub(crate) fn styled_window() -> TestServices {
    TestServices::styled_window().with_theme(|| SimpleTheme::new().as_style())
}

pub fn register(registry: &mut Registry) {
    resource_benchmarks::register(registry);
    apply_styling::register(registry);
    selector_benchmark::register(registry);
    style_non_active::register(registry);
    style_apply_detach_complex::register(registry);
    style_activation::register(registry);
    control_theme_change::register(registry);
}
