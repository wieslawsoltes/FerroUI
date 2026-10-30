use super::*;
use crate::rich_text::rich_text_field_story::{InstructionBoundary, VisibleCoverage};
use std::collections::HashSet;

#[path = "rich_text_link_native_cases.rs"]
mod native_cases;

fn plain(text: &str) -> FieldStory {
    // The measured fixture input uses CR as its structural paragraph separator.
    // Match the existing visible document normalization, identifying each LF
    // explicitly instead of making the projection infer structure from text.
    let visible = text.replace("\r\n", "\n").replace('\r', "\n");
    let breaks = visible
        .encode_utf16()
        .enumerate()
        .filter_map(|(position, unit)| (unit == 10).then_some(VisibleOffset::new(position)))
        .collect();
    FieldStory::new(&visible, breaks).unwrap()
}

fn range(story: &FieldStory, start: usize, end: usize) -> FieldStoryRange {
    story
        .range(FieldStoryOffset::new(start), FieldStoryOffset::new(end))
        .unwrap()
}

fn field(id: u64, start: usize, end: usize, target: &str) -> LinkField {
    LinkField::new(
        LinkFieldId::new(id),
        VisibleOffset::new(start)..VisibleOffset::new(end),
        LinkArgument::from_target(target, false).unwrap(),
    )
}

fn projected_text(story: &FieldStory) -> String {
    story.text(&story.whole_range()).unwrap()
}

fn assert_native_snapshot(
    story: &FieldStory,
    range: &FieldStoryRange,
    expected: &native_cases::NativeLinkSnapshot,
) {
    assert_eq!((range.start().get(), range.end().get()), expected.range);
    assert_eq!(story.length(), expected.story_utf16_length);
    assert_eq!(projected_text(story), expected.story);
    assert_eq!(story.text(range).unwrap(), expected.text);
    assert_eq!(
        story.utf16(range).unwrap().len(),
        expected.text_utf16_length
    );
}

fn native_hresult(error: FieldStoryError) -> u32 {
    match error {
        FieldStoryError::InvalidArgument => 0x80070057,
        FieldStoryError::ParagraphOnly => 0x8000ffff,
        other => panic!("measured native transition reached an unexpected boundary: {other:?}"),
    }
}

#[test]
fn rich_link_native_transition_oracle_uses_observed_before_ranges() {
    assert_eq!(
        native_cases::SOURCE_SHA256,
        "8e0fc9f9605aefd02ea20949d1b9fd3305f427d1fb493f600969bcda16623987"
    );
    assert_eq!(
        native_cases::MEASURED_COMMIT,
        "dc99ba00a692c51391fa18a767f7d16a4f6f2415"
    );
    assert_eq!(
        native_cases::PROBE_GIT_BLOB,
        "4accd7fdb21888f9fe4843f3c957420ab2cc38ab"
    );
    assert_eq!(native_cases::ENGINE_VERSION, "6.2.26100.33438");
    let mut fixtures = HashSet::new();
    let mut current_fixture = "";
    let mut story = plain("");
    let mut expected_step = 0;
    let mut changed_getters = 0;
    let mut failed_setters = 0;
    for case in native_cases::CASES {
        if current_fixture != case.fixture {
            assert!(fixtures.insert(case.fixture), "fixture rows are contiguous");
            current_fixture = case.fixture;
            expected_step = 0;
            story = plain(case.initial_text);
        }
        assert_eq!(case.step, expected_step, "{} {}", case.fixture, case.role);
        expected_step += 1;
        // These are the observed positions after any preceding setup. This
        // test deliberately does not claim paired live-range tracking parity.
        let before = range(&story, case.before.range.0, case.before.range.1);
        assert_native_snapshot(&story, &before, &case.before);
        let old_story = story.clone();
        let mut acted = before.clone();
        let mut result = None;
        let hresult = match case.operation {
            "Get" => match story.get_link(&before) {
                Ok(value) => {
                    acted = value.range;
                    result = value.argument.map(|argument| argument.canonical());
                    if (acted.start().get(), acted.end().get()) != case.before.range {
                        changed_getters += 1;
                    }
                    if result.is_some() {
                        0
                    } else {
                        1
                    }
                }
                Err(error) => native_hresult(error),
            },
            "Set" => match story.set_link(&before, case.argument) {
                Ok(value) => {
                    story = value.story;
                    acted = value.range;
                    0
                }
                Err(error) => {
                    failed_setters += 1;
                    native_hresult(error)
                }
            },
            other => panic!("unexpected literal operation {other}"),
        };
        assert_eq!(
            hresult, case.hresult,
            "{}:{} {}",
            case.fixture, case.step, case.role
        );
        assert_eq!(
            result.as_deref(),
            case.result,
            "{}:{}",
            case.fixture,
            case.step
        );
        assert_native_snapshot(&story, &acted, &case.after);
        assert_eq!(
            story.visible_text(),
            old_story.visible_text(),
            "Link preserves visible characters"
        );
        // The native oracle establishes selection preservation; this pure
        // service has no selection handle and cannot mutate editor selection.
        assert_eq!(case.before.selection, case.after.selection);
        if hresult & 0x80000000 != 0 || case.operation == "Get" {
            assert!(story.same_snapshot(&old_story));
        }
    }
    assert_eq!(native_cases::CASES.len(), 137);
    assert_eq!(fixtures.len(), 59);
    assert_eq!(changed_getters, 15);
    assert_eq!(failed_setters, 12);
}

