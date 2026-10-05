use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::media::text_formatting::formatting_buffer_helper::FormattingBufferHelper;
use crate::media::text_formatting::indexed_text_run::IndexedTextRun;
use crate::media::text_formatting::{ShapedTextRun, TextRun};
use crate::media::FlowDirection;
use crate::utilities::ArrayBuilder;

thread_local! {
    static INSTANCE: RefCell<BidiReorderer> = const { RefCell::new(BidiReorderer::new()) };
}

/// Reorders text runs according to their bidi level.
///
/// To avoid allocations, the scratch buffers of this type are kept per thread
/// and reused.
pub(crate) struct BidiReorderer {
    runs: ArrayBuilder<OrderedBidiRun>,
    ranges: ArrayBuilder<BidiRange>,
}

impl BidiReorderer {
    const fn new() -> Self {
        Self { runs: ArrayBuilder::new(), ranges: ArrayBuilder::new() }
    }

    /// Reorders `text_runs` into visual order using the reorderer of the
    /// current thread and returns the logical run table.
    pub fn bidi_reorder(
        text_runs: &mut [Rc<dyn TextRun>],
        flow_direction: FlowDirection,
        first_text_source_index: i32,
    ) -> Rc<[IndexedTextRun]> {
        // The instance is taken out of its slot while it is used, so no
        // borrow is held across the calls into the runs.
        let mut instance = INSTANCE.with(|instance| std::mem::replace(&mut *instance.borrow_mut(), Self::new()));

        let result = instance.bidi_reorder_core(text_runs, flow_direction, first_text_source_index);

        FormattingBufferHelper::clear_then_reset_if_too_large_builder(&mut instance.runs);
        FormattingBufferHelper::clear_then_reset_if_too_large_builder(&mut instance.ranges);

        INSTANCE.with(|slot| *slot.borrow_mut() = instance);

        result
    }

    fn bidi_reorder_core(
        &mut self,
        text_runs: &mut [Rc<dyn TextRun>],
        flow_direction: FlowDirection,
        mut first_text_source_index: i32,
    ) -> Rc<[IndexedTextRun]> {
        debug_assert!(self.runs.length() == 0);
        debug_assert!(self.ranges.length() == 0);

        if text_runs.is_empty() {
            return Rc::from(Vec::new());
        }

        let mut previous_level: Option<i8> = None;

        // Build up the collection of ordered runs.
        for (i, text_run) in text_runs.iter().enumerate() {
            let ordered_run = OrderedBidiRun::new(
                i as i32,
                text_run.clone(),
                Self::get_run_bidi_level(&**text_run, flow_direction, previous_level),
            );

            previous_level = Some(ordered_run.level);

            self.runs.add_item(ordered_run);

            if i > 0 {
                self.runs[i - 1].next_run_index = i as i32;
            }
        }

        // Reorder them into visual order.
        let first_index = self.linear_reorder();
        let mut indexed_text_runs = Vec::with_capacity(text_runs.len());

        for (i, current_run) in text_runs.iter().enumerate() {
            indexed_text_runs.push(IndexedTextRun {
                text_run: RefCell::new(Some(current_run.clone())),
                text_source_character_index: first_text_source_index,
                run_index: Cell::new(i as i32),
                next_run_index: Cell::new(i as i32 + 1),
            });

            first_text_source_index += current_run.length();
        }

        // Shape-time already produces glyphs in visual order (RTL buffers have descending
        // clusters), so L2 reversal of glyphs is no longer needed here — we only shuffle
        // the run span into visual order.
        let mut index = 0usize;
        let mut current_index = first_index;

        while current_index >= 0 {
            let current = &mut self.runs[current_index as usize];

            if let Some(run) = current.run.take() {
                text_runs[index] = run;
            }

            let indexed_run = &indexed_text_runs[index];

            indexed_run.run_index.set(current.run_index);

            indexed_run.next_run_index.set(current.next_run_index);

            index += 1;

            current_index = current.next_run_index;
        }

        Rc::from(indexed_text_runs)
    }

