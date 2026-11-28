use super::ProviderError;
use crate::automation::peers::AutomationPeer;
use ferroui_base::media::Color;
use ferroui_base::{Point, Rect, Ref};
use std::any::Any;
use std::fmt;
use std::ops::Range;
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

/// Placement of a text range in its scrolling viewport. A point is in the
/// visual root's coordinate space, like the rectangles returned by the range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TextScrollPlacement {
    TopLeft,
    BottomRight,
    Top,
    Bottom,
    Left,
    Right,
    Nearest,
    Point(Point),
}

/// Formatting attributes available through the portable text pattern. Font size
/// and paragraph measurements are in typographic points (72 points per inch).
/// Unknown native attributes retain their identifier and return `NotSupported`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextAttribute {
    FontName,
    FontSize,
    FontWeight,
    /// The native font slant, as `normal`, `italic`, or `oblique`.
    FontStyle,
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

pub type TextAttributeSet = Vec<(TextAttribute, TextAttributeValue)>;

/// Effective attributes and the half-open UTF-16 range over which they apply.
#[derive(Clone, Debug, PartialEq)]
pub struct TextAttributeRun {
    pub range: Range<usize>,
    pub attributes: TextAttributeSet,
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
    /// Returns the live half-open UTF-16 offsets when the provider supports them.
    fn text_offsets(&self) -> Result<Option<Range<usize>>, ProviderError> {
        Ok(None)
    }
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
    /// Replaces this selected range atomically and preserves the other selections.
    fn replace_in_selection(&self, _replacement: &dyn ITextRangeProvider) -> Result<bool, ProviderError> {
        Ok(false)
    }
    fn scroll_into_view(&self, align_to_top: bool) -> Result<(), ProviderError>;
    /// Scrolls without changing the caret or selection. Returns `false` if
    /// the provider cannot make the requested text visible.
    fn scroll_to(&self, _placement: TextScrollPlacement) -> Result<bool, ProviderError> {
        Ok(false)
    }
    fn get_children(&self) -> Result<Vec<Ref<AutomationPeer>>, ProviderError>;
}
