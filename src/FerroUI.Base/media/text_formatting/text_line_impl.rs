use std::any::Any;
use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

use crate::media::text_formatting::bidi_reorderer::BidiReorderer;
use crate::media::text_formatting::indexed_text_run::IndexedTextRun;
use crate::media::text_formatting::unicode::Codepoint;
use crate::media::text_formatting::{
    DrawableTextRun, ITextDrawingSink, JustificationProperties, LogicalDirection, ShapedTextRun, TextBounds,
    TextCollapsingProperties, TextLine, TextLineBreak, TextLineMetrics, TextParagraphProperties, TextRun,
    TextRunBounds,
};
use crate::media::{BaselineAlignment, CharacterHit, FlowDirection, TextAlignment};
use crate::utilities::MathUtilities;
use crate::{Point, Rect, Vector};

/// The line produced by the text formatter.
///
/// The run list is only written by [`TextLineImpl::replace_text_run`] (used
/// by justification) and by [`TextLineImpl::finalize_line`]; everything else
/// reads it.
pub struct TextLineImpl {
    indexed_text_runs: RefCell<Option<Rc<[IndexedTextRun]>>>,
    text_runs: RefCell<Vec<Rc<dyn TextRun>>>,
    paragraph_width: f64,
    paragraph_properties: Rc<dyn TextParagraphProperties>,
    text_line_metrics: Cell<TextLineMetrics>,
    text_line_break: RefCell<Option<Rc<TextLineBreak>>>,
    resolved_flow_direction: FlowDirection,
    first_text_source_index: i32,
    length: i32,
    has_collapsed: bool,

    ink_bounds: Cell<Rect>,
    bounds: Cell<Rect>,
}