    fn get_run_bidi_level(run: &dyn TextRun, flow_direction: FlowDirection, previous_level: Option<i8>) -> i8 {
        if let Some(shaped_text_run) = run.downcast_ref::<ShapedTextRun>() {
            return shaped_text_run.bidi_level();
        }

        let default_level: i8 = if flow_direction == FlowDirection::LeftToRight { 0 } else { 1 };

        if run.as_text_end_of_line().is_some() {
            return 0;
        }

        if let Some(previous_level) = previous_level {
            return previous_level;
        }

        default_level
    }

    /// Reorders the runs from logical to visual order.
    /// <https://github.com/fribidi/linear-reorder/blob/f2f872257d4d8b8e137fcf831f254d6d4db79d3c/linear-reorder.c>
    ///
    /// Returns the first run index in visual order.
    fn linear_reorder(&mut self) -> i32 {
        let mut run_index = 0i32;
        let mut range_index = -1i32;

        while run_index >= 0 {
            let run_level = self.runs[run_index as usize].level;
            let next_run_index = self.runs[run_index as usize].next_run_index;

            while range_index >= 0
                && self.ranges[range_index as usize].level > run_level
                && self.ranges[range_index as usize].previous_range_index >= 0
                && self.ranges[self.ranges[range_index as usize].previous_range_index as usize].level >= run_level
            {
                range_index = self.merge_range_with_previous(range_index);
            }

            if range_index >= 0 && self.ranges[range_index as usize].level >= run_level {
                let range = &mut self.ranges[range_index as usize];

                // Attach run to the range.
                if (run_level & 1) != 0 {
                    // Odd, range goes to the right of run.
                    self.runs[run_index as usize].next_run_index = range.left_run_index;
                    range.left_run_index = run_index;
                } else {
                    // Even, range goes to the left of run.
                    self.runs[range.right_run_index as usize].next_run_index = run_index;
                    range.right_run_index = run_index;
                }

                range.level = run_level;
            } else {
                let range = BidiRange {
                    level: run_level,
                    left_run_index: run_index,
                    right_run_index: run_index,
                    previous_range_index: range_index,
                };
                self.ranges.add_item(range);
                range_index = self.ranges.length() as i32 - 1;
            }

            run_index = next_run_index;
        }

        while range_index >= 0 && self.ranges[range_index as usize].previous_range_index >= 0 {
            range_index = self.merge_range_with_previous(range_index);
        }

        // Terminate.
        let range = self.ranges[range_index as usize];
        self.runs[range.right_run_index as usize].next_run_index = -1;

        self.runs[range.left_run_index as usize].run_index
    }

    fn merge_range_with_previous(&mut self, index: i32) -> i32 {
        let previous_index = self.ranges[index as usize].previous_range_index;
        let previous_level = self.ranges[previous_index as usize].level;

        let (left_index, right_index) = if (previous_level & 1) != 0 {
            // Odd, previous goes to the right of range.
            (index, previous_index)
        } else {
            // Even, previous goes to the left of range.
            (previous_index, index)
        };

        // Stitch them
        let left = self.ranges[left_index as usize];
        let right = self.ranges[right_index as usize];
        self.runs[left.right_run_index as usize].next_run_index = self.runs[right.left_run_index as usize].run_index;

        let previous = &mut self.ranges[previous_index as usize];
        previous.left_run_index = left.left_run_index;
        previous.right_run_index = right.right_run_index;

        previous_index
    }
}

#[derive(Clone, Copy)]
struct BidiRange {
    level: i8,
    left_run_index: i32,
    right_run_index: i32,
    /// -1 if none
    previous_range_index: i32,
}

pub(crate) struct OrderedBidiRun {
    run_index: i32,
    level: i8,
    /// The run; moved back into the run list when the visual order is written.
    run: Option<Rc<dyn TextRun>>,
    /// -1 if none
    next_run_index: i32,
}

impl OrderedBidiRun {
    fn new(run_index: i32, run: Rc<dyn TextRun>, level: i8) -> Self {
        Self { run_index, run: Some(run), level, next_run_index: -1 }
    }
}
