use std::fmt;
use std::rc::Rc;

use crate::media::text_formatting::{TextCollapsingProperties, TextTrailingCharacterEllipsis, TextTrailingWordEllipsis};
use crate::media::{TextCollapsingCreateInfo, TextTrimming};

/// Trims text at the end, at character or word granularity.
pub struct TextTrailingTrimming {
    ellipsis: String,
    is_word_based: bool,
}

impl TextTrailingTrimming {
    pub fn new(ellipsis: &str, is_word_based: bool) -> Self {
        Self { ellipsis: ellipsis.to_owned(), is_word_based }
    }
}

impl TextTrimming for TextTrailingTrimming {
    fn create_collapsing_properties(&self, create_info: &TextCollapsingCreateInfo) -> Rc<dyn TextCollapsingProperties> {
        if self.is_word_based {
            return Rc::new(TextTrailingWordEllipsis::new(
                &self.ellipsis,
                create_info.width,
                create_info.text_run_properties.clone(),
                create_info.flow_direction,
            ));
        }

        Rc::new(TextTrailingCharacterEllipsis::new(
            &self.ellipsis,
            create_info.width,
            create_info.text_run_properties.clone(),
            create_info.flow_direction,
        ))
    }
}

impl fmt::Display for TextTrailingTrimming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.is_word_based { "WordEllipsis" } else { "CharacterEllipsis" })
    }
}