#[test]
fn rich_link_projection_keeps_visible_coordinates_and_explicit_paragraph_structure() {
    let source = "ab\ncd\r\nef";
    let story = FieldStory::new(source, vec![VisibleOffset::new(2)]).unwrap();
    assert_eq!(story.visible_text(), source);
    assert_eq!(projected_text(&story), "ab\rcd\r\nef\r");
    assert_eq!(story.visible_length(), 9);
    assert_eq!(story.length(), 10);
    assert_eq!(
        story
            .project_visible(VisibleOffset::new(9), InstructionBoundary::After)
            .unwrap(),
        FieldStoryOffset::new(9)
    );
}

#[test]
fn rich_link_projection_tags_all_utf16_positions_without_guessing_markers() {
    let original = plain("a😀b");
    let changed = original
        .set_link(&range(&original, 1, 3), Some("\"x\""))
        .unwrap();
    let story = changed.story;
    assert_eq!(projected_text(&story), "aHYPERLINK \"x\"😀b\r");
    assert_eq!(story.length(), 18);
    assert_eq!(
        story
            .project_visible(VisibleOffset::new(1), InstructionBoundary::Before)
            .unwrap()
            .get(),
        1
    );
    assert_eq!(
        story
            .project_visible(VisibleOffset::new(1), InstructionBoundary::After)
            .unwrap()
            .get(),
        14
    );
    assert_eq!(
        story
            .project_visible(VisibleOffset::new(2), InstructionBoundary::After)
            .unwrap()
            .get(),
        15
    );
    assert_eq!(
        story.location(FieldStoryOffset::new(1)).unwrap(),
        FieldLocation::Instruction {
            field: story.fields()[0].id(),
            utf16_offset: 0
        }
    );
    assert_eq!(
        story.location(FieldStoryOffset::new(14)).unwrap(),
        FieldLocation::Visible(VisibleOffset::new(1))
    );
    assert_eq!(
        story.location(FieldStoryOffset::new(17)).unwrap(),
        FieldLocation::FinalParagraphMark { after: false }
    );
    assert_eq!(
        story.location(FieldStoryOffset::new(18)).unwrap(),
        FieldLocation::FinalParagraphMark { after: true }
    );
    for position in 0..=story.length() {
        let position = FieldStoryOffset::new(position);
        assert_eq!(
            story
                .resolve_location(story.location(position).unwrap())
                .unwrap(),
            position
        );
    }
    assert_eq!(story.utf16(&range(&story, 15, 16)).unwrap(), [0xde00]);
    assert_eq!(
        story.text(&range(&story, 15, 16)).unwrap_err(),
        FieldStoryError::InvalidUtf16Boundary
    );
}

