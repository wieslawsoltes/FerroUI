//! Portable editor tools. Hosts supply measured geometry and apply checked source or view plans.
mod canvas_guides;
mod canvas_guide_edit;
mod components;
pub mod component_styles;
pub mod component_palette;
mod geometry;
mod grid_tracks;
pub mod scenario;
mod layout;
mod layout_attributes;
mod table_columns;
mod layout_helpers;
mod layout_wrap;
mod properties;
mod selection;
pub mod token_handoff;

pub use canvas_guide_edit::{CanvasGuideDraft, CanvasGuideEdit, CanvasGuideUpdate};
pub use canvas_guides::{CANVAS_MIN_ZOOM, CANVAS_RULER, CanvasGuide, CanvasGuideAxis, CanvasGuideSettings, CanvasRuler, CanvasSnapPreferences, ruler_ticks};
pub use components::{
    ComponentAction, ComponentCatalog, ComponentEntry, ComponentKind, ComponentReference,
    ComponentResourceContext, ComponentResourceLocation, ComponentResourceResolution,
    ComponentResourceSources, IncludedResourceDocument, ItemTemplateChange, ItemTemplateDeclaration,
    ItemTemplateMember, ItemTemplateResource, ItemTemplateSlot, ItemTemplateValue, ResourceCopySources, item_template_resources,
    keyed_item_template,
    component_fragment,
    component_property_attribute, component_property_elements, component_property_extractable,
    component_property_object, component_property_resource_owners,
    component_resource_expression, component_resource_location, component_resource_references,
    component_resource_resolution,
    plan_component_action, plan_component_action_in_context, plan_property_object,
    resolve_component_resource, resolve_component_resource_in_context, visible_component_resources,
    visible_component_resources_in_context,
};
pub use components::extraction::{DictionaryExtraction, DictionarySite, dictionary_extraction_site, plan_dictionary_extraction};
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
pub use layout_attributes::{authored_layout_property_span, plan_layout_attributes};
pub use table_columns::{AuthoredColumn, ColumnChange, ColumnChoice, ColumnField, ColumnFieldKind, ColumnKind, TableColumns, TableKind, nearest_table, plan_column_change, table_columns};
pub use layout_wrap::{SelectionWrap, WrapKind, plan_selection_wrap};
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
