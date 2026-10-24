use super::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn mixed_delimiters_and_utf16_offsets() {
    let document = TextDocument::from_text("a😀\r\nb\rc\n");
    assert_eq!(document.text_length(), 9);
    let lines: Vec<_> = document.lines().collect();
    assert_eq!(
        lines,
        vec![
            DocumentLine {
                line_number: 1,
                offset: 0,
                length: 3,
                delimiter_length: 2
            },
            DocumentLine {
                line_number: 2,
                offset: 5,
                length: 1,
                delimiter_length: 1
            },
            DocumentLine {
                line_number: 3,
                offset: 7,
                length: 1,
                delimiter_length: 1
            },
            DocumentLine {
                line_number: 4,
                offset: 9,
                length: 0,
                delimiter_length: 0
            },
        ]
    );
    assert_eq!(document.get_location(4).unwrap(), TextLocation::new(1, 5));
    assert_eq!(document.get_location(5).unwrap(), TextLocation::new(2, 1));
    assert_eq!(document.get_offset(TextLocation::new(1, 100)).unwrap(), 3);
    assert_eq!(document.get_offset(TextLocation::new(1, 0)).unwrap(), 0);
    assert!(document.get_line_by_number(0).is_err());
    assert!(document.get_location(10).is_err());
}

#[test]
fn empty_document_has_one_empty_line() {
    let document = TextDocument::new();
    assert_eq!(document.line_count(), 1);
    assert_eq!(document.get_line_by_offset(0).unwrap().length, 0);
    assert_eq!(document.get_location(0).unwrap(), TextLocation::new(1, 1));
    assert!(!document.undo().unwrap());
    assert!(!document.redo().unwrap());
}

#[test]
fn utf16_lone_surrogates_are_preserved_in_storage_history_and_snapshots() {
    let document = TextDocument::from_utf16(&[0xd800, 13, 10, 0xdc00]);
    let snapshot = document.create_snapshot();
    document.replace_utf16(1, 2, &[0xdfff]).unwrap();
    assert_eq!(
        document.get_text_utf16(0, 3).unwrap(),
        [0xd800, 0xdfff, 0xdc00]
    );
    document.undo().unwrap();
    assert_eq!(document.get_text_utf16(0, 4).unwrap(), snapshot.to_utf16());
    assert_eq!(snapshot.to_utf16(), [0xd800, 13, 10, 0xdc00]);
}

#[test]
fn anchor_in_empty_document() {
    let document = TextDocument::new();
    let before = document.create_anchor(0).unwrap();
    let after = document.create_anchor(0).unwrap();
    before.set_movement_type(AnchorMovementType::BeforeInsertion);
    after.set_movement_type(AnchorMovementType::AfterInsertion);
    document.insert(0, "x").unwrap();
    assert_eq!(before.offset().unwrap(), 0);
    assert_eq!(after.offset().unwrap(), 1);
}

#[test]
fn anchors_survive_deletion_boundaries_and_delete_only_strict_interior() {
    let document = TextDocument::from_text("          ");
    let surviving: Vec<_> = (0..=10)
        .map(|i| {
            let a = document.create_anchor(i).unwrap();
            a.set_survive_deletion(true);
            a
        })
        .collect();
    let deletable: Vec<_> = (0..=10)
        .map(|i| document.create_anchor(i).unwrap())
        .collect();
    document.remove(1, 8).unwrap();
    for i in 0..=10 {
        let expected = if i <= 1 {
            i
        } else if i <= 8 {
            1
        } else {
            i - 8
        };
        assert_eq!(surviving[i].offset().unwrap(), expected);
        assert_eq!(deletable[i].is_deleted(), (2..=8).contains(&i));
        if !deletable[i].is_deleted() {
            assert_eq!(deletable[i].offset().unwrap(), expected);
        }
    }
}

#[test]
fn move_anchors_during_replacement() {
    let document = TextDocument::from_text("abcd");
    let start = document.create_anchor(1).unwrap();
    let deleted = document.create_anchor(2).unwrap();
    let left = document.create_anchor(2).unwrap();
    let right = document.create_anchor(2).unwrap();
    let end = document.create_anchor(3).unwrap();
    left.set_survive_deletion(true);
    right.set_survive_deletion(true);
    left.set_movement_type(AnchorMovementType::BeforeInsertion);
    right.set_movement_type(AnchorMovementType::AfterInsertion);
    document.replace(1, 2, "BxC").unwrap();
    assert_eq!(start.offset().unwrap(), 1);
    assert!(deleted.is_deleted());
    assert_eq!(left.offset().unwrap(), 1);
    assert_eq!(right.offset().unwrap(), 4);
    assert_eq!(end.offset().unwrap(), 4);
}

