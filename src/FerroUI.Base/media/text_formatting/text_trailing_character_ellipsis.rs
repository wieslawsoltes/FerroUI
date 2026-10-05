use std::rc::Rc;

use crate::media::text_formatting::text_ellipsis_helper::TextEllipsisHelper;
use crate::media::text_formatting::{TextCharacters, TextCollapsingProperties, TextLine, TextRun, TextRunProperties};
use crate::media::FlowDirection;

/// A collapsing properties to collapse whole line toward the end
/// at character granularity.
pub struct TextTrailingCharacterEllipsis {
    width: f64,
    symbol: Rc<dyn TextRun>,
    flow_direction: FlowDirection,
}

impl TextTrailingCharacterEllipsis {
    /// Construct a text trailing character ellipsis collapsing properties.
    ///
    /// * `ellipsis` — text used as collapsing symbol.
    /// * `width` — width in which collapsing is constrained to.
    /// * `text_run_properties` — text run properties of ellipsis symbol.
    /// * `flow_direction` — the flow direction of the collapsed line.
    pub fn new(
        ellipsis: &str,
        width: f64,
        text_run_properties: Rc<dyn TextRunProperties>,
        flow_direction: FlowDirection,
    ) -> Self {
        Self { width, symbol: Rc::new(TextCharacters::from_str(ellipsis, text_run_properties)), flow_direction }
    }
}

impl TextCollapsingProperties for TextTrailingCharacterEllipsis {
    fn width(&self) -> f64 {
        self.width
    }

    fn symbol(&self) -> &Rc<dyn TextRun> {
        &self.symbol
    }

    fn flow_direction(&self) -> FlowDirection {
        self.flow_direction
    }

    fn collapse(&self, text_line: &dyn TextLine) -> Option<Vec<Rc<dyn TextRun>>> {
        TextEllipsisHelper::collapse(text_line, self, false)
    }
}
