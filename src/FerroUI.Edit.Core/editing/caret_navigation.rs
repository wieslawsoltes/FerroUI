use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretMovement {
    CharacterLeft,
    CharacterRight,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    DocumentStart,
    DocumentEnd,
    LineUp,
    LineDown,
}

/// Returns a UTF-16 caret boundary. Extended grapheme clusters (including
/// combining marks, emoji sequences and CRLF) are indivisible to navigation.
pub fn next_caret_position(text: &str, offset: usize, movement: CaretMovement) -> usize {
    let len = text.encode_utf16().count();
    let offset = offset.min(len);
    match movement {
        CaretMovement::DocumentStart => return 0,
        CaretMovement::DocumentEnd => return len,
        CaretMovement::LineStart
        | CaretMovement::LineEnd
        | CaretMovement::LineUp
        | CaretMovement::LineDown => {
            let lines = line_ranges(text);
            let index = lines
                .partition_point(|(start, _, _)| *start <= offset)
                .saturating_sub(1);
            let (start, end, _) = lines[index];
            return match movement {
                CaretMovement::LineStart => start,
                CaretMovement::LineEnd => end,
                CaretMovement::LineUp => {
                    if index == 0 {
                        0
                    } else {
                        let line = lines[index - 1];
                        (line.0 + offset - start).min(line.1)
                    }
                }
                _ => {
                    if index + 1 >= lines.len() {
                        len
                    } else {
                        let line = lines[index + 1];
                        (line.0 + offset - start).min(line.1)
                    }
                }
            };
        }
        _ => {}
    }
    if matches!(
        movement,
        CaretMovement::CharacterLeft | CaretMovement::CharacterRight
    ) {
        let mut position = 0;
        for grapheme in text.graphemes(true) {
            let next = position + grapheme.encode_utf16().count();
            if movement == CaretMovement::CharacterLeft && next >= offset {
                return position;
            }
            if movement == CaretMovement::CharacterRight && next > offset {
                return next;
            }
            position = next;
        }
        return len;
    }
    let mut position = 0;
    let mut previous_word = 0;
    for word in text.split_word_bounds() {
        let end = position + word.encode_utf16().count();
        let is_word = word.chars().any(|c| c.is_alphanumeric() || c == '_');
        if movement == CaretMovement::WordLeft {
            if position >= offset {
                break;
            }
            if is_word {
                previous_word = position;
            }
        } else if is_word && end > offset {
            return end;
        }
        position = end;
    }
    if movement == CaretMovement::WordLeft {
        previous_word
    } else {
        len
    }
}

pub(crate) fn line_ranges(text: &str) -> Vec<(usize, usize, usize)> {
    let units: Vec<_> = text.encode_utf16().collect();
    let mut lines = Vec::new();
    let (mut start, mut cursor) = (0, 0);
    while cursor < units.len() {
        if units[cursor] == 13 || units[cursor] == 10 {
            let end = cursor;
            cursor += 1;
            if units[end] == 13 && units.get(cursor) == Some(&10) {
                cursor += 1;
            }
            lines.push((start, end, cursor));
            start = cursor;
        } else {
            cursor += 1;
        }
    }
    lines.push((start, units.len(), units.len()));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extended_graphemes_surrogates_and_crlf_are_atomic() {
        let s = "a\u{301}😀\r\nz";
        for (a, b) in [(0, 2), (2, 4), (4, 6), (6, 7)] {
            assert_eq!(next_caret_position(s, a, CaretMovement::CharacterRight), b);
            assert_eq!(next_caret_position(s, b, CaretMovement::CharacterLeft), a);
        }
    }
    #[test]
    fn navigation_handles_short_lines_and_final_empty_line() {
        assert_eq!(
            next_caret_position("abcd\r\nx\n", 3, CaretMovement::LineDown),
            7
        );
        assert_eq!(
            next_caret_position("abcd\r\nx\n", 8, CaretMovement::LineUp),
            6
        );
        assert_eq!(
            next_caret_position("one + two", 9, CaretMovement::WordLeft),
            6
        );
    }
}
