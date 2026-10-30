//! Native source Fluent docking theme and compiled resources.
// The shared 0.3.0 emitter clones Copy literals and repeats nullable command
// setter branches. Confine these allowances to generated markup; handwritten
// source code remains checked with -D warnings.
#[allow(clippy::clone_on_copy, clippy::if_same_then_else)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_presets, compiled_xaml};
mod dock_fluent_theme;
mod register_types;
pub use dock_fluent_theme::DockFluentTheme;
pub use register_types::{register_types, ASSEMBLY};
mod dock_fluent_theme_manager;
pub use dock_fluent_theme_manager::DockFluentThemeManager;
#[cfg(test)]
mod theme_manager_tests;
