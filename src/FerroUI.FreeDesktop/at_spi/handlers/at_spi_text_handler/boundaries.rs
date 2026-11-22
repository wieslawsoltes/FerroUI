use super::{AccessibleText, TextSegment, empty, scalar_range, to_offset};
use ferroui_base::media::text_formatting::unicode::{SentenceBreakEnumerator, WordBreakEnumerator};
use std::cmp::Reverse;
use std::ops::Range;

#[derive(Clone, Copy)]
pub(super) enum Direction {
    Before,
    At,
    After,
}

pub(super) fn character_range(length: i32, offset: i32, direction: Direction) -> Range<i32> {
    let position = match direction {
        Direction::Before if offset <= 0 => return 0..0,
        Direction::Before => offset.min(length) - 1,
        Direction::At => offset,
        Direction::After if offset < -1 => return length..length,
        Direction::After => offset.saturating_add(1),
    };
    scalar_range(length, position)
}

pub(super) fn segment(text: &AccessibleText, offset: i32, boundary: u32, direction: Direction) -> TextSegment {
    let len = text.len();
    if boundary == super::BOUNDARY_CHAR {
        let span = character_range(len, offset, direction);
        return text.segment(span.start, span.end);
    }

    let requested = offset.clamp(0, len);
    let mut marks = vec![0];
    let mut following = 0;
    let add = |index| {
        if marks.last() != Some(&index) {
            marks.push(index);
            following += usize::from(index > requested);
        }
        following >= 2
    };
    if !visit_marks(text, boundary, add) {
        return empty();
    }
    if marks.last() != Some(&len) {
        marks.push(len);
    }

    let current = marks.partition_point(|&mark| mark <= requested) - 1;
    let index = match direction {
        Direction::Before if offset <= 0 || current == 0 => return empty(),
        Direction::Before => current - 1,
        Direction::At => current,
        Direction::After if offset < 0 => 0,
        Direction::After => current + 1,
    };
    match marks.get(index..index + 2) {
        Some(pair) => text.segment(pair[0], pair[1]),
        None => text.segment(len, len),
    }
}

// Enumerates real word and sentence marks. Document-end sentinels belong to
// the caller, so a paragraph can supply marks without adding any at its edges.
fn visit_marks(text: &AccessibleText, boundary: u32, mut emit: impl FnMut(i32) -> bool) -> bool {
    use super::{BOUNDARY_SENTENCE_END, BOUNDARY_SENTENCE_START, BOUNDARY_WORD_END, BOUNDARY_WORD_START};

    let mut add = |unit_offset| {
        let index = text.offsets.partition_point(|&offset| offset < unit_offset);
        emit(to_offset(index))
    };
    match boundary {
        BOUNDARY_WORD_START | BOUNDARY_WORD_END => {
            let mut words = WordBreakEnumerator::new(&text.units);
            while let Some(word) = words.move_next() {
                // Unicode word breaking also yields punctuation and space
                // segments. Legacy word boundaries delimit lexical words.
                if char::decode_utf16(word.text().iter().copied()).filter_map(Result::ok)
                    .any(|character| character.is_alphanumeric() || character == '_')
                {
                    let unit = if boundary == BOUNDARY_WORD_START { word.offset() }
                        else { word.offset() + word.length() };
                    if add(unit) {
                        break;
                    }
                }
            }
        }
        BOUNDARY_SENTENCE_START | BOUNDARY_SENTENCE_END => {
            let mut sentences = SentenceBreakEnumerator::new(&text.units);
            while let Some(sentence) = sentences.move_next() {
                let start = sentence.offset();
                let end = if boundary == BOUNDARY_SENTENCE_START {
                    start
                } else {
                    let meaningful = sentence.text().iter().rposition(|&unit| {
                        !char::from_u32(u32::from(unit)).is_some_and(char::is_whitespace)
                    });
                    start + meaningful.map_or(0, |index| index + 1)
                };
                if add(end) {
                    break;
                }
            }
        }
        _ => return false,
    }
    true
}

