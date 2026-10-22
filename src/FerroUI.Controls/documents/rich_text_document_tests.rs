use super::*;
use crate::test_support::test_scope;
use ferroui_base::media::{FontStyle, FontWeight};
use ferroui_base::{Ref, Thickness};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn utf16_offsets_reject_surrogate_splits_without_mutation() {
    let document = RichTextDocument::from_text("a😀b");
    assert_eq!(document.text_length(), 4);
    assert_eq!(document.get_text(1..3).unwrap(), "😀");
    let before = document.snapshot();
    let revision = document.revision();
    assert!(matches!(
        document.replace_text(2..3, "x"),
        Err(DocumentError::InvalidUtf16Boundary { offset: 2 })
    ));
    assert_eq!(document.snapshot(), before);
    assert_eq!(document.revision(), revision);
}

#[test]
fn paragraph_splice_merges_boundaries_and_restores_snapshot() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("first\r\nsecond\rthird");
    assert_eq!(document.text(), "first\nsecond\nthird");
    let snapshot = document.snapshot();
    document.replace_text(3..9, "X\nY").unwrap();
    assert_eq!(document.text(), "firX\nYond\nthird");
    document.restore(&snapshot).unwrap();
    assert_eq!(document.text(), "first\nsecond\nthird");
    assert_eq!(document.snapshot(), snapshot);
}

