//! Native source Browser docking overlay and compiled resources.
// The shared 0.3.0 emitter clones Copy literals and repeats nullable command
// setter branches. Confine these allowances to generated markup; handwritten
// source code remains checked with -D warnings.
#[allow(clippy::clone_on_copy, clippy::if_same_then_else)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_xaml};
mod browser_tab_theme;
mod platform_metadata;
#[doc(hidden)]
pub use platform_metadata::{OnPlatformDoubleMetadata, OnPlatformThicknessMetadata};
mod register_types;
pub use browser_tab_theme::BrowserTabTheme;
pub use register_types::{register_types, ASSEMBLY};
#[cfg(test)]
mod theme_tests;
