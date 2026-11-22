use super::{EditorOptions, Selection};
use crate::document::{
    DocumentEdit, DocumentError, OffsetChangeMappingType, SimpleSegment, TextDocument,
    TextSnapshot, TextUtilities,
};
use crate::unicode_case::{CaseLocale, is_uppercase_letter};

/// Text transformations shared by editor commands and nonvisual hosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextTransformation {
    RemoveLeadingWhitespace,
    RemoveTrailingWhitespace,
    TabsToSpaces,
    SpacesToTabs,
    LeadingTabsToSpaces,
    LeadingSpacesToTabs,
    Uppercase,
    Lowercase,
    InvertCase,
    DeleteLines,
}

impl TextTransformation {
    /// The anchor-mapping policy used when applying this command's edits.
    /// Casing and tab conversion retain anchors in replaced characters; line
    /// deletion and whitespace removal use ordinary deletion semantics.
    pub const fn offset_change_mapping(self) -> OffsetChangeMappingType {
        match self {
            Self::Uppercase
            | Self::Lowercase
            | Self::InvertCase
            | Self::TabsToSpaces
            | Self::SpacesToTabs
            | Self::LeadingTabsToSpaces
            | Self::LeadingSpacesToTabs => OffsetChangeMappingType::CharacterReplace,
            Self::RemoveLeadingWhitespace | Self::RemoveTrailingWhitespace | Self::DeleteLines => {
                OffsetChangeMappingType::Normal
            }
        }
    }
}

/// Plans a transformation at original UTF-16 offsets without changing text or
/// selections. The returned edits are ordered and do not overlap. Apply them
/// before changing the source document; an editor should first check its
/// protected ranges with `EditorSession::validate_document_edits`.
/// `EditorSession::transform_text` applies this command's anchor-mapping policy.
/// Nonvisual callers can apply edits in descending order within an update using
/// `TextDocument::replace_with_mapping` and `offset_change_mapping`; the generic
/// `TextDocument::apply_edits` method uses normal replacement mapping instead.
///
/// Except for `DeleteLines`, an empty selection or a rectangle with zero
/// selected UTF-16 units means the whole document. In `DeleteLines`, such a
/// rectangle targets its active line.
/// Text transformations ignore empty companion carets when other ranges are
/// selected. Line transformations include their lines, but do not fill gaps
/// between disjoint selections. An endpoint at a new line's start excludes that
/// line from whitespace operations and includes it in `DeleteLines`.
///
/// Tabs expand to a fixed indentation size, independent of their column.
/// Case conversion uses invariant simple mappings and keeps UTF-16 lengths.
/// Invert Case operates on individual UTF-16 units. Unpaired surrogates and
/// unchanged characters remain outside the planned edits.
pub fn plan_text_transformation(
    document: &TextDocument,
    selection: &Selection,
    options: &EditorOptions,
    transformation: TextTransformation,
) -> Result<Vec<DocumentEdit>, DocumentError> {
    plan_text_transformation_with(
        &document.create_snapshot(),
        selection,
        options,
        transformation,
        CaseLocale::Invariant,
        |range| Ok(std::iter::once(range)),
    )
}

