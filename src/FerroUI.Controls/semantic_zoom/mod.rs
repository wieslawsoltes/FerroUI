//! WinUI's `SemanticZoom` and its interface of the views (docs/porting/winui-controls.md, section
//! 10.20).

#[allow(clippy::module_inception)]
mod semantic_zoom;
mod semantic_zoom_information;
mod semantic_zoom_transition;

#[cfg(test)]
mod semantic_zoom_tests;

pub use semantic_zoom::SemanticZoom;
pub(crate) use semantic_zoom::ViewChangeInput;
pub use semantic_zoom_information::{
    as_semantic_zoom_information, register_semantic_zoom_information, ISemanticZoomInformation, SemanticZoomLocation,
    SemanticZoomViewChangedEventArgs,
};