impl TextLineImpl {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        text_runs: Vec<Rc<dyn TextRun>>,
        first_text_source_index: i32,
        length: i32,
        paragraph_width: f64,
        paragraph_properties: Rc<dyn TextParagraphProperties>,
        resolved_flow_direction: FlowDirection,
        line_break: Option<Rc<TextLineBreak>>,
        has_collapsed: bool,
    ) -> Rc<Self> {
        Rc::new(Self {
            indexed_text_runs: RefCell::new(None),
            text_runs: RefCell::new(text_runs),
            paragraph_width,
            paragraph_properties,
            text_line_metrics: Cell::new(TextLineMetrics::default()),
            text_line_break: RefCell::new(line_break),
            resolved_flow_direction,
            first_text_source_index,
            length,
            has_collapsed,
            ink_bounds: Cell::new(Rect::default()),
            bounds: Cell::new(Rect::default()),
        })
    }

    /// Orders two text bounds by their left edge.
    pub(crate) fn text_bounds_comparer(x: &TextBounds, y: &TextBounds) -> std::cmp::Ordering {
        x.rectangle().left().total_cmp(&y.rectangle().left())
    }

    /// The logical run table of the line; `None` before [`TextLineImpl::finalize_line`].
    pub(crate) fn indexed_text_runs(&self) -> Option<Rc<[IndexedTextRun]>> {
        self.indexed_text_runs.borrow().clone()
    }

    /// The bounds of the line (start, 0, width including trailing whitespace, height).
    #[allow(dead_code)] // used by the text layout (a later step)
    pub(crate) fn bounds(&self) -> Rect {
        self.bounds.get()
    }

    /// The ink bounds of the line.
    #[allow(dead_code)] // used by the text layout (a later step)
    pub(crate) fn ink_bounds(&self) -> Rect {
        self.ink_bounds.get()
    }

    /// The vertical offset that aligns a run in a line according to the
    /// run's baseline alignment.
    ///
    /// Panics when the run has no properties (and therefore no baseline alignment).
    pub fn get_baseline_offset(text_line: &dyn TextLine, text_run: &dyn DrawableTextRun) -> f64 {
        let baseline = text_run.baseline();
        let baseline_alignment = text_run.properties().map(|properties| properties.baseline_alignment());

        let mut baseline_offset = -baseline;

        match baseline_alignment {
            Some(BaselineAlignment::Baseline) => {
                baseline_offset += text_line.baseline();
            }
            Some(BaselineAlignment::Top | BaselineAlignment::TextTop) => {
                baseline_offset += text_line.height() - text_line.extent() + text_run.size().height / 2.0;
            }
            Some(BaselineAlignment::Center) => {
                baseline_offset += text_line.height() / 2.0 + baseline - text_run.size().height / 2.0;
            }
            Some(BaselineAlignment::Subscript | BaselineAlignment::Bottom | BaselineAlignment::TextBottom) => {
                baseline_offset += text_line.height() - text_run.size().height + baseline;
            }
            Some(BaselineAlignment::Superscript) => {
                baseline_offset += baseline;
            }
            None => panic!("Specified argument was out of the range of valid values. (Parameter 'baselineAlignment')"),
        }

        baseline_offset
    }

    pub(crate) fn replace_text_run(&self, index: usize, text_run: Rc<dyn TextRun>) {
        let old_run = self.text_runs.borrow()[index].clone();

        if same_run(&old_run, &text_run) {
            return;
        }

        self.text_runs.borrow_mut()[index] = text_run.clone();

        let Some(indexed_text_runs) = self.indexed_text_runs() else {
            return;
        };

        for indexed_text_run in indexed_text_runs.iter() {
            let is_old_run =
                indexed_text_run.text_run.borrow().as_ref().is_some_and(|current| same_run(current, &old_run));

            if is_old_run {
                *indexed_text_run.text_run.borrow_mut() = Some(text_run);

                break;
            }
        }
    }

    fn get_run_character_hit(run: &dyn TextRun, current_position: i32, distance: f64) -> CharacterHit {
        if let Some(shaped_run) = run.downcast_ref::<ShapedTextRun>() {
            let glyph_run = shaped_run.glyph_run();

            let (character_hit, _) = glyph_run.get_character_hit_from_distance(distance);

            let offset = 0.max(current_position - glyph_run.metrics().first_cluster);

            CharacterHit::with_trailing_length(
                offset + character_hit.first_character_index(),
                character_hit.trailing_length(),
            )
        } else if let Some(drawable_text_run) = run.as_drawable() {
            if distance < drawable_text_run.size().width / 2.0 {
                CharacterHit::new(current_position)
            } else {
                CharacterHit::with_trailing_length(current_position, run.length())
            }
        } else {
            CharacterHit::with_trailing_length(current_position, run.length())
        }
    }

    fn find_indexed_run(indexed_text_runs: &[IndexedTextRun], current_position: i32) -> usize {
        let mut index = 0;

        while indexed_text_runs[index].text_source_character_index != current_position {
            if index + 1 == indexed_text_runs.len() {
                break;
            }

            index += 1;
        }

        index
    }

    fn get_preceding_distance(&self, first_index: i32) -> f64 {
        let text_runs = self.text_runs.borrow();

        let mut distance = 0.0;

        for current_run in text_runs.iter().take(first_index.max(0) as usize) {
            if let Some(drawable_text_run) = current_run.as_drawable() {
                distance += drawable_text_run.size().width;
            }
        }

        distance
    }

    fn get_run_direction(text_run: Option<&dyn TextRun>, current_direction: FlowDirection) -> FlowDirection {
        if let Some(shaped_text_run) = text_run.and_then(|text_run| text_run.downcast_ref::<ShapedTextRun>()) {
            return if shaped_text_run.shaped_buffer().is_left_to_right() {
                FlowDirection::LeftToRight
            } else {
                FlowDirection::RightToLeft
            };
        }

        current_direction
    }

    fn get_run_text_source_position(&self, visual_run_index: usize) -> i32 {
        if let Some(indexed_text_runs) = self.indexed_text_runs() {
            for indexed_text_run in indexed_text_runs.iter() {
                if indexed_text_run.run_index.get() == visual_run_index as i32 {
                    return indexed_text_run.text_source_character_index;
                }
            }
        }

        let text_runs = self.text_runs.borrow();

        let mut position = self.first_text_source_index;

        for text_run in text_runs.iter().take(visual_run_index) {
            position += text_run.length();
        }

        position
    }

    fn get_last_directional_run_index(
        indexed_text_runs: &[IndexedTextRun],
        mut indexed_run_index: usize,
        flow_direction: FlowDirection,
        directional_width: &mut f64,
    ) -> i32 {
        let mut last_run_index = indexed_text_runs[indexed_run_index].run_index.get();

        // Find consecutive runs of same direction
        while indexed_run_index + 1 < indexed_text_runs.len() {
            indexed_run_index += 1;

            let next_indexed_run = &indexed_text_runs[indexed_run_index];

            if next_indexed_run.run_index.get() != last_run_index + 1 {
                break;
            }

            let next_run = next_indexed_run.text_run.borrow();

            let Some(next_run) = next_run.as_ref() else {
                break;
            };

            let next_direction = Self::get_run_direction(Some(&**next_run), flow_direction);

            if next_direction != flow_direction {
                break;
            }

            if let Some(next_drawable) = next_run.as_drawable() {
                *directional_width += next_drawable.size().width;
            }

            last_run_index = next_indexed_run.run_index.get();
        }

        last_run_index
    }

    /// Merges `current_bounds` into `last_bounds` when both have the same
    /// direction and abut; gives `current_bounds` back otherwise.
    fn try_merge_with_last_bounds(current_bounds: TextBounds, last_bounds: &mut TextBounds) -> Option<TextBounds> {
        if current_bounds.flow_direction() != last_bounds.flow_direction() {
            return Some(current_bounds);
        }

        // The two edges are computed by summing glyph advances along different paths, so
        // abutting bounds can land an ULP apart - compare them the way the rest of layout
        // compares coordinates, or a single directional span gets reported as two.
        if MathUtilities::are_close(current_bounds.rectangle().left(), last_bounds.rectangle().right()) {
            last_bounds.text_run_bounds_mut().extend(current_bounds.text_run_bounds().iter().cloned());

            last_bounds.set_rectangle(last_bounds.rectangle().union(current_bounds.rectangle()));

            return None;
        }

        if MathUtilities::are_close(current_bounds.rectangle().right(), last_bounds.rectangle().left()) {
            for (i, run_bounds) in current_bounds.text_run_bounds().iter().enumerate() {
                last_bounds.text_run_bounds_mut().insert(i, run_bounds.clone());
            }

            last_bounds.set_rectangle(last_bounds.rectangle().union(current_bounds.rectangle()));

            return None;
        }

        Some(current_bounds)
    }

    fn get_previous_character_hit(&self, character_hit: CharacterHit, is_backspace_delete: bool) -> CharacterHit {
        if self.text_runs.borrow().is_empty() || self.indexed_text_runs.borrow().is_none() {
            return CharacterHit::default();
        }

        if character_hit.trailing_length() > 0 && character_hit.first_character_index() <= self.first_text_source_index {
            return CharacterHit::new(self.first_text_source_index);
        }

        let character_index = character_hit.first_character_index() + character_hit.trailing_length();

        if character_index <= self.first_text_source_index {
            return CharacterHit::new(self.first_text_source_index);
        }

        let (current_run, current_position) =
            self.get_run_at_character_index(character_index, LogicalDirection::Backward);

        let mut previous_character_hit = character_hit;

        if let Some(current_run) = &current_run {
            if let Some(shaped_run) = current_run.downcast_ref::<ShapedTextRun>() {
                let glyph_run = shaped_run.glyph_run();

                // Determine the start of the first hit in local positions.
                let run_offset = 0.max(character_index - current_position);

                let first_cluster = glyph_run.metrics().first_cluster;

                // Current position is a text source index and first cluster is relative to the GlyphRun's buffer.
                let text_source_offset = current_position - first_cluster;

                if is_backspace_delete {
                    let characters = glyph_run.characters();
                    let characters = characters.span();

                    let mut length = 0i32;

                    loop {
                        let (codepoint, count) = Codepoint::read_at(characters, length as usize);

                        if codepoint == Codepoint::REPLACEMENT_CODEPOINT {
                            break;
                        }

                        let mut count = count as i32;

                        if codepoint.value() == 0x0D {
                            let (next_codepoint, lf_count) = Codepoint::read_at(characters, (length + count) as usize);

                            if next_codepoint.value() == 0x0A {
                                count += lf_count as i32;
                            }
                        }

                        if length + count >= run_offset {
                            break;
                        }

                        length += count;
                    }

                    previous_character_hit = CharacterHit::new(character_index - run_offset + length);
                } else {
                    previous_character_hit =
                        glyph_run.get_previous_caret_character_hit(CharacterHit::new(first_cluster + run_offset));

                    if text_source_offset > 0 {
                        previous_character_hit = CharacterHit::with_trailing_length(
                            text_source_offset + previous_character_hit.first_character_index(),
                            previous_character_hit.trailing_length(),
                        );
                    }
                }
            } else {
                previous_character_hit = CharacterHit::new(current_position);
            }
        }

        if character_index == previous_character_hit.first_character_index() + previous_character_hit.trailing_length() {
            return character_hit;
        }

        previous_character_hit
    }

    /// Returns the bounds, the covered length and the new position.
    #[allow(clippy::too_many_arguments)]
    fn get_text_run_bounds_right_to_left(
        &self,
        first_run_index: i32,
        last_run_index: i32,
        mut end_x: f64,
        first_text_source_index: i32,
        mut current_position: i32,
        mut remaining_length: i32,
    ) -> (TextBounds, i32, i32) {
        let text_runs = self.text_runs.borrow();
        let height = self.height();
        let line_end = self.first_text_source_index + self.length;

        let mut covered_length = 0;
        let mut text_run_bounds: Vec<TextRunBounds> = Vec::new();
        let mut start_x = end_x;

        let mut i = last_run_index;

        while i >= first_run_index {
            let current_run = &text_runs[i as usize];

            if let Some(shaped_text_run) = current_run.downcast_ref::<ShapedTextRun>() {
                let run_bounds = self.get_run_bounds(
                    current_run,
                    shaped_text_run,
                    start_x,
                    first_text_source_index,
                    remaining_length,
                    current_position,
                );

                let run_bounds_rectangle = run_bounds.rectangle();
                let run_bounds_index = run_bounds.text_source_character_index();
                let run_bounds_length = run_bounds.length();

                if run_bounds_index < line_end {
                    text_run_bounds.insert(0, run_bounds);
                }

                if i == last_run_index {
                    end_x = run_bounds_rectangle.right();

                    start_x = end_x;
                }

                start_x -= run_bounds_rectangle.width;

                current_position = run_bounds_index + run_bounds_length;

                covered_length += run_bounds_length;

                remaining_length -= run_bounds_length;
            } else {
                if current_position < line_end {
                    if let Some(drawable_text_run) = current_run.as_drawable() {
                        start_x -= drawable_text_run.size().width;

                        let run_bounds = TextRunBounds::new(
                            Rect::new(start_x, 0.0, drawable_text_run.size().width, height),
                            current_position,
                            current_run.length(),
                            current_run.clone(),
                        );

                        text_run_bounds.insert(0, run_bounds);
                    } else {
                        // Add potential TextEndOfParagraph
                        let run_bounds = TextRunBounds::new(
                            Rect::new(end_x, 0.0, 0.0, height),
                            current_position,
                            current_run.length(),
                            current_run.clone(),
                        );

                        text_run_bounds.push(run_bounds);
                    }
                }

                current_position += current_run.length();

                covered_length += current_run.length();

                remaining_length -= current_run.length();
            }

            if remaining_length <= 0 {
                break;
            }

            i -= 1;
        }

        let run_width = end_x - start_x;

        let bounds = Rect::new(start_x, 0.0, run_width, height);

        (TextBounds::new(bounds, FlowDirection::RightToLeft, text_run_bounds), covered_length, current_position)
    }

    /// Returns the bounds, the covered length and the new position.
    #[allow(clippy::too_many_arguments)]
    fn get_text_bounds_left_to_right(
        &self,
        first_run_index: i32,
        last_run_index: i32,
        mut start_x: f64,
        first_text_source_index: i32,
        mut current_position: i32,
        mut remaining_length: i32,
    ) -> (TextBounds, i32, i32) {
        let text_runs = self.text_runs.borrow();
        let height = self.height();
        let line_end = self.first_text_source_index + self.length;

        let mut covered_length = 0;
        let mut text_run_bounds: Vec<TextRunBounds> = Vec::with_capacity(1);
        let mut end_x = start_x;

        let mut i = first_run_index;

        while i <= last_run_index {
            let current_run = &text_runs[i as usize];

            if let Some(shaped_text_run) = current_run.downcast_ref::<ShapedTextRun>() {
                let run_bounds = self.get_run_bounds(
                    current_run,
                    shaped_text_run,
                    end_x,
                    first_text_source_index,
                    remaining_length,
                    current_position,
                );

                let run_bounds_rectangle = run_bounds.rectangle();
                let run_bounds_index = run_bounds.text_source_character_index();
                let run_bounds_length = run_bounds.length();

                if run_bounds_index < line_end {
                    text_run_bounds.push(run_bounds);
                }

                current_position = run_bounds_index + run_bounds_length;

                if i == first_run_index {
                    start_x = run_bounds_rectangle.left();
                }

                end_x = run_bounds_rectangle.right();

                covered_length += run_bounds_length;

                remaining_length -= run_bounds_length;
            } else {
                if current_position < line_end {
                    if let Some(drawable_text_run) = current_run.as_drawable() {
                        let run_bounds = TextRunBounds::new(
                            Rect::new(end_x, 0.0, drawable_text_run.size().width, height),
                            current_position,
                            current_run.length(),
                            current_run.clone(),
                        );

                        text_run_bounds.push(run_bounds);

                        end_x += drawable_text_run.size().width;
                    } else {
                        // Add potential TextEndOfParagraph
                        let run_bounds = TextRunBounds::new(
                            Rect::new(end_x, 0.0, 0.0, height),
                            current_position,
                            current_run.length(),
                            current_run.clone(),
                        );

                        text_run_bounds.push(run_bounds);
                    }
                }

                current_position += current_run.length();

                covered_length += current_run.length();

                remaining_length -= current_run.length();
            }

            if remaining_length <= 0 {
                break;
            }

            i += 1;
        }

        let run_width = end_x - start_x;

        let bounds = Rect::new(start_x, 0.0, run_width, height);

        (TextBounds::new(bounds, FlowDirection::LeftToRight, text_run_bounds), covered_length, current_position)
    }

    fn get_run_bounds(
        &self,
        text_run: &Rc<dyn TextRun>,
        current_run: &ShapedTextRun,
        current_x: f64,
        first_text_source_index: i32,
        remaining_length: i32,
        current_position: i32,
    ) -> TextRunBounds {
        let is_left_to_right = current_run.bidi_level() % 2 == 0;
        let glyph_run = current_run.glyph_run();

        let mut start_x = current_x;
        let mut end_x = current_x;

        // Determine the start of the first hit in local positions
        let run_offset = 0.max(first_text_source_index - current_position);
        let first_cluster = glyph_run.metrics().first_cluster;

        // The start index needs to be relative to the first cluster
        let mut start_index = first_cluster + run_offset;
        let end_index = start_index + remaining_length;

        // Current position is a text source index and first cluster is relative to the GlyphRun's buffer.
        let text_source_offset = current_position - first_cluster;

        debug_assert!(text_source_offset >= 0);

        let mut cluster_offset = 0;

        // Cluster boundary correction
        if run_offset > 0 {
            let (character_hit, _) = glyph_run.find_nearest_character_hit(start_index);
            let cluster_start = character_hit.first_character_index();
            let cluster_end = cluster_start + character_hit.trailing_length();

            if cluster_start < start_index && cluster_end > start_index {
                // Remember the cluster correction offset
                cluster_offset = start_index - cluster_start;

                // Move to the start of the cluster
                start_index -= cluster_offset;
            }
        }

        // Find the visual start and end position of the hit
        let start_offset = glyph_run.get_distance_from_character_hit(CharacterHit::new(start_index));
        let end_offset = glyph_run.get_distance_from_character_hit(CharacterHit::new(end_index));

        if is_left_to_right {
            end_x = start_x + end_offset;
            start_x += start_offset;
        } else {
            // We need the distance from right to left and get_distance_from_character_hit returns a
            // distance from left to right so we need to adjust the offsets
            let run_width = current_run.size().width;

            start_x -= run_width - start_offset;
            end_x -= run_width - end_offset;
        }

        // Find the start of the hit
        let (start_hit, _) = glyph_run.find_nearest_character_hit(start_index);
        let mut start_hit_index = start_hit.first_character_index();

        // If the requested text range starts at the trailing edge we need to move at the end of the hit
        if start_hit_index < start_index {
            start_hit_index += start_hit.trailing_length();
        }

        // Find the next possible position that contains the end_index
        let (nearest_end_hit, _) = glyph_run.find_nearest_character_hit(end_index);

        let end_hit_index = if nearest_end_hit.first_character_index() < end_index {
            // The hit is inside or at the trailing edge
            nearest_end_hit.first_character_index() + nearest_end_hit.trailing_length()
        } else {
            // The hit is at the leading edge
            nearest_end_hit.first_character_index()
        };

        let covered_length = 0.max((start_hit_index - end_hit_index).abs() - cluster_offset);

        // Normalize bounds
        if end_x < start_x {
            std::mem::swap(&mut end_x, &mut start_x);
        }

        let run_width = end_x - start_x;

        // We need to adjust the local position to the text source
        let text_source_index = text_source_offset + start_hit_index + cluster_offset;

        TextRunBounds::new(
            Rect::new(start_x, 0.0, run_width, self.height()),
            text_source_index,
            covered_length,
            text_run.clone(),
        )
    }

    pub fn finalize_line(&self) {
        let indexed_text_runs = {
            let mut text_runs = self.text_runs.borrow_mut();

            BidiReorderer::bidi_reorder(
                &mut text_runs,
                self.paragraph_properties.flow_direction(),
                self.first_text_source_index,
            )
        };

        *self.indexed_text_runs.borrow_mut() = Some(indexed_text_runs);

        self.text_line_metrics.set(self.create_line_metrics());

        if self.text_line_break.borrow().is_none() {
            let text_runs = self.text_runs.borrow();

            if text_runs.len() > 1 {
                let last_run = &text_runs[text_runs.len() - 1];

                if last_run.as_text_end_of_line().is_some() {
                    *self.text_line_break.borrow_mut() =
                        Some(Rc::new(TextLineBreak::new(Some(last_run.clone()), FlowDirection::LeftToRight, false)));
                }
            }
        }
    }

    /// Returns the run at the character index and the text position of the run.
    fn get_run_at_character_index(
        &self,
        codepoint_index: i32,
        direction: LogicalDirection,
    ) -> (Option<Rc<dyn TextRun>>, i32) {
        let mut run_index = 0usize;
        let mut text_position = self.first_text_source_index;

        let Some(indexed_text_runs) = self.indexed_text_runs() else {
            return (None, text_position);
        };

        let text_runs_length = self.text_runs.borrow().len();

        let mut current_run: Option<Rc<dyn TextRun>> = None;

        while run_index < indexed_text_runs.len() {
            let indexed_run = &indexed_text_runs[run_index];
            current_run = indexed_run.text_run.borrow().clone();

            if let Some(run) = &current_run {
                if let Some(shaped_run) = run.downcast_ref::<ShapedTextRun>() {
                    let mut first_cluster = shaped_run.glyph_run().metrics().first_cluster;

                    first_cluster += 0.max(indexed_run.text_source_character_index - first_cluster);

                    if direction == LogicalDirection::Forward {
                        if codepoint_index >= first_cluster && codepoint_index < first_cluster + run.length() {
                            return (current_run, text_position);
                        }
                    } else if codepoint_index > first_cluster && codepoint_index <= first_cluster + run.length() {
                        return (current_run, text_position);
                    }
                } else if direction == LogicalDirection::Forward {
                    if text_position == codepoint_index {
                        return (current_run, text_position);
                    }
                } else if text_position + run.length() == codepoint_index {
                    return (current_run, text_position);
                }

                if run_index + 1 >= text_runs_length {
                    return (current_run, text_position);
                }

                text_position += run.length();
            }

            run_index += 1;
        }

        (current_run, text_position)
    }

    fn create_line_metrics(&self) -> TextLineMetrics {
        let text_runs = self.text_runs.borrow();
        let default_text_run_properties = self.paragraph_properties.default_text_run_properties();

        let font_metrics = default_text_run_properties.cached_glyph_typeface().metrics();
        let font_rendering_em_size = default_text_run_properties.font_rendering_em_size();
        let scale = font_rendering_em_size / font_metrics.design_em_height as f64;
        let mut width_including_whitespace = 0f64;
        let mut trailing_whitespace_length = 0;
        let mut new_line_length = 0;
        let mut ascent = font_metrics.ascent as f64 * scale;
        let mut descent = font_metrics.descent as f64 * scale;
        let mut line_gap = font_metrics.line_gap as f64 * scale;

        let line_height = self.paragraph_properties.line_height();
        let line_spacing = self.paragraph_properties.line_spacing();

        for run in text_runs.iter() {
            if let Some(text_run) = run.downcast_ref::<ShapedTextRun>() {
                let text_metrics = text_run.text_metrics();

                if ascent > text_metrics.ascent {
                    ascent = text_metrics.ascent;
                }

                if descent < text_metrics.descent {
                    descent = text_metrics.descent;
                }

                if line_gap < text_metrics.line_gap {
                    line_gap = text_metrics.line_gap;
                }
            } else if let Some(drawable_text_run) = run.as_drawable() {
                if drawable_text_run.baseline() > -ascent {
                    ascent = -drawable_text_run.baseline();
                }

                let bottom = drawable_text_run.size().height - drawable_text_run.baseline();

                if bottom > descent {
                    descent = bottom;
                }
            }
        }

        let mut ink_bounds = Rect::default();

        for run in text_runs.iter() {
            if let Some(text_run) = run.downcast_ref::<ShapedTextRun>() {
                let glyph_run = text_run.glyph_run();
                // Align the ink bounds at the common baseline
                let offset_y = -ascent - text_run.baseline();

                let run_bounds = glyph_run.ink_bounds().translate(Vector::new(width_including_whitespace, offset_y));

                ink_bounds = ink_bounds.union(run_bounds);

                width_including_whitespace += text_run.size().width;
            } else if let Some(drawable_text_run) = run.as_drawable() {
                // Align the bounds at the common baseline
                let offset_y = -ascent - drawable_text_run.baseline();

                ink_bounds = ink_bounds.union(Rect::from_position_size(
                    Point::new(width_including_whitespace, offset_y),
                    drawable_text_run.size(),
                ));

                width_including_whitespace += drawable_text_run.size().width;
            }
        }

        let half_line_gap = line_gap * 0.5;
        let natural_height = descent - ascent + line_gap;
        let mut baseline = -ascent + half_line_gap;
        let mut height = natural_height;

        if !line_height.is_nan() && !MathUtilities::is_zero(line_height) {
            if line_height <= natural_height {
                // Clamp to the specified line height
                height = line_height;
                baseline = -ascent;
            } else {
                // Center the text vertically within the specified line height
                height = line_height;
                let extra = line_height - (descent - ascent);
                baseline = -ascent + extra / 2.0;
            }
        }

        height += line_spacing;

        let mut width = width_including_whitespace;

        let is_rtl = self.paragraph_properties.flow_direction() == FlowDirection::RightToLeft;

        for i in 0..text_runs.len() {
            let index = if is_rtl { i } else { text_runs.len() - 1 - i };
            let current_run = &text_runs[index];

            if let Some(shaped_text) = current_run.downcast_ref::<ShapedTextRun>() {
                let glyph_run = shaped_text.glyph_run();
                let glyph_run_metrics = glyph_run.metrics();

                new_line_length += glyph_run_metrics.new_line_length;

                if glyph_run_metrics.trailing_whitespace_length == 0 {
                    break;
                }

                trailing_whitespace_length += glyph_run_metrics.trailing_whitespace_length;

                let whitespace_width = glyph_run.bounds().width - glyph_run_metrics.width;

                width -= whitespace_width;

                if glyph_run_metrics.trailing_whitespace_length != current_run.length() {
                    // This run has visible content before its own trailing whitespace, so it -
                    // not an earlier run - is the true end of the line's visible content. An
                    // earlier run's own trailing whitespace is interior to the line (followed by
                    // this run's visible content) and must not be excluded from the width too.
                    break;
                }
            }
        }

        let extent = ink_bounds.height;
        // The height of overhanging pixels at the bottom
        let overhang_after = ink_bounds.bottom() - height + half_line_gap;
        // The width of overhanging pixels at the natural alignment point. Positive value means we are inside.
        let overhang_leading = ink_bounds.left();
        // The width of overhanging pixels at the end of the natural bounds. Positive value means we are inside.
        let overhang_trailing = width_including_whitespace - ink_bounds.right();
        let has_overflowed = MathUtilities::greater_than(width, self.paragraph_width);

        let start = self.get_paragraph_offset_x(width, width_including_whitespace);

        self.ink_bounds.set(ink_bounds.translate(Vector::new(start, 0.0)));

        self.bounds.set(Rect::new(start, 0.0, width_including_whitespace, height));

        TextLineMetrics {
            has_overflowed,
            height,
            extent,
            newline_length: new_line_length,
            start,
            text_baseline: baseline,
            trailing_whitespace_length,
            width,
            width_including_trailing_whitespace: width_including_whitespace,
            overhang_leading,
            overhang_trailing,
            overhang_after,
        }
    }

    /// Gets the text line offset x.
    ///
    /// * `width` — the line width.
    /// * `width_including_trailing_whitespace` — the paragraph width including whitespace.
    fn get_paragraph_offset_x(&self, width: f64, width_including_trailing_whitespace: f64) -> f64 {
        if self.paragraph_width == f64::INFINITY {
            return 0.0;
        }

        let mut text_alignment = self.paragraph_properties.text_alignment();
        let paragraph_flow_direction = self.paragraph_properties.flow_direction();

        if text_alignment == TextAlignment::Justify {
            text_alignment = TextAlignment::Start;
        }

        match text_alignment {
            TextAlignment::Start => {
                text_alignment = if paragraph_flow_direction == FlowDirection::LeftToRight {
                    TextAlignment::Left
                } else {
                    TextAlignment::Right
                };
            }
            TextAlignment::End => {
                text_alignment = if paragraph_flow_direction == FlowDirection::RightToLeft {
                    TextAlignment::Left
                } else {
                    TextAlignment::Right
                };
            }
            TextAlignment::DetectFromContent => {
                text_alignment = if self.resolved_flow_direction == FlowDirection::LeftToRight {
                    TextAlignment::Left
                } else {
                    TextAlignment::Right
                };
            }
            _ => {}
        }

        match text_alignment {
            TextAlignment::Center => {
                let mut start = (self.paragraph_width - width) / 2.0;

                if paragraph_flow_direction == FlowDirection::RightToLeft {
                    start -= width_including_trailing_whitespace - width;
                }

                start.max(0.0)
            }
            TextAlignment::Right => (self.paragraph_width - width_including_trailing_whitespace).max(0.0),
            _ => 0.0,
        }
    }
}