pub(super) fn plan_text_transformation_with<E: From<DocumentError>, Parts: IntoIterator<Item = SimpleSegment>>(
    snapshot: &TextSnapshot,
    selection: &Selection,
    options: &EditorOptions,
    transformation: TextTransformation,
    locale: CaseLocale,
    mut writable_parts: impl FnMut(SimpleSegment) -> Result<Parts, E>,
) -> Result<Vec<DocumentEdit>, E> {
    for segment in selection.segments() {
        if segment.end_offset() > snapshot.text_length() {
            return Err(DocumentError::InvalidRange {
                offset: segment.start_offset(),
                length: segment.length(),
                text_length: snapshot.text_length(),
            }.into());
        }
    }

    let indentation_size = options.indentation_size.clamp(1, 256);
    let mut edits = Vec::new();
    let mut plan_range = |range: SimpleSegment| -> Result<(), E> {
        if range.length != 0 {
            for part in writable_parts(range)? {
                transform_segment(snapshot, part, transformation, indentation_size, locale, &mut edits)?;
            }
        }
        Ok(())
    };
    match transformation {
        TextTransformation::RemoveLeadingWhitespace
        | TextTransformation::RemoveTrailingWhitespace
        | TextTransformation::LeadingTabsToSpaces
        | TextTransformation::LeadingSpacesToTabs
        | TextTransformation::DeleteLines => {
            let deleting_lines = transformation == TextTransformation::DeleteLines;
            for (first, last) in line_ranges(snapshot, selection, deleting_lines)? {
                if deleting_lines {
                    let first_line = snapshot.get_line_by_number(first)?;
                    let last_line = snapshot.get_line_by_number(last)?;
                    plan_range(SimpleSegment::new(
                        first_line.offset,
                        last_line.offset + last_line.total_length() - first_line.offset,
                    ))?;
                    continue;
                }
                for number in first..=last {
                    let line = snapshot.get_line_by_number(number)?;
                    let segment = if transformation == TextTransformation::RemoveTrailingWhitespace
                    {
                        let suffix =
                            TextUtilities::get_whitespace_before(snapshot, line.end_offset())?;
                        // A whitespace-only line contains leading whitespace,
                        // so trailing-whitespace cleanup leaves it unchanged.
                        if suffix.offset == line.offset {
                            continue;
                        }
                        suffix
                    } else {
                        TextUtilities::get_whitespace_after(snapshot, line.offset)?
                    };
                    plan_range(segment)?;
                }
            }
        }
        _ => {
            if selection.is_empty() || selection.is_rectangle_without_text() {
                // Process one logical line at a time rather than allocating a
                // full-document UTF-16 or UTF-8 projection.
                for number in 1..=snapshot.line_count() {
                    let line = snapshot.get_line_by_number(number)?;
                    plan_range(SimpleSegment::new(line.offset, line.length))?;
                }
            } else {
                for segment in selection.segments().iter().filter(|s| !s.is_empty()) {
                    let mut offset = segment.start_offset();
                    let end = segment.end_offset();
                    while offset < end {
                        let line = snapshot.get_line_by_offset(offset)?;
                        let line_end = line.end_offset().min(end);
                        if offset < line_end {
                            plan_range(SimpleSegment::new(offset, line_end - offset))?;
                        }
                        offset = (line.offset + line.total_length()).min(end);
                    }
                }
            }
        }
    }
    Ok(edits)
}

pub(super) fn line_ranges(
    snapshot: &TextSnapshot,
    selection: &Selection,
    deleting_lines: bool,
) -> Result<Vec<(usize, usize)>, DocumentError> {
    if deleting_lines && selection.is_rectangle_without_text() {
        let line = snapshot.get_line_by_offset(selection.primary().caret)?.line_number;
        return Ok(vec![(line, line)]);
    }
    if !deleting_lines && (selection.is_empty() || selection.is_rectangle_without_text()) {
        return Ok(vec![(1, snapshot.line_count())]);
    }
    let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(selection.segments().len());
    for segment in selection.segments() {
        let first = snapshot
            .get_line_by_offset(segment.start_offset())?
            .line_number;
        let last_offset =
            if deleting_lines || segment.length() == 0 || segment.end_virtual_space() != 0 {
                segment.end_offset()
            } else {
                segment.end_offset() - 1
            };
        let last = snapshot.get_line_by_offset(last_offset)?.line_number;
        match ranges.last_mut() {
            Some((_, previous_last)) if first <= previous_last.saturating_add(1) => {
                *previous_last = (*previous_last).max(last);
            }
            _ => ranges.push((first, last)),
        }
    }
    Ok(ranges)
}

