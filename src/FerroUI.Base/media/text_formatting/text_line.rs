use std::any::Any;
use std::cell::Ref;
use std::rc::Rc;

use crate::media::text_formatting::{
    ITextDrawingSink, JustificationProperties, TextBounds, TextCollapsingProperties, TextLineBreak, TextRun,
};
use crate::media::CharacterHit;
use crate::Point;

/// Represents a line of text that is used for text rendering.
///
/// All character indices and lengths are UTF-16 code units in text source
/// coordinates.
pub trait TextLine: 'static {
    /// Gets the text runs that are contained within a line (visual order).
    fn text_runs(&self) -> Ref<'_, [Rc<dyn TextRun>]>;

    /// Gets the first text source index in the line.
    fn first_text_source_index(&self) -> i32;

    /// Gets the total number of text source positions of the line.
    fn length(&self) -> i32;

    /// Gets the state of the line when broken by line breaking process.
    fn text_line_break(&self) -> Option<Rc<TextLineBreak>>;

    /// Gets the distance from the top to the baseline of the current line of text.
    fn baseline(&self) -> f64;

    /// Gets the distance from the top-most to bottom-most black pixel in a line.
    fn extent(&self) -> f64;

    /// Gets a value that indicates whether the line is collapsed.
    fn has_collapsed(&self) -> bool;

    /// Gets a value that indicates whether content of the line overflows the
    /// specified paragraph width.
    fn has_overflowed(&self) -> bool;

    /// Gets the height of a line of text.
    fn height(&self) -> f64;

    /// Gets the number of newline characters at the end of a line.
    fn new_line_length(&self) -> i32;

    /// Gets the distance that black pixels extend beyond the bottom alignment edge of a line.
    fn overhang_after(&self) -> f64;

    /// Gets the distance that black pixels extend prior to the left leading alignment edge of the line.
    fn overhang_leading(&self) -> f64;

    /// Gets the distance that black pixels extend following the right trailing alignment edge of the line.
    fn overhang_trailing(&self) -> f64;

    /// Gets the distance from the start of a paragraph to the starting point of a line.
    fn start(&self) -> f64;

    /// Gets the number of whitespace code points beyond the last non-blank
    /// character in a line.
    fn trailing_whitespace_length(&self) -> i32;

    /// Gets the width of a line of text, excluding trailing whitespace characters.
    fn width(&self) -> f64;

    /// Gets the width of a line of text, including trailing whitespace characters.
    fn width_including_trailing_whitespace(&self) -> f64;

    /// Draws the text line onto the drawing context at the given origin.
    fn draw(&self, drawing_context: &mut dyn ITextDrawingSink, line_origin: Point);

    /// Create a collapsed line based on collapsed text properties. Returns
    /// the line itself when nothing has to be collapsed.
    fn collapse(
        self: Rc<Self>,
        collapsing_properties_list: &[Option<Rc<dyn TextCollapsingProperties>>],
    ) -> Rc<dyn TextLine>;

    /// Justifies the line by applying given justification properties.
    fn justify(&self, justification_properties: &dyn JustificationProperties);

    /// Gets the character hit corresponding to the specified distance from
    /// the beginning of the line.
    fn get_character_hit_from_distance(&self, distance: f64) -> CharacterHit;

    /// Gets the distance from the beginning of the line to the specified character hit.
    fn get_distance_from_character_hit(&self, character_hit: CharacterHit) -> f64;

    /// Gets the next character hit for caret navigation.
    fn get_next_caret_character_hit(&self, character_hit: CharacterHit) -> CharacterHit;

    /// Gets the previous character hit for caret navigation.
    fn get_previous_caret_character_hit(&self, character_hit: CharacterHit) -> CharacterHit;

    /// Gets the previous character hit after backspacing.
    fn get_backspace_caret_character_hit(&self, character_hit: CharacterHit) -> CharacterHit;

    /// Get an array of bounding rectangles of a range of characters within a text line.
    fn get_text_bounds(&self, first_text_source_character_index: i32, text_length: i32) -> Vec<TextBounds>;

    /// Releases the resources of the line.
    fn dispose(&self);

    /// The line as [`Any`], for downcasting to the concrete line type.
    fn as_any(&self) -> &dyn Any;
}