/// Reference equality of two runs.
fn same_run(x: &Rc<dyn TextRun>, y: &Rc<dyn TextRun>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(x), Rc::as_ptr(y))
}

impl TextLine for TextLineImpl {
    fn text_runs(&self) -> Ref<'_, [Rc<dyn TextRun>]> {
        Ref::map(self.text_runs.borrow(), |text_runs| text_runs.as_slice())
    }

    fn first_text_source_index(&self) -> i32 {
        self.first_text_source_index
    }

    fn length(&self) -> i32 {
        self.length
    }

    fn text_line_break(&self) -> Option<Rc<TextLineBreak>> {
        self.text_line_break.borrow().clone()
    }

    fn has_collapsed(&self) -> bool {
        self.has_collapsed
    }

    fn has_overflowed(&self) -> bool {
        self.text_line_metrics.get().has_overflowed
    }

    fn baseline(&self) -> f64 {
        self.text_line_metrics.get().text_baseline
    }

    fn extent(&self) -> f64 {
        self.text_line_metrics.get().extent
    }

    fn height(&self) -> f64 {
        self.text_line_metrics.get().height
    }

    fn new_line_length(&self) -> i32 {
        self.text_line_metrics.get().newline_length
    }

    fn overhang_after(&self) -> f64 {
        self.text_line_metrics.get().overhang_after
    }

    fn overhang_leading(&self) -> f64 {
        self.text_line_metrics.get().overhang_leading
    }

    fn overhang_trailing(&self) -> f64 {
        self.text_line_metrics.get().overhang_trailing
    }

    fn trailing_whitespace_length(&self) -> i32 {
        self.text_line_metrics.get().trailing_whitespace_length
    }

    fn start(&self) -> f64 {
        self.text_line_metrics.get().start
    }

    fn width(&self) -> f64 {
        self.text_line_metrics.get().width
    }

    fn width_including_trailing_whitespace(&self) -> f64 {
        self.text_line_metrics.get().width_including_trailing_whitespace
    }

    fn draw(&self, drawing_context: &mut dyn ITextDrawingSink, line_origin: Point) {
        let mut current_x = line_origin.x + self.start();
        let current_y = line_origin.y;

        let text_runs = self.text_runs.borrow();

        for text_run in text_runs.iter() {
            if let Some(drawable_text_run) = text_run.as_drawable() {
                let offset_y = Self::get_baseline_offset(self, drawable_text_run);

                drawable_text_run.draw(drawing_context, Point::new(current_x, current_y + offset_y));

                current_x += drawable_text_run.size().width;
            }
        }
    }

    fn collapse(
        self: Rc<Self>,
        collapsing_properties_list: &[Option<Rc<dyn TextCollapsingProperties>>],
    ) -> Rc<dyn TextLine> {
        if collapsing_properties_list.is_empty() {
            return self;
        }

        let Some(collapsing_properties) = &collapsing_properties_list[0] else {
            return self;
        };

        let Some(collapsed_runs) = collapsing_properties.collapse(&*self) else {
            return self;
        };

        let has_runs = !collapsed_runs.is_empty();

        let collapsed_line = TextLineImpl::new(
            collapsed_runs,
            self.first_text_source_index,
            self.length,
            self.paragraph_width,
            self.paragraph_properties.clone(),
            self.resolved_flow_direction,
            self.text_line_break(),
            true,
        );

        if has_runs {
            collapsed_line.finalize_line();
        }

        collapsed_line
    }

    fn justify(&self, justification_properties: &dyn JustificationProperties) {
        justification_properties.justify(self);

        self.text_line_metrics.set(self.create_line_metrics());
    }

    fn get_character_hit_from_distance(&self, mut distance: f64) -> CharacterHit {
        let text_runs = self.text_runs.borrow();

        if text_runs.is_empty() {
            return CharacterHit::new(self.first_text_source_index);
        }

        distance -= self.start();

        let text_runs_length = text_runs.len();

        let mut last_index = text_runs_length as i32 - 1;

        if text_runs[last_index as usize].as_text_end_of_line().is_some() {
            last_index -= 1;
        }

        if last_index < 0 {
            return CharacterHit::new(self.first_text_source_index);
        }

        if distance <= 0.0 {
            return Self::get_run_character_hit(&*text_runs[0], self.get_run_text_source_position(0), 0.0);
        }

        if distance >= self.width_including_trailing_whitespace() {
            return Self::get_run_character_hit(
                &*text_runs[last_index as usize],
                self.get_run_text_source_position(last_index as usize),
                distance,
            );
        }

        // process hit that happens within the line
        let mut character_hit = CharacterHit::default();
        let mut current_distance = 0.0;

        let mut i = 0usize;

        while i as i32 <= last_index {
            let mut current_run = &text_runs[i];
            let mut current_visual_index = i;

            if current_run
                .downcast_ref::<ShapedTextRun>()
                .is_some_and(|shaped_run| !shaped_run.shaped_buffer().is_left_to_right())
            {
                let mut right_to_left_index = i;

                while right_to_left_index + 1 <= text_runs_length - 1 {
                    right_to_left_index += 1;

                    let next_shaped = text_runs[right_to_left_index].downcast_ref::<ShapedTextRun>();

                    if next_shaped.is_none_or(|next_shaped| next_shaped.shaped_buffer().is_left_to_right()) {
                        break;
                    }

                    right_to_left_index += 1;
                }

                let mut j = i;

                // The loop condition is upstream's (it tests `i`, not `j`); the loop
                // ends through the bounds check and the return below.
                while i <= right_to_left_index {
                    if j > text_runs_length - 1 {
                        break;
                    }

                    current_run = &text_runs[j];
                    current_visual_index = j;

                    let Some(shaped_run) = current_run.downcast_ref::<ShapedTextRun>() else {
                        j += 1;

                        continue;
                    };

                    let shaped_run_width = shaped_run.size().width;

                    if current_distance + shaped_run_width <= distance {
                        current_distance += shaped_run_width;

                        j += 1;

                        continue;
                    }

                    return Self::get_run_character_hit(
                        &**current_run,
                        self.get_run_text_source_position(j),
                        distance - current_distance,
                    );
                }
            }

            character_hit = Self::get_run_character_hit(
                &**current_run,
                self.get_run_text_source_position(current_visual_index),
                distance - current_distance,
            );

            if let Some(drawable_text_run) = current_run.as_drawable() {
                if i < text_runs_length - 1 && current_distance + drawable_text_run.size().width < distance {
                    current_distance += drawable_text_run.size().width;

                    i += 1;

                    continue;
                }
            } else {
                i += 1;

                continue;
            }

            break;
        }

        character_hit
    }

    fn get_distance_from_character_hit(&self, character_hit: CharacterHit) -> f64 {
        let indexed_text_runs = match self.indexed_text_runs() {
            Some(indexed_text_runs) if !indexed_text_runs.is_empty() => indexed_text_runs,
            _ => return self.start(),
        };

        let line_end = self.first_text_source_index + self.length;

        let character_index = (character_hit.first_character_index() + character_hit.trailing_length()).min(line_end);

        let mut current_position = self.first_text_source_index;

        let mut current_text_run: Option<Rc<dyn TextRun>> = None;

        let mut indexed_run_index = Self::find_indexed_run(&indexed_text_runs, current_position);

        while current_position < line_end {
            let current_indexed_run = &indexed_text_runs[indexed_run_index];

            current_text_run = current_indexed_run.text_run.borrow().clone();

            let Some(text_run) = &current_text_run else {
                break;
            };

            if current_indexed_run.text_source_character_index + text_run.length() <= character_hit.first_character_index()
                && current_position + text_run.length() < line_end
            {
                current_position += text_run.length();

                indexed_run_index = Self::find_indexed_run(&indexed_text_runs, current_position);

                continue;
            }

            break;
        }

        let Some(current_text_run) = current_text_run else {
            return self.start();
        };

        let current_indexed_run = &indexed_text_runs[indexed_run_index];

        let mut directional_width = 0.0;
        let first_run_index = current_indexed_run.run_index.get();

        let current_direction = Self::get_run_direction(Some(&*current_text_run), self.resolved_flow_direction);

        let current_x = self.start() + self.get_preceding_distance(first_run_index);

        if let Some(current_drawable) = current_text_run.as_drawable() {
            directional_width = current_drawable.size().width;
        }

        let last_run_index = Self::get_last_directional_run_index(
            &indexed_text_runs,
            indexed_run_index,
            current_direction,
            &mut directional_width,
        );

        match current_direction {
            FlowDirection::RightToLeft => self
                .get_text_run_bounds_right_to_left(
                    first_run_index,
                    last_run_index,
                    current_x + directional_width,
                    character_index,
                    current_position,
                    1,
                )
                .0
                .rectangle()
                .right(),
            FlowDirection::LeftToRight => self
                .get_text_bounds_left_to_right(
                    first_run_index,
                    last_run_index,
                    current_x,
                    character_index,
                    current_position,
                    1,
                )
                .0
                .rectangle()
                .left(),
        }
    }

    fn get_next_caret_character_hit(&self, character_hit: CharacterHit) -> CharacterHit {
        if self.text_runs.borrow().is_empty() || self.indexed_text_runs.borrow().is_none() {
            return CharacterHit::default();
        }

        let mut current_character_hit = character_hit;
        let character_index = character_hit.first_character_index() + character_hit.trailing_length();

        let (current_run, current_position) =
            self.get_run_at_character_index(character_index, LogicalDirection::Forward);

        let mut next_character_hit = character_hit;

        if let Some(current_run) = &current_run {
            if let Some(shaped_run) = current_run.downcast_ref::<ShapedTextRun>() {
                let glyph_run = shaped_run.glyph_run();

                let offset = 0.max(current_position - glyph_run.metrics().first_cluster);

                if character_hit.first_character_index() < current_position && offset > 0 {
                    // Crossing from a previous run: find the nearest character hit at the first cluster
                    let (nearest, _) = glyph_run.find_nearest_character_hit(glyph_run.metrics().first_cluster);

                    next_character_hit = CharacterHit::with_trailing_length(
                        nearest.first_character_index() + offset,
                        nearest.trailing_length(),
                    );
                } else {
                    if offset > 0 {
                        current_character_hit = CharacterHit::with_trailing_length(
                            0.max(character_hit.first_character_index() - offset),
                            character_hit.trailing_length(),
                        );
                    }

                    next_character_hit = glyph_run.get_next_caret_character_hit(current_character_hit);

                    if offset > 0 {
                        next_character_hit = CharacterHit::with_trailing_length(
                            next_character_hit.first_character_index() + offset,
                            next_character_hit.trailing_length(),
                        );
                    }
                }
            } else {
                next_character_hit = CharacterHit::new(current_position + current_run.length());
            }
        }

        if character_index == next_character_hit.first_character_index() + next_character_hit.trailing_length() {
            return character_hit;
        }

        next_character_hit
    }

    fn get_previous_caret_character_hit(&self, character_hit: CharacterHit) -> CharacterHit {
        self.get_previous_character_hit(character_hit, false)
    }

    fn get_backspace_caret_character_hit(&self, character_hit: CharacterHit) -> CharacterHit {
        self.get_previous_character_hit(character_hit, true)
    }

    /// Panics when `text_length` is zero.
    fn get_text_bounds(&self, first_text_source_index: i32, text_length: i32) -> Vec<TextBounds> {
        if text_length == 0 {
            panic!("textLength ('0') must be a non-zero value. (Parameter 'textLength')");
        }

        let indexed_text_runs = match self.indexed_text_runs() {
            Some(indexed_text_runs) if !indexed_text_runs.is_empty() => indexed_text_runs,
            _ => return Vec::new(),
        };

        let line_end = self.first_text_source_index + self.length;

        let mut current_position = self.first_text_source_index;
        let mut remaining_length = text_length;

        // We can return early if the requested text range is before the line's text range.
        if first_text_source_index + text_length < self.first_text_source_index {
            let indexed_text_run = &indexed_text_runs[0];
            let current_direction =
                Self::get_run_direction(indexed_text_run.text_run.borrow().as_deref(), self.resolved_flow_direction);

            return vec![TextBounds::new(Rect::new(0.0, 0.0, 0.0, self.height()), current_direction, Vec::new())];
        }

        // We can return early if the requested text range is after the line's text range.
        if first_text_source_index >= line_end {
            let indexed_text_run = &indexed_text_runs[indexed_text_runs.len() - 1];
            let current_direction =
                Self::get_run_direction(indexed_text_run.text_run.borrow().as_deref(), self.resolved_flow_direction);

            return vec![TextBounds::new(
                Rect::new(self.width_including_trailing_whitespace(), 0.0, 0.0, self.height()),
                current_direction,
                Vec::new(),
            )];
        }

        let mut result: Vec<TextBounds> = Vec::new();

        while remaining_length > 0 && current_position < line_end {
            let indexed_run_index = Self::find_indexed_run(&indexed_text_runs, current_position);

            let current_indexed_run = &indexed_text_runs[indexed_run_index];

            let Some(current_text_run) = current_indexed_run.text_run.borrow().clone() else {
                break;
            };

            let current_direction = Self::get_run_direction(Some(&*current_text_run), self.resolved_flow_direction);

            if current_indexed_run.text_source_character_index + current_text_run.length() <= first_text_source_index {
                current_position += current_text_run.length();

                continue;
            }

            let current_x = self.start() + self.get_preceding_distance(current_indexed_run.run_index.get());
            let mut directional_width = 0.0;

            if let Some(current_drawable) = current_text_run.as_drawable() {
                directional_width = current_drawable.size().width;
            }

            let first_run_index = current_indexed_run.run_index.get();
            let last_run_index = Self::get_last_directional_run_index(
                &indexed_text_runs,
                indexed_run_index,
                current_direction,
                &mut directional_width,
            );

            let (current_bounds, covered_length, new_position) = match current_direction {
                FlowDirection::RightToLeft => self.get_text_run_bounds_right_to_left(
                    first_run_index,
                    last_run_index,
                    current_x + directional_width,
                    first_text_source_index,
                    current_position,
                    remaining_length,
                ),
                FlowDirection::LeftToRight => self.get_text_bounds_left_to_right(
                    first_run_index,
                    last_run_index,
                    current_x,
                    first_text_source_index,
                    current_position,
                    remaining_length,
                ),
            };

            current_position = new_position;

            let unmerged_bounds = match result.last_mut() {
                Some(last_bounds) => Self::try_merge_with_last_bounds(current_bounds, last_bounds),
                None => Some(current_bounds),
            };

            if let Some(current_bounds) = unmerged_bounds {
                result.push(current_bounds);
            }

            if covered_length <= 0 {
                panic!("Covered length must be greater than zero.");
            }

            remaining_length -= covered_length;
        }

        result.sort_by(Self::text_bounds_comparer);

        result
    }

    fn dispose(&self) {
        for text_run in self.text_runs.borrow().iter() {
            if let Some(shaped_text_run) = text_run.downcast_ref::<ShapedTextRun>() {
                shaped_text_run.dispose();
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