fn transform_segment(
    snapshot: &TextSnapshot,
    segment: SimpleSegment,
    transformation: TextTransformation,
    indentation_size: usize,
    locale: CaseLocale,
    edits: &mut Vec<DocumentEdit>,
) -> Result<(), DocumentError> {
    if segment.length == 0 {
        return Ok(());
    }
    if matches!(
        transformation,
        TextTransformation::RemoveLeadingWhitespace
            | TextTransformation::RemoveTrailingWhitespace
            | TextTransformation::DeleteLines
    ) {
        push_edit(edits, segment.offset, segment.length, "");
        return Ok(());
    }
    let units = snapshot.get_text_utf16(segment.offset, segment.length)?;
    match transformation {
        TextTransformation::TabsToSpaces | TextTransformation::LeadingTabsToSpaces => {
            let spaces = " ".repeat(indentation_size);
            for (index, unit) in units.iter().enumerate() {
                if *unit == b'\t' as u16 {
                    // Keep each tab's mapping independent so an anchor between
                    // adjacent tabs follows its own expanded column boundary.
                    edits.push(DocumentEdit::new(segment.offset + index, 1, &spaces));
                }
            }
        }
        TextTransformation::SpacesToTabs | TextTransformation::LeadingSpacesToTabs => {
            let mut index = 0;
            while index < units.len() {
                if units[index] != b' ' as u16 {
                    index += 1;
                    continue;
                }
                let start = index;
                while index < units.len() && units[index] == b' ' as u16 {
                    index += 1;
                }
                let count = (index - start) / indentation_size;
                for group in 0..count {
                    edits.push(DocumentEdit::new(
                        segment.offset + start + group * indentation_size,
                        indentation_size,
                        "\t",
                    ));
                }
            }
        }
        TextTransformation::Uppercase | TextTransformation::Lowercase => {
            let mut offset = segment.offset;
            for decoded in char::decode_utf16(units) {
                match decoded {
                    Ok(character) => {
                        let mapped = if transformation == TextTransformation::Lowercase {
                            locale.lower(character)
                        } else {
                            locale.upper(character)
                        };
                        push_case_edit(edits, offset, character, mapped);
                        offset += character.len_utf16();
                    }
                    Err(_) => offset += 1,
                }
            }
        }
        TextTransformation::InvertCase => {
            for (index, unit) in units.into_iter().enumerate() {
                if let Some(character) = char::from_u32(u32::from(unit)) {
                    let mapped = if is_uppercase_letter(unit) {
                        locale.lower(character)
                    } else {
                        locale.upper(character)
                    };
                    push_case_edit(edits, segment.offset + index, character, mapped);
                }
            }
        }
        _ => unreachable!("pure deletions were handled before reading the source"),
    }
    Ok(())
}

fn push_case_edit(edits: &mut Vec<DocumentEdit>, offset: usize, original: char, mapped: char) {
    if original != mapped {
        let mut buffer = [0; 4];
        push_edit(edits, offset, original.len_utf16(), mapped.encode_utf8(&mut buffer));
    }
}

