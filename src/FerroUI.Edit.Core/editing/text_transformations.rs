use super::{EditorOptions, Selection};
use crate::document::{
    DocumentEdit, DocumentError, OffsetChangeMappingType, SimpleSegment, TextDocument,
    TextSnapshot, TextUtilities,
};

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
/// Except for `DeleteLines`, an empty selection means the whole document.
/// Text transformations ignore empty companion carets when other ranges are
/// selected. Line transformations include their lines, but do not fill gaps
/// between disjoint selections. An endpoint at a new line's start excludes that
/// line from whitespace operations and includes it in `DeleteLines`.
///
/// Tabs expand to a fixed indentation size, independent of their column.
/// Case conversion uses locale-independent Unicode scalar mappings and may
/// expand a scalar into several characters. Unpaired surrogates and unchanged
/// characters remain outside the planned edits.
pub fn plan_text_transformation(
    document: &TextDocument,
    selection: &Selection,
    options: &EditorOptions,
    transformation: TextTransformation,
) -> Result<Vec<DocumentEdit>, DocumentError> {
    let snapshot = document.create_snapshot();
    for segment in selection.segments() {
        if segment.end_offset() > snapshot.text_length() {
            return Err(DocumentError::InvalidRange {
                offset: segment.start_offset(),
                length: segment.length(),
                text_length: snapshot.text_length(),
            });
        }
    }

    let indentation_size = options.indentation_size.clamp(1, 256);
    let mut edits = Vec::new();
    match transformation {
        TextTransformation::RemoveLeadingWhitespace
        | TextTransformation::RemoveTrailingWhitespace
        | TextTransformation::LeadingTabsToSpaces
        | TextTransformation::LeadingSpacesToTabs
        | TextTransformation::DeleteLines => {
            let deleting_lines = transformation == TextTransformation::DeleteLines;
            for (first, last) in line_ranges(&snapshot, selection, deleting_lines)? {
                if deleting_lines {
                    let first_line = snapshot.get_line_by_number(first)?;
                    let last_line = snapshot.get_line_by_number(last)?;
                    push_edit(
                        &mut edits,
                        first_line.offset,
                        last_line.offset + last_line.total_length() - first_line.offset,
                        "",
                    );
                    continue;
                }
                for number in first..=last {
                    let line = snapshot.get_line_by_number(number)?;
                    let segment = if transformation == TextTransformation::RemoveTrailingWhitespace
                    {
                        let suffix =
                            TextUtilities::get_whitespace_before(&snapshot, line.end_offset())?;
                        // A whitespace-only line contains leading whitespace,
                        // so trailing-whitespace cleanup leaves it unchanged.
                        if suffix.offset == line.offset {
                            continue;
                        }
                        suffix
                    } else {
                        TextUtilities::get_whitespace_after(&snapshot, line.offset)?
                    };
                    match transformation {
                        TextTransformation::RemoveLeadingWhitespace
                        | TextTransformation::RemoveTrailingWhitespace => {
                            push_edit(&mut edits, segment.offset, segment.length, "");
                        }
                        TextTransformation::LeadingTabsToSpaces => {
                            transform_segment(
                                &snapshot,
                                segment,
                                TextTransformation::TabsToSpaces,
                                indentation_size,
                                &mut edits,
                            )?;
                        }
                        TextTransformation::LeadingSpacesToTabs => {
                            transform_segment(
                                &snapshot,
                                segment,
                                TextTransformation::SpacesToTabs,
                                indentation_size,
                                &mut edits,
                            )?;
                        }
                        _ => unreachable!("line transformation was checked before iteration"),
                    }
                }
            }
        }
        _ => {
            if selection.is_empty() {
                // Process one logical line at a time rather than allocating a
                // full-document UTF-16 or UTF-8 projection.
                for number in 1..=snapshot.line_count() {
                    let line = snapshot.get_line_by_number(number)?;
                    transform_segment(
                        &snapshot,
                        SimpleSegment::new(line.offset, line.length),
                        transformation,
                        indentation_size,
                        &mut edits,
                    )?;
                }
            } else {
                for segment in selection.segments().iter().filter(|s| !s.is_empty()) {
                    let mut offset = segment.start_offset();
                    let end = segment.end_offset();
                    while offset < end {
                        let line = snapshot.get_line_by_offset(offset)?;
                        let line_end = line.end_offset().min(end);
                        if offset < line_end {
                            transform_segment(
                                &snapshot,
                                SimpleSegment::new(offset, line_end - offset),
                                transformation,
                                indentation_size,
                                &mut edits,
                            )?;
                        }
                        offset = (line.offset + line.total_length()).min(end);
                    }
                }
            }
        }
    }
    Ok(edits)
}

