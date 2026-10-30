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
    /// The line-metrics mode of the paragraph that made the break; a wrapped
    /// continuation is reused only by a paragraph with the same mode.
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
    pub fn new(text_end_of_line: Option<Rc<dyn TextRun>>, flow_direction: FlowDirection, is_split: bool) -> Self {
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
    /// paginated view (linked rich text containers) keeps the original break and
    /// gives a fork to each measure and arrange pass, so reflow keeps the
    /// paragraph's bidi state without consuming the preceding page's continuation.
    /// An addition for rich text (`docs/porting/winui-rich-text.md`).
    #[doc(hidden)]
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

    /// Forks a continuation while replacing its embedded drawable objects.
    ///
    /// A paginated host can measure an object with the next page's constraints
    /// without reshaping its retained text. The callback is invoked without a
    /// cache borrow and only for non-shaped drawable runs. `None` keeps the
    /// original object; a replacement must also be a non-shaped drawable with the
    /// same source length, otherwise it is ignored.
    #[doc(hidden)]
    pub fn fork_with_object_runs(&self, mut replace: impl FnMut(&Rc<dyn TextRun>) -> Option<Rc<dyn TextRun>>) -> Rc<Self> {
        let fork = self.fork();
        // Keep the fork's shaping references installed while invoking user code.
        let remaining = fork.remaining_runs.borrow().clone();
        if let Some(remaining) = remaining {
            let mapped = remaining
                .iter()
                .map(|run| {
                    if run.is::<ShapedTextRun>() || run.as_drawable().is_none() {
                        return run.clone();
                    }
                    replace(run)
                        .filter(|candidate| {
                            !candidate.is::<ShapedTextRun>()
                                && candidate.as_drawable().is_some()
                                && candidate.length() == run.length()
                        })
                        .unwrap_or_else(|| run.clone())
                })
                .collect();
            // Shaped handles are identical: their ownership moves between the vectors.
            *fork.remaining_runs.borrow_mut() = Some(mapped);
        }
        fork
    }

    /// Releases an unused continuation after a fork has been accepted.
    #[doc(hidden)]
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

/// Deviation (DEVIATIONS.md, Text formatting): upstream's `WrappingTextLineBreak` never
/// disposes the runs left over by a wrapped line that no next line consumed; dropping
/// the break here releases their shaping references.
impl Drop for TextLineBreak {
    fn drop(&mut self) {
        if let Some(runs) = self.remaining_runs.get_mut().take() {
            for run in runs {
                if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                    shaped.dispose();
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "text_line_break_object_tests.rs"]
mod object_tests;
