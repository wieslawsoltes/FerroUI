//! Portable editor tools that produce source transactions. A host supplies measured geometry;
//! layout, property and selection commands never mutate preview objects directly.
mod geometry;
mod layout;
mod properties;
mod selection;

pub use geometry::{
    align, distribute, Affine, Alignment, Axis, Point, Rect, SnapGuide, SnapIndex, SnapResult,
    SnapSettings,
};
pub use layout::{
    DockSide, GridLength, GridLines, LayoutAction, LayoutAdapter, LayoutCapabilities,
    LayoutContext, LayoutRegistry, RelativeRelation, ResolvedLayout,
};
pub use properties::{
    EditorRegistry, ExpressionPolicy, PropertyDescriptor, PropertyKind, PropertyRegistry,
    PropertyState, ValueEditor,
};
pub use selection::{outermost, MoveGesture, Selection, SelectionMode};

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
