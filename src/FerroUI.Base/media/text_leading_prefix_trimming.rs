use std::fmt;
use std::rc::Rc;

use crate::media::text_formatting::{TextCollapsingProperties, TextLeadingPrefixCharacterEllipsis};
use crate::media::{TextCollapsingCreateInfo, TextTrimming};

/// Trims text after a prefix of a fixed length; the suffix grows from the end.
pub struct TextLeadingPrefixTrimming {
    ellipsis: String,
    prefix_length: i32,
}

impl TextLeadingPrefixTrimming {
    pub fn new(ellipsis: &str, prefix_length: i32) -> Self {
        Self { ellipsis: ellipsis.to_owned(), prefix_length }
    }
}

impl TextTrimming for TextLeadingPrefixTrimming {
    fn create_collapsing_properties(&self, create_info: &TextCollapsingCreateInfo) -> Rc<dyn TextCollapsingProperties> {
        Rc::new(TextLeadingPrefixCharacterEllipsis::new(
            &self.ellipsis,
            self.prefix_length,
            create_info.width,
            create_info.text_run_properties.clone(),
            create_info.flow_direction,
        ))
    }
}

impl fmt::Display for TextLeadingPrefixTrimming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PrefixCharacterEllipsis")
    }
}
