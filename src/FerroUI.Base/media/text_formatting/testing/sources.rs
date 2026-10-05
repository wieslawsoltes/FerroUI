use std::rc::Rc;

use crate::media::text_formatting::text_run::{text_run_any, TextRun};
use crate::media::text_formatting::{
    DrawableTextRun, ITextDrawingSink, ITextSource, TextCharacters, TextEndOfParagraph, TextRunProperties,
};
use crate::utilities::ReadOnlyMemory;
use crate::{Point, Size};

/// A text source over one string: every run is the rest of the text.
pub struct SingleBufferTextSource {
    text: ReadOnlyMemory<u16>,
    default_properties: Rc<dyn TextRunProperties>,
    add_end_of_paragraph: bool,
}

impl SingleBufferTextSource {
    pub fn new(text: &str, default_properties: Rc<dyn TextRunProperties>) -> Self {
        Self::with_end_of_paragraph(text, default_properties, false)
    }

    pub fn with_end_of_paragraph(
        text: &str,
        default_properties: Rc<dyn TextRunProperties>,
        add_end_of_paragraph: bool,
    ) -> Self {
        Self { text: ReadOnlyMemory::<u16>::from_str(text), default_properties, add_end_of_paragraph }
    }

    fn end(&self) -> Option<Rc<dyn TextRun>> {
        if self.add_end_of_paragraph {
            Some(Rc::new(TextEndOfParagraph::new()))
        } else {
            None
        }
    }
}

impl ITextSource for SingleBufferTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        if text_source_index as usize >= self.text.len() {
            return self.end();
        }

        let run_text = self.text.slice_from(text_source_index as usize);

        if run_text.is_empty() {
            return self.end();
        }

        Some(Rc::new(TextCharacters::new(run_text, self.default_properties.clone())))
    }
}

/// A text source of five runs of ten characters, each in its own buffer.
pub struct MultiBufferTextSource {
    run_texts: [ReadOnlyMemory<u16>; 5],
    default_style: Rc<dyn TextRunProperties>,
}

impl MultiBufferTextSource {
    pub fn new(default_style: Rc<dyn TextRunProperties>) -> Self {
        Self {
            run_texts: ["A123456789", "B123456789", "C123456789", "D123456789", "E123456789"]
                .map(ReadOnlyMemory::<u16>::from_str),
            default_style,
        }
    }
}

impl ITextSource for MultiBufferTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        if text_source_index >= 50 {
            return None;
        }

        let index = (text_source_index / 10) as usize;

        Some(Rc::new(TextCharacters::new(self.run_texts[index].clone(), self.default_style.clone())))
    }
}

/// A text source over a list of runs. An index inside a run of text
/// characters gives the rest of that run.
pub struct ListTextSource {
    runs: Vec<Rc<dyn TextRun>>,
}

impl ListTextSource {
    pub fn new(runs: Vec<Rc<dyn TextRun>>) -> Self {
        Self { runs }
    }
}

impl ITextSource for ListTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        let mut offset = 0;

        for run in &self.runs {
            if text_source_index >= offset && text_source_index - offset < run.length() {
                if run.length() == 1 || text_source_index == offset {
                    return Some(run.clone());
                }

                let characters = run.downcast_ref::<TextCharacters>().expect("only text characters can be sliced");

                return Some(Rc::new(TextCharacters::new(
                    characters.text().slice_from((text_source_index - offset) as usize),
                    characters.run_properties().clone(),
                )));
            }

            offset += run.length();
        }

        None
    }
}

/// A run without text, size or properties (a hidden run).
pub struct InvisibleRun {
    length: i32,
}

impl InvisibleRun {
    pub fn new(length: i32) -> Self {
        Self { length }
    }
}

impl TextRun for InvisibleRun {
    fn length(&self) -> i32 {
        self.length
    }

    text_run_any!();
}

/// A text source that returns only an end of paragraph.
pub struct EndOfLineTextSource;

impl ITextSource for EndOfLineTextSource {
    fn get_text_run(&self, _text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        Some(Rc::new(TextEndOfParagraph::new()))
    }
}

/// A drawable run of a fixed size without text (an embedded object).
pub struct CustomDrawableRun {
    size: Size,
    baseline: f64,
    properties: Option<Rc<dyn TextRunProperties>>,
    length: i32,
}

#[allow(dead_code)] // the full harness API is kept for the tests built on top
impl CustomDrawableRun {
    /// A run of 14 x 14 with a baseline of 14 (the shape upstream's tests use), one position long.
    pub fn new(properties: Rc<dyn TextRunProperties>) -> Self {
        Self::with_size(Size::new(14.0, 14.0), 14.0, properties)
    }

    pub fn with_size(size: Size, baseline: f64, properties: Rc<dyn TextRunProperties>) -> Self {
        Self { size, baseline, properties: Some(properties), length: 1 }
    }

    /// A run without properties, as upstream's test runs are.
    pub fn without_properties(size: Size, baseline: f64) -> Self {
        Self { size, baseline, properties: None, length: 1 }
    }

    pub fn with_length(mut self, length: i32) -> Self {
        self.length = length;
        self
    }
}

impl TextRun for CustomDrawableRun {
    fn length(&self) -> i32 {
        self.length
    }

    fn properties(&self) -> Option<&Rc<dyn TextRunProperties>> {
        self.properties.as_ref()
    }

    fn as_drawable(&self) -> Option<&dyn DrawableTextRun> {
        Some(self)
    }

    text_run_any!();
}

impl DrawableTextRun for CustomDrawableRun {
    fn size(&self) -> Size {
        self.size
    }

    fn baseline(&self) -> f64 {
        self.baseline
    }

    fn draw(&self, _drawing_context: &mut dyn ITextDrawingSink, _origin: Point) {}
}
