use std::rc::Rc;

use crate::media::text_formatting::TextRun;
use crate::Rect;

/// The bounding rectangle of text run.
#[derive(Clone)]
pub struct TextRunBounds {
    text_source_character_index: i32,
    length: i32,
    rectangle: Rect,
    text_run: Rc<dyn TextRun>,
}

impl TextRunBounds {
    pub(crate) fn new(bounds: Rect, first_character_index: i32, length: i32, text_run: Rc<dyn TextRun>) -> Self {
        Self { rectangle: bounds, text_source_character_index: first_character_index, length, text_run }
    }

    /// Character index of first character of text run.
    pub fn text_source_character_index(&self) -> i32 {
        self.text_source_character_index
    }

    /// Character length of text run.
    pub fn length(&self) -> i32 {
        self.length
    }

    /// Text run bounding rectangle.
    pub fn rectangle(&self) -> Rect {
        self.rectangle
    }

    /// Text run.
    pub fn text_run(&self) -> &Rc<dyn TextRun> {
        &self.text_run
    }
}
