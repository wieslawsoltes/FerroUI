use std::collections::VecDeque;
use std::rc::Rc;

use crate::media::text_formatting::text_line_impl::TextLineImpl;
use crate::media::text_formatting::unicode::LineBreakEnumerator;
use crate::media::text_formatting::{
    GlyphInfo, JustificationProperties, ShapedBuffer, ShapedTextRun, TabAlignment, TextLine, TextRun,
};

/// Justifies a line by widening the gaps at its break opportunities.
///
/// Internal upstream; public so the Skia unit tests reach it.
#[allow(dead_code)] // constructed by the text layout (a later step)
pub struct InterWordJustification {
    width: f64,
}

#[allow(dead_code)] // see the struct
impl InterWordJustification {
    pub fn new(width: f64) -> Self {
        Self { width }
    }

    fn collect_break_opportunities(
        text_runs: &[(usize, i32, Rc<dyn TextRun>)],
        first_position: i32,
        justification_start: i32,
        break_opportunities: &mut VecDeque<i32>,
    ) {
        let mut current_position = first_position;
        let mut run_index = 0usize;

        let mut buffer: Vec<u16> = Vec::new();

        while run_index < text_runs.len() {
            // Runs that carry no text (an embedded object, an end of line) interrupt the text, so
            // each stretch of text runs between them is searched on its own.
            let segment_start = current_position;
            let mut segment_length = 0i32;
            let mut segment_end = run_index;

            while segment_end < text_runs.len() {
                let text_run = &text_runs[segment_end].2;
                let text = text_run.text_span();

                // A run whose length doesn't match its text can't be mapped back to text source
                // positions by offset, so it ends the segment instead of shifting everything
                // after it.
                if text.is_empty() || text.len() as i32 != text_run.length() {
                    break;
                }

                segment_length += text_run.length();
                segment_end += 1;
            }

            if segment_length == 0 {
                current_position += text_runs[run_index].2.length();
                run_index += 1;

                continue;
            }

            buffer.clear();

            for (_, _, text_run) in &text_runs[run_index..segment_end] {
                buffer.extend_from_slice(text_run.text_span());
            }

            let mut line_break_enumerator = LineBreakEnumerator::new(&buffer);

            while let Some(current_break) = line_break_enumerator.move_next() {
                if current_break.required() || current_break.position_wrap() as i32 == segment_length {
                    continue;
                }

                // The extra advance must land on the glyph that ENDS at the break boundary
                // (the last glyph before the break), so the widened gap sits on the break
                // itself. For whitespace breaks the line breaker has already pulled
                // the measure position back onto the trailing whitespace glyph
                // (position_measure < position_wrap), so that position is the target as-is.
                // For zero-width breaks - CJK/Korean ideograph boundaries, hyphens and other
                // break-after punctuation - position_measure == position_wrap and points one
                // glyph PAST the boundary; step back one so we widen the gap the break
                // represents rather than the following gap. This also keeps the last visible
                // glyph of a CJK/Korean line unstretched: its only inbound break is the
                // segment-final one, already excluded above.
                let mut target = segment_start + current_break.position_measure() as i32;

                if current_break.position_measure() == current_break.position_wrap() {
                    target -= 1;
                }

                if target >= justification_start {
                    break_opportunities.push_back(target);
                }
            }

            current_position = segment_start + segment_length;
            run_index = segment_end;
        }
    }
}

impl JustificationProperties for InterWordJustification {
    fn width(&self) -> f64 {
        self.width
    }

