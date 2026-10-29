//! ferroui-xaml-interactions-draggable
//!
//! Behaviors that drag elements in place, ported from the `Xaml.Behaviors.Interactions.Draggable`
//! project of Xaml.Behaviors (see `NOTICE.md`):
//!
//! - [`CanvasDragBehavior`]: moves a child of a canvas with the pointer;
//! - [`GridDragBehavior`]: swaps the cells of a child of a grid with the child under the pointer;
//! - [`ItemDragBehavior`] and [`ListReorderDragBehavior`]: reorder the items of an items control,
//!   the second with a placeholder at the insertion point;
//! - [`MouseDragElementBehavior`] and [`MultiMouseDragElementBehavior`]: move one or several
//!   controls by a translation;
//! - [`AutoScrollDuringDragBehavior`]: scrolls a scroll viewer while the pointer is dragged near
//!   its edges;
//! - [`SelectionAdorner`]: the corner handles shown over a dragged element.
//!
//! The styles of the reference (`Styles.xaml`) make tab controls, list boxes, canvases and grids
//! with the class `draggable` draggable; the build of the crate compiles them, and an application
//! includes them with
//! `<StyleInclude Source="ferres://FerroUI.Xaml.Interactions.Draggable/Styles.xaml" />`.
//!
//! The namespace of the crate is `FerroUI.Xaml.Interactions.Draggable`, mapped into the default
//! XML namespace of the framework: documents name its types without a prefix once
//! [`register_types`] ran.

// The compiled markup names the types of the crate by the name of the crate, as the markup of any
// other crate does.
extern crate self as ferroui_xaml_interactions_draggable;

// `compiled_xaml` (the compiled markup of the styles document) and `compiled_markup` (the loader
// table of the crate and `register()`), which the build script generates.
ferroui_markup_xaml::include_compiled_xaml!();

#[cfg(any(test, feature = "document-assets"))]
mod assets;
mod auto_scroll_during_drag_behavior;
mod canvas_drag_behavior;
mod grid_drag_behavior;
mod item_drag_behavior;
mod list_reorder_drag_behavior;
mod mouse_drag_element_behavior;
mod multi_mouse_drag_element_behavior;
mod register_types;
mod routed_handlers;
mod rust_paths;
mod selection_adorner;

pub use auto_scroll_during_drag_behavior::AutoScrollDuringDragBehavior;
pub use canvas_drag_behavior::CanvasDragBehavior;
pub use grid_drag_behavior::GridDragBehavior;
pub use item_drag_behavior::ItemDragBehavior;
pub use list_reorder_drag_behavior::ListReorderDragBehavior;
pub use mouse_drag_element_behavior::MouseDragElementBehavior;
pub use multi_mouse_drag_element_behavior::MultiMouseDragElementBehavior;
pub use register_types::{register_types, ASSEMBLY};
pub use selection_adorner::SelectionAdorner;

#[cfg(test)]
mod tests;
