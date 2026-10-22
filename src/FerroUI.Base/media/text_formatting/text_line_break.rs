use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::media::text_formatting::{ShapedTextRun, TextLineMetricsMode, TextRun};
use crate::media::FlowDirection;

/// Represents the state of a line break: how the line ended and what the
/// next line has to continue with.
pub struct TextLineBreak {
    text_end_of_line: Option<Rc<dyn TextRun>>,
    flow_direction: FlowDirection,
    is_split: bool,
    line_metrics_mode: TextLineMetricsMode,
    /// The runs left over by a wrapped line (upstream `WrappingTextLineBreak`).
    pub(crate) remaining_runs: RefCell<Option<Vec<Rc<dyn TextRun>>>>,
    /// Whether this is a wrapping line break (stays true after the runs were acquired).
    pub(crate) is_wrapping: Cell<bool>,
}

impl TextLineBreak {
    /// Creates a line break.
    ///
    /// `text_end_of_line` is the end of line run (a run whose
    /// `as_text_end_of_line` is `Some`) when the line ended with one.
    pub fn new(
        text_end_of_line: Option<Rc<dyn TextRun>>,
        flow_direction: FlowDirection,
        is_split: bool,
    ) -> Self {
        Self {
            text_end_of_line,
            flow_direction,
            is_split,
            line_metrics_mode: TextLineMetricsMode::GlyphTypeface,
            remaining_runs: RefCell::new(None),
            is_wrapping: Cell::new(false),
        }
    }

    pub(crate) fn with_line_metrics_mode(mut self, mode: TextLineMetricsMode) -> Self {
        self.line_metrics_mode = mode;
        self
    }

    pub(crate) fn line_metrics_mode(&self) -> TextLineMetricsMode {
        self.line_metrics_mode
    }

    /// Get the end of line run.
    pub fn text_end_of_line(&self) -> Option<&Rc<dyn TextRun>> {
        self.text_end_of_line.as_ref()
    }

    /// Get the flow direction for remaining characters.
    pub fn flow_direction(&self) -> FlowDirection {
        self.flow_direction
    }

    /// Gets whether there were remaining runs after this line break, that
    /// were split up by the `TextFormatter` during the formatting process.
    pub fn is_split(&self) -> bool {
        self.is_split
    }

    /// Makes an independently consumable continuation of a formatted line.
    ///
    /// Formatting consumes the remaining shaped runs of a wrapping break. A
    /// paginated view keeps the original break and gives a fork to each
    /// measure/arrange pass, so reflow preserves the paragraph's bidi state
    /// without consuming the preceding page's continuation.
    pub fn fork(&self) -> Rc<Self> {
        let remaining = self.remaining_runs.borrow().as_ref().map(|runs| {
            runs.iter()
                .map(|run| {
                    if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                        shaped.add_reference();
                    }
                    run.clone()
                })
                .collect()
        });
        Rc::new(Self {
            text_end_of_line: self.text_end_of_line.clone(),
            flow_direction: self.flow_direction,
            is_split: self.is_split,
            line_metrics_mode: self.line_metrics_mode,
            remaining_runs: RefCell::new(remaining),
            is_wrapping: Cell::new(self.is_wrapping.get()),
        })
    }

    /// Releases an unused continuation after a fork has been accepted. A
    /// formatted page retains only its final continuation, rather than keeping
    /// every wrapped paragraph remainder alive on every preceding line.
    pub fn release_remaining_runs(&self) {
        let remaining = self.remaining_runs.borrow_mut().take();
        if let Some(runs) = remaining {
            for run in runs {
                if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                    shaped.dispose();
                }
            }
        }
    }
}

impl Drop for TextLineBreak {
    fn drop(&mut self) {
        // A break at the end of a clipped page may never be consumed. Its
        // remaining runs own one shaping reference just like a visible line.
        if let Some(runs) = self.remaining_runs.get_mut().take() {
            for run in runs {
                if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                    shaped.dispose();
                }
            }
        }
    }
}
