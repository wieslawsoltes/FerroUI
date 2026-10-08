use std::cell::RefCell;
use std::rc::Rc;

use crate::media::text_formatting::formatting_object_pool::{FormattingObjectPool, RentedList};
use crate::media::text_formatting::text_line_impl::TextLineImpl;
use crate::media::text_formatting::text_run_cache::CachedShapingResult;
use crate::media::text_formatting::unicode::{
    BidiAlgorithm, BidiData, Codepoint, GraphemeEnumerator, LineBreak, LineBreakEnumerator,
};
use crate::media::text_formatting::wrapping_text_line_break::WrappingTextLineBreak;
use crate::media::text_formatting::{
    GlyphInfo, ITextSource, ShapedBuffer, ShapedTextRun, SplitResult, TextCharacters,
    TextFormatter, TextLine, TextLineBreak, TextParagraphProperties, TextRun, TextRunCache, TextRunProperties,
    TextShaper, TextShaperOptions, UnshapedTextRun, DEFAULT_TEXT_SOURCE_LENGTH,
};
use crate::media::{FlowDirection, FontManager, TextWrapping};
use crate::utilities::{MathUtilities, ReadOnlyMemory};

const DEFAULT_TEXT: [u16; DEFAULT_TEXT_SOURCE_LENGTH as usize] = [b'a' as u16; DEFAULT_TEXT_SOURCE_LENGTH as usize];

thread_local! {
    static EMPTY: ReadOnlyMemory<u16> = ReadOnlyMemory::from_vec(vec![b' ' as u16]);

    /// The bidi scratch state of the thread; reused by every formatted line.
    static BIDI: RefCell<Option<Box<(BidiData, BidiAlgorithm)>>> = const { RefCell::new(None) };
}

/// The text formatter.
///
/// Internal upstream; public so that the Skia unit tests reach it.
pub struct TextFormatterImpl;

impl TextFormatterImpl {
    pub fn new() -> Self {
        Self
    }

    /// Formats a line from cached shaped runs, skipping shaping and bidi processing.
    fn format_line_from_cache(
        cached: &CachedShapingResult,
        first_text_source_index: i32,
        paragraph_width: f64,
        paragraph_properties: &Rc<dyn TextParagraphProperties>,
        object_pool: &FormattingObjectPool,
    ) -> Rc<TextLineImpl> {
        let resolved_flow_direction = cached.resolved_flow_direction;

        let next_line_break = cached
            .text_end_of_line
            .as_ref()
            .map(|text_end_of_line| Rc::new(TextLineBreak::new(Some(text_end_of_line.clone()), resolved_flow_direction, false)));

        match paragraph_properties.text_wrapping() {
            TextWrapping::NoWrap => {
                let line_runs = Self::add_ref_shaped_runs(&cached.shaped_runs);

                let text_line = TextLineImpl::new(
                    line_runs,
                    first_text_source_index,
                    cached.text_source_length,
                    paragraph_width,
                    paragraph_properties.clone(),
                    resolved_flow_direction,
                    next_line_break,
                    false,
                );

                text_line.finalize_line();

                text_line
            }
            TextWrapping::WrapWithOverflow | TextWrapping::Wrap => {
                let mut runs = Self::add_ref_shaped_runs(&cached.shaped_runs);

                Self::perform_text_wrapping(
                    &mut runs,
                    false,
                    first_text_source_index,
                    paragraph_width,
                    paragraph_properties,
                    resolved_flow_direction,
                    next_line_break.as_ref(),
                    object_pool,
                )
            }
        }
    }

    /// Produces a list of text runs for a line, adding an extra reference to each
    /// [`ShapedTextRun`] so that the caller owns a disposable reference.
    fn add_ref_shaped_runs(runs: &[Rc<dyn TextRun>]) -> Vec<Rc<dyn TextRun>> {
        let mut result = Vec::with_capacity(runs.len());

        for run in runs {
            if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                shaped.add_reference();
            }

            result.push(run.clone());
        }

