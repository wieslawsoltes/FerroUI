use ferroui_edit_core::document::DocumentLine;

/// Raised before a viewport batch constructs its first visual line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisualLineConstructionStartEventArgs {
    pub first_line_in_view: DocumentLine,
}
impl VisualLineConstructionStartEventArgs {
    pub fn new(first_line_in_view: DocumentLine) -> Self {
        Self { first_line_in_view }
    }
}
