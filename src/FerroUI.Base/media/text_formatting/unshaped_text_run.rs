use std::rc::Rc;

use crate::media::text_formatting::text_run::{text_run_any, TextRun};
use crate::media::text_formatting::unicode::Script;
use crate::media::text_formatting::TextRunProperties;
use crate::utilities::ReadOnlyMemory;

/// A group of characters that can be shaped.
pub struct UnshapedTextRun {
    text: ReadOnlyMemory<u16>,
    properties: Rc<dyn TextRunProperties>,
    bidi_level: i8,
    script: Script,
}

impl UnshapedTextRun {
    pub fn new(text: ReadOnlyMemory<u16>, properties: Rc<dyn TextRunProperties>, bidi_level: i8) -> Self {
        Self::with_script(text, properties, bidi_level, Script::Unknown)
    }

    pub(crate) fn with_script(
        text: ReadOnlyMemory<u16>,
        properties: Rc<dyn TextRunProperties>,
        bidi_level: i8,
        script: Script,
    ) -> Self {
        Self { text, properties, bidi_level, script }
    }

    /// The script of the text of the run: common when it holds no script of its own, unknown when
    /// it was not itemized.
    pub(crate) fn script(&self) -> Script {
        self.script
    }

    /// The bidi level of the run.
    pub fn bidi_level(&self) -> i8 {
        self.bidi_level
    }

    /// The run's properties (never absent for an unshaped run).
    pub fn run_properties(&self) -> &Rc<dyn TextRunProperties> {
        &self.properties
    }
}

impl TextRun for UnshapedTextRun {
    fn length(&self) -> i32 {
        self.text.len() as i32
    }

    fn text(&self) -> ReadOnlyMemory<u16> {
        self.text.clone()
    }

    fn text_span(&self) -> &[u16] {
        self.text.span()
    }

    fn properties(&self) -> Option<&Rc<dyn TextRunProperties>> {
        Some(&self.properties)
    }

    text_run_any!();
}
