use crate::TextEditor;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::media::IImage;
use ferroui_base::BoxedValue;
use ferroui_edit_core::completion::CompletionItem;
use ferroui_edit_core::document::{DocumentEdit, DocumentError, SimpleSegment};
use ferroui_edit_core::editing::EditError;
use std::rc::Rc;

/// A native completion item with rich presentation and an insertion action.
///
/// The window validates its tracked replacement against the editor's read-only
/// provider before calling `complete`. Custom actions that edit other ranges
/// should use `EditorSession::apply_document_edits` for the same protection and
/// undo behavior. `source_event` is the key or pointer event requesting insertion.
pub trait ICompletionData {
    fn text(&self) -> &str;
    fn priority(&self) -> f64 {
        0.0
    }
    fn insertion_text(&self) -> &str {
        self.text()
    }
    fn content(&self) -> Option<BoxedValue> {
        Some(Rc::new(self.text().to_owned()))
    }
    fn description(&self) -> Option<BoxedValue> {
        None
    }
    fn image(&self) -> Option<Rc<dyn IImage>> {
        None
    }
    fn complete(
        &self,
        editor: &TextEditor,
        segment: SimpleSegment,
        _source_event: Option<&dyn IRoutedEventArgs>,
    ) -> Result<bool, DocumentError> {
        replace_completion(editor, segment, self.insertion_text())
    }
}
fn replace_completion(
    editor: &TextEditor,
    segment: SimpleSegment,
    text: &str,
) -> Result<bool, DocumentError> {
    editor
        .session()
        .apply_document_edits(&[DocumentEdit::new(segment.offset, segment.length, text)])
        .map_err(completion_edit_error)?;
    editor
        .set_caret_offset(segment.offset + text.encode_utf16().count())
        .map_err(completion_edit_error)?;
    Ok(true)
}
pub(crate) fn completion_edit_error(error: EditError) -> DocumentError {
    match error {
        EditError::Document(error) => error,
        _ => DocumentError::ReadOnly,
    }
}
impl ICompletionData for CompletionItem {
    fn text(&self) -> &str {
        &self.text
    }
    fn priority(&self) -> f64 {
        self.priority
    }
    fn insertion_text(&self) -> &str {
        self.insertion_text.as_deref().unwrap_or(&self.text)
    }
    fn description(&self) -> Option<BoxedValue> {
        (!self.description.is_empty()).then(|| Rc::new(self.description.clone()) as BoxedValue)
    }
}

/// Identity-preserving native property/data-template value for a completion item.
#[derive(Clone)]
pub struct CompletionDataRef(pub Rc<dyn ICompletionData>);
impl PartialEq for CompletionDataRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
pub type CompletionAction = dyn Fn(
    &TextEditor,
    SimpleSegment,
    Option<&dyn IRoutedEventArgs>,
) -> Result<bool, DocumentError>;

/// Convenient rich item; implement `ICompletionData` directly for dynamic data.
#[derive(Clone, Default)]
pub struct CompletionData {
    pub text: String,
    pub insertion_text: Option<String>,
    pub priority: f64,
    pub content: Option<BoxedValue>,
    pub description: Option<BoxedValue>,
    pub image: Option<Rc<dyn IImage>>,
    pub action: Option<Rc<CompletionAction>>,
}
impl CompletionData {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }
}
impl ICompletionData for CompletionData {
    fn text(&self) -> &str {
        &self.text
    }
    fn insertion_text(&self) -> &str {
        self.insertion_text.as_deref().unwrap_or(&self.text)
    }
    fn priority(&self) -> f64 {
        self.priority
    }
    fn content(&self) -> Option<BoxedValue> {
        self.content
            .clone()
            .or_else(|| Some(Rc::new(self.text.clone())))
    }
    fn description(&self) -> Option<BoxedValue> {
        self.description.clone()
    }
    fn image(&self) -> Option<Rc<dyn IImage>> {
        self.image.clone()
    }
    fn complete(
        &self,
        editor: &TextEditor,
        segment: SimpleSegment,
        source_event: Option<&dyn IRoutedEventArgs>,
    ) -> Result<bool, DocumentError> {
        if let Some(action) = &self.action {
            action(editor, segment, source_event)
        } else {
            replace_completion(editor, segment, self.insertion_text())
        }
    }
}