#[test]
fn character_replace_retains_interior_and_clamps_shrinking_anchors() {
    let document = TextDocument::from_text("0123456789");
    let anchors: Vec<_> = (0..=10)
        .map(|i| document.create_anchor(i).unwrap())
        .collect();
    document
        .replace_with_mapping(2, 6, "abc", OffsetChangeMappingType::CharacterReplace)
        .unwrap();
    for (i, a) in anchors.iter().enumerate() {
        assert!(!a.is_deleted());
        assert_eq!(
            a.offset().unwrap(),
            if i <= 5 {
                i
            } else if i <= 8 {
                5
            } else {
                i - 3
            }
        );
    }
    document.undo().unwrap();
    assert_eq!(document.text(), "0123456789");
}

#[test]
fn character_replace_empty_replacement_is_normal_deletion() {
    let document = TextDocument::from_text("abcd");
    let anchor = document.create_anchor(2).unwrap();
    document
        .replace_with_mapping(1, 2, "", OffsetChangeMappingType::CharacterReplace)
        .unwrap();
    assert!(anchor.is_deleted());
}

#[test]
fn remove_and_insert_applies_affinity_at_both_boundaries() {
    let document = TextDocument::from_text("abcd");
    let start = document.create_anchor(1).unwrap();
    let end = document.create_anchor(3).unwrap();
    end.set_movement_type(AnchorMovementType::BeforeInsertion);
    document
        .replace_with_mapping(1, 2, "xyz", OffsetChangeMappingType::RemoveAndInsert)
        .unwrap();
    assert_eq!(start.offset().unwrap(), 4);
    assert_eq!(end.offset().unwrap(), 1);
}

#[test]
fn default_anchor_can_be_kept_before_insertion() {
    let document = TextDocument::from_text("ab");
    let default = document.create_anchor(1).unwrap();
    let after = document.create_anchor(1).unwrap();
    after.set_movement_type(AnchorMovementType::AfterInsertion);
    document
        .insert_with_movement(1, "xyz", AnchorMovementType::BeforeInsertion)
        .unwrap();
    assert_eq!(default.offset().unwrap(), 1);
    assert_eq!(after.offset().unwrap(), 4);
}

#[test]
fn all_deletion_flags_are_set_before_any_deleted_event() {
    let document = TextDocument::from_text("abcd");
    let a = document.create_anchor(1).unwrap();
    let b = document.create_anchor(2).unwrap();
    let observed = Rc::new(Cell::new(false));
    let observed2 = observed.clone();
    a.deleted(move || {
        assert!(b.is_deleted());
        observed2.set(true);
    });
    document.remove(0, 3).unwrap();
    assert!(observed.get());
}

#[test]
fn grouped_undo_redo_and_original_marker() {
    let document = TextDocument::from_text("abc");
    let guard = document.begin_update().unwrap();
    document.insert(3, "d").unwrap();
    let nested = document.begin_update().unwrap();
    document.replace(0, 1, "A").unwrap();
    nested.finish().unwrap();
    guard.finish().unwrap();
    assert_eq!(document.undo_count(), 1);
    assert_eq!(document.text(), "Abcd");
    assert!(document.is_modified());
    document.undo().unwrap();
    assert_eq!(document.text(), "abc");
    assert!(!document.is_modified());
    document.redo().unwrap();
    assert_eq!(document.text(), "Abcd");
    document.mark_as_original_file();
    assert!(!document.is_modified());
    document.undo().unwrap();
    assert!(document.is_modified());
    document.redo().unwrap();
    assert!(!document.is_modified());
}

#[test]
fn continuing_undo_groups_matches_empty_group_rules() {
    let document = TextDocument::new();
    document.insert(0, "a").unwrap();
    {
        let _g = document.begin_continued_update().unwrap();
        document.insert(1, "b").unwrap();
    }
    document.undo().unwrap();
    assert_eq!(document.text(), "");
    document.insert(0, "a").unwrap();
    {
        let _g = document.begin_update().unwrap();
    }
    {
        let _g = document.begin_continued_update().unwrap();
        document.insert(1, "b").unwrap();
    }
    document.undo().unwrap();
    assert_eq!(document.text(), "a");
}