#[derive(Default)]
struct NearestMarks {
    before: [Option<i32>; 2],
    after: [Option<Reverse<i32>>; 2],
}

impl NearestMarks {
    fn add(&mut self, mark: i32, requested: i32) {
        if mark <= requested {
            retain_largest_two(&mut self.before, mark);
        } else {
            retain_largest_two(&mut self.after, Reverse(mark));
        }
    }
}

fn retain_largest_two<T: Copy + Ord>(values: &mut [Option<T>; 2], value: T) {
    if values.contains(&Some(value)) {
        return;
    }
    if values[0].is_none_or(|first| value > first) {
        values[1] = values[0];
        values[0] = Some(value);
    } else if values[1].is_none_or(|second| value > second) {
        values[1] = Some(value);
    }
}

/// Resolves a contextual range using complete hard-break paragraphs. Word and
/// sentence segmentation reset at CR/LF. Legacy ranges can include text across
/// paragraphs, so look past blank lines until the required marks are found.
pub(super) fn indexed_range(
    offset: i32,
    length: i32,
    boundary: u32,
    direction: Direction,
    paragraph_at: impl Fn(i32) -> Option<TextSegment>,
) -> Option<Range<i32>> {
    // A long stretch of separators can lack lexical marks. In that case a
    // single full-story scan is cheaper than a provider call for every line.
    const PARAGRAPH_BUDGET: usize = 16;
    if length == 0 || matches!(direction, Direction::Before) && offset <= 0 {
        return Some(0..0);
    }
    let direction = match direction {
        Direction::After if offset < 0 => Direction::At,
        direction => direction,
    };
    let requested = offset.clamp(0, length);
    let (left_marks, right_marks) = match direction {
        Direction::Before => (2, 0),
        Direction::At => (1, 1),
        Direction::After => (0, 2),
    };
    let mut paragraphs_read = 0;
    let mut collect = |position, marks: &mut NearestMarks| {
        if paragraphs_read == PARAGRAPH_BUDGET {
            return None;
        }
        paragraphs_read += 1;
        let (text, start, end) = paragraph_at(position)?;
        if start < 0 || end > length || !(start..end).contains(&position)
            || end < length && !text.ends_with(['\r', '\n'])
        {
            return None;
        }
        let text = AccessibleText::new(&text);
        if text.len() != end - start {
            return None;
        }
        let recognized = visit_marks(&text, boundary, |mark| {
            marks.add(start + mark, requested);
            false
        });
        recognized.then_some(start..end)
    };

    let mut marks = NearestMarks::default();
    marks.add(0, requested);
    marks.add(length, requested);
    let mut span = collect(requested.min(length - 1), &mut marks)?;
    while left_marks > 0 && span.start > 0 {
        if marks.before[left_marks - 1].is_some_and(|mark| mark >= span.start) {
            break;
        }
        let previous = collect(span.start - 1, &mut marks)?;
        if previous.end != span.start {
            return None;
        }
        span.start = previous.start;
    }
    while right_marks > 0 && span.end < length {
        if marks.after[right_marks - 1].is_some_and(|Reverse(mark)| mark < span.end) {
            break;
        }
        let next = collect(span.end, &mut marks)?;
        if next.start != span.end {
            return None;
        }
        span.end = next.end;
    }
    let result = match direction {
        Direction::Before => marks.before[1].zip(marks.before[0]).map(|(start, end)| start..end),
        Direction::At => marks.before[0].zip(marks.after[0]).map(|(start, Reverse(end))| start..end),
        Direction::After => marks.after[0].zip(marks.after[1]).map(|(Reverse(start), Reverse(end))| start..end),
    };
    Some(result.unwrap_or(match direction {
        Direction::Before => 0..0,
        _ => length..length,
    }))
}

