use std::fmt;
use std::rc::Rc;

use crate::media::text_formatting::TextCollapsingProperties;
use crate::media::{TextCollapsingCreateInfo, TextTrimming};

/// No trimming.
pub(crate) struct TextNoneTrimming;

impl TextTrimming for TextNoneTrimming {
    /// Panics: text that is not trimmed has no collapsing properties.
    fn create_collapsing_properties(&self, _create_info: &TextCollapsingCreateInfo) -> Rc<dyn TextCollapsingProperties> {
        panic!("Specified method is not supported.");
    }
}

impl fmt::Display for TextNoneTrimming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("None")
    }
}
