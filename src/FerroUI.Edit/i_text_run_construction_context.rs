use crate::visual_line_error::VisualLineError;
use crate::TextArea;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::text_formatting::TextRunProperties;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{Ref, WeakRef};
use ferroui_edit_core::document::{DocumentLine, TextSnapshot};
use std::cell::RefCell;
use std::rc::Rc;

/// Stable document and framework inputs for element generation and formatting.
/// All document offsets and lengths are measured in UTF-16 code units.
pub trait ITextRunConstructionContext {
    fn document(&self) -> &TextSnapshot;
    fn first_document_line(&self) -> DocumentLine;
    fn last_document_line(&self) -> DocumentLine;
    fn global_text_run_properties(&self) -> &Rc<dyn TextRunProperties>;
    fn current_display_column(&self) -> usize;
    fn wide_space_width(&self) -> f64;
    fn text_view(&self) -> Option<Ref<TextArea>>;

    fn get_text(
        &self,
        offset: usize,
        length: usize,
    ) -> Result<ReadOnlyMemory<u16>, VisualLineError> {
        Ok(ReadOnlyMemory::from_vec(
            self.document().get_text_utf16(offset, length)?,
        ))
    }
}

/// Inputs that require a live view are optional for standalone line formatting.
#[derive(Clone, Default)]
pub struct VisualLineBuildOptions {
    pub text_view: Option<WeakRef<TextArea>>,
    pub wide_space_width: f64,
    pub end_of_line: Option<EndOfLineMarkerOptions>,
}

/// Glyphs are native formatted runs outside the document-backed elements.
/// Enabling the marker adds one formatter position. The empty end-of-file glyph
/// does not add an exposed visual column.
#[derive(Clone)]
pub struct EndOfLineMarkerOptions {
    pub crlf_glyph: String,
    pub cr_glyph: String,
    pub lf_glyph: String,
    pub foreground: Option<Rc<dyn IBrush>>,
}
impl Default for EndOfLineMarkerOptions {
    fn default() -> Self {
        Self {
            crlf_glyph: "¶".into(),
            cr_glyph: "\\r".into(),
            lf_glyph: "\\n".into(),
            foreground: Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                128, 128, 128,
            )))),
        }
    }
}

#[derive(Clone)]
pub(crate) struct TextRunConstructionContext {
    pub document: TextSnapshot,
    pub first_line: DocumentLine,
    pub last_line: DocumentLine,
    pub properties: Rc<dyn TextRunProperties>,
    pub display_column: usize,
    pub options: VisualLineBuildOptions,
    pub cached_text: RefCell<Option<(usize, ReadOnlyMemory<u16>)>>,
}

impl ITextRunConstructionContext for TextRunConstructionContext {
    fn document(&self) -> &TextSnapshot {
        &self.document
    }
    fn first_document_line(&self) -> DocumentLine {
        self.first_line
    }
    fn last_document_line(&self) -> DocumentLine {
        self.last_line
    }
    fn global_text_run_properties(&self) -> &Rc<dyn TextRunProperties> {
        &self.properties
    }
    fn current_display_column(&self) -> usize {
        self.display_column
    }
    fn wide_space_width(&self) -> f64 {
        self.options.wide_space_width
    }
    fn text_view(&self) -> Option<Ref<TextArea>> {
        self.options.text_view.as_ref().and_then(WeakRef::upgrade)
    }
    fn get_text(
        &self,
        offset: usize,
        length: usize,
    ) -> Result<ReadOnlyMemory<u16>, VisualLineError> {
        if let Some((start, text)) = &*self.cached_text.borrow() {
            if offset >= *start {
                let relative = offset - *start;
                if relative
                    .checked_add(length)
                    .is_some_and(|end| end <= text.len())
                {
                    return Ok(text.slice(relative, length));
                }
            }
        }
        let text = ReadOnlyMemory::from_vec(self.document.get_text_utf16(offset, length)?);
        *self.cached_text.borrow_mut() = Some((offset, text.clone()));
        Ok(text)
    }
}