        result
    }

    /// Split a sequence of runs into two segments at specified length.
    ///
    /// Both lists of the result are rented from `object_pool` and have to be
    /// returned to it.
    pub(crate) fn split_text_runs(
        text_runs: &[Rc<dyn TextRun>],
        length: i32,
        object_pool: &FormattingObjectPool,
    ) -> SplitResult<RentedList<Rc<dyn TextRun>>> {
        Self::split_text_runs_with_length(text_runs, length, object_pool).0
    }

    /// Split a sequence of runs into two segments at specified length. The actual
    /// length of the first segment (which may differ from `length`
    /// when the split lands on a cluster boundary) is returned as the second
    /// value. This lets the wrap caller avoid a separate
    /// second pass to sum run lengths.
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn split_text_runs_with_length(
        text_runs: &[Rc<dyn TextRun>],
        length: i32,
        object_pool: &FormattingObjectPool,
    ) -> (SplitResult<RentedList<Rc<dyn TextRun>>>, i32) {
        if length == 0 {
            let mut second = object_pool.text_run_lists.rent();

            second.extend(text_runs.iter().cloned());

            return (SplitResult::new(None, Some(second)), 0);
        }

        let mut first = object_pool.text_run_lists.rent();
        let mut current_length = 0;

        for i in 0..text_runs.len() {
            let current_run = &text_runs[i];
            let current_run_length = current_run.length();

            if current_length + current_run_length < length {
                current_length += current_run_length;

                continue;
            }

            let first_count = if current_run_length >= 1 { i + 1 } else { i };

            if first_count > 1 {
                first.extend(text_runs[..i].iter().cloned());
            }

            let mut second_count = text_runs.len() - first_count;

            if current_length + current_run_length == length {
                let second = if second_count > 0 {
                    let mut second = object_pool.text_run_lists.rent();

                    let offset = if current_run_length >= 1 { 1 } else { 0 };

                    for j in 0..second_count {
                        second.push(text_runs[i + j + offset].clone());
                    }

                    Some(second)
                } else {
                    None
                };

                first.push(current_run.clone());

                return (SplitResult::new(Some(first), second), current_length + current_run_length);
            } else {
                second_count += 1;

                let mut second = object_pool.text_run_lists.rent();
                let mut added_first_length = 0;
                let trailing_loop_start;

                if let Some(shaped_text_characters) = current_run.downcast_ref::<ShapedTextRun>() {
                    let split = shaped_text_characters.split(length - current_length);

                    if let Some(split_first) = split.first {
                        added_first_length = split_first.length();
                        first.push(split_first);
                    }

                    if let Some(split_second) = split.second {
                        second.push(split_second);
                    }

                    // The split produced fresh ShapedTextRuns for each half, so the
                    // caller's reference to the original is no longer needed — release it.
                    shaped_text_characters.dispose();

                    // current_run is consumed by the split; the trailing loop adds the
                    // runs *after* it.
                    trailing_loop_start = 1;
                } else if current_length == 0 {
                    // Non-splittable run at the very start of the list, asked to split
                    // strictly inside it. Snapping before would leave first empty and
                    // the wrap caller would loop forever — same situation as the wrap
                    // algorithm's "include at least one cluster" overflow rule. Place
                    // current_run in first and let the line overflow.
                    first.push(current_run.clone());
                    added_first_length = current_run_length;
                    trailing_loop_start = 1;
                } else {
                    // Non-splittable run at the split point. Snap the boundary BEFORE
                    // it: current_run goes into second along with the remaining runs,
                    // first ends at current_length (shorter than requested but
                    // content-preserving). Without this branch the run would be
                    // dropped from both halves.
                    trailing_loop_start = 0;
                }

                for j in trailing_loop_start..second_count {
                    second.push(text_runs[i + j].clone());
                }

                return (SplitResult::new(Some(first), Some(second)), current_length + added_first_length);
            }
        }

        first.extend(text_runs.iter().cloned());

        (SplitResult::new(Some(first), None), current_length)
    }

    /// Shape specified text runs with specified paragraph embedding.
    ///
    /// Returns a rented list of shaped text characters and the resolved flow
    /// direction.
    fn shape_text_runs(
        text_runs: &[Rc<dyn TextRun>],
        paragraph_properties: &dyn TextParagraphProperties,
        object_pool: &FormattingObjectPool,
        font_manager: &FontManager,
    ) -> (RentedList<Rc<dyn TextRun>>, FlowDirection) {
        let flow_direction = paragraph_properties.flow_direction();
        let mut shaped_runs = object_pool.text_run_lists.rent();

        if text_runs.is_empty() {
            return (shaped_runs, flow_direction);
        }

        // The scratch state is taken out of its slot while it is used, so no
        // borrow is held across the calls into fonts, runs and the shaper.
        let mut bidi = BIDI
            .with(|slot| slot.borrow_mut().take())
            .unwrap_or_else(|| Box::new((BidiData::new(), BidiAlgorithm::new())));

        let (bidi_data, bidi_algorithm) = &mut *bidi;

        bidi_data.reset();
        bidi_data.set_paragraph_embedding_level(flow_direction as i8);

        for text_run in text_runs {
            let text = text_run.text_span();

            if !text.is_empty() {
                bidi_data.append(text);
            } else if text_run.length() == DEFAULT_TEXT_SOURCE_LENGTH {
                bidi_data.append(&DEFAULT_TEXT);
            } else {
                bidi_data.append(&vec![b'a' as u16; text_run.length().max(0) as usize]);
            }
        }

        bidi_algorithm.process(bidi_data);

        let resolved_embedding_level = bidi_algorithm.resolve_embedding_level(bidi_data.classes());

        let resolved_flow_direction =
            if (resolved_embedding_level & 1) == 0 { FlowDirection::LeftToRight } else { FlowDirection::RightToLeft };

        let mut processed_runs = object_pool.text_run_lists.rent();
        let mut grouped_runs = object_pool.unshaped_text_run_lists.rent();

        Self::coalesce_levels(text_runs, bidi_algorithm.resolved_levels(), font_manager, &mut processed_runs);

        bidi_data.reset();
        bidi_algorithm.reset();

        BIDI.with(|slot| *slot.borrow_mut() = Some(bidi));

        let text_shaper = TextShaper::current();

        let mut index = 0;

        while index < processed_runs.len() {
            let current_run = &processed_runs[index];

            match current_run.clone().downcast_rc::<UnshapedTextRun>() {
                Some(mut shapeable_run) => {
                    grouped_runs.clear();
                    grouped_runs.push(shapeable_run.clone());

                    let mut text = shapeable_run.text();
                    let properties = shapeable_run.run_properties().clone();

                    while index + 1 < processed_runs.len() {
                        let Some(next_run) = processed_runs[index + 1].clone().downcast_rc::<UnshapedTextRun>() else {
                            break;
                        };

                        if shapeable_run.bidi_level() == next_run.bidi_level() {
                            if let Some(joined_text) = Self::try_join_contiguous_memories(&text, &next_run.text()) {
                                if Self::can_shape_together(&*properties, &**next_run.run_properties()) {
                                    grouped_runs.push(next_run.clone());
                                    index += 1;
                                    shapeable_run = next_run;
                                    text = joined_text;
                                    continue;
                                }
                            }
                        }

                        break;
                    }

                    let shaper_options = TextShaperOptions::with_all(
                        properties.cached_glyph_typeface(),
                        properties.font_rendering_em_size(),
                        shapeable_run.bidi_level(),
                        properties.culture_info().cloned(),
                        paragraph_properties.default_incremental_tab(),
                        paragraph_properties.letter_spacing(),
                        properties.font_features().map(|features| Rc::new(features.to_vec())),
                    );

                    Self::shape_together(&grouped_runs, &text, &shaper_options, &text_shaper, &mut shaped_runs);
                }
                None => {
                    shaped_runs.push(current_run.clone());
                }
            }

            index += 1;
        }

        object_pool.text_run_lists.return_list(processed_runs);
        object_pool.unshaped_text_run_lists.return_list(grouped_runs);

        (shaped_runs, resolved_flow_direction)
    }

    /// Tries to join two potentially contiguous memory regions.
    ///
    /// Returns a memory region representing the union of the two regions when
    /// they are contiguous parts of the same buffer.
    fn try_join_contiguous_memories(x: &ReadOnlyMemory<u16>, y: &ReadOnlyMemory<u16>) -> Option<ReadOnlyMemory<u16>> {
        if !x.shares_owner_with(y) {
            return None;
        }

        let owner = x.owner()?;

        let x_range = (x.offset_in_owner(), x.len());
        let y_range = (y.offset_in_owner(), y.len());

        let (first_range, second_range) = if x_range.0 <= y_range.0 { (x_range, y_range) } else { (y_range, x_range) };

        if first_range.0 + first_range.1 == second_range.0 {
            return Some(ReadOnlyMemory::with_range(owner.clone(), first_range.0, x_range.1 + y_range.1));
        }

        None
    }

    fn can_shape_together(x: &dyn TextRunProperties, y: &dyn TextRunProperties) -> bool {
        MathUtilities::are_close(x.font_rendering_em_size(), y.font_rendering_em_size())
            && x.typeface() == y.typeface()
            && x.baseline_alignment() == y.baseline_alignment()
    }

    fn shape_together(
        text_runs: &[Rc<UnshapedTextRun>],
        text: &ReadOnlyMemory<u16>,
        options: &TextShaperOptions,
        text_shaper: &TextShaper,
        results: &mut Vec<Rc<dyn TextRun>>,
    ) {
        let mut shaped_buffer = text_shaper.shape_text(text, options);

        let mut previous_length = 0;

        for current_run in text_runs {
            let split_result = shaped_buffer.split(previous_length + current_run.length());

            // Split by text, not by glyph count: a run can legitimately shape to no glyphs at
            // all and still own its characters. Shapers drop default ignorables that the font
            // cannot hide behind a space glyph, so a run holding nothing but a line break can
            // come back empty. Skipping it there would delete its characters from the line and
            // leave the caller stuck at the same text position.
            match split_result.first {
                Some(first) if !first.text().is_empty() => {
                    previous_length = 0;

                    results.push(Rc::new(ShapedTextRun::new(first, current_run.run_properties().clone())));
                }
                _ => {
                    previous_length += current_run.length();
                }
            }

            match split_result.second {
                None => return,
                Some(second) => shaped_buffer = second,
            }
        }
    }

    /// Coalesces ranges of the same bidi level to form [`UnshapedTextRun`]s.
    ///
    /// * `text_characters` — the text characters to form unshaped runs from.
    /// * `levels` — the bidi levels (one per codepoint).
    /// * `font_manager` — the font manager to use.
    /// * `processed_runs` — a list that will be filled with the processed runs.
    fn coalesce_levels(
        text_characters: &[Rc<dyn TextRun>],
        levels: &[i8],
        font_manager: &FontManager,
        processed_runs: &mut Vec<Rc<dyn TextRun>>,
    ) {
        if levels.is_empty() {
            return;
        }

        let mut level_index = 0usize;
        let mut run_level = levels[0];

        let mut previous_properties: Option<Rc<dyn TextRunProperties>> = None;
        let mut current_run: Option<&TextCharacters> = None;
        let mut run_text: ReadOnlyMemory<u16> = ReadOnlyMemory::empty();

        for text_run in text_characters {
            let mut j = 0usize;
            current_run = text_run.downcast_ref::<TextCharacters>();

            let Some(current_run) = current_run else {
                processed_runs.push(text_run.clone());

                level_index += text_run.length().max(0) as usize;

                continue;
            };

            run_text = current_run.text();

            while j < run_text.len() {
                let (_, count) = Codepoint::read_at(run_text.span(), j);

                if level_index + 1 == levels.len() {
                    break;
                }

                level_index += 1;
                j += count;

                if j == run_text.len() {
                    current_run.get_shapeable_characters(
                        run_text.slice(0, j),
                        run_level,
                        font_manager,
                        &mut previous_properties,
                        processed_runs,
                    );

                    run_level = levels[level_index];

                    continue;
                }

                if levels[level_index] == run_level {
                    continue;
                }

                // End of this run
                current_run.get_shapeable_characters(
                    run_text.slice(0, j),
                    run_level,
                    font_manager,
                    &mut previous_properties,
                    processed_runs,
                );

                run_text = run_text.slice_from(j);

                j = 0;

                // Move to next run
                run_level = levels[level_index];
            }
        }

        let Some(current_run) = current_run else {
            return;
        };

        if run_text.is_empty() {
            return;
        }

        current_run.get_shapeable_characters(run_text, run_level, font_manager, &mut previous_properties, processed_runs);
    }

    /// Fetches text runs.
    ///
    /// Returns the (rented) formatted text runs, the end of line, if any, and
    /// the processed text source length.
    fn fetch_text_runs(
        text_source: &dyn ITextSource,
        first_text_source_index: i32,
        object_pool: &FormattingObjectPool,
    ) -> (RentedList<Rc<dyn TextRun>>, Option<Rc<dyn TextRun>>, i32) {
        let mut text_source_length = 0;

        let mut end_of_line = None;

        let mut text_runs = object_pool.text_run_lists.rent();

        let mut text_run_enumerator = TextRunEnumerator::new(text_source, first_text_source_index);

        while let Some(text_run) = text_run_enumerator.move_next() {
            if let Some(text_end_of_line) = text_run.as_text_end_of_line() {
                text_source_length += text_end_of_line.length();

                end_of_line = Some(text_run.clone());

                text_runs.push(text_run);

                break;
            }

            if let Some(text_characters) = text_run.downcast_ref::<TextCharacters>() {
                if let Some(run_line_break) = Self::try_get_line_break(text_characters) {
                    let split_result = TextCharacters::new(
                        text_characters.text().slice(0, run_line_break.position_wrap()),
                        text_characters.run_properties().clone(),
                    );

                    text_runs.push(Rc::new(split_result));

                    text_source_length += run_line_break.position_wrap() as i32;

                    return (text_runs, end_of_line, text_source_length);
                }
            }

            text_source_length += text_run.length();

            text_runs.push(text_run);
        }

        (text_runs, end_of_line, text_source_length)
    }

    fn try_get_line_break(text_run: &dyn TextRun) -> Option<LineBreak> {
        let text = text_run.text_span();

        if text.is_empty() {
            return None;
        }

        let mut line_break_enumerator = LineBreakEnumerator::new(text);

        while let Some(line_break) = line_break_enumerator.move_next() {
            if !line_break.required() {
                continue;
            }

            return Some(line_break);
        }

        None
    }

    fn measure_length(text_runs: &[Rc<dyn TextRun>], paragraph_width: f64) -> i32 {
        let mut measured_length = 0;
        let mut current_width = 0.0;

        for (run_index, current_run) in text_runs.iter().enumerate() {
            if let Some(shaped_text_characters) = current_run.downcast_ref::<ShapedTextRun>() {
                // cluster-width prefix sum lets us answer "how much fits"
                // in O(log clusters) instead of walking every glyph. The total
                // advance and the per-cluster start char are cached on the
                // ShapedBuffer (which lives in the run cache), so the first
                // layout pays the O(glyphs) cost and every subsequent layout
                // is constant-time.
                let buffer = shaped_text_characters.shaped_buffer();

                if buffer.length() == 0 {
                    continue;
                }

                let remaining = paragraph_width - current_width;
                let buffer_width = buffer.total_glyph_advance();

                if !MathUtilities::greater_than(buffer_width, remaining) {
                    // Whole buffer fits: consume it and continue to the next run.
                    current_width += buffer_width;
                    measured_length += current_run.length();
                    continue;
                }

                // Some part of the buffer overflows: find the cluster boundary.
                let mut run_length = buffer.find_leading_char_count_within_width(remaining);

                // "Include at least one cluster" rule preserves the existing
                // contract that the caller always advances by at least one
                // grapheme even when the first cluster overflows the line.
                if run_length == 0 && measured_length == 0 {
                    run_length = buffer.first_cluster_char_length();
                }

                measured_length += run_length;

                if run_index < text_runs.len() - 1 && run_length == current_run.length() {
                    if let Some(end_of_line) = text_runs[run_index + 1].as_text_end_of_line() {
                        measured_length += end_of_line.length();
                    }
                }

                return measured_length;
            } else if let Some(drawable_text_run) = current_run.as_drawable() {
                if MathUtilities::greater_than(current_width + drawable_text_run.size().width, paragraph_width) {
                    return measured_length;
                }

                measured_length += current_run.length();
                current_width += drawable_text_run.size().width;
            } else {
                measured_length += current_run.length();
            }
        }

        measured_length
    }

    /// Creates an empty text line.
    pub fn create_empty_text_line(
        first_text_source_index: i32,
        paragraph_width: f64,
        paragraph_properties: &Rc<dyn TextParagraphProperties>,
    ) -> Rc<TextLineImpl> {
        let flow_direction = paragraph_properties.flow_direction();
        let properties = paragraph_properties.default_text_run_properties();
        let glyph_typeface = properties.cached_glyph_typeface();
        let glyph = glyph_typeface.character_to_glyph_map().get_glyph(b' ' as i32);

        let shaped_buffer = ShapedBuffer::new(
            EMPTY.with(|empty| empty.clone()),
            1,
            glyph_typeface,
            properties.font_rendering_em_size(),
            flow_direction as i8,
        );
        shaped_buffer.set(0, GlyphInfo::new(glyph, first_text_source_index, 0.0));

        let text_runs: Vec<Rc<dyn TextRun>> = vec![Rc::new(ShapedTextRun::new(shaped_buffer, properties.clone()))];

        let line = TextLineImpl::new(
            text_runs,
            first_text_source_index,
            0,
            paragraph_width,
            paragraph_properties.clone(),
            flow_direction,
            None,
            false,
        );

        line.finalize_line();

        line
    }

    /// Performs text wrapping and returns the wrapped text line.
    ///
    /// * `text_runs` — the text runs to wrap.
    /// * `can_reuse_text_run_list` — whether `text_runs` can be reused to store the split runs.
    /// * `first_text_source_index` — the first text source index.
    /// * `paragraph_width` — the paragraph width.
    /// * `paragraph_properties` — the text paragraph properties.
    /// * `current_line_break` — the current line break if the line was explicitly broken.
    /// * `object_pool` — a pool used to get reusable formatting objects.
    #[allow(clippy::too_many_arguments)]
    fn perform_text_wrapping(
        text_runs: &mut Vec<Rc<dyn TextRun>>,
        can_reuse_text_run_list: bool,
        first_text_source_index: i32,
        paragraph_width: f64,
        paragraph_properties: &Rc<dyn TextParagraphProperties>,
        resolved_flow_direction: FlowDirection,
        current_line_break: Option<&Rc<TextLineBreak>>,
        object_pool: &FormattingObjectPool,
    ) -> Rc<TextLineImpl> {
        if text_runs.is_empty() {
            return Self::create_empty_text_line(first_text_source_index, paragraph_width, paragraph_properties);
        }

        let measured_length = Self::find_wrap_length(text_runs, paragraph_width, &**paragraph_properties);

        let (split, split_length) = Self::split_text_runs_with_length(text_runs, measured_length, object_pool);

        let (mut pre_split_runs, mut post_split_runs) = split.deconstruct();

        let has_post_split_runs = post_split_runs.as_ref().is_some_and(|runs| !runs.is_empty());

        let text_line_break = match &mut post_split_runs {
            Some(post_split_runs) if has_post_split_runs => {
                let post_split_count = post_split_runs.len();

                // reuse the list as much as possible:
                // if can_reuse_text_run_list == true it's coming from previous remaining runs
                let mut remaining_runs = if can_reuse_text_run_list {
                    let mut remaining_runs = std::mem::take(text_runs);
                    remaining_runs.clear();
                    // ensure capacity up front so pushing does not resize mid-loop.
                    remaining_runs.reserve(post_split_count);
                    remaining_runs
                } else {
                    Vec::with_capacity(post_split_count)
                };

                // The runs are moved out of the rented list, which is returned below.
                remaining_runs.append(post_split_runs);

                Some(Rc::new(WrappingTextLineBreak::new(None, resolved_flow_direction, remaining_runs)))
            }
            _ => current_line_break.and_then(|line_break| line_break.text_end_of_line()).map(|text_end_of_line| {
                Rc::new(TextLineBreak::new(Some(text_end_of_line.clone()), resolved_flow_direction, false))
            }),
        };

        let text_line = match &mut pre_split_runs {
            None => Self::create_empty_text_line(first_text_source_index, paragraph_width, paragraph_properties),
            Some(pre_split_runs) => {
                if has_post_split_runs {
                    Self::reset_trailing_whitespace_bidi_levels(
                        pre_split_runs,
                        paragraph_properties.flow_direction(),
                        object_pool,
                    );
                }

                // split_text_runs has already computed the actual length of the first
                // segment (the cluster boundary may land slightly off the requested
                // length), so we just need to materialise the run list for the line:
                // no second-pass length sum required.
                let remaining_text_runs: Vec<Rc<dyn TextRun>> = pre_split_runs.drain(..).collect();

                let text_line = TextLineImpl::new(
                    remaining_text_runs,
                    first_text_source_index,
                    split_length,
                    paragraph_width,
                    paragraph_properties.clone(),
                    resolved_flow_direction,
                    text_line_break,
                    false,
                );

                text_line.finalize_line();

                text_line
            }
        };

        object_pool.text_run_lists.return_optional(pre_split_runs);
        object_pool.text_run_lists.return_optional(post_split_runs);

        text_line
    }

    /// The first part of upstream's `PerformTextWrapping`: finds the length
    /// (in text source positions) of the wrapped line.
    fn find_wrap_length(
        text_runs: &[Rc<dyn TextRun>],
        paragraph_width: f64,
        paragraph_properties: &dyn TextParagraphProperties,
    ) -> i32 {
        let mut measured_length = Self::measure_length(text_runs, paragraph_width);

        if measured_length == 0 {
            if paragraph_properties.text_wrapping() == TextWrapping::NoWrap {
                for text_run in text_runs {
                    measured_length += text_run.length();
                }
            } else {
                let first_run = &text_runs[0];

                if first_run.is::<ShapedTextRun>() {
                    let mut grapheme_enumerator = GraphemeEnumerator::new(first_run.text_span());

                    measured_length = match grapheme_enumerator.move_next() {
                        Some(grapheme) => grapheme.length() as i32,
                        None => 1,
                    };
                } else {
                    measured_length = first_run.length();
                }
            }
        }

        let mut current_length = 0;

        let mut last_wrap_position = 0;

        let mut current_position = 0;

        let wrapping_mode = paragraph_properties.text_wrapping();
        let run_count = text_runs.len();

        let mut index = 0;

        while index < run_count {
            let mut break_found = false;

            let mut current_run = &text_runs[index];
            let mut current_run_length = current_run.length();

            if current_run.is::<ShapedTextRun>() {
                let mut line_breaker = LineBreakEnumerator::new(current_run.text_span());

                while let Some(mut line_break) = line_breaker.move_next() {
                    if line_break.required() && current_length + line_break.position_measure() as i32 <= measured_length {
                        // Explicit break found
                        break_found = true;

                        current_position = current_length + line_break.position_wrap() as i32;

                        break;
                    }

                    if current_length + line_break.position_measure() as i32 > measured_length {
                        if wrapping_mode == TextWrapping::WrapWithOverflow {
                            if last_wrap_position > 0 {
                                current_position = last_wrap_position;

                                break_found = true;

                                break;
                            }

                            // Find next possible wrap position (overflow)
                            if index < run_count - 1 {
                                if line_break.position_wrap() as i32 != current_run_length {
                                    // We already found the next possible wrap position.
                                    break_found = true;

                                    current_position = current_length + line_break.position_wrap() as i32;

                                    break;
                                }

                                while let Some(next_line_break) = line_breaker.move_next() {
                                    line_break = next_line_break;

                                    current_position += line_break.position_wrap() as i32;

                                    if line_break.position_wrap() as i32 != current_run_length {
                                        break;
                                    }

                                    index += 1;

                                    if index >= run_count {
                                        break;
                                    }

                                    current_run = &text_runs[index];
                                    current_run_length = current_run.length();

                                    line_breaker = LineBreakEnumerator::new(current_run.text_span());
                                }
                            } else {
                                current_position = current_length + line_break.position_wrap() as i32;
                            }

                            if current_position == 0 && measured_length > 0 {
                                current_position = measured_length;
                            }

                            break_found = true;

                            break;
                        }

                        // We overflowed so we use the last available wrap position.
                        current_position = if last_wrap_position == 0 { measured_length } else { last_wrap_position };

                        break_found = true;

                        break;
                    }

                    if line_break.position_measure() != line_break.position_wrap()
                        || line_break.position_wrap() as i32 != current_run_length
                    {
                        last_wrap_position = current_length + line_break.position_wrap() as i32;
                    }
                }
            }

            if !break_found {
                current_length += current_run_length;

                index += 1;

                continue;
            }

            measured_length = current_position;

            break;
        }

        measured_length
    }

    fn reset_trailing_whitespace_bidi_levels(
        line_text_runs: &mut Vec<Rc<dyn TextRun>>,
        paragraph_flow_direction: FlowDirection,
        object_pool: &FormattingObjectPool,
    ) {
        if line_text_runs.is_empty() {
            return;
        }

        let last_text_run_index = line_text_runs.len() - 1;

        let last_text_run = line_text_runs[last_text_run_index].clone();

        let Some(shaped_text) = last_text_run.downcast_ref::<ShapedTextRun>() else {
            return;
        };

        let paragraph_embedding_level = paragraph_flow_direction as i8;

        if shaped_text.bidi_level() == paragraph_embedding_level {
            return;
        }

        let trailing_whitespace_length = shaped_text.glyph_run().metrics().trailing_whitespace_length;

        if trailing_whitespace_length == 0 {
            return;
        }

        let split_index = shaped_text.length() - trailing_whitespace_length;

        let (text_runs, trailing_whitespace_runs) =
            Self::split_text_runs(std::slice::from_ref(&last_text_run), split_index, object_pool).deconstruct();

        let trailing_whitespace_runs = trailing_whitespace_runs.map(|mut trailing_whitespace_runs| {
            for run in trailing_whitespace_runs.iter_mut() {
                let replacement = run.downcast_ref::<ShapedTextRun>().and_then(|shaped_text_run| {
                    let new_buffer = shaped_text_run.shaped_buffer().with_bidi_level(paragraph_embedding_level);

                    if Rc::ptr_eq(&new_buffer, shaped_text_run.shaped_buffer()) {
                        return None;
                    }

                    let replacement: Rc<dyn TextRun> =
                        Rc::new(ShapedTextRun::new(new_buffer, shaped_text_run.run_properties().clone()));

                    shaped_text_run.dispose();

                    Some(replacement)
                });

                if let Some(replacement) = replacement {
                    *run = replacement;
                }
            }

            line_text_runs.remove(last_text_run_index);

            if let Some(text_runs) = &text_runs {
                line_text_runs.extend(text_runs.iter().cloned());
            }

            line_text_runs.extend(trailing_whitespace_runs.iter().cloned());

            trailing_whitespace_runs
        });

        object_pool.text_run_lists.return_optional(text_runs);
        object_pool.text_run_lists.return_optional(trailing_whitespace_runs);
    }
}

