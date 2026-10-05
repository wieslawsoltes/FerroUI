//! A control with two views: a collapsible pane and an area for content.

#[allow(clippy::module_inception)]
mod split_view;
mod split_view_display_mode;
mod split_view_pane_placement;
mod split_view_template_settings;

pub use split_view::{SplitView, SplitViewImpl, SplitViewImplExt, SplitViewVTable};
pub use split_view_display_mode::SplitViewDisplayMode;
pub use split_view_pane_placement::SplitViewPanePlacement;
pub use split_view_template_settings::SplitViewTemplateSettings;

#[cfg(test)]
mod split_view_tests;
