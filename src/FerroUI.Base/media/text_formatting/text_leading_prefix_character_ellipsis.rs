use std::rc::Rc;

use crate::media::text_formatting::formatting_object_pool::{FormattingObjectPool, RentedList};
use crate::media::text_formatting::logical_text_run_enumerator::LogicalTextRunEnumerator;
use crate::media::text_formatting::text_formatter_impl::TextFormatterImpl;
use crate::media::text_formatting::{
    DrawableTextRun, ShapedTextRun, TextCharacters, TextCollapsingProperties, TextFormatter, TextLine, TextRun,
    TextRunProperties,
};
use crate::media::FlowDirection;
use crate::utilities::MathUtilities;

/// Ellipsis based on a fixed length leading prefix and suffix growing from the end at character granularity.
pub struct TextLeadingPrefixCharacterEllipsis {
    prefix_length: i32,
    width: f64,
    symbol: Rc<dyn TextRun>,
    flow_direction: FlowDirection,
}

impl TextLeadingPrefixCharacterEllipsis {
    /// Construct a text trailing character ellipsis collapsing properties.
    ///
    /// * `ellipsis` — text used as collapsing symbol.
    /// * `prefix_length` — length of leading prefix.
    /// * `width` — width in which collapsing is constrained to.
    /// * `text_run_properties` — text run properties of ellipsis symbol.
    /// * `flow_direction` — the flow direction of the collapsed line.
    ///
    /// Panics when `prefix_length` is negative.
    pub fn new(
        ellipsis: &str,
        prefix_length: i32,
        width: f64,
        text_run_properties: Rc<dyn TextRunProperties>,
        flow_direction: FlowDirection,
    ) -> Self {
        if prefix_length < 0 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'prefixLength')");
        }

        Self {
            prefix_length,
            width,
            symbol: Rc::new(TextCharacters::from_str(ellipsis, text_run_properties)),
            flow_direction,
        }
    }

    /// The part of upstream's `Collapse` that runs with the logical runs
    /// rented; the caller returns `logical_runs` afterwards.
    fn collapse_core(
        &self,
        logical_runs: &[Rc<dyn TextRun>],
        object_pool: &FormattingObjectPool,
    ) -> Option<Vec<Rc<dyn TextRun>>> {
        let shaped_symbol = <dyn TextFormatter>::create_symbol(&*self.symbol, self.flow_direction);

        if MathUtilities::less_than(self.width, shaped_symbol.glyph_run().bounds().width) {
            return Some(Vec::new());
        }

        // Overview of ellipsis structure
        // Prefix length run | Ellipsis symbol | Post split run growing from the end |
        let total_budget = self.width - shaped_symbol.size().width;
        let mut available_width = total_budget;
        let mut chars_before_current_run = 0;

        for current_run in logical_runs {
            if let Some(shaped_run) = current_run.downcast_ref::<ShapedTextRun>() {
                // Per-run check: does THIS run alone exceed what's left?
                if MathUtilities::greater_than(shaped_run.size().width, available_width) {
                    let measured_length = shaped_run.try_measure_characters(available_width).unwrap_or(0);

                    let total_fit_chars = chars_before_current_run + measured_length;

                    if total_fit_chars > 0 {
                        let mut collapsed_runs = object_pool.text_run_lists.rent();

                        let mut rented_pre_split_runs: Option<RentedList<Rc<dyn TextRun>>> = None;
                        let mut rented_post_split_runs: Option<RentedList<Rc<dyn TextRun>>> = None;
                        let mut reversed_suffix: Option<RentedList<Rc<dyn TextRun>>> = None;

                        self.collapse_overflowing(
                            logical_runs,
                            object_pool,
                            shaped_symbol,
                            total_budget,
                            total_fit_chars,
                            &mut collapsed_runs,
                            &mut rented_pre_split_runs,
                            &mut rented_post_split_runs,
                            &mut reversed_suffix,
                        );

                        let result = collapsed_runs.drain(..).collect();

                        object_pool.text_run_lists.return_optional(rented_pre_split_runs);
                        object_pool.text_run_lists.return_optional(rented_post_split_runs);
                        object_pool.text_run_lists.return_optional(reversed_suffix);
                        object_pool.text_run_lists.return_list(collapsed_runs);

                        return Some(result);
                    }

                    return Some(vec![shaped_symbol]);
                }

                available_width -= shaped_run.size().width;
            } else if let Some(drawable_text_run) = current_run.as_drawable() {
                available_width -= drawable_text_run.size().width;
            }

            chars_before_current_run += current_run.length();
        }

        None
    }

    /// Fills `collapsed_runs` with prefix, symbol and suffix. The rented
    /// lists are handed back to the caller, which returns them to the pool.
    #[allow(clippy::too_many_arguments)]
    fn collapse_overflowing(
        &self,
        logical_runs: &[Rc<dyn TextRun>],
        object_pool: &FormattingObjectPool,
        shaped_symbol: Rc<ShapedTextRun>,
        total_budget: f64,
        total_fit_chars: i32,
        collapsed_runs: &mut RentedList<Rc<dyn TextRun>>,
        rented_pre_split_runs: &mut Option<RentedList<Rc<dyn TextRun>>>,
        rented_post_split_runs: &mut Option<RentedList<Rc<dyn TextRun>>>,
        reversed_suffix: &mut Option<RentedList<Rc<dyn TextRun>>>,
    ) {
        // Split at GLOBAL character index total_fit_chars-capped-by-prefix_length.
        let prefix_cutoff = self.prefix_length.min(total_fit_chars);

        let effective_post_split_runs: Option<&[Rc<dyn TextRun>]> = if prefix_cutoff > 0 {
            let (pre_split_runs, post_split_runs) =
                TextFormatterImpl::split_text_runs(logical_runs, prefix_cutoff, object_pool).deconstruct();

            *rented_pre_split_runs = pre_split_runs;
            *rented_post_split_runs = post_split_runs;

            if let Some(pre_split_runs) = rented_pre_split_runs.as_ref() {
                collapsed_runs.extend(pre_split_runs.iter().cloned());
            }

            rented_post_split_runs.as_deref()
        } else {
            Some(logical_runs)
        };

        collapsed_runs.push(shaped_symbol);

        let Some(effective_post_split_runs) = effective_post_split_runs else {
            return;
        };

        if total_fit_chars <= self.prefix_length {
            return;
        }

        // Suffix budget = total budget minus the actual prefix width.
        let mut available_suffix_width = total_budget;

        if let Some(pre_split_runs) = rented_pre_split_runs.as_ref() {
            for run in pre_split_runs {
                // A shaped run is a drawable run: both of upstream's cases subtract the run's width.
                if let Some(pre_drawable) = run.as_drawable() {
                    available_suffix_width -= pre_drawable.size().width;
                }
            }
        }

        // Walk the post-split runs from the logical tail back toward the
        // prefix, fitting trailing characters into available_suffix_width.
        // We collect each split into reversed_suffix here (so the LAST
        // logical run lands at index 0) and then drain reversed_suffix
        // backwards when appending to collapsed_runs, which restores
        // LOGICAL order. Finalizing the line handles the visual re-bidi.
        let reversed_suffix = reversed_suffix.insert(object_pool.text_run_lists.rent());

        for run in effective_post_split_runs.iter().rev() {
            if let Some(end_shaped_run) = run.downcast_ref::<ShapedTextRun>() {
                if let Some((suffix_count, suffix_width)) =
                    end_shaped_run.try_measure_characters_backwards(available_suffix_width)
                {
                    available_suffix_width -= suffix_width;

                    let split_at = run.length() - suffix_count;

                    if split_at > 0 {
                        let split_suffix = end_shaped_run.split(split_at);

                        if let Some(second) = split_suffix.second {
                            reversed_suffix.push(second);
                        }
                    } else if suffix_count > 0 {
                        // The whole run fits in the remaining suffix budget, so no
                        // split is needed; use the run as-is. (Splitting at 0 panics.)
                        reversed_suffix.push(run.clone());
                    }
                    // else: suffix_count == 0, nothing of this run survives.
                }
            }
        }

        collapsed_runs.extend(reversed_suffix.iter().rev().cloned());
    }
}

impl TextCollapsingProperties for TextLeadingPrefixCharacterEllipsis {
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
        // Materialize runs in LOGICAL order. The consumer (the line's collapse)
        // wraps our result in a new line and runs the BiDi reorderer when
        // finalizing it, so we must hand back runs in logical order — not the
        // visual order exposed via the line's text runs.
        let object_pool = FormattingObjectPool::instance();
        let mut logical_runs = object_pool.text_run_lists.rent();

        {
            let mut enumerator = LogicalTextRunEnumerator::new(text_line);

            while let Some(run) = enumerator.move_next() {
                logical_runs.push(run);
            }
        }

        let result = self.collapse_core(&logical_runs, object_pool);

        object_pool.text_run_lists.return_list(logical_runs);

        result
    }
}