impl TextFormatter for TextFormatterImpl {
    fn format_line(
        &self,
        text_source: &dyn ITextSource,
        first_text_source_index: i32,
        paragraph_width: f64,
        paragraph_properties: &Rc<dyn TextParagraphProperties>,
        previous_line_break: Option<&Rc<TextLineBreak>>,
    ) -> Option<Rc<dyn TextLine>> {
        self.format_line_with_cache(
            text_source,
            first_text_source_index,
            paragraph_width,
            paragraph_properties,
            previous_line_break,
            None,
        )
    }

    fn format_line_with_cache(
        &self,
        text_source: &dyn ITextSource,
        first_text_source_index: i32,
        paragraph_width: f64,
        paragraph_properties: &Rc<dyn TextParagraphProperties>,
        previous_line_break: Option<&Rc<TextLineBreak>>,
        text_run_cache: Option<&TextRunCache>,
    ) -> Option<Rc<dyn TextLine>> {
        let object_pool = FormattingObjectPool::instance();
        let font_manager = FontManager::current();

        // we've wrapped the previous line and need to continue wrapping: ignore the text source and do that instead
        if let Some(previous_line_break) = previous_line_break {
            if WrappingTextLineBreak::is_wrapping(previous_line_break) {
                if let Some(mut remaining_runs) = WrappingTextLineBreak::acquire_remaining_runs(previous_line_break) {
                    if paragraph_properties.text_wrapping() != TextWrapping::NoWrap {
                        return Some(Self::perform_text_wrapping(
                            &mut remaining_runs,
                            true,
                            first_text_source_index,
                            paragraph_width,
                            paragraph_properties,
                            previous_line_break.flow_direction(),
                            Some(previous_line_break),
                            object_pool,
                        ));
                    }
                }
            }
        }

        // Try to use cached shaped runs to avoid redundant shaping/bidi processing.
        if let Some(text_run_cache) = text_run_cache {
            if let Some(cached) = text_run_cache.try_get_shaped_runs(first_text_source_index) {
                return Some(Self::format_line_from_cache(
                    &cached,
                    first_text_source_index,
                    paragraph_width,
                    paragraph_properties,
                    object_pool,
                ));
            }
        }

        let (fetched_runs, text_end_of_line, text_source_length) =
            Self::fetch_text_runs(text_source, first_text_source_index, object_pool);

        if fetched_runs.is_empty() {
            object_pool.text_run_lists.return_list(fetched_runs);

            return None;
        }

        let (mut shaped_text_runs, resolved_flow_direction) =
            Self::shape_text_runs(&fetched_runs, &**paragraph_properties, object_pool, &font_manager);

        let next_line_break = text_end_of_line
            .as_ref()
            .map(|text_end_of_line| Rc::new(TextLineBreak::new(Some(text_end_of_line.clone()), resolved_flow_direction, false)));

        // Store shaped runs in cache for reuse. The cache takes its own references;
        // the formatter keeps the fresh-from-shape references for the current line.
        if let Some(text_run_cache) = text_run_cache {
            text_run_cache.add(
                first_text_source_index,
                CachedShapingResult::new(
                    Rc::from(&shaped_text_runs[..]),
                    resolved_flow_direction,
                    text_end_of_line,
                    text_source_length,
                ),
            );
        }

        let text_line = match paragraph_properties.text_wrapping() {
            TextWrapping::NoWrap => {
                // The runs are moved out of the rented list, which is returned below.
                let line_runs: Vec<Rc<dyn TextRun>> = shaped_text_runs.drain(..).collect();

                let text_line = TextLineImpl::new(
                    line_runs,
                    first_text_source_index,
                    text_source_length,
                    paragraph_width,
                    paragraph_properties.clone(),
                    resolved_flow_direction,
                    next_line_break,
                    false,
                );

                text_line.finalize_line();

                text_line
            }
            TextWrapping::WrapWithOverflow | TextWrapping::Wrap => Self::perform_text_wrapping(
                &mut shaped_text_runs,
                false,
                first_text_source_index,
                paragraph_width,
                paragraph_properties,
                resolved_flow_direction,
                next_line_break.as_ref(),
                object_pool,
            ),
        };

        object_pool.text_run_lists.return_list(shaped_text_runs);
        object_pool.text_run_lists.return_list(fetched_runs);

        Some(text_line)
    }
}

struct TextRunEnumerator<'a> {
    text_source: &'a dyn ITextSource,
    pos: i32,
}

impl<'a> TextRunEnumerator<'a> {
    fn new(text_source: &'a dyn ITextSource, first_text_source_index: i32) -> Self {
        Self { text_source, pos: first_text_source_index }
    }

    fn move_next(&mut self) -> Option<Rc<dyn TextRun>> {
        let current = self.text_source.get_text_run(self.pos)?;

        if current.length() == 0 {
            return None;
        }

        self.pos += current.length();

        Some(current)
    }
}