#[test]
fn rich_link_projection_preserves_sentinel_and_supplementary_target_units() {
    let original = plain("ab");
    let changed = original
        .set_link(&range(&original, 0, 1), Some("\u{fddf}\"😀\""))
        .unwrap();
    let story = changed.story;
    assert_eq!(projected_text(&story), "\u{fddf}HYPERLINK \"😀\"ab\r");
    assert_eq!(story.length(), 18);
    let argument = story.fields()[0].argument();
    assert!(argument.has_sentinel());
    assert_eq!(argument.target(), "😀");
    assert_eq!(argument.canonical(), "\u{fddf}\"😀\"");
    for position in 0..=story.length() {
        let position = FieldStoryOffset::new(position);
        assert_eq!(
            story
                .resolve_location(story.location(position).unwrap())
                .unwrap(),
            position
        );
    }
}

#[test]
fn rich_link_hidden_ranges_do_not_turn_into_visible_carets() {
    let original = plain("a😀b");
    let story = original
        .set_link(&range(&original, 1, 3), Some("\"x\""))
        .unwrap()
        .story;
    assert_eq!(
        story.visible_coverage(&range(&story, 1, 14)).unwrap(),
        VisibleCoverage::None
    );
    assert_eq!(
        story.visible_coverage(&range(&story, 1, 1)).unwrap(),
        VisibleCoverage::None
    );
    assert_eq!(
        story.visible_coverage(&range(&story, 1, 16)).unwrap(),
        VisibleCoverage::Text(VisibleOffset::new(1)..VisibleOffset::new(3))
    );
    assert_eq!(
        story.visible_coverage(&range(&story, 0, 14)).unwrap(),
        VisibleCoverage::Text(VisibleOffset::new(0)..VisibleOffset::new(1))
    );
    assert_eq!(
        story.visible_coverage(&range(&story, 17, 18)).unwrap(),
        VisibleCoverage::None
    );
    assert_eq!(
        story.visible_coverage(&range(&story, 17, 17)).unwrap(),
        VisibleCoverage::Caret(VisibleOffset::new(4))
    );
    assert_eq!(
        story.visible_coverage(&range(&story, 18, 18)).unwrap(),
        VisibleCoverage::None
    );
}

#[test]
fn rich_link_stale_ranges_fail_on_new_snapshots_and_old_snapshots_remain_readable() {
    let original = plain("alpha");
    let old_range = range(&original, 0, 5);
    let created = original.set_link(&old_range, Some("\"x\"")).unwrap();
    assert_eq!(
        created.story.get_link(&old_range).unwrap_err(),
        FieldStoryError::StaleRange
    );
    assert_eq!(
        created.story.text(&old_range).unwrap_err(),
        FieldStoryError::StaleRange
    );
    assert_eq!(
        created.story.visible_coverage(&old_range).unwrap_err(),
        FieldStoryError::StaleRange
    );
    assert_eq!(original.text(&old_range).unwrap(), "alpha");
    let updated = created
        .story
        .set_link(&created.range, Some("\"longer\""))
        .unwrap();
    assert_eq!(
        updated.story.get_link(&created.range).unwrap_err(),
        FieldStoryError::StaleRange
    );
    assert_eq!(
        created
            .story
            .get_link(&created.range)
            .unwrap()
            .argument
            .unwrap()
            .target(),
        "x"
    );
    assert_eq!(
        updated
            .story
            .get_link(&updated.range)
            .unwrap()
            .argument
            .unwrap()
            .target(),
        "longer"
    );
}

