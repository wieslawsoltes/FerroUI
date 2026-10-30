//! Portable editor tools that produce source transactions. A host supplies measured geometry;
//! layout, property and selection commands never mutate preview objects directly.
mod components;
mod geometry;
mod grid_tracks;
mod layout;
mod layout_attributes;
mod layout_helpers;
mod properties;
mod selection;

pub use components::{
    ComponentAction, ComponentCatalog, ComponentEntry, ComponentKind, ComponentReference,
    ComponentResourceContext, ComponentResourceResolution, component_fragment,
    component_property_attribute, component_property_elements, component_property_object,
    component_resource_expression, component_resource_references, component_resource_resolution,
    plan_component_action, plan_component_action_in_context, plan_property_object,
    resolve_component_resource, resolve_component_resource_in_context, visible_component_resources,
    visible_component_resources_in_context,
};
pub use geometry::{
    Affine, Alignment, Axis, Point, Rect, SnapGuide, SnapIndex, SnapResult, SnapSettings, align,
    distribute,
};
pub use grid_tracks::{
    GridTrackChange, GridTrackPlacement, grid_track_values, plan_grid_track_change,
    plan_grid_track_change_with_placements,
};
pub use layout::{
    DockSide, GridLength, GridLines, LayoutAction, LayoutAdapter, LayoutCapabilities,
    LayoutContext, LayoutRegistry, RelativeRelation, ResolvedLayout,
};
pub use layout_attributes::plan_layout_attributes;
pub use layout_helpers::{
    LayoutGuide, LayoutGuideGeometry, LayoutGuideRole, LayoutHandle, LayoutHandleAction,
    LayoutHelperRequest, LayoutHelperScene, LayoutPlacement,
};
pub use properties::{
    EditorRegistry, ExpressionPolicy, PropertyDescriptor, PropertyKind, PropertyRegistry,
    PropertyState, PropertyValueShape, ValueEditor,
};
pub use selection::{MoveGesture, Selection, SelectionMode, outermost};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolError(pub String);
impl std::fmt::Display for ToolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for ToolError {}
pub type Result<T> = std::result::Result<T, ToolError>;
pub(crate) fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(ToolError(message.into()))
    }
}
pub(crate) fn error(error: impl std::fmt::Display) -> ToolError {
    ToolError(error.to_string())
}

#[cfg(test)]
mod tests;
