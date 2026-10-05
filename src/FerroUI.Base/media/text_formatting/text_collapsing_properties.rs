use std::rc::Rc;

use crate::media::text_formatting::formatting_object_pool::FormattingObjectPool;
use crate::media::text_formatting::logical_text_run_enumerator::LogicalTextRunEnumerator;
use crate::media::text_formatting::text_formatter_impl::TextFormatterImpl;
use crate::media::text_formatting::{TextLine, TextRun};
use crate::media::FlowDirection;

/// Properties of text collapsing.
pub trait TextCollapsingProperties: 'static {
    /// Gets the width in which the collapsible range is constrained to.
    fn width(&self) -> f64;

    /// Gets the text run that is used as collapsing symbol.
    fn symbol(&self) -> &Rc<dyn TextRun>;

    /// Gets the flow direction that is used for collapsing.
    fn flow_direction(&self) -> FlowDirection;

    /// Collapses given text line. Returns the collapsed runs, or `None` when
    /// the line does not have to be collapsed.
    fn collapse(&self, text_line: &dyn TextLine) -> Option<Vec<Rc<dyn TextRun>>>;
}

impl dyn TextCollapsingProperties {
    /// Creates a list of runs for given collapsed length which includes specified symbol at the end.
    ///
    /// * `text_line` — the text line.
    /// * `collapsed_length` — the collapsed length.
    /// * `shaped_symbol` — the symbol.
    ///
    /// Returns the collapsed runs, in logical order.
    pub fn create_collapsed_runs(
        text_line: &dyn TextLine,
        collapsed_length: i32,
        shaped_symbol: Rc<dyn TextRun>,
    ) -> Vec<Rc<dyn TextRun>> {
        if collapsed_length <= 0 {
            return vec![shaped_symbol];
        }

        let object_pool = FormattingObjectPool::instance();

        let mut text_runs = object_pool.text_run_lists.rent();

        {
            let mut text_run_enumerator = LogicalTextRunEnumerator::new(text_line);

            let mut text_runs_length = 0;

            while let Some(text_run) = text_run_enumerator.move_next() {
                if text_runs_length >= collapsed_length {
                    break;
                }

                text_runs_length += text_run.length();

                text_runs.push(text_run);
            }
        }

        let (mut pre_split_runs, post_split_runs) =
            TextFormatterImpl::split_text_runs(&text_runs, collapsed_length, object_pool).deconstruct();

        let mut collapsed_runs = Vec::with_capacity(pre_split_runs.as_ref().map_or(0, |runs| runs.len()) + 1);

        if let Some(pre_split_runs) = &mut pre_split_runs {
            collapsed_runs.append(pre_split_runs);
        }

        collapsed_runs.push(shaped_symbol);

        object_pool.text_run_lists.return_list(text_runs);
        object_pool.text_run_lists.return_optional(pre_split_runs);
        object_pool.text_run_lists.return_optional(post_split_runs);

        collapsed_runs
    }
}
