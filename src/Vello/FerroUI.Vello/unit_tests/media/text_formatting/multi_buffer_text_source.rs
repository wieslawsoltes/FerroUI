//! Port of upstream's `Media/TextFormatting/MultiBufferTextSource.cs` of the
//! Skia unit tests.

use ferroui_base::media::text_formatting::{GenericTextRunProperties, ITextSource, TextCharacters, TextRun};
use ferroui_base::utilities::ReadOnlyMemory;
use std::rc::Rc;

pub(crate) struct MultiBufferTextSource {
    run_texts: [&'static str; 5],
    default_style: Rc<GenericTextRunProperties>,
}

impl MultiBufferTextSource {
    pub fn new(default_style: Rc<GenericTextRunProperties>) -> Self {
        Self {
            default_style,
            run_texts: ["A123456789", "B123456789", "C123456789", "D123456789", "E123456789"],
        }
    }
}

impl ITextSource for MultiBufferTextSource {
    fn get_text_run(&self, text_source_index: i32) -> Option<Rc<dyn TextRun>> {
        if text_source_index >= 50 {
            return None;
        }

        let index = (text_source_index / 10) as usize;

        let run_text = self.run_texts[index];

        Some(Rc::new(TextCharacters::new(ReadOnlyMemory::from_str(run_text), self.default_style.clone())))
    }
}