#[test]
fn nested_formatting_survives_partial_edits_and_format_application() {
    let _scope = test_scope();
    let document = RichTextDocument::new();
    let paragraph = Paragraph::new();
    let bold = Bold::new();
    let italic = Italic::new();
    italic.add_child_text("abcdef");
    bold.add_child(italic);
    paragraph.inlines().add(bold);
    document.blocks().add(paragraph);
    document
        .apply_format(
            2..4,
            &CharacterFormat {
                font_weight: Some(FontWeight::Normal),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        document.character_format_at(1).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        document.character_format_at(3).unwrap().font_weight,
        Some(FontWeight::Normal)
    );
    assert_eq!(
        document.character_format_at(5).unwrap().font_style,
        Some(FontStyle::Italic)
    );
    document.replace_text(3..3, "😀").unwrap();
    assert_eq!(document.text(), "abc😀def");
    assert_eq!(
        document.character_format_at(5).unwrap().font_style,
        Some(FontStyle::Italic)
    );
}

#[test]
fn semantic_snapshot_containers_keep_defaults_under_inherited_parent_formatting() {
    let _scope = test_scope();
    let document = RichTextDocument::new();
    let wrapped = InlineSnapshot {
        format: CharacterFormat::default(),
        kind: InlineSnapshotKind::Bold(vec![InlineSnapshot {
            format: CharacterFormat::default(),
            kind: InlineSnapshotKind::Italic(vec![InlineSnapshot {
                format: CharacterFormat::default(),
                kind: InlineSnapshotKind::Underline(vec![InlineSnapshot {
                    format: CharacterFormat::default(),
                    kind: InlineSnapshotKind::Run("styled".to_owned()),
                }]),
            }]),
        }]),
    };
    document
        .restore(&DocumentSnapshot {
            paragraphs: vec![ParagraphSnapshot {
                format: ParagraphFormat {
                    character: CharacterFormat {
                        font_weight: Some(FontWeight::Normal),
                        font_style: Some(FontStyle::Normal),
                        text_decorations: Some(None),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                inlines: vec![wrapped],
            }],
            ..Default::default()
        })
        .unwrap();
    let format = document.character_format_at(1).unwrap();
    assert_eq!(format.font_weight, Some(FontWeight::Bold));
    assert_eq!(format.font_style, Some(FontStyle::Italic));
    assert_eq!(format.text_decorations.unwrap().unwrap().len(), 1);
    let fragment = document.slice(0..6).unwrap();
    let target = RichTextDocument::new();
    target.replace_range(0..0, &fragment).unwrap();
    assert_eq!(
        target.character_format_at(1).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    target
        .apply_format(
            0..6,
            &CharacterFormat {
                font_weight: Some(FontWeight::Normal),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        target.character_format_at(1).unwrap().font_weight,
        Some(FontWeight::Normal)
    );
}

#[test]
fn hyperlink_default_formatting_and_explicit_overrides_round_trip() {
    use ferroui_base::media::{TextDecorationCollection, TextDecorationLocation, TextDecorations};

    let _scope = test_scope();
    let document = RichTextDocument::new();
    let paragraph = Paragraph::new();
    let parent = Span::new();
    parent.set_text_decorations(Some(TextDecorations::strikethrough()));
    let link = Hyperlink::with_text("link", Some("https://example.test"));
    parent.inlines().add(link.clone());
    paragraph.inlines().add(parent);
    document.blocks().add(paragraph);

    let assigned = CharacterFormat::from_element(&link);
    assert_eq!(
        assigned,
        CharacterFormat::default(),
        "the intrinsic current value must not become a local assignment"
    );
    let effective = document.character_format_at(0).unwrap();
    let decorations = effective.text_decorations.unwrap().unwrap();
    assert_eq!(decorations.len(), 1);
    assert_eq!(
        decorations.get(0).location(),
        TextDecorationLocation::Underline
    );
    let before = document.snapshot();
    document.restore(&before).unwrap();
    assert_eq!(document.snapshot(), before);

    // Explicit null, empty and custom local decorations override both the
    // constructor value and the parent's inherited strikethrough.
    for decorations in [
        None,
        Some(TextDecorationCollection::new()),
        Some(TextDecorations::overline()),
    ] {
        link.set_text_decorations(decorations.clone());
        let expected = CharacterFormat {
            text_decorations: Some(decorations),
            ..Default::default()
        };
        assert_eq!(CharacterFormat::from_element(&link), expected);
        let before = document.snapshot();
        document.restore(&before).unwrap();
        assert_eq!(document.snapshot(), before);
        assert_eq!(CharacterFormat::from_element(&link), expected);
    }

    link.set_underline_style(UnderlineStyle::None);
    assert!(link.text_decorations().is_none());
    let before = document.snapshot();
    document.restore(&before).unwrap();
    assert_eq!(document.snapshot(), before);
    assert!(link.text_decorations().is_none());
    link.set_underline_style(UnderlineStyle::Single);
    assert_eq!(
        CharacterFormat::from_element(&link),
        CharacterFormat {
            text_decorations: Some(Some(TextDecorations::underline())),
            ..Default::default()
        },
        "changing UnderlineStyle explicitly updates the assigned decorations"
    );
    let before = document.snapshot();
    document.restore(&before).unwrap();
    assert_eq!(document.snapshot(), before);
}

#[test]
fn semantic_current_defaults_preserve_assignments_and_style_precedence_after_restore() {
    use ferroui_base::data::BindingPriority;
    use ferroui_base::media::{TextDecorationLocation, TextDecorations};

    let _scope = test_scope();
    let document = RichTextDocument::new();
    let paragraph = Paragraph::new();
    paragraph.set_font_weight(FontWeight::Normal);
    paragraph.set_font_style(FontStyle::Normal);
    paragraph.set_value(Inline::text_decorations_property(), None);
    let bold = Bold::new();
    let italic = Italic::new();
    let underline = Underline::new();
    let link = Hyperlink::with_text("hh", Some("https://example.test"));
    bold.add_child_text("bb");
    italic.add_child_text("ii");
    underline.add_child_text("uu");
    paragraph.inlines().add(bold.clone());
    paragraph.inlines().add(italic.clone());
    paragraph.inlines().add(underline.clone());
    paragraph.inlines().add(link.clone());
    document.blocks().add(paragraph);

    let before = document.snapshot();
    for inline in &before.paragraphs[0].inlines {
        assert_eq!(inline.format, CharacterFormat::default());
    }
    document.restore(&before).unwrap();
    assert_eq!(
        document.snapshot(),
        before,
        "reused semantic elements retain assigned state"
    );
    let fresh = RichTextDocument::new();
    fresh.restore(&before).unwrap();
    assert_eq!(
        fresh.snapshot(),
        before,
        "new semantic elements retain assigned state"
    );
    for model in [&document, &fresh] {
        assert_eq!(
            model.character_format_at(1).unwrap().font_weight,
            Some(FontWeight::Bold)
        );
        assert_eq!(
            model.character_format_at(3).unwrap().font_style,
            Some(FontStyle::Italic)
        );
        for offset in [5, 7] {
            let decorations = model
                .character_format_at(offset)
                .unwrap()
                .text_decorations
                .unwrap()
                .unwrap();
            assert_eq!(decorations.len(), 1);
            assert_eq!(
                decorations.get(0).location(),
                TextDecorationLocation::Underline
            );
        }
    }

    // A restore must not promote constructor defaults above application styles.
    bold.set_value_with_priority(
        TextElement::font_weight_property(),
        FontWeight::Light,
        BindingPriority::Style,
    );
    italic.set_value_with_priority(
        TextElement::font_style_property(),
        FontStyle::Oblique,
        BindingPriority::Style,
    );
    underline.set_value_with_priority(
        Inline::text_decorations_property(),
        None,
        BindingPriority::Style,
    );
    link.set_value_with_priority(
        Inline::text_decorations_property(),
        Some(TextDecorations::overline()),
        BindingPriority::Style,
    );
    assert_eq!(
        document.character_format_at(1).unwrap().font_weight,
        Some(FontWeight::Light)
    );
    assert_eq!(
        document.character_format_at(3).unwrap().font_style,
        Some(FontStyle::Oblique)
    );
    assert!(matches!(
        document.character_format_at(5).unwrap().text_decorations,
        Some(None)
    ));
    let decorations = document
        .character_format_at(7)
        .unwrap()
        .text_decorations
        .unwrap()
        .unwrap();
    assert_eq!(
        decorations.get(0).location(),
        TextDecorationLocation::Overline
    );

    // Explicit values remain exact and take precedence over those styles.
    bold.set_font_weight(FontWeight::Normal);
    italic.set_font_style(FontStyle::Normal);
    underline.set_text_decorations(None);
    link.set_text_decorations(None);
    let explicit = document.snapshot();
    document.restore(&explicit).unwrap();
    assert_eq!(document.snapshot(), explicit);
    assert_eq!(
        document.character_format_at(1).unwrap().font_weight,
        Some(FontWeight::Normal)
    );
    assert_eq!(
        document.character_format_at(3).unwrap().font_style,
        Some(FontStyle::Normal)
    );
    for offset in [5, 7] {
        assert!(matches!(
            document
                .character_format_at(offset)
                .unwrap()
                .text_decorations,
            Some(None)
        ));
    }
}

#[test]
fn nested_edit_batches_raise_one_revision_and_one_event() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("abc");
    let events = Rc::new(RefCell::new(Vec::new()));
    let output = events.clone();
    document.changed(move |event| output.borrow_mut().push(event.clone()));
    {
        let _outer = document.begin_edit();
        document.replace_text(1..1, "x").unwrap();
        {
            let _inner = document.begin_edit();
            document.replace_text(2..2, "y").unwrap();
        }
        assert_eq!(document.text(), "axybc");
        assert!(events.borrow().is_empty());
    }
    assert_eq!(document.revision(), 1);
    assert_eq!(events.borrow().len(), 1);
    assert_eq!(events.borrow()[0].range, 1..1);
    assert_eq!(events.borrow()[0].inserted_length, 2);
}

#[test]
fn live_pointers_have_gravity_and_weak_document_ownership() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("ab");
    let left = document.create_pointer(1, TextGravity::Backward).unwrap();
    let right = document.create_pointer(1, TextGravity::Forward).unwrap();
    document.replace_text(1..1, "😀").unwrap();
    assert_eq!(left.offset(), 1);
    assert_eq!(right.offset(), 3);
    drop(document);
    assert!(!left.is_valid());
    assert!(!right.is_valid());
}

#[test]
fn grapheme_navigation_uses_formatter_unicode_boundaries() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("A👩‍👩‍👧‍👦e\u{301}Z");
    let family_end = 1 + "👩‍👩‍👧‍👦".encode_utf16().count();
    assert_eq!(document.next_grapheme(1), family_end);
    assert_eq!(document.previous_grapheme(family_end), 1);
    assert_eq!(document.next_grapheme(family_end), family_end + 2);
    assert!(!document.is_grapheme_boundary(family_end + 1));
    assert_eq!(document.next_grapheme(usize::MAX), document.text_length());
}

#[test]
fn invalid_format_is_atomic_and_document_handle_is_shared() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("hello");
    let other = document.clone();
    let snapshot = document.snapshot();
    assert!(
        document
            .apply_format(
                0..2,
                &CharacterFormat {
                    font_size: Some(f64::INFINITY),
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(document.snapshot(), snapshot);
    assert_eq!(document.revision(), 0);
    other.replace_text(0..5, "shared").unwrap();
    assert_eq!(document.text(), "shared");
    assert_eq!(document, other);
}

#[test]
fn normal_typing_retains_run_and_surrounding_content_identity() {
    let _scope = test_scope();
    let document = RichTextDocument::new();
    let paragraph = Paragraph::new();
    let run = Run::with_text(Some("ab"));
    let control = crate::Button::new();
    let container = InlineUIContainer::with_child(control.clone());
    paragraph.inlines().add(run.clone());
    paragraph.inlines().add(container.clone());
    document.blocks().add(paragraph.clone());
    document.replace_text(1..1, "😀").unwrap();
    assert_eq!(run.text().as_deref(), Some("a😀b"));
    assert_eq!(paragraph.inlines().get(0), run.upcast::<Inline>());
    assert_eq!(paragraph.inlines().get(1), container.upcast::<Inline>());
    assert_eq!(document.text(), "a😀b\u{fffc}");
}

#[test]
fn embedded_control_offsets_and_snapshot_restore_preserve_control() {
    let _scope = test_scope();
    let document = RichTextDocument::new();
    let paragraph = Paragraph::new();
    let child = crate::Button::new();
    paragraph.add_child_text("a");
    paragraph.inlines().add_control(child.clone());
    paragraph.add_child_text("b");
    paragraph.inlines().add(InlineUIContainer::new());
    document.blocks().add(paragraph);
    assert_eq!(document.text(), "a\u{fffc}b");
    assert_eq!(document.text_length(), 3);
    let snapshot = document.snapshot();
    document.replace_text(1..2, "x\ny").unwrap();
    assert_eq!(document.text(), "ax\nyb");
    assert!(child.parent().is_none());
    document.restore(&snapshot).unwrap();
    assert_eq!(document.text(), "a\u{fffc}b");
    let container = document.paragraphs()[0]
        .inlines()
        .get(1)
        .cast::<InlineUIContainer>()
        .unwrap();
    assert_eq!(container.child(), Some(child.upcast::<crate::Control>()));
}

#[test]
fn duplicate_embedded_controls_and_foreign_parents_are_rejected_atomically() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("original");
    let child: Ref<crate::Control> = crate::Button::new().upcast();
    let control = InlineSnapshot {
        format: CharacterFormat::default(),
        kind: InlineSnapshotKind::Embedded(Some(child.clone())),
    };
    let duplicated = DocumentSnapshot {
        paragraphs: vec![ParagraphSnapshot {
            format: ParagraphFormat::default(),
            inlines: vec![control.clone(), control.clone()],
        }],
        ..Default::default()
    };
    assert_eq!(
        document.restore(&duplicated),
        Err(DocumentError::DuplicateEmbeddedControl)
    );
    let foreign = InlineUIContainer::with_child(child);
    let other = DocumentSnapshot {
        paragraphs: vec![ParagraphSnapshot {
            format: ParagraphFormat::default(),
            inlines: vec![control],
        }],
        ..Default::default()
    };
    assert_eq!(
        document.restore(&other),
        Err(DocumentError::EmbeddedControlAlreadyParented)
    );
    assert_eq!(document.text(), "original");
    assert_eq!(document.revision(), 0);
    assert!(foreign.child().is_some());
}

#[test]
fn owned_snapshot_freezes_font_features_and_decoration_values() {
    let _scope = test_scope();
    use ferroui_base::media::{
        FontFeature, FontFeatureCollection, TextDecoration, TextDecorationCollection,
        TextDecorationLocation,
    };
    let document = RichTextDocument::new();
    let paragraph = Paragraph::with_text("styled");
    let run = paragraph.inlines().get(0).cast::<Run>().unwrap();
    let features = FontFeatureCollection::from_items([FontFeature::parse("liga=0")]);
    let decoration = TextDecoration::new();
    decoration.set_location(TextDecorationLocation::Underline);
    run.set_font_features(Some(features.clone()));
    run.set_text_decorations(Some(TextDecorationCollection::from_items([
        decoration.clone()
    ])));
    document.blocks().add(paragraph);
    let snapshot = document.snapshot();
    features.add(FontFeature::parse("kern=0"));
    decoration.set_location(TextDecorationLocation::Strikethrough);
    let format = &snapshot.paragraphs[0].inlines[0].format;
    assert_eq!(
        format
            .font_features
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        format
            .text_decorations
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .get(0)
            .location(),
        TextDecorationLocation::Underline
    );
    document.restore(&snapshot).unwrap();
    let restored = document.paragraphs()[0]
        .inlines()
        .get(0)
        .cast::<Run>()
        .unwrap();
    restored
        .font_features()
        .unwrap()
        .add(FontFeature::parse("calt=0"));
    assert_eq!(
        format
            .font_features
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn text_pointers_track_disjoint_changes_inside_one_batch() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("abc");
    let pointer = document.create_pointer(2, TextGravity::Forward).unwrap();
    {
        let _batch = document.begin_edit();
        document.replace_text(0..0, "X").unwrap();
        document.replace_text(4..4, "Y").unwrap();
    }
    assert_eq!(document.text(), "XabcY");
    assert_eq!(pointer.offset(), 3);
    assert_eq!(document.revision(), 1);
}

#[test]
fn reentrant_handlers_receive_revisions_in_order() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("abc");
    let weak = document.downgrade();
    document.changed(move |event| {
        if event.revision == 1 {
            weak.upgrade().unwrap().replace_text(0..0, "Y").unwrap();
        }
    });
    let revisions = Rc::new(RefCell::new(Vec::new()));
    let target = revisions.clone();
    document.changed(move |event| target.borrow_mut().push(event.revision));
    let pointer = document.create_pointer(2, TextGravity::Forward).unwrap();
    document.replace_text(0..0, "X").unwrap();
    assert_eq!(&*revisions.borrow(), &[1, 2]);
    assert_eq!(pointer.offset(), 4);
    assert_eq!(document.text(), "YXabc");
}

#[test]
fn hyperlink_events_cancellation_input_and_split_behavior_are_preserved() {
    let _scope = test_scope();
    use ferroui_base::input::Key;
    use std::cell::Cell;
    let document = RichTextDocument::new();
    let paragraph = Paragraph::new();
    let link = Hyperlink::with_text("abcd", Some("https://example.invalid"));
    let calls = Rc::new(Cell::new(0));
    let target = calls.clone();
    link.click(move |args| {
        target.set(target.get() + 1);
        args.set_handled(true);
    });
    link.set_tab_index(7);
    paragraph.inlines().add(link.clone());
    document.blocks().add(paragraph);
    let revision = document.revision();
    link.set_is_pointer_over(true);
    link.pointer_pressed();
    assert!(link.pointer_released(true));
    assert_eq!(document.revision(), revision);
    assert_eq!(calls.get(), 1);
    document.replace_text(2..2, "\n").unwrap();
    let mut links = Vec::new();
    for paragraph in document.paragraphs() {
        for inline in paragraph.inlines().snapshot().iter() {
            if let Some(link) = inline.cast::<Hyperlink>() {
                links.push(link);
            }
        }
    }
    assert!(links.len() >= 2);
    for link in links {
        assert_eq!(link.tab_index(), 7);
        assert!(link.key_down(Key::Enter));
        assert!(link.key_up(Key::Enter));
    }
    assert!(calls.get() >= 3);
}

#[test]
fn paragraph_margin_validation_matches_nonnegative_source_contract() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("a\nb");
    let snapshot = document.snapshot();
    assert!(
        document
            .apply_paragraph_format(
                0..3,
                &ParagraphFormat {
                    margin: Some(Thickness::uniform(-1.0)),
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(document.snapshot(), snapshot);
    assert_eq!(document.revision(), 0);
}

#[test]
fn collection_back_pointers_do_not_keep_blocks_alive() {
    let _scope = test_scope();
    let paragraph = Paragraph::with_text("content");
    let weak_paragraph = paragraph.downgrade();
    let document = RichTextDocument::new();
    document.blocks().add(paragraph);
    let weak_document = document.downgrade();
    drop(document);
    assert!(weak_document.upgrade().is_none());
    assert!(weak_paragraph.upgrade().is_none());
}

#[test]
fn range_slices_keep_nested_containers_and_paragraph_boundaries() {
    let _scope = test_scope();
    let document = RichTextDocument::new();
    let paragraph = Paragraph::new();
    let bold = Bold::new();
    bold.add_child_text("ab😀cd");
    paragraph.inlines().add(bold);
    document.blocks().add(paragraph);
    document.blocks().add(Paragraph::with_text("tail"));
    let fragment = document.slice(1..8).unwrap();
    assert_eq!(fragment.text(), "b😀cd\nt");
    assert!(matches!(
        fragment.paragraphs[0].inlines[0].kind,
        InlineSnapshotKind::Bold(_)
    ));
    assert_eq!(fragment.paragraphs.len(), 2);
    let separator = document.slice(6..7).unwrap();
    assert_eq!(separator.text(), "\n");
}

#[test]
fn shared_formatting_collections_invalidate_and_unsubscribe_on_replacement() {
    let _scope = test_scope();
    use ferroui_base::media::{
        FontFeature, FontFeatureCollection, TextDecoration, TextDecorationCollection,
    };
    let document = RichTextDocument::from_text("abc");
    let run = document.paragraphs()[0]
        .inlines()
        .get(0)
        .cast::<Run>()
        .unwrap();
    let features = FontFeatureCollection::new();
    run.set_font_features(Some(features.clone()));
    let revision = document.revision();
    features.add(FontFeature::parse("liga=0"));
    assert!(document.revision() > revision);
    run.set_font_features(None);
    let revision = document.revision();
    features.add(FontFeature::parse("kern=0"));
    assert_eq!(document.revision(), revision);

    let decoration = TextDecoration::new();
    let decorations = TextDecorationCollection::from_items([decoration.clone()]);
    run.set_text_decorations(Some(decorations.clone()));
    let revision = document.revision();
    decoration.set_stroke_thickness(3.0);
    assert!(document.revision() > revision);
    assert!(decoration.property_changed_subscriber_count() > 0);
    run.set_text_decorations(None);
    assert_eq!(decoration.property_changed_subscriber_count(), 0);
    let revision = document.revision();
    decoration.set_stroke_thickness(4.0);
    decorations.add(TextDecoration::new());
    assert_eq!(document.revision(), revision);
}

#[test]
fn word_navigation_skips_inter_word_whitespace_for_editing() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("one two\u{2003}three");
    assert_eq!(document.next_word(0), 4);
    assert_eq!(document.next_word(3), 4);
    assert_eq!(document.previous_word(4), 0);
    assert_eq!(document.previous_word(8), 4);
    assert_eq!(document.next_word(4), 8);
}

#[test]
fn empty_paragraph_keeps_insertion_format_after_split() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("abc");
    document
        .apply_format(
            0..3,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            },
        )
        .unwrap();
    document.replace_text(3..3, "\n").unwrap();
    assert_eq!(document.text(), "abc\n");
    assert_eq!(
        document.character_format_at(4).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    document.replace_text(4..4, "x").unwrap();
    assert_eq!(document.text(), "abc\nx");
    assert_eq!(
        document.character_format_at(5).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
}
