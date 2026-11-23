//! ferroui-controls-teaching-tip
//!
//! `TeachingTip` of WinUI: a dismissible message that points at a target, on a popup. The
//! control theme and its resources are part of this crate: an application that uses the control
//! includes them, and one that does not links none of this crate:
//!
//! ```xml
//! <StyleInclude Source="ferres://FerroUI.Controls.TeachingTip/Themes/WinUI/WinUI.xaml" />
//! ```

// The compiled markup names the types of the crate by the name of the crate.
extern crate self as ferroui_controls_teaching_tip;

// `compiled_xaml` (the compiled markup of the theme documents) and `compiled_markup` (the loader
// table of the crate and `register()`), which the build script generates.
ferroui_markup_xaml::include_compiled_xaml!();

#[cfg(any(test, feature = "document-assets"))]
mod assets;
mod markup;
mod register_types;
mod rust_paths;
mod teaching_tip;

pub use markup::TeachingTipThemeKeys;
pub use register_types::{register_types, ASSEMBLY};
pub use teaching_tip::{
    TeachingTip, TeachingTipCloseReason, TeachingTipClosedEventArgs, TeachingTipClosingEventArgs,
    TeachingTipPlacementMode, TeachingTipTailVisibility,
};

#[cfg(test)]
mod tests;
