use super::{ITextRangeProvider, ProviderError, TextAttributeRun, TextAttributeSet};
use crate::automation::peers::AutomationPeer;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Point, Ref};
use std::ops::Range;
use std::rc::Rc;

/// The number of independent text selections supported by a text provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum SupportedTextSelection {
    None = 0,
    Single = 1,
    Multiple = 2,
}

/// A committed change to the text exposed by a provider. `offset` is the
/// UTF-16 position shared by the removed and inserted fragments. Both offsets
/// identify the same boundary in the provider's Unicode text projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEditDelta {
    pub offset: usize,
    pub character_offset: usize,
    pub removed: String,
    pub inserted: String,
}

/// A portable text-pattern contract. Text offsets and maximum lengths use UTF-16
/// code units. Points and rectangles use the visual root's coordinate space, as
/// [`AutomationPeer::get_bounding_rectangle`] does; platform bridges convert to
/// native screen coordinates at their boundary.
pub trait ITextProvider {
    fn peer(&self) -> Ref<AutomationPeer>;
    /// Whether vertical line navigation is part of this text input's behavior.
    fn is_multiline(&self) -> bool { false }
    fn document_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError>;
    fn get_selection(&self) -> Result<Vec<Rc<dyn ITextRangeProvider>>, ProviderError>;
    fn get_visible_ranges(&self) -> Result<Vec<Rc<dyn ITextRangeProvider>>, ProviderError>;
    fn range_from_child(
        &self,
        child: &Ref<AutomationPeer>,
    ) -> Result<Rc<dyn ITextRangeProvider>, ProviderError>;
    fn range_from_point(&self, point: Point) -> Result<Rc<dyn ITextRangeProvider>, ProviderError>;
    /// Finds an embedded object's text span when a client asks for object hit testing.
    /// Points use the same visual-root coordinates as `range_from_point`.
    fn embedded_range_from_point(
        &self,
        _point: Point,
    ) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError> {
        Ok(None)
    }
    /// Creates a range for a platform API that addresses the story by UTF-16
    /// offset. Providers without absolute offset support return `None`.
    fn range_from_offsets(
        &self,
        _offsets: Range<usize>,
    ) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError> {
        Ok(None)
    }
    /// Returns the zero-based document line at a UTF-16 offset. Delimiters
    /// belong to the line they end, and the final offset is a valid caret position.
    /// Soft wraps do not affect these indices. Support both logical line queries together.
    fn logical_line_index(&self, _offset: usize) -> Result<Option<usize>, ProviderError> {
        Ok(None)
    }
    /// Returns the UTF-16 span of a zero-based document line, including its delimiter.
    fn logical_line_range(&self, _index: usize) -> Result<Option<Range<usize>>, ProviderError> {
        Ok(None)
    }
    /// Converts a Unicode scalar offset to a UTF-16 code unit offset, including
    /// the end of the document. Providers can opt into indexed conversions.
    fn utf16_offset_from_character(&self, _offset: usize) -> Result<Option<usize>, ProviderError> {
        Ok(None)
    }
    /// Converts a UTF-16 code unit offset to the index of its Unicode scalar.
    fn character_offset_from_utf16(&self, _offset: usize) -> Result<Option<usize>, ProviderError> {
        Ok(None)
    }
    /// Opts into indexed platform reads and returns the scalar count of the
    /// exposed text. The provider's offset conversions and range reads must
    /// agree with this projection, including any masking applied to the value.
    fn text_character_count(&self) -> Result<Option<usize>, ProviderError> {
        Ok(None)
    }
    /// Applies a user edit to a UTF-16 range. Read-only and protected ranges
    /// return an error. Providers without range editing return `false`.
    fn replace_text(&self, _offsets: Range<usize>, _text: &str) -> Result<bool, ProviderError> {
        Ok(false)
    }
    /// Copies a UTF-16 range to the native clipboard. When cutting, removes
    /// that exact range only after the clipboard accepts it.
    fn copy_text_to_clipboard(
        &self,
        _offsets: Range<usize>,
        _cut: bool,
    ) -> LocalBoxFuture<Result<bool, ProviderError>> {
        Box::pin(std::future::ready(Ok(false)))
    }
    /// Inserts native clipboard text at an explicit UTF-16 offset.
    fn paste_text_from_clipboard(&self, _offset: usize) -> LocalBoxFuture<Result<bool, ProviderError>> {
        Box::pin(std::future::ready(Ok(false)))
    }
    /// The attributes inherited by unformatted text in this provider.
    fn default_text_attributes(&self) -> Result<TextAttributeSet, ProviderError> {
        Ok(Vec::new())
    }
    /// Queries the entire effective attribute run at an absolute UTF-16 offset.
    fn text_attribute_run(&self, _offset: usize) -> Result<Option<TextAttributeRun>, ProviderError> {
        Ok(None)
    }
    /// Reads the effective presentation of a UTF-16 span in document order.
    /// Run bounds are absolute and clipped to the request. Providers without
    /// styled text support return `None`. An empty span has no runs.
    fn text_style_runs(&self, _span: Range<usize>) -> Result<Option<Vec<TextAttributeRun>>, ProviderError> {
        Ok(None)
    }
    fn supported_text_selection(&self) -> Result<SupportedTextSelection, ProviderError>;

    /// Observes content and formatting changes, including replacement of the document.
    fn text_changed(&self, handler: Rc<dyn Fn()>) -> Result<Rc<dyn IDisposable>, ProviderError>;
    /// Observes changes that can invalidate a client's cached formatting.
    fn text_attributes_changed(&self, _handler: Rc<dyn Fn()>) -> Result<Option<Rc<dyn IDisposable>>, ProviderError> {
        Ok(None)
    }
    /// Observes committed content deltas in document order. Providers without
    /// a content-delta feed return `None`.
    fn text_edited(
        &self,
        _handler: Rc<dyn Fn(&TextEditDelta)>,
    ) -> Result<Option<Rc<dyn IDisposable>>, ProviderError> {
        Ok(None)
    }
    /// Observes changes to either selection endpoint, including the insertion point.
    fn text_selection_changed(
        &self,
        handler: Rc<dyn Fn()>,
    ) -> Result<Rc<dyn IDisposable>, ProviderError>;
}

impl PartialEq for dyn ITextProvider {
    fn eq(&self, other: &Self) -> bool {
        self.peer() == other.peer()
    }
}