#[test]
fn rich_link_equal_target_imports_preserve_distinct_identity_until_explicit_append() {
    let story = FieldStory::from_fields(
        "abc",
        vec![],
        vec![field(7, 0, 1, "x"), field(8, 1, 2, "x")],
    )
    .unwrap();
    let lookup = story.get_link(&range(&story, 0, 28)).unwrap();
    assert_eq!(
        (lookup.range.start().get(), lookup.range.end().get()),
        (0, 14)
    );
    assert_eq!(story.fields().len(), 2);
    let no_op = story.set_link(&lookup.range, Some("\"x\"")).unwrap();
    assert!(no_op.change.is_none());
    assert!(story.same_snapshot(&no_op.story));
    let appended = story
        .set_link(&range(&story, 28, 29), Some("\"x\""))
        .unwrap();
    assert_eq!(appended.story.fields()[0], story.fields()[0]);
    assert_eq!(appended.story.fields()[1].id(), LinkFieldId::new(8));
    assert_eq!(
        appended.story.fields()[1].result(),
        VisibleOffset::new(1)..VisibleOffset::new(3)
    );
    assert_eq!(appended.story.fields().len(), 2);
    assert_eq!(
        (appended.range.start().get(), appended.range.end().get()),
        (28, 29)
    );
    assert_eq!(appended.change.unwrap().id, LinkFieldId::new(8));
}

#[test]
fn rich_link_target_update_preserves_unrelated_fields_and_returns_exact_changed_identity() {
    let story = FieldStory::from_fields(
        "ab cd",
        vec![],
        vec![field(21, 0, 2, "x"), field(89, 3, 5, "y")],
    )
    .unwrap();
    let before_second = story.fields()[1].clone();
    let changed = story
        .set_link(&range(&story, 13, 14), Some("\"new target\""))
        .unwrap();
    assert_eq!(changed.story.fields()[0].id(), LinkFieldId::new(21));
    assert_eq!(changed.story.fields()[1], before_second);
    assert_eq!(changed.story.visible_text(), "ab cd");
    let change = changed.change.unwrap();
    assert_eq!(change.id, LinkFieldId::new(21));
    assert_eq!(change.before.unwrap(), story.fields()[0]);
    assert_eq!(change.after.unwrap(), changed.story.fields()[0]);
    assert_eq!(
        (changed.range.start().get(), changed.range.end().get()),
        (22, 23)
    );
}

#[test]
fn rich_link_unlink_keeps_visible_text_and_never_reuses_a_retired_identity() {
    let original = plain("alpha");
    let first = original
        .set_link(&range(&original, 0, 5), Some("\"x\""))
        .unwrap();
    let retired = first.story.fields()[0].id();
    let cleared = first
        .story
        .set_link(&range(&first.story, 14, 16), None)
        .unwrap();
    assert!(cleared.story.fields().is_empty());
    assert_eq!(projected_text(&cleared.story), "alpha\r");
    assert_eq!(
        (cleared.range.start().get(), cleared.range.end().get()),
        (1, 3)
    );
    let change = cleared.change.unwrap();
    assert_eq!(change.id, retired);
    assert!(change.after.is_none());
    let second = cleared
        .story
        .set_link(&range(&cleared.story, 0, 5), Some("\"x\""))
        .unwrap();
    assert!(second.story.fields()[0].id().get() > retired.get());
    assert_eq!(first.story.fields()[0].id(), retired);
}

#[test]
fn rich_link_literal_instruction_words_are_ordinary_visible_text() {
    let original = plain("HYPERLINK \"x\"alpha");
    assert!(original.fields().is_empty());
    assert!(original
        .get_link(&range(&original, 0, 18))
        .unwrap()
        .argument
        .is_none());
    let changed = original
        .set_link(&range(&original, 0, 9), Some("\"x\""))
        .unwrap();
    assert_eq!(changed.story.visible_text(), "HYPERLINK \"x\"alpha");
    assert_eq!(
        projected_text(&changed.story),
        "HYPERLINK \"x\"HYPERLINK \"x\"alpha\r"
    );
}