    fn justify(&self, text_line: &dyn TextLine) {
        let Some(line_impl) = text_line.as_any().downcast_ref::<TextLineImpl>() else {
            return;
        };

        let paragraph_width = self.width;

        if paragraph_width.is_infinite() {
            return;
        }

        let mut break_opportunities = VecDeque::new();

        // Break positions are logical UTF-16 positions; the line's drawing list
        // has already been reordered visually for bidi. Keep the visual index
        // alongside each logical run so replacement updates the correct glyphs.
        let text_runs: Vec<(usize, i32, Rc<dyn TextRun>)> = if let Some(indexed) = line_impl.indexed_text_runs() {
            indexed
                .iter()
                .filter_map(|run| {
                    run.text_run
                        .borrow()
                        .clone()
                        .map(|text_run| (run.run_index.get() as usize, run.text_source_character_index, text_run))
                })
                .collect()
        } else {
            let mut position = text_line.first_text_source_index();
            line_impl
                .text_runs()
                .iter()
                .enumerate()
                .map(|(index, run)| {
                    let result = (index, position, run.clone());
                    position += run.length();
                    result
                })
                .collect()
        };

        // Tab fields before the final tab keep their measured anchors. The
        // final field can fill the remaining line only when its leading edge is
        // anchored (Left in LTR, Right in RTL); expanding a centered, trailing or
        // decimal field would move that anchor. A trailing tab therefore also
        // prevents earlier text from receiving justification space.
        let mut justification_start = text_line.first_text_source_index();
        for (_, position, run) in &text_runs {
            if let Some(shaped) = run.downcast_ref::<ShapedTextRun>() {
                if let Some(alignment) = shaped.tab_alignment() {
                    let leading = if shaped.bidi_level() & 1 == 0 {
                        alignment == TabAlignment::Left
                    } else {
                        alignment == TabAlignment::Right
                    };
                    justification_start = if leading {
                        position + run.length()
                    } else {
                        text_line.first_text_source_index() + text_line.length()
                    };
                }
            }
        }

        Self::collect_break_opportunities(
            &text_runs,
            text_line.first_text_source_index(),
            justification_start,
            &mut break_opportunities,
        );

        if break_opportunities.is_empty() {
            return;
        }

        // Fill the visible content to the paragraph width, not the width including trailing
        // whitespace. A wrapped line keeps the space at its wrap point as trailing whitespace,
        // which can push the width including trailing whitespace to or past the paragraph width;
        // using it here would leave remaining_space at zero and the line unjustified. The distributed
        // space only ever lands on visible glyphs (trailing whitespace gets no break), so the
        // visible content reaches the margin and the trailing whitespace hangs past it.
        let remaining_space = (paragraph_width - line_impl.width()).max(0.0);
        let spacing = remaining_space / break_opportunities.len() as f64;

        for (run_index, current_position, text_run) in text_runs {
            let run_length = text_run.length();
            let run_end = current_position + run_length;

            let shaped_text = if text_run.text_span().is_empty() {
                None
            } else {
                text_run.downcast_ref::<ShapedTextRun>()
            };
            let mut writable_buffer: Option<Rc<ShapedBuffer>> = None;

            // Consume only the break opportunities that fall inside this run's range. The queue
            // is in ascending position order, so once the front break is at or past run_end it
            // belongs to a later run and must be left for it, instead of draining the whole
            // queue against this run.
            while let Some(&character_index) = break_opportunities.front() {
                if character_index >= run_end {
                    break;
                }

                break_opportunities.pop_front();

                // Skip stale breaks and breaks in runs we cannot justify (non-shaped).
                let Some(shaped_text) = shaped_text else {
                    continue;
                };

                if character_index < current_position {
                    continue;
                }

                // Copy-on-write: the run's own ShapedBuffer may share its glyph storage
                // with a TextRunCache entry, a split sibling or a with_bidi_level alias, so
                // mutating it in place would corrupt those. Adjust a private clone and swap in a
                // fresh run below.
                let writable_buffer =
                    writable_buffer.get_or_insert_with(|| shaped_text.shaped_buffer().clone_writable());

                let glyph_run = shaped_text.glyph_run();

                let offset = 0.max(current_position - glyph_run.metrics().first_cluster);
                let glyph_index = glyph_run.find_glyph_index(character_index - offset);
                let glyph_info = writable_buffer.get(glyph_index);

                writable_buffer.set(
                    glyph_index,
                    GlyphInfo::new(
                        glyph_info.glyph_index,
                        glyph_info.glyph_cluster,
                        glyph_info.glyph_advance + spacing,
                    ),
                );
            }

            if let (Some(writable_buffer), Some(shaped_text)) = (writable_buffer, shaped_text) {
                let justified_run: Rc<dyn TextRun> = Rc::new(shaped_text.with_shaped_buffer(writable_buffer));

                line_impl.replace_text_run(run_index, justified_run);

                shaped_text.dispose();
            }
        }
    }
}