#[test]
fn empty_continuation_can_still_continue_the_previous_group() {
    let document = TextDocument::new();
    document.insert(0, "a").unwrap();
    {
        let _g = document.begin_continued_update().unwrap();
    }
    {
        let _g = document.begin_continued_update().unwrap();
        document.insert(1, "b").unwrap();
    }
    document.undo().unwrap();
    assert_eq!(document.text(), "");
}

#[test]
fn read_only_invalid_ranges_and_overlap_do_not_mutate_history() {
    let document = TextDocument::from_text("abcdef");
    assert!(document.replace(2, usize::MAX, "x").is_err());
    assert!(
        document
            .apply_edits(&[DocumentEdit::new(1, 3, "x"), DocumentEdit::new(2, 1, "y")])
            .is_err()
    );
    assert_eq!(document.text(), "abcdef");
    assert!(!document.can_undo());
    document.set_is_read_only(true);
    assert_eq!(document.insert(1, "x"), Err(DocumentError::ReadOnly));
    assert!(!document.is_modified());
}

#[test]
fn batch_edits_use_original_offsets_and_one_history_entry() {
    let document = TextDocument::from_text("abcdef");
    document
        .apply_edits(&[DocumentEdit::new(0, 1, "AA"), DocumentEdit::new(3, 2, "D")])
        .unwrap();
    assert_eq!(document.text(), "AAbcDf");
    assert_eq!(document.undo_count(), 1);
    document.undo().unwrap();
    assert_eq!(document.text(), "abcdef");
}

#[test]
fn event_order_grouped_notifications_and_explicit_dispose() {
    let document = TextDocument::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let e = events.clone();
    document.update_started(move || e.borrow_mut().push("start"));
    let e = events.clone();
    document.changing(move |_| e.borrow_mut().push("changing"));
    let e = events.clone();
    document.changed(move |_| e.borrow_mut().push("changed"));
    let e = events.clone();
    let subscription = document.text_changed(move || e.borrow_mut().push("text"));
    let e = events.clone();
    document.update_finished(move || e.borrow_mut().push("finish"));
    {
        let _g = document.begin_update().unwrap();
        document.insert(0, "a").unwrap();
        document.insert(1, "b").unwrap();
    }
    assert_eq!(
        *events.borrow(),
        [
            "start", "changing", "changed", "changing", "changed", "text", "finish"
        ]
    );
    subscription.dispose();
    events.borrow_mut().clear();
    document.insert(2, "c").unwrap();
    assert_eq!(*events.borrow(), ["start", "changing", "changed", "finish"]);
}

#[test]
fn changed_can_read_document_but_reentrant_writes_fail_without_borrow_panic() {
    let document = TextDocument::new();
    let doc = document.clone();
    let sub = document.changed(move |change| {
        assert_eq!(doc.text(), "abc");
        assert_eq!(change.inserted_text.text(), "abc");
        assert_eq!(doc.insert(0, "X"), Err(DocumentError::ReentrantChange));
    });
    document.insert(0, "abc").unwrap();
    sub.dispose();
    assert_eq!(document.text(), "abc");
    document.insert(3, "d").unwrap();
}

#[test]
fn text_changed_additional_edit_joins_current_group() {
    let document = TextDocument::new();
    let doc = document.clone();
    let count = Rc::new(Cell::new(0));
    let observed = count.clone();
    let sub = document.text_changed(move || {
        observed.set(observed.get() + 1);
        if doc.text() == "a" {
            doc.insert(1, "b").unwrap();
        }
    });
    document.insert(0, "a").unwrap();
    sub.dispose();
    assert_eq!(document.text(), "ab");
    assert_eq!(count.get(), 2);
    assert_eq!(document.undo_count(), 1);
    document.undo().unwrap();
    assert_eq!(document.text(), "");
}