fn line_ranges(
    snapshot: &TextSnapshot,
    selection: &Selection,
    deleting_lines: bool,
) -> Result<Vec<(usize, usize)>, DocumentError> {
    if selection.is_empty() && !deleting_lines {
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
    edits: &mut Vec<DocumentEdit>,
) -> Result<(), DocumentError> {
    if segment.length == 0 {
        return Ok(());
    }
    let units = snapshot.get_text_utf16(segment.offset, segment.length)?;
    match transformation {
        TextTransformation::TabsToSpaces => {
            let spaces = " ".repeat(indentation_size);
            for (index, unit) in units.iter().enumerate() {
                if *unit == b'\t' as u16 {
                    // Keep each tab's mapping independent so an anchor between
                    // adjacent tabs follows its own expanded column boundary.
                    edits.push(DocumentEdit::new(segment.offset + index, 1, &spaces));
                }
            }
        }
        TextTransformation::SpacesToTabs => {
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
        TextTransformation::Uppercase
        | TextTransformation::Lowercase
        | TextTransformation::InvertCase => {
            let mut offset = segment.offset;
            for decoded in char::decode_utf16(units) {
                match decoded {
                    Ok(character) => {
                        let lowercase = transformation == TextTransformation::Lowercase
                            || transformation == TextTransformation::InvertCase
                                && character.is_uppercase();
                        if lowercase {
                            push_case_edit(edits, offset, character, character.to_lowercase());
                        } else {
                            push_case_edit(edits, offset, character, character.to_uppercase());
                        }
                        offset += character.len_utf16();
                    }
                    Err(_) => offset += 1,
                }
            }
        }
        _ => unreachable!("only content transformations are passed to this helper"),
    }
    Ok(())
}

fn push_case_edit(
    edits: &mut Vec<DocumentEdit>,
    offset: usize,
    original: char,
    transformed: impl Iterator<Item = char> + Clone,
) {
    if transformed.clone().eq(std::iter::once(original)) {
        return;
    }
    if let Some(previous) = edits.last_mut() {
        if previous.offset + previous.length == offset {
            previous.length += original.len_utf16();
            previous.text.extend(transformed);
            return;
        }
    }
    edits.push(DocumentEdit::new(
        offset,
        original.len_utf16(),
        transformed.collect::<String>(),
    ));
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
            vec![DocumentEdit::new(2, 2, "ASS"), DocumentEdit::new(5, 2, "𐐀")]
        );
        assert_eq!(document.text(), "😀ASS 𐐀 İZ");
        assert!(document.undo().unwrap());
        apply(
            &document,
            Selection::empty(0),
            TextTransformation::Lowercase,
        );
        assert_eq!(document.text(), "😀aß 𐐨 i\u{307}z");
    }

    #[test]
    fn invert_case_preserves_nonletters_and_can_expand_mappings() {
        let document = TextDocument::from_text("Abß-𐐀𐐨😀1");
        apply(
            &document,
            Selection::empty(3),
            TextTransformation::InvertCase,
        );
        assert_eq!(document.text(), "aBSS-𐐨𐐀😀1");
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