#[test]
fn rich_link_source_checked_arguments_keep_non_uri_text_and_first_quote_termination() {
    let original = plain("alpha");
    let changed = original
        .set_link(&range(&original, 0, 5), Some("\u{fddf}\"not a URI\"tail\""))
        .unwrap();
    assert_eq!(changed.story.fields()[0].argument().target(), "not a URI");
    assert_eq!(
        changed
            .story
            .get_link(&changed.range)
            .unwrap()
            .argument
            .unwrap()
            .canonical(),
        "\u{fddf}\"not a URI\""
    );
    assert_eq!(
        projected_text(&changed.story),
        "\u{fddf}HYPERLINK \"not a URI\"alpha\r"
    );
}

#[test]
fn rich_link_invalid_arguments_preserve_the_original_story() {
    let story = plain("alpha");
    let selected = range(&story, 0, 5);
    for argument in [
        None,
        Some(""),
        Some("\"\""),
        Some("https://example"),
        Some("\"a\0b\""),
        Some("\"unterminated"),
    ] {
        assert_eq!(
            story.set_link(&selected, argument).unwrap_err(),
            FieldStoryError::InvalidArgument
        );
        assert_eq!(projected_text(&story), "alpha\r");
        assert!(story.fields().is_empty());
        assert_eq!(story.text(&selected).unwrap(), "alpha");
    }
}

#[test]
fn rich_link_contextual_empty_arguments_clear_the_complete_existing_field() {
    let original = plain("alpha");
    let created = original
        .set_link(&range(&original, 0, 5), Some("\"x\""))
        .unwrap();
    for argument in [None, Some(""), Some("\"\"")] {
        let cleared = created
            .story
            .set_link(&range(&created.story, 14, 16), argument)
            .unwrap();
        assert_eq!(cleared.story.visible_text(), "alpha");
        assert!(cleared.story.fields().is_empty());
        assert_eq!(
            (cleared.range.start().get(), cleared.range.end().get()),
            (1, 3)
        );
    }
    assert_eq!(created.story.fields().len(), 1);
}

#[test]
fn rich_link_cross_paragraph_result_is_one_field_and_marker_only_fails() {
    let original = plain("ab\rcd");
    assert_eq!(
        original
            .set_link(&range(&original, 2, 3), Some("\"x\""))
            .unwrap_err(),
        FieldStoryError::ParagraphOnly
    );
    let changed = original
        .set_link(&range(&original, 1, 4), Some("\"x\""))
        .unwrap();
    assert_eq!(changed.story.fields().len(), 1);
    assert_eq!(
        changed.story.fields()[0].result(),
        VisibleOffset::new(1)..VisibleOffset::new(4)
    );
    assert_eq!(changed.story.visible_text(), "ab\ncd");
    assert_eq!(projected_text(&changed.story), "aHYPERLINK \"x\"b\rcd\r");
    assert_eq!(
        changed.story.text(&changed.range).unwrap(),
        "HYPERLINK \"x\"b\rc"
    );
}

#[test]
fn rich_link_unmeasured_instruction_and_plain_prefix_requests_are_explicit() {
    let original = plain("p alpha");
    let story = original
        .set_link(&range(&original, 2, 7), Some("\"x\""))
        .unwrap()
        .story;
    assert!(matches!(
        story.get_link(&range(&story, 0, 20)),
        Err(FieldStoryError::Unsupported(_))
    ));
    for (start, end) in [(2, 3), (3, 4), (3, 20)] {
        let unsupported = range(&story, start, end);
        assert!(matches!(
            story.get_link(&unsupported),
            Err(FieldStoryError::Unsupported(_))
        ));
        assert!(matches!(
            story.set_link(&unsupported, Some("\"y\"")),
            Err(FieldStoryError::Unsupported(_))
        ));
    }
    assert_eq!(
        story
            .get_link(&range(&story, 2, 20))
            .unwrap()
            .argument
            .unwrap()
            .target(),
        "x"
    );
    assert!(story
        .get_link(&range(&story, 0, 1))
        .unwrap()
        .argument
        .is_none());
}