#[test]
fn snapshots_versions_translate_forward_and_backward() {
    let document = TextDocument::from_text("initial text");
    let first = document.create_snapshot();
    document.replace(0, 7, "nw").unwrap();
    document.insert(1, "e").unwrap();
    let second = document.create_snapshot();
    let a = first.version().unwrap();
    let b = second.version().unwrap();
    assert_eq!(a.compare_age(&b).unwrap(), std::cmp::Ordering::Less);
    let changes = a.get_changes_to(&b).unwrap();
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].inserted_text.text(), "nw");
    assert_eq!(changes[1].inserted_text.text(), "e");
    let backward = b.get_changes_to(&a).unwrap();
    assert_eq!(backward[0].inserted_text.text(), "");
    assert_eq!(backward[1].inserted_text.text(), "initial");
    assert_eq!(
        a.move_offset_to(&b, 8, AnchorMovementType::Default)
            .unwrap(),
        4
    );
    assert_eq!(
        b.move_offset_to(&a, 4, AnchorMovementType::Default)
            .unwrap(),
        8
    );
    assert_eq!(first.text(), "initial text");
    assert_eq!(second.text(), "new text");
    assert_eq!(
        a.compare_age(&TextDocument::new().version()),
        Err(DocumentError::DifferentDocument)
    );
}

#[test]
fn undo_limit_and_branching_invalidates_redo() {
    let document = TextDocument::new();
    document.set_undo_size_limit(2);
    document.insert(0, "a").unwrap();
    document.insert(1, "b").unwrap();
    document.insert(2, "c").unwrap();
    assert_eq!(document.undo_count(), 2);
    document.undo().unwrap();
    document.undo().unwrap();
    assert_eq!(document.text(), "a");
    assert!(!document.undo().unwrap());
    assert!(document.can_redo());
    document.insert(1, "X").unwrap();
    assert!(!document.can_redo());
}

#[test]
fn snapshots_are_send_sync_and_can_cross_threads() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<TextSnapshot>();
    assert_send_sync::<DocumentVersion>();
    let document = TextDocument::from_text("abc");
    let snapshot = document.create_snapshot();
    let result = std::thread::spawn(move || snapshot.text()).join().unwrap();
    assert_eq!(result, "abc");
}

#[test]
fn dropping_long_version_chain_does_not_recurse() {
    let mut provider = TextSourceVersionProvider::new();
    let first = provider.current_version();
    for _ in 0..40_000 {
        provider
            .append_change(DocumentChangeEventArgs::new(0, "", "x"))
            .unwrap();
    }
    drop(provider);
    drop(first);
}

#[test]
fn invalid_custom_offset_map_is_rejected() {
    let document = TextDocument::from_text("abcd");
    let map = OffsetChangeMap::from_single_element(OffsetChangeMapEntry::new(9, 2, 1));
    assert_eq!(
        document.replace_with_map(1, 2, TextSnapshot::from_text("x"), map),
        Err(DocumentError::InvalidOffsetMap)
    );
    assert_eq!(document.text(), "abcd");
}

#[test]
fn randomized_anchor_tree_edits_affinities_deletions_and_reclamation() {
    let document = TextDocument::from_text(&"a".repeat(1000));
    let mut random = 0x91ae2c8fu64;
    let mut next = || {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        random as usize
    };
    let mut anchors: Vec<(TextAnchor, usize, bool, AnchorMovementType)> = Vec::new();
    for step in 0..1600 {
        match next() % 4 {
            0 => {
                for _ in 0..next() % 20 {
                    let offset = next() % (document.text_length() + 1);
                    let anchor = document.create_anchor(offset).unwrap();
                    let survive = next() % 2 == 0;
                    let movement = if next() % 2 == 0 {
                        AnchorMovementType::BeforeInsertion
                    } else {
                        AnchorMovementType::AfterInsertion
                    };
                    anchor.set_survive_deletion(survive);
                    anchor.set_movement_type(movement);
                    anchors.push((anchor, offset, survive, movement));
                }
            }
            1 if !anchors.is_empty() => {
                let index = next() % anchors.len();
                anchors.swap_remove(index);
            }
            _ => {
                let offset = next() % (document.text_length() + 1);
                let removed = (next() % 40).min(document.text_length() - offset);
                let inserted = next() % 45;
                document
                    .replace(offset, removed, &"a".repeat(inserted))
                    .unwrap();
                anchors.retain_mut(|(anchor, expected, survive, movement)| {
                    if *expected > offset && *expected < offset + removed {
                        if !*survive {
                            assert!(anchor.is_deleted(), "step {step}");
                            return false;
                        }
                        *expected = offset
                            + if *movement == AnchorMovementType::AfterInsertion {
                                inserted
                            } else {
                                0
                            };
                    } else if *expected >= offset + removed
                        && (*expected > offset
                            || removed == 0 && *movement == AnchorMovementType::AfterInsertion)
                    {
                        *expected = *expected - removed + inserted;
                    }
                    true
                });
            }
        }
        document.validate_anchor_tree();
        for (anchor, expected, _, _) in &anchors {
            assert_eq!(anchor.offset().unwrap(), *expected, "step {step}");
        }
    }
    drop(anchors);
    document.validate_anchor_tree();
    document.insert(0, "z").unwrap();
}

