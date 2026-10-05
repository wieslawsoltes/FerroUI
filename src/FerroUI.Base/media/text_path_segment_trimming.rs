use std::fmt;
use std::rc::Rc;

use crate::media::text_formatting::TextCollapsingProperties;
use crate::media::{TextCollapsingCreateInfo, TextPathSegmentEllipsis, TextTrimming};

/// Provides path segment-based text trimming with ellipsis for file paths
/// and similar text content.
pub struct TextPathSegmentTrimming {
    ellipsis: String,
}

impl TextPathSegmentTrimming {
    /// Creates the trimming with the ellipsis string to use for trimming.
    pub fn new(ellipsis: &str) -> Self {
        Self { ellipsis: ellipsis.to_owned() }
    }
}

impl TextTrimming for TextPathSegmentTrimming {
    fn create_collapsing_properties(&self, create_info: &TextCollapsingCreateInfo) -> Rc<dyn TextCollapsingProperties> {
        Rc::new(TextPathSegmentEllipsis::new(
            &self.ellipsis,
            create_info.width,
            create_info.text_run_properties.clone(),
            create_info.flow_direction,
        ))
    }
}

impl fmt::Display for TextPathSegmentTrimming {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PathSegmentEllipsis")
    }
}