#[test]
fn rich_link_surrogate_interiors_remain_distinct_from_valid_combining_boundaries() {
    let original = plain("a😀e\u{301}Z");
    assert!(matches!(
        original.set_link(&range(&original, 2, 3), Some("\"x\"")),
        Err(FieldStoryError::Unsupported(_))
    ));
    let emoji = original
        .set_link(&range(&original, 1, 3), Some("\"x\""))
        .unwrap();
    assert!(matches!(
        emoji.story.get_link(&range(&emoji.story, 15, 16)),
        Err(FieldStoryError::Unsupported(_))
    ));
    let accent = original
        .set_link(&range(&original, 4, 5), Some("\"x\""))
        .unwrap();
    assert_eq!(
        accent.story.fields()[0].result(),
        VisibleOffset::new(4)..VisibleOffset::new(5)
    );
    assert_eq!(
        accent
            .story
            .visible_slice(VisibleOffset::new(4)..VisibleOffset::new(5))
            .unwrap(),
        "\u{301}"
    );
}

#[test]
fn rich_link_final_mark_and_unmeasured_empty_sentinel_have_explicit_boundaries() {
    let story = plain("abc");
    assert!(story
        .get_link(&range(&story, 3, 4))
        .unwrap()
        .argument
        .is_none());
    assert!(matches!(
        story.set_link(&range(&story, 3, 4), Some("\"x\"")),
        Err(FieldStoryError::Unsupported(_))
    ));
    assert_eq!(
        story
            .set_link(&range(&story, 3, 3), Some("\"x\""))
            .unwrap_err(),
        FieldStoryError::InvalidArgument
    );
    assert!(matches!(
        story.set_link(&range(&story, 0, 1), Some("\u{fddf}\"\"")),
        Err(FieldStoryError::Unsupported(_))
    ));
}

#[test]
fn rich_link_projection_constructor_rejects_ambiguous_identity_and_invalid_ranges() {
    assert!(matches!(
        FieldStory::from_fields(
            "abc",
            vec![],
            vec![field(1, 0, 2, "x"), field(1, 2, 3, "y")]
        ),
        Err(FieldStoryError::InvalidField(_))
    ));
    assert!(matches!(
        FieldStory::from_fields(
            "abc",
            vec![],
            vec![field(1, 0, 2, "x"), field(2, 1, 3, "y")]
        ),
        Err(FieldStoryError::InvalidField(_))
    ));
    assert!(matches!(
        FieldStory::from_fields("abc", vec![], vec![field(0, 0, 1, "x")]),
        Err(FieldStoryError::InvalidField(_))
    ));
    assert!(matches!(
        FieldStory::from_fields("abc", vec![], vec![field(1, 2, 2, "x")]),
        Err(FieldStoryError::InvalidField(_))
    ));
    assert!(matches!(
        FieldStory::from_fields("abc", vec![], vec![field(1, 2, usize::MAX, "x")]),
        Err(FieldStoryError::InvalidField(_))
    ));
    assert_eq!(
        FieldStory::from_fields("a😀b", vec![], vec![field(1, 2, 3, "x")]).unwrap_err(),
        FieldStoryError::InvalidUtf16Boundary
    );
    assert_eq!(
        FieldStory::from_fields("abc", vec![], vec![field(u64::MAX, 0, 1, "x")]).unwrap_err(),
        FieldStoryError::CapacityExceeded
    );
}

#[test]
fn rich_link_projection_validates_explicit_breaks_and_tagged_positions() {
    for breaks in [
        vec![VisibleOffset::new(0)],
        vec![VisibleOffset::new(1), VisibleOffset::new(1)],
        vec![VisibleOffset::new(3), VisibleOffset::new(1)],
        vec![VisibleOffset::new(usize::MAX)],
    ] {
        assert_eq!(
            FieldStory::new("a\nb\nc", breaks).unwrap_err(),
            FieldStoryError::InvalidParagraphBreak
        );
    }
    let story = plain("abc");
    assert_eq!(
        story
            .range(FieldStoryOffset::new(2), FieldStoryOffset::new(1))
            .unwrap_err(),
        FieldStoryError::InvalidRange
    );
    assert_eq!(
        story
            .range(FieldStoryOffset::new(0), FieldStoryOffset::new(5))
            .unwrap_err(),
        FieldStoryError::OutOfBounds
    );
    assert_eq!(
        story.location(FieldStoryOffset::new(5)).unwrap_err(),
        FieldStoryError::OutOfBounds
    );
    assert_eq!(
        story
            .project_visible(VisibleOffset::new(4), InstructionBoundary::Before)
            .unwrap_err(),
        FieldStoryError::OutOfBounds
    );
    assert_eq!(
        story
            .resolve_location(FieldLocation::Instruction {
                field: LinkFieldId::new(99),
                utf16_offset: 0
            })
            .unwrap_err(),
        FieldStoryError::UnknownField
    );
}

