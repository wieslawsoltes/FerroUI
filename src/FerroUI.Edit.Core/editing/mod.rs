mod caret_navigation;
mod editor_session;
mod indentation;
mod read_only_section_provider;
mod rectangle_selection;
mod selection;
mod text_transformations;

pub use caret_navigation::{next_caret_position, CaretMovement};
pub use editor_session::{EditError, EditorOptions, EditorSession};
pub use indentation::{
    CSharpIndentationStrategy, DefaultIndentationStrategy, IDocumentAccessor, IIndentationStrategy,
    IndentationError, IndentationStrategy, TextDocumentAccessor,
};
pub use read_only_section_provider::{
    IReadOnlySectionProvider, NoReadOnlySections, TextSegmentReadOnlySectionProvider,
};
pub use rectangle_selection::RectangleSelection;
pub use selection::{Selection, SelectionSegment};
pub use text_transformations::{plan_text_transformation, TextTransformation};
