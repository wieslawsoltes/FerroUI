use std::rc::Rc;

use crate::media::text_formatting::logical_text_run_enumerator::LogicalTextRunEnumerator;
use crate::media::text_formatting::unicode::{Codepoint, WordBreakClass, WordBreakEnumerator};
use crate::media::text_formatting::{
    DrawableTextRun, ShapedTextRun, TextCollapsingProperties, TextFormatter, TextLine, TextRun,
};

pub(crate) struct TextEllipsisHelper;

impl TextEllipsisHelper {
    pub fn collapse(
        text_line: &dyn TextLine,
        properties: &dyn TextCollapsingProperties,
        is_word_ellipsis: bool,
    ) -> Option<Vec<Rc<dyn TextRun>>> {
        let mut text_runs_enumerator = LogicalTextRunEnumerator::new(text_line);

        if text_runs_enumerator.count() == 0 {
            return None;
        }

        let shaped_symbol = <dyn TextFormatter>::create_symbol(&**properties.symbol(), properties.flow_direction());

        if properties.width() < shaped_symbol.glyph_run().bounds().width {
            // Not enough space to fit in the symbol
            return Some(Vec::new());
        }

        let mut available_width = properties.width() - shaped_symbol.size().width;

        let mut collapsed_length = 0;

        while let Some(current_run) = text_runs_enumerator.move_next() {
            if let Some(shaped_run) = current_run.downcast_ref::<ShapedTextRun>() {
                let text_run_width = shaped_run.size().width;

                if text_run_width > available_width {
                    let mut measured_length = shaped_run.try_measure_characters(available_width).unwrap_or(0);

                    if measured_length > 0 && is_word_ellipsis && measured_length < text_line.length() {
                        let mut current_break_position = 0;

                        let text = current_run.text_span();
                        let mut word_breaker = WordBreakEnumerator::new(text);

                        while current_break_position < measured_length {
                            let Some(word_segment) = word_breaker.move_next() else {
                                break;
                            };

                            // A word segment can carry an empty text, so the length is the only
                            // code-unit width valid for every segment.
                            let next_break_position = (word_segment.offset() + word_segment.length()) as i32;

                            if next_break_position == 0 {
                                break;
                            }

                            if next_break_position >= measured_length {
                                break;
                            }

                            let (first_codepoint, _) = Codepoint::read_at(text, word_segment.offset());

                            if first_codepoint.word_break_class() == WordBreakClass::WSegSpace {
                                // UAX #29 allows boundaries on both sides of WSegSpace; use the start
                                // to preserve the existing behavior of trimming spaces before the ellipsis.
                                current_break_position = word_segment.offset() as i32;

                                continue;
                            }

                            current_break_position = next_break_position;
                        }

                        measured_length = current_break_position;
                    }

                    collapsed_length += measured_length;

                    return Some(<dyn TextCollapsingProperties>::create_collapsed_runs(
                        text_line,
                        collapsed_length,
                        shaped_symbol,
                    ));
                }

                available_width -= text_run_width;
            } else if let Some(drawable_run) = current_run.as_drawable() {
                // The whole run needs to fit into available space
                if drawable_run.size().width > available_width {
                    return Some(<dyn TextCollapsingProperties>::create_collapsed_runs(
                        text_line,
                        collapsed_length,
                        shaped_symbol,
                    ));
                }

                available_width -= drawable_run.size().width;
            }

            collapsed_length += current_run.length();
        }

        None
    }
}