#[test]
fn rich_link_crossing_setter_failure_preserves_both_existing_fields() {
    let story = FieldStory::from_fields(
        "ab cd",
        vec![],
        vec![field(1, 0, 2, "x"), field(2, 3, 5, "y")],
    )
    .unwrap();
    let before = projected_text(&story);
    let fields = story.fields().to_vec();
    assert_eq!(
        story
            .set_link(&range(&story, 14, 30), Some("\"z\""))
            .unwrap_err(),
        FieldStoryError::InvalidArgument
    );
    assert_eq!(projected_text(&story), before);
    assert_eq!(story.fields(), fields);
}

#[test]
fn rich_link_empty_story_retains_its_synthetic_mark_without_creating_a_field() {
    let story = plain("");
    assert_eq!(story.visible_text(), "");
    assert_eq!(story.length(), 1);
    assert_eq!(projected_text(&story), "\r");
    assert!(story
        .get_link(&range(&story, 0, 0))
        .unwrap()
        .argument
        .is_none());
    assert_eq!(
        story
            .set_link(&range(&story, 0, 0), Some("\"x\""))
            .unwrap_err(),
        FieldStoryError::InvalidArgument
    );
}

#[test]
fn rich_link_unmeasured_prepend_and_equal_target_gap_preserve_existing_fields() {
    for (text, fields, start, end) in [
        ("ab", vec![field(9, 1, 2, "x")], 0, 1),
        (
            "abc",
            vec![field(7, 0, 1, "x"), field(9, 2, 3, "x")],
            14,
            15,
        ),
    ] {
        let story = FieldStory::from_fields(text, vec![], fields.clone()).unwrap();
        let before = projected_text(&story);
        let acted = range(&story, start, end);
        assert!(matches!(
            story.set_link(&acted, Some("\"x\"")),
            Err(FieldStoryError::Unsupported(_))
        ));
        assert_eq!(story.fields(), fields.as_slice());
        assert_eq!(projected_text(&story), before);
        assert_eq!(
            story.text(&acted).unwrap(),
            if start == 0 { "a" } else { "b" }
        );

        // A distinct target does not need an unmeasured same-target merge.
        let distinct = story.set_link(&acted, Some("\"y\"")).unwrap();
        for original in &fields {
            assert!(distinct.story.fields().contains(original));
        }
        assert_eq!(distinct.story.fields().len(), fields.len() + 1);
        assert_eq!(distinct.story.visible_text(), text);
    }
}

#[test]
fn rich_link_projection_import_preserves_retired_identity_watermark() {
    let story = FieldStory::from_fields_with_next_id("ab", Vec::new(), Vec::new(), 42).unwrap();
    assert_eq!(story.next_field_id_watermark(), 42);
    let changed = story.set_link(&range(&story, 0, 2), Some("\"x\"")).unwrap();
    assert_eq!(changed.story.fields()[0].id().get(), 42);
    assert_eq!(changed.story.next_field_id_watermark(), 43);
    assert!(FieldStory::from_fields_with_next_id("ab", Vec::new(), Vec::new(), 0).is_err());
    assert!(
        FieldStory::from_fields_with_next_id("ab", Vec::new(), vec![field(2, 0, 2, "x")], 2)
            .is_err()
    );
}
