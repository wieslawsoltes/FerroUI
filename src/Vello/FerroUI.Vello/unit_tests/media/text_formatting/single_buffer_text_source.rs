//! Port of upstream's `Media/TextFormatting/SingleBufferTextSource.cs` of the
//! Skia unit tests.

use ferroui_base::media::text_formatting::{
    GenericTextRunProperties, ITextSource, TextCharacters, TextEndOfParagraph, TextRun,
};
use ferroui_base::utilities::ReadOnlyMemory;
use std::rc::Rc;

pub(crate) struct SingleBufferTextSource {
    text: ReadOnlyMemory<u16>,
    default_generic_properties_run_properties: Rc<GenericTextRunProperties>,
    add_end_of_paragraph: bool,
}

impl SingleBufferTextSource {
    pub fn new(
        text: &str,
        default_properties: Rc<GenericTextRunProperties>,
        add_end_of_paragraph: bool,
    ) -> Self {
        Self {
            text: ReadOnlyMemory::from_str(text),
            default_generic_properties_run_properties: default_properties,
            add_end_of_paragraph,
        }
    }
}

impl ITextSource for SingleBufferTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        if text_source_index as usize >= self.text.len() {
            return if self.add_end_of_paragraph { Some(Rc::new(TextEndOfParagraph::new())) } else { None };
        }

        let run_text = self.text.slice_from(text_source_index as usize);

        if run_text.is_empty() {
            return if self.add_end_of_paragraph { Some(Rc::new(TextEndOfParagraph::new())) } else { None };
        }

        Some(Rc::new(TextCharacters::new(run_text, self.default_generic_properties_run_properties.clone())))
    }
}