fn push_edit(edits: &mut Vec<DocumentEdit>, offset: usize, length: usize, text: &str) {
    if length == 0 {
        return;
    }
    if let Some(previous) = edits.last_mut() {
        if previous.offset + previous.length == offset {
            previous.length += length;
            previous.text.push_str(text);
            return;
        }
    }
    edits.push(DocumentEdit::new(offset, length, text));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::{
        EditError, EditorSession, SelectionSegment, TextSegmentReadOnlySectionProvider,
    };
    use std::rc::Rc;

    fn apply(
        document: &TextDocument,
        selection: Selection,
        transformation: TextTransformation,
    ) -> Vec<DocumentEdit> {
        let plan = plan_text_transformation(
            document,
            &selection,
            &EditorOptions::default(),
            transformation,
        )
        .unwrap();
        document.apply_edits(&plan).unwrap();
        plan
    }

    #[test]
    fn whitespace_cleanup_defaults_to_the_document_and_preserves_delimiters() {
        let document = TextDocument::from_text("  a \t\r\n\t \r  b\t\n\u{2003}c \n");
        let original = document.text();
        apply(
            &document,
            Selection::empty(3),
            TextTransformation::RemoveTrailingWhitespace,
        );
        assert_eq!(document.text(), "  a\r\n\t \r  b\n\u{2003}c\n");
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), original);
        apply(
            &document,
            Selection::empty(3),
            TextTransformation::RemoveLeadingWhitespace,
        );
        assert_eq!(document.text(), "a \t\r\n\rb\t\n\u{2003}c \n");
    }

    #[test]
    fn line_cleanup_excludes_an_untouched_endpoint_line_in_either_direction() {
        for selection in [Selection::simple(1, 4), Selection::simple(4, 1)] {
            let document = TextDocument::from_text(" a\r\n b\r\n c");
            apply(
                &document,
                selection,
                TextTransformation::RemoveLeadingWhitespace,
            );
            assert_eq!(document.text(), "a\r\n b\r\n c");
        }
    }

    #[test]
    fn disjoint_line_selections_do_not_modify_the_gap_and_include_companion_carets() {
        let document = TextDocument::from_text("  a\n  b\n  c\n  d");
        let selection = Selection::from_segments(
            [SelectionSegment::new(2, 3), SelectionSegment::empty(10)],
            1,
        );
        apply(
            &document,
            selection,
            TextTransformation::RemoveLeadingWhitespace,
        );
        assert_eq!(document.text(), "a\n  b\nc\n  d");
    }

    #[test]
    fn tabs_expand_by_fixed_width_and_leading_conversion_leaves_body_tabs() {
        let document = TextDocument::from_text("\ta\tb\t\n \tx\t");
        apply(
            &document,
            Selection::empty(0),
            TextTransformation::LeadingTabsToSpaces,
        );
        assert_eq!(document.text(), "    a\tb\t\n     x\t");
        assert!(document.undo().unwrap());
        apply(
            &document,
            Selection::empty(0),
            TextTransformation::TabsToSpaces,
        );
        assert_eq!(document.text(), "    a    b    \n     x    ");
    }

    #[test]
    fn space_groups_preserve_remainders_and_do_not_cross_other_characters() {
        let document = TextDocument::from_text("     a   b        c\r\n   \n ");
        apply(
            &document,
            Selection::empty(4),
            TextTransformation::SpacesToTabs,
        );
        assert_eq!(document.text(), "\t a   b\t\tc\r\n   \n ");
        assert!(document.undo().unwrap());
        apply(
            &document,
            Selection::empty(0),
            TextTransformation::LeadingSpacesToTabs,
        );
        assert_eq!(document.text(), "\t a   b        c\r\n   \n ");
    }

    #[test]
    fn selected_space_conversion_respects_range_boundaries() {
        let document = TextDocument::from_text("a        z\r\n  b");
        let plan = apply(
            &document,
            Selection::simple(3, 8),
            TextTransformation::SpacesToTabs,
        );
        assert_eq!(plan, vec![DocumentEdit::new(3, 4, "\t")]);
        assert_eq!(document.text(), "a  \t  z\r\n  b");
    }

    #[test]
    fn case_conversion_uses_unicode_scalars_and_original_utf16_offsets() {
        let document = TextDocument::from_text("😀aß 𐐨 İZ");
        let plan = apply(
            &document,
            Selection::simple(2, 8),
            TextTransformation::Uppercase,
        );
        assert_eq!(
            plan,
            vec![DocumentEdit::new(2, 1, "A"), DocumentEdit::new(5, 2, "𐐀")]
        );
        assert_eq!(document.text(), "😀Aß 𐐀 İZ");
        assert!(document.undo().unwrap());
        apply(
            &document,
            Selection::empty(0),
            TextTransformation::Lowercase,
        );
        assert_eq!(document.text(), "😀aß 𐐨 İz");
    }

    #[test]
    fn invert_case_uses_utf16_units_and_uppercase_letter_category() {
        let document = TextDocument::from_text("Abß-𐐀𐐨😀1 ǅⅠⅰⒶ");
        apply(
            &document,
            Selection::empty(3),
            TextTransformation::InvertCase,
        );
        assert_eq!(document.text(), "aBß-𐐀𐐨😀1 ǄⅠⅠⒶ");
    }

    #[test]
    fn casing_language_matches_reference_and_keeps_undo_selection_offsets() {
        use TextTransformation::{InvertCase, Lowercase, Uppercase};

        let original = "i I İ ı ß ẞ ﬃ ΟΣ ΣΣ I\u{307} 𐐨 𐐀";
        let invariant = [
            "I I İ ı ß ẞ ﬃ ΟΣ ΣΣ I\u{307} 𐐀 𐐀",
            "i i İ ı ß ß ﬃ οσ σσ i\u{307} 𐐨 𐐨",
            "I i İ ı ß ß ﬃ οσ σσ i\u{307} 𐐨 𐐀",
        ];
        let default = [
            "I I İ I ß ẞ ﬃ ΟΣ ΣΣ I\u{307} 𐐀 𐐀",
            "i i i ı ß ß ﬃ οσ σσ i\u{307} 𐐨 𐐨",
            "I i i I ß ß ﬃ οσ σσ i\u{307} 𐐨 𐐀",
        ];
        let turkic = [
            "İ I İ I ß ẞ ﬃ ΟΣ ΣΣ I\u{307} 𐐀 𐐀",
            "i ı i ı ß ß ﬃ οσ σσ ı\u{307} 𐐨 𐐨",
            "İ ı i I ß ß ﬃ οσ σσ ı\u{307} 𐐨 𐐀",
        ];
        for (tag, mapped) in [
            ("", invariant),
            ("en-US", default),
            ("lt-LT", default),
            ("TR-cy", turkic),
            ("az-Cyrl-AZ", turkic),
        ] {
            for (operation, expected) in [Uppercase, Lowercase, InvertCase].into_iter().zip(mapped) {
                let document = TextDocument::from_text(original);
                let session = EditorSession::new(document.clone());
                let length = document.text_length();
                let selection = Selection::simple(length, 0);
                session.set_selection(selection.clone()).unwrap();
                session.transform_text_in_language_checked(operation, tag, || true).unwrap();
                assert_eq!(document.text(), expected, "{tag:?} {operation:?}");
                assert_eq!(document.text_length(), length);
                assert_eq!(session.selection(), selection);
                assert_eq!(document.undo_count(), 1);
                assert!(session.undo().unwrap());
                assert_eq!(document.text(), original);
                assert_eq!(session.selection(), selection);
                assert!(session.redo().unwrap());
                assert_eq!(document.text(), expected);
                assert_eq!(session.selection(), selection);
            }
        }
    }

    #[test]
    fn case_plans_preserve_unpaired_surrogates_and_split_selection_pairs() {
        let original = [
            b'a' as u16,
            0xd800,
            b'b' as u16,
            0xdc00,
            0xd801,
            0xdc28,
            b'c' as u16,
        ];
        let document = TextDocument::from_utf16(&original);
        apply(
            &document,
            Selection::empty(0),
            TextTransformation::Uppercase,
        );
        assert_eq!(
            document.create_snapshot().to_utf16(),
            [
                b'A' as u16,
                0xd800,
                b'B' as u16,
                0xdc00,
                0xd801,
                0xdc00,
                b'C' as u16
            ]
        );
        assert!(document.undo().unwrap());
        let split_pair = Selection::from_segments(
            [SelectionSegment::new(0, 5), SelectionSegment::new(5, 7)],
            0,
        );
        apply(&document, split_pair, TextTransformation::Uppercase);
        assert_eq!(
            document.create_snapshot().to_utf16(),
            [
                b'A' as u16,
                0xd800,
                b'B' as u16,
                0xdc00,
                0xd801,
                0xdc28,
                b'C' as u16
            ]
        );
    }

    #[test]
    fn segment_transformations_ignore_empty_companion_carets() {
        let document = TextDocument::from_text("abc def ghi");
        apply(
            &document,
            Selection::from_segments([SelectionSegment::new(0, 3), SelectionSegment::empty(6)], 0),
            TextTransformation::Uppercase,
        );
        assert_eq!(document.text(), "ABC def ghi");
    }

    #[test]
    fn delete_lines_includes_the_selection_endpoint_line() {
        let document = TextDocument::from_text("a\r\nb\rc\nd");
        let plan = apply(
            &document,
            Selection::simple(1, 3),
            TextTransformation::DeleteLines,
        );
        assert_eq!(plan, vec![DocumentEdit::new(0, 5, "")]);
        assert_eq!(document.text(), "c\nd");
        assert!(document.undo().unwrap());
        apply(
            &document,
            Selection::empty(5),
            TextTransformation::DeleteLines,
        );
        assert_eq!(document.text(), "a\r\nb\rd");
    }

    #[test]
    fn delete_line_plan_uses_the_active_row_of_a_rectangle_with_no_text() {
        let document = TextDocument::from_text("aaa\r\nbbb\r\nccc\r\nlast");
        for (primary, offset) in [(0, 0), (2, 10)] {
            let rectangle = Selection::from_rectangle_segments(
                [SelectionSegment::empty(1), SelectionSegment::empty(6), SelectionSegment::empty(11)],
                primary,
            );
            let plan = plan_text_transformation(&document, &rectangle, &EditorOptions::default(), TextTransformation::DeleteLines).unwrap();
            assert_eq!(plan, [DocumentEdit::new(offset, 5, "")]);
        }
    }

    #[test]
    fn multicaret_line_deletion_coalesces_coverage_and_groups_undo() {
        let document = TextDocument::from_text("aa\nbb\ncc\ndd\nee");
        let original = document.text();
        let plan = apply(
            &document,
            Selection::from_segments(
                [
                    SelectionSegment::empty(0),
                    SelectionSegment::empty(2),
                    SelectionSegment::new(6, 10),
                ],
                0,
            ),
            TextTransformation::DeleteLines,
        );
        assert_eq!(
            plan,
            vec![DocumentEdit::new(0, 3, ""), DocumentEdit::new(6, 6, "")]
        );
        assert_eq!(document.text(), "bb\nee");
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), original);
        assert!(!document.can_undo());
    }

    #[test]
    fn deleting_the_last_line_preserves_its_preceding_delimiter() {
        let document = TextDocument::from_text("a\r\nlast");
        apply(
            &document,
            Selection::empty(7),
            TextTransformation::DeleteLines,
        );
        assert_eq!(document.text(), "a\r\n");
        assert!(apply(
            &document,
            Selection::empty(3),
            TextTransformation::DeleteLines
        )
        .is_empty());
    }

    #[test]
    fn unchanged_protected_characters_are_not_part_of_a_case_edit() {
        let document = TextDocument::from_text("a,b");
        let editor = EditorSession::new(document.clone());
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        protected.add(1, 1).unwrap();
        editor.set_read_only_section_provider(protected);
        let plan = plan_text_transformation(
            &document,
            &Selection::empty(0),
            &EditorOptions::default(),
            TextTransformation::Uppercase,
        )
        .unwrap();
        assert_eq!(
            plan,
            vec![DocumentEdit::new(0, 1, "A"), DocumentEdit::new(2, 1, "B")]
        );
        editor.apply_document_edits(&plan).unwrap();
        assert_eq!(document.text(), "A,B");
    }

    #[test]
    fn protected_changed_ranges_reject_the_whole_plan_before_mutation() {
        let document = TextDocument::from_text("a,b");
        let editor = EditorSession::new(document.clone());
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        protected.add(2, 1).unwrap();
        editor.set_read_only_section_provider(protected);
        let plan = plan_text_transformation(
            &document,
            &Selection::empty(0),
            &EditorOptions::default(),
            TextTransformation::Uppercase,
        )
        .unwrap();
        assert!(matches!(
            editor.apply_document_edits(&plan),
            Err(EditError::ReadOnly)
        ));
        assert_eq!(document.text(), "a,b");
        assert!(!document.can_undo());
    }

    #[test]
    fn protected_transformations_match_reference_fragments_and_selection_history() {
        use TextTransformation::{Lowercase, SpacesToTabs, TabsToSpaces, Uppercase};

        for (transformation, original, selection, protected, expected, mapped) in [
            (Uppercase, "abXycd\r\n ef", (0, 10), (2, 2), "ABXyCD\r\n Ef", (0, 10)),
            (Uppercase, "abXycd\r\n ef", (10, 0), (2, 2), "ABXyCD\r\n Ef", (10, 0)),
            (Lowercase, "ABSAVECD", (0, 0), (2, 4), "abSAVEcd", (0, 0)),
            (TabsToSpaces, "\tx\t\r\n\tx", (0, 0), (1, 3), "    x\t\r\n    x", (0, 0)),
            (SpacesToTabs, "          x", (0, 10), (4, 2), "\t  \tx", (0, 4)),
            (SpacesToTabs, "      x", (0, 6), (2, 1), "      x", (0, 6)),
            (Uppercase, "abXycd", (0, 0), (0, 6), "abXycd", (0, 0)),
        ] {
            let document = TextDocument::from_text(original);
            let editor = EditorSession::new(document.clone());
            let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
            policy.add(protected.0, protected.1).unwrap();
            editor.set_read_only_section_provider(policy);
            let selection = Selection::simple(selection.0, selection.1);
            let mapped = Selection::simple(mapped.0, mapped.1);
            editor.set_selection(selection.clone()).unwrap();
            editor.transform_text(transformation).unwrap();
            assert_eq!(document.text(), expected, "{transformation:?}: {original:?}");
            assert_eq!(editor.selection(), mapped);
            assert_eq!(document.undo_count(), usize::from(original != expected));
            if original != expected {
                assert!(editor.undo().unwrap());
                assert_eq!(document.text(), original);
                assert_eq!(editor.selection(), selection);
                assert!(editor.redo().unwrap());
                assert_eq!(document.text(), expected);
                assert_eq!(editor.selection(), mapped);
            }
        }
    }

    #[test]
    fn line_cleanup_and_leading_conversions_preserve_protected_whitespace() {
        use TextTransformation::{
            LeadingSpacesToTabs, LeadingTabsToSpaces, RemoveLeadingWhitespace,
            RemoveTrailingWhitespace,
        };

        for (transformation, original, protected, expected) in [
            (RemoveTrailingWhitespace, "a   \r\nb   ", (2, 1), "a \r\nb"),
            (RemoveLeadingWhitespace, "   a\r\n  b", (1, 1), " a\r\nb"),
            (LeadingTabsToSpaces, "\t\tx\r\ny", (0, 1), "\t    x\r\ny"),
            (LeadingSpacesToTabs, "          x", (4, 2), "\t  \tx"),
        ] {
            let document = TextDocument::from_text(original);
            let editor = EditorSession::new(document.clone());
            let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
            policy.add(protected.0, protected.1).unwrap();
            editor.set_read_only_section_provider(policy);
            editor.transform_text(transformation).unwrap();
            assert_eq!(document.text(), expected);
            assert_eq!(document.undo_count(), 1);
            assert!(editor.undo().unwrap());
            assert_eq!(document.text(), original);
        }
    }

    #[test]
    fn transform_leaves_a_surrogate_pair_intact_when_protection_splits_it() {
        let original = "a\u{10428}B";
        let document = TextDocument::from_text(original);
        let editor = EditorSession::new(document.clone());
        let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        policy.add(2, 1).unwrap();
        editor.set_read_only_section_provider(policy);
        editor.transform_text(TextTransformation::InvertCase).unwrap();
        assert_eq!(document.text(), "A\u{10428}b");
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), original);
    }

    #[test]
    fn invalid_selections_are_errors_and_noop_plans_do_not_add_history() {
        let document = TextDocument::from_text("abc");
        for selection in [Selection::empty(4), Selection::simple(0, usize::MAX)] {
            assert!(matches!(
                plan_text_transformation(
                    &document,
                    &selection,
                    &EditorOptions::default(),
                    TextTransformation::Uppercase
                ),
                Err(DocumentError::InvalidRange { .. })
            ));
        }
        assert_eq!(document.text(), "abc");
        assert!(apply(
            &document,
            Selection::empty(0),
            TextTransformation::Lowercase
        )
        .is_empty());
        assert!(!document.can_undo());
    }

    #[test]
    fn casing_mapping_preserves_anchors_inside_a_changed_word() {
        let document = TextDocument::from_text("abcd");
        let anchor = document.create_anchor(2).unwrap();
        let transformation = TextTransformation::Uppercase;
        let plan = plan_text_transformation(
            &document,
            &Selection::empty(0),
            &EditorOptions::default(),
            transformation,
        )
        .unwrap();
        let update = document.begin_update().unwrap();
        for edit in plan.iter().rev() {
            document
                .replace_with_mapping(
                    edit.offset,
                    edit.length,
                    &edit.text,
                    transformation.offset_change_mapping(),
                )
                .unwrap();
        }
        update.finish().unwrap();
        assert_eq!(document.text(), "ABCD");
        assert!(!anchor.is_deleted());
        assert_eq!(anchor.offset().unwrap(), 2);
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "abcd");
        assert_eq!(anchor.offset().unwrap(), 2);
    }

    #[test]
    fn tab_conversion_mapping_preserves_boundaries_between_adjacent_groups() {
        for (original, transformation, before, after, expected) in [
            ("\t\tx", TextTransformation::TabsToSpaces, 1, 4, "        x"),
            ("        x", TextTransformation::SpacesToTabs, 4, 1, "\t\tx"),
        ] {
            let document = TextDocument::from_text(original);
            let anchor = document.create_anchor(before).unwrap();
            let plan = plan_text_transformation(
                &document,
                &Selection::empty(0),
                &EditorOptions::default(),
                transformation,
            )
            .unwrap();
            assert_eq!(plan.len(), 2);
            let update = document.begin_update().unwrap();
            for edit in plan.iter().rev() {
                document
                    .replace_with_mapping(
                        edit.offset,
                        edit.length,
                        &edit.text,
                        transformation.offset_change_mapping(),
                    )
                    .unwrap();
            }
            update.finish().unwrap();
            assert_eq!(document.text(), expected);
            assert!(!anchor.is_deleted());
            assert_eq!(anchor.offset().unwrap(), after);
            assert!(document.undo().unwrap());
            assert_eq!(document.text(), original);
            assert_eq!(anchor.offset().unwrap(), before);
        }
    }

    #[test]
    fn indentation_size_is_bounded_for_direct_planner_callers() {
        for (requested, expected) in [(0, 1), (usize::MAX, 256)] {
            let document = TextDocument::from_text("\t");
            let options = EditorOptions {
                indentation_size: requested,
                ..EditorOptions::default()
            };
            let plan = plan_text_transformation(
                &document,
                &Selection::empty(0),
                &options,
                TextTransformation::TabsToSpaces,
            )
            .unwrap();
            document.apply_edits(&plan).unwrap();
            assert_eq!(document.text(), " ".repeat(expected));
        }
    }
}