#[test]
fn dense_suffix_anchors_follow_lazy_shifts_and_individual_removal() {
    let document = TextDocument::from_text(&"a".repeat(20_000));
    let mut anchors: Vec<_> = (0..20_000)
        .map(|i| document.create_anchor(i).unwrap())
        .collect();
    for _ in 0..20 {
        document.insert(0, "prefix").unwrap();
    }
    document.validate_anchor_tree();
    for (i, anchor) in anchors.iter().enumerate() {
        assert_eq!(anchor.offset().unwrap(), i + 120);
    }
    anchors.drain(100..19_900);
    document.validate_anchor_tree();
    document.remove(0, 120).unwrap();
    assert_eq!(anchors[0].offset().unwrap(), 0);
    assert_eq!(anchors[199].offset().unwrap(), 19_999);
}

#[test]
fn overflowing_invalid_mapping_input_returns_error() {
    let document = TextDocument::new();
    assert!(
        document
            .replace_with_mapping(
                usize::MAX,
                usize::MAX,
                "abc",
                OffsetChangeMappingType::CharacterReplace
            )
            .is_err()
    );
    assert_eq!(document.text(), "");
}

#[test]
fn generic_source_changes_and_document_changes_have_distinct_boundary_affinity() {
    let generic = TextChangeEventArgs::new(1, "ab", "xyz");
    let document = DocumentChangeEventArgs::new(1, "ab", "xyz");
    assert_eq!(generic.get_new_offset(1, AnchorMovementType::Default), 4);
    assert_eq!(document.get_new_offset(1, AnchorMovementType::Default), 1);
    assert_eq!(
        generic.get_new_offset(3, AnchorMovementType::BeforeInsertion),
        1
    );
    assert_eq!(
        document.get_new_offset(3, AnchorMovementType::BeforeInsertion),
        4
    );
    let mut provider = TextSourceVersionProvider::new();
    let first = provider.current_version();
    provider.append_change(generic).unwrap();
    assert_eq!(
        first
            .move_offset_to(&provider.current_version(), 1, AnchorMovementType::Default)
            .unwrap(),
        4
    );
}

#[test]
fn policy_and_committed_history_notifications_allow_reentrant_queries() {
    let document = TextDocument::new();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let events = observed.clone();
    let read = document.clone();
    let subscription = document.property_changed(move |name| {
        if matches!(name, "IsReadOnly" | "CanUndo" | "CanRedo" | "IsModified") {
            events.borrow_mut().push((
                name.to_owned(),
                read.is_read_only(),
                read.can_undo(),
                read.can_redo(),
                read.is_modified(),
            ));
        }
    });
    document.set_is_read_only(true);
    document.set_is_read_only(true);
    document.set_is_read_only(false);
    assert_eq!(observed.borrow().len(), 2);
    document.insert(0, "x").unwrap();
    assert!(
        observed
            .borrow()
            .iter()
            .any(|(name, _, undo, _, _)| name == "CanUndo" && *undo)
    );
    document.undo().unwrap();
    assert!(
        observed
            .borrow()
            .iter()
            .any(|(name, _, _, redo, _)| name == "CanRedo" && *redo)
    );
    document.undo_stack().clear_redo_stack().unwrap();
    assert!(
        observed
            .borrow()
            .iter()
            .any(|(name, _, _, redo, _)| name == "CanRedo" && !*redo)
    );
    document.insert(0, "y").unwrap();
    document.mark_as_original_file();
    assert!(
        observed
            .borrow()
            .iter()
            .any(|(name, _, _, _, modified)| name == "IsModified" && !*modified)
    );
    document.set_undo_size_limit(0);
    assert!(!document.can_undo());
    subscription.dispose();
}