pub(super) fn line_range(
    offset: i32,
    length: i32,
    by_end: bool,
    direction: Direction,
    line_at: impl Fn(i32) -> TextSegment,
) -> Range<i32> {
    let containing = |position| {
        if position >= length {
            return length..length;
        }
        let line = line_at(position);
        let (_, start, end) = line;
        if !by_end {
            return start..end;
        }
        let content_end = line_content_end(&line);
        if position < content_end {
            let previous = if start == 0 { 0 } else { line_content_end(&line_at(start - 1)) };
            previous..content_end
        } else {
            let following = if end >= length { length } else { line_content_end(&line_at(end)) };
            content_end..following
        }
    };
    let range = containing(offset.max(0));
    match direction {
        Direction::Before if range.start == 0 => 0..0,
        Direction::Before => containing(range.start - 1),
        Direction::At => range,
        Direction::After if offset < 0 => containing(0),
        Direction::After => containing(range.end),
    }
}

fn line_content_end(line: &TextSegment) -> i32 {
    let (text, _, end) = line;
    if text.ends_with("\r\n") {
        end - 2
    } else if text.ends_with(['\r', '\n', '\u{85}', '\u{2028}', '\u{2029}']) {
        end - 1
    } else {
        *end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::at_spi::handlers::at_spi_text_handler::{
        BOUNDARY_SENTENCE_END, BOUNDARY_SENTENCE_START, BOUNDARY_WORD_END, BOUNDARY_WORD_START,
    };

    #[test]
    fn indexed_paragraphs_preserve_contextual_boundaries_across_lines() {
        for lines in [
            &[][..],
            &["  abc. Next?😀\r\n", "\r", " \t \n", "👩‍💻\r\n", "𐐷  x.\n", "\n", "last!"],
            &["  \r", "\u{200d}Word. Another!\u{2029}tail\n", "e\u{301}\r\n", "😀  \n"],
            &["\r\n", "   \n", "\r\n"],
            &["𐐀. A\u{301}  next"],
        ] {
            let mut length = 0;
            let paragraphs: Vec<_> = lines.iter().map(|line| {
                let start = length;
                length += to_offset(line.chars().count());
                (line.to_string(), start, length)
            }).collect();
            let story = AccessibleText::new(&lines.concat());
            for boundary in [BOUNDARY_WORD_START, BOUNDARY_WORD_END, BOUNDARY_SENTENCE_START, BOUNDARY_SENTENCE_END] {
                for offset in -2..=length + 2 {
                    for direction in [Direction::Before, Direction::At, Direction::After] {
                        let span = indexed_range(offset, length, boundary, direction, |position| {
                            paragraphs.iter().find(|(_, start, end)| (*start..*end).contains(&position)).cloned()
                        }).unwrap();
                        assert_eq!(story.segment(span.start, span.end), segment(&story, offset, boundary, direction),
                            "{lines:?}, boundary {boundary}, offset {offset}");
                    }
                }
            }
        }
    }

    #[test]
    fn indexed_words_only_load_paragraphs_that_contain_their_boundaries() {
        let paragraph = "a  word. After.\r\n";
        let width = to_offset(paragraph.chars().count());
        let offset = width * 999 + 4;
        let loads = std::cell::Cell::new(0);
        let span = indexed_range(offset, width * 1000, BOUNDARY_WORD_START, Direction::At, |position| {
            loads.set(loads.get() + 1);
            let start = position / width * width;
            Some((paragraph.into(), start, start + width))
        }).unwrap();
        assert_eq!(span, offset - 1..offset + 5);
        assert_eq!(loads.get(), 1);
    }

    #[test]
    fn long_runs_without_words_use_the_full_context_fallback() {
        let span = indexed_range(100, 400, BOUNDARY_WORD_START, Direction::At, |position| {
            Some(("\n".into(), position, position + 1))
        });
        assert_eq!(span, None);
    }
}
