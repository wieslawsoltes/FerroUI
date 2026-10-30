//! Rebase paragraph styles for provisional IME text only. The application owns
//! committed ranges; this projection never mutates its immutable collection.
use ferroui_base::media::text_formatting::TextParagraphStyles;
use ferroui_base::utilities::ValueSpan;
use std::ops::Range;
use std::rc::Rc;

fn paragraphs(text: &[u16]) -> Vec<Range<usize>> {
    let mut output = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while cursor < text.len() {
        let unit = text[cursor];
        cursor += 1;
        if !matches!(unit, 10 | 13 | 0x85 | 0x2028 | 0x2029) { continue; }
        if unit == 13 && text.get(cursor) == Some(&10) { cursor += 1; }
        output.push(start..cursor);
        start = cursor;
    }
    output.push(start..text.len());
    output
}

/// Project the exact native edit, rather than inferring a change from common
/// text. Repeated paragraphs can make an insertion at the start look identical
/// to an append. `removed` is in original UTF-16 positions; `insertion` is the
/// caret after that removal, and `inserted_length` belongs to visible preedit.
pub(super) fn compose(
    styles: Option<TextParagraphStyles>, original: &[u16], visible: &[u16],
    removed: Range<usize>, insertion: usize, inserted_length: usize,
) -> Option<TextParagraphStyles> {
    let styles = styles?;
    if removed.is_empty() && inserted_length == 0 { return Some(styles); }
    // Ignore stale native edit coordinates rather than guessing another edit
    // or indexing outside an application-owned style/text snapshot.
    if removed.start > removed.end || removed.end > original.len() { return Some(styles); }
    let removed_length = removed.end - removed.start;
    let provisional_length = original.len() - removed_length;
    if insertion > provisional_length || provisional_length.checked_add(inserted_length) != Some(visible.len()) {
        return Some(styles);
    }
    let inserted_end = insertion + inserted_length;
    let original_paragraphs = paragraphs(original);
    // Match TextLayout's normalized ordering, including later tied starts and
    // a zero-length final paragraph. Each cursor only moves forward: O(P + S)
    // on normal ordered styles, O(P + S log S) for an unordered caller.
    let mut spans: Vec<_> = styles.iter().filter(|style| style.start() >= 0 && style.length() >= 0).collect();
    if !spans.windows(2).all(|pair| pair[0].start() <= pair[1].start()) {
        spans.sort_by_key(|style| style.start());
    }
    let mut paragraph_index = 0;
    let mut style_index = 0;
    let mut result = Vec::new();
    for paragraph in paragraphs(visible) {
        let after_insert = paragraph.start > insertion && paragraph.start >= inserted_end;
        let provisional = if paragraph.start <= insertion { paragraph.start }
            else if after_insert { paragraph.start - inserted_length }
            else { insertion };
        // A new line inside the insertion inherits the source paragraph. A
        // surviving paragraph starting at the deletion cut belongs to the
        // suffix, including a cut through the LF half of a CRLF terminator.
        // A separator-only merge still starts before the cut, so its prefix
        // keeps the original style.
        let source = if provisional > removed.start
            || (provisional == removed.start
                && (inserted_length == 0 || insertion != removed.start || after_insert)) {
            provisional + removed_length
        } else { provisional };
        while paragraph_index + 1 < original_paragraphs.len()
            && original_paragraphs[paragraph_index + 1].start <= source { paragraph_index += 1; }
        let start = original_paragraphs[paragraph_index].start as i32;
        while style_index < spans.len() && spans[style_index].start() <= start { style_index += 1; }
        if let Some(style) = style_index.checked_sub(1).and_then(|index| spans.get(index))
            .filter(|style| style.start() == start || start < style.start().saturating_add(style.length())) {
            result.push(ValueSpan::new(paragraph.start as i32, (paragraph.end - paragraph.start) as i32, style.value().clone()));
        }
    }
    Some(Rc::from(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::FlowDirection;
    use ferroui_base::media::text_formatting::TextParagraphStyle;

    #[test]
    fn provisional_paragraph_splits_and_merges_preserve_utf16_styles_without_mutation() {
        let before: Vec<_> = "A🦀\r\nשלום\nlast".encode_utf16().collect();
        let styles: TextParagraphStyles = Rc::from(vec![
            ValueSpan::new(0, 5, TextParagraphStyle { indent: 12.0, ..Default::default() }),
            ValueSpan::new(5, 5, TextParagraphStyle { flow_direction: Some(FlowDirection::RightToLeft), ..Default::default() }),
        ]);
        let split: Vec<_> = "A🦀\n\r\nשלום\nlast".encode_utf16().collect();
        let actual = compose(Some(styles.clone()), &before, &split, 3..3, 3, 1).unwrap();
        assert_eq!(actual.len(), 3);
        assert_eq!(actual.iter().map(|style| (style.start(), style.length())).collect::<Vec<_>>(), vec![(0,4),(4,2),(6,5)]);
        assert_eq!(actual[1].value().indent, 12.0);
        assert_eq!(actual[2].value().flow_direction, Some(FlowDirection::RightToLeft));
        let joined: Vec<_> = "A🦀שלום\nlast".encode_utf16().collect();
        let joined = compose(Some(styles.clone()), &before, &joined, 3..5, 3, 0).unwrap();
        assert_eq!(joined.len(), 1);
        assert_eq!(joined[0].value().indent, 12.0);
        assert_eq!((styles[1].start(), styles[1].length()), (5,5));
        assert!(Rc::ptr_eq(&compose(Some(styles.clone()), &before, &before, 0..0, 0, 0).unwrap(), &styles));
    }

    #[test]
    fn repeated_paragraph_preedit_uses_the_actual_insertion_and_replacement() {
        let before: Vec<_> = "A\nA\nA\n".encode_utf16().collect();
        let styles: TextParagraphStyles = Rc::from(vec![
            ValueSpan::new(0, 2, TextParagraphStyle { indent: 10.0, ..Default::default() }),
            ValueSpan::new(2, 2, TextParagraphStyle { indent: 20.0, ..Default::default() }),
            ValueSpan::new(4, 2, TextParagraphStyle { indent: 30.0, ..Default::default() }),
            ValueSpan::new(6, 0, TextParagraphStyle { indent: 40.0, ..Default::default() }),
        ]);
        let visible: Vec<_> = "A\nA\nA\nA\n".encode_utf16().collect();
        let indents = |styles: TextParagraphStyles| styles.iter().map(|style| style.value().indent).collect::<Vec<_>>();
        assert_eq!(indents(compose(Some(styles.clone()), &before, &visible, 0..0, 0, 2).unwrap()),
            vec![10.0, 10.0, 20.0, 30.0, 40.0]);
        // Replacing the first two paragraphs with three identical paragraphs
        // is visually the same string, but ownership of the suffix differs.
        assert_eq!(indents(compose(Some(styles.clone()), &before, &visible, 0..4, 0, 6).unwrap()),
            vec![10.0, 10.0, 10.0, 30.0, 40.0]);
        assert_eq!(indents(compose(Some(styles.clone()), &before, &before, 0..4, 0, 4).unwrap()),
            vec![10.0, 10.0, 30.0, 40.0], "identical text still has actual preedit replacement ownership");
        assert_eq!(styles[1].value().indent, 20.0);
    }

    #[test]
    fn provisional_lookup_normalizes_order_and_later_tied_starts() {
        let before: Vec<_> = "A\nB\n".encode_utf16().collect();
        let styles: TextParagraphStyles = Rc::from(vec![
            ValueSpan::new(2, 2, TextParagraphStyle { indent: 20.0, ..Default::default() }),
            ValueSpan::new(0, 2, TextParagraphStyle { indent: 10.0, ..Default::default() }),
            ValueSpan::new(2, 2, TextParagraphStyle { indent: 25.0, ..Default::default() }),
            ValueSpan::new(4, 0, TextParagraphStyle { indent: 30.0, ..Default::default() }),
            ValueSpan::new(-1, 2, TextParagraphStyle { indent: 99.0, ..Default::default() }),
        ]);
        let visible: Vec<_> = "X\nA\nB\n".encode_utf16().collect();
        let actual = compose(Some(styles), &before, &visible, 0..0, 0, 2).unwrap();
        assert_eq!(actual.iter().map(|style| style.value().indent).collect::<Vec<_>>(), [10.0, 10.0, 25.0, 30.0]);
    }

    #[test]
    fn empty_preedit_keeps_surviving_paragraphs_at_the_deletion_cut() {
        let before: Vec<_> = "A\nB\nC\n".encode_utf16().collect();
        let styles: TextParagraphStyles = Rc::from(vec![
            ValueSpan::new(0, 2, TextParagraphStyle { indent: 10.0, ..Default::default() }),
            ValueSpan::new(2, 2, TextParagraphStyle { indent: 20.0, ..Default::default() }),
            ValueSpan::new(4, 2, TextParagraphStyle { indent: 30.0, ..Default::default() }),
            ValueSpan::new(6, 0, TextParagraphStyle { indent: 40.0, ..Default::default() }),
        ]);
        for (removed, text, expected) in [
            (0..2, "B\nC\n", vec![20.0, 30.0, 40.0]),
            (0..3, "\nC\n", vec![20.0, 30.0, 40.0]),
            (2..4, "A\nC\n", vec![10.0, 30.0, 40.0]),
            (1..2, "AB\nC\n", vec![10.0, 30.0, 40.0]),
        ] {
            let visible: Vec<_> = text.encode_utf16().collect();
            let actual = compose(Some(styles.clone()), &before, &visible, removed.clone(), removed.start, 0).unwrap();
            assert_eq!(actual.iter().map(|style| style.value().indent).collect::<Vec<_>>(), expected, "{removed:?}");
        }
        let crlf: Vec<_> = "A\r\nB".encode_utf16().collect();
        let styles: TextParagraphStyles = Rc::from(vec![
            ValueSpan::new(0, 3, TextParagraphStyle { indent: 10.0, ..Default::default() }),
            ValueSpan::new(3, 1, TextParagraphStyle { indent: 20.0, ..Default::default() }),
        ]);
        let visible: Vec<_> = "A\rB".encode_utf16().collect();
        let actual = compose(Some(styles), &crlf, &visible, 2..3, 2, 0).unwrap();
        assert_eq!(actual.iter().map(|style| style.value().indent).collect::<Vec<_>>(), [10.0, 20.0]);
    }

    #[test]
    fn provisional_styles_rebase_one_hundred_thousand_paragraphs_in_order() {
        const COUNT: usize = 100_000;
        let before: Vec<_> = "A\n".repeat(COUNT).encode_utf16().collect();
        let mut spans: Vec<_> = (0..COUNT).map(|index| ValueSpan::new((index * 2) as i32, 2,
            TextParagraphStyle { indent: index as f64, ..Default::default() })).collect();
        spans.push(ValueSpan::new((COUNT * 2) as i32, 0, TextParagraphStyle { indent: COUNT as f64, ..Default::default() }));
        let styles: TextParagraphStyles = Rc::from(spans);
        let mut visible = vec![b'A' as u16, 10]; visible.extend_from_slice(&before);
        let actual = compose(Some(styles.clone()), &before, &visible, 0..0, 0, 2).unwrap();
        assert_eq!(actual.len(), COUNT + 2);
        assert_eq!(actual[0].value().indent, 0.0);
        for (index, style) in actual.iter().skip(1).enumerate() {
            assert_eq!(style.start(), ((index + 1) * 2) as i32);
            assert_eq!(style.value().indent, index as f64);
            assert_eq!(style.length(), if index == COUNT { 0 } else { 2 });
        }
        assert_eq!(styles.len(), COUNT + 1);
    }
}
