use super::ProviderError;
use crate::automation::peers::AutomationPeer;
use ferroui_base::media::Color;
use ferroui_base::{Rect, Ref};
use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// Text-pattern movement units, in the order used when a provider promotes a
/// unit that its content cannot represent to the next supported larger unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum TextUnit {
    Character = 0,
    Format = 1,
    Word = 2,
    Line = 3,
    Paragraph = 4,
    Page = 5,
    Document = 6,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum TextPatternRangeEndpoint {
    Start = 0,
    End = 1,
}

/// Formatting attributes available through the portable text pattern. Font size
/// and paragraph measurements are in typographic points (72 points per inch).
/// Unknown native attributes retain their identifier and return `NotSupported`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextAttribute {
    FontName,
    FontSize,
    FontWeight,
    IsItalic,
    IsReadOnly,
    IsHidden,
    ForegroundColor,
    BackgroundColor,
    UnderlineColor,
    UnderlineStyle,
    OverlineColor,
    OverlineStyle,
    StrikethroughColor,
    StrikethroughStyle,
    IsSubscript,
    IsSuperscript,
    HorizontalTextAlignment,
    IndentationFirstLine,
    IndentationLeading,
    IndentationTrailing,
    MarginTop,
    MarginBottom,
    MarginLeading,
    MarginTrailing,
    TextFlowDirections,
    Unknown(i32),
}

/// `Mixed` and `NotSupported` are values, separate from provider errors. A native
/// bridge maps them to its reserved automation attribute values.
#[derive(Clone, Debug, PartialEq)]
pub enum TextAttributeValue {
    Bool(bool),
    Integer(i32),
    Number(f64),
    String(String),
    Color(Color),
    Mixed,
    NotSupported,
}

/// Failures that a platform bridge can translate without parsing error strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextProviderError {
    ElementNotAvailable,
    IncompatibleRange,
    InvalidArgument,
    UnsupportedAttribute,
    SelectionNotSupported,
    MultipleSelectionNotSupported,
    GeometryUnavailable,
}

impl fmt::Display for TextProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ElementNotAvailable => {
                "The text owner or the range's document is no longer available."
            }
            Self::IncompatibleRange => {
                "Text ranges must belong to the same text provider and document."
            }
            Self::InvalidArgument => "The text-pattern argument is invalid.",
            Self::UnsupportedAttribute => "The text provider does not support this attribute.",
            Self::SelectionNotSupported => "Text selection is disabled for this control.",
            Self::MultipleSelectionNotSupported => {
                "The control supports one contiguous text selection."
            }
            Self::GeometryUnavailable => "The text control has no usable layout in a visual root.",
        })
    }
}
impl std::error::Error for TextProviderError {}

/// A live range of text exposed to accessibility. Range operations change the
/// range object; only selection methods change the control's selection. Ranges
/// follow edits and become unavailable when their host displays another document.
pub trait ITextRangeProvider {
    fn as_any(&self) -> &dyn Any;
    /// The text-pattern owner, also used by platform bridges for screen transforms.
    fn peer(&self) -> Result<Ref<AutomationPeer>, ProviderError>;
    fn clone_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError>;
    fn compare(&self, range: &dyn ITextRangeProvider) -> Result<bool, ProviderError>;
    fn compare_endpoints(
        &self,
        endpoint: TextPatternRangeEndpoint,
        range: &dyn ITextRangeProvider,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<i32, ProviderError>;
    fn expand_to_enclosing_unit(&self, unit: TextUnit) -> Result<(), ProviderError>;
    fn find_attribute(
        &self,
        attribute: TextAttribute,
        value: &TextAttributeValue,
        backwards: bool,
    ) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError>;
    fn find_text(
        &self,
        text: &str,
        backwards: bool,
        ignore_case: bool,
    ) -> Result<Option<Rc<dyn ITextRangeProvider>>, ProviderError>;
    fn get_attribute_value(
        &self,
        attribute: TextAttribute,
    ) -> Result<TextAttributeValue, ProviderError>;
    fn get_bounding_rectangles(&self) -> Result<Vec<Rect>, ProviderError>;
    fn get_enclosing_element(&self) -> Result<Ref<AutomationPeer>, ProviderError>;
    /// `None` retrieves all text. A finite limit never splits a surrogate pair.
    fn get_text(&self, max_length: Option<usize>) -> Result<String, ProviderError>;
    fn move_by(&self, unit: TextUnit, count: i32) -> Result<i32, ProviderError>;
    fn move_endpoint_by_unit(
        &self,
        endpoint: TextPatternRangeEndpoint,
        unit: TextUnit,
        count: i32,
    ) -> Result<i32, ProviderError>;
    fn move_endpoint_by_range(
        &self,
        endpoint: TextPatternRangeEndpoint,
        range: &dyn ITextRangeProvider,
        target_endpoint: TextPatternRangeEndpoint,
    ) -> Result<(), ProviderError>;
    fn select(&self) -> Result<(), ProviderError>;
    fn add_to_selection(&self) -> Result<(), ProviderError>;
    fn remove_from_selection(&self) -> Result<(), ProviderError>;
    fn scroll_into_view(&self, align_to_top: bool) -> Result<(), ProviderError>;
    fn get_children(&self) -> Result<Vec<Ref<AutomationPeer>>, ProviderError>;
}
