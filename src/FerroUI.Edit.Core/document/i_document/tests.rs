use super::*;
use crate::document::TextSnapshot;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;

#[test]
fn object_safe_document_edits_preserve_utf16_sources_and_anchor_affinity() {
    let concrete = TextDocument::from_text("ab");
    let document = concrete.as_document();
    assert!(document.is_same_document_as(concrete.as_document().as_ref()));
    assert!(!document.is_same_document_as(TextDocument::from_text("ab").as_document().as_ref()));
    let before = document.create_anchor(1).unwrap();
    let after = document.create_anchor(1).unwrap();
    after.set_movement_type(AnchorMovementType::AfterInsertion);
    let source = TextSnapshot::from_utf16(&[0xd800, 10, 0xdc00]);
    document
        .insert_source_with_movement(1, &source, AnchorMovementType::BeforeInsertion)
        .unwrap();
    assert_eq!(before.offset().unwrap(), 1);
    assert_eq!(after.offset().unwrap(), 4);
    assert_eq!(
        document.get_text_utf16(0, 5).unwrap(),
        [97, 0xd800, 10, 0xdc00, 98]
    );
    assert_eq!(document.line_count(), 2);
    assert_eq!(document.get_location(4).unwrap(), TextLocation::new(2, 2));
    assert_eq!(document.get_offset_at(2, 100).unwrap(), 5);
    document
        .replace_source(1, 1, &TextSnapshot::from_text("😀"))
        .unwrap();
    assert_eq!(
        document.get_text_utf16(0, 6).unwrap(),
        [97, 0xd83d, 0xde00, 10, 0xdc00, 98]
    );
    assert_eq!(source.to_utf16(), [0xd800, 10, 0xdc00]);
    concrete.undo().unwrap();
    concrete.undo().unwrap();
    assert_eq!(document.text(), "ab");
}

#[test]
fn document_line_interface_retains_identity_and_reports_deletion() {
    let concrete = TextDocument::from_text("a\r\nb\n");
    let document = concrete.as_document();
    let first = document.get_line_by_number(1).unwrap();
    let second = document.get_line_by_offset(3).unwrap();
    assert!(first.is_same_line_as(second.previous_line().unwrap().as_ref()));
    assert!(second.is_same_line_as(first.next_line().unwrap().as_ref()));
    assert_eq!(second.segment().unwrap(), SimpleSegment::new(3, 1));
    document.insert(0, "x").unwrap();
    assert_eq!(second.offset().unwrap(), 4);
    assert_eq!(second.line_number().unwrap(), 2);
    document.remove(2, 2).unwrap();
    assert!(second.is_deleted());
    assert_eq!(second.offset(), Err(DocumentError::DeletedLine));
    assert_eq!(second.segment(), Err(DocumentError::DeletedLine));
    assert_eq!(second.length(), 1);
    assert!(second.previous_line().is_none());
    assert!(first.is_same_line_as(document.get_line_by_number(1).unwrap().as_ref()));
    assert_eq!(first.total_length(), 4);
    assert_eq!(first.delimiter_length(), 1);
    assert_eq!(first.end_offset().unwrap(), 3);
}

#[test]
fn anchor_interface_exposes_identity_and_ordered_deletion_without_borrows() {
    let concrete = TextDocument::from_text("abcd");
    let document = concrete.as_document();
    let retained = concrete.create_anchor(1).unwrap();
    let first = retained.as_text_anchor();
    let second = document.create_anchor(2).unwrap();
    assert!(first.is_same_anchor_as(retained.as_text_anchor().as_ref()));
    assert!(!first.is_same_anchor_as(second.as_ref()));
    let observed = Rc::new(Cell::new(false));
    let flag = observed.clone();
    let other = second.clone();
    let doc = document.clone();
    let subscription = first.deleted(Rc::new(move || {
        assert!(other.is_deleted());
        assert_eq!(doc.text(), "d");
        assert_eq!(doc.insert(0, "x"), Err(DocumentError::ReentrantChange));
        flag.set(true);
    }));
    document.remove(0, 3).unwrap();
    assert!(observed.get());
    assert_eq!(first.location(), Err(DocumentError::DeletedAnchor));
    assert_eq!(first.line(), Err(DocumentError::DeletedAnchor));
    first.set_survive_deletion(true);
    assert!(first.survive_deletion());
    assert!(first.is_same_anchor_as(retained.as_text_anchor().as_ref()));
    subscription.dispose();
}

#[test]
fn interface_edit_and_group_events_keep_distinct_meanings() {
    let concrete = TextDocument::new();
    let document = concrete.as_document();
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut subscriptions = Vec::new();
    for before in [true, false] {
        let events = events.clone();
        let doc = document.clone();
        let handler: Rc<dyn Fn(&DocumentChangeEventArgs)> = Rc::new(move |change| {
            assert_eq!(
                doc.insert(0, "forbidden"),
                Err(DocumentError::ReentrantChange)
            );
            events.borrow_mut().push((
                if before { "before" } else { "after" },
                doc.text(),
                change.offset,
            ));
        });
        subscriptions.push(if before {
            document.text_changing(handler)
        } else {
            document.text_changed(handler)
        });
    }
    let sink = events.clone();
    let doc = document.clone();
    subscriptions.push(document.change_completed(Rc::new(move || {
        sink.borrow_mut().push(("complete", doc.text(), 0))
    })));
    document.start_undoable_action().unwrap();
    document.insert(0, "a").unwrap();
    {
        let nested = document.open_undo_group().unwrap();
        document.insert(1, "b").unwrap();
        nested.finish().unwrap();
    }
    assert_eq!(events.borrow().len(), 4);
    document.end_undoable_action().unwrap();
    assert_eq!(
        *events.borrow(),
        [
            ("before", "".into(), 0),
            ("after", "a".into(), 0),
            ("before", "a".into(), 1),
            ("after", "ab".into(), 1),
            ("complete", "ab".into(), 0),
        ]
    );
    assert_eq!(concrete.undo_count(), 1);
    assert_eq!(document.end_undoable_action(), Err(DocumentError::NoUpdate));
    for subscription in subscriptions {
        subscription.dispose();
    }
    concrete.undo().unwrap();
    assert_eq!(events.borrow().len(), 5);
    assert_eq!(document.text(), "");
}

#[test]
fn boxed_group_drop_and_panic_release_nested_updates() {
    let concrete = TextDocument::new();
    let document = concrete.as_document();
    {
        let _group = document.open_undo_group().unwrap();
        document.insert(0, "a").unwrap();
        document.insert(1, "b").unwrap();
    }
    assert_eq!(concrete.undo_count(), 1);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _group = document.open_undo_group().unwrap();
        document.insert(2, "c").unwrap();
        panic!("application failure");
    }));
    assert!(result.is_err());
    assert!(!concrete.is_in_update());
    concrete.undo().unwrap();
    assert_eq!(document.text(), "ab");
}

#[test]
fn filename_metadata_is_shared_reentrant_and_independent_of_text_history() {
    let concrete = TextDocument::from_text("source");
    let document = concrete.as_document();
    let version = concrete.version();
    assert_eq!(document.file_name(), None);
    let observed = Rc::new(RefCell::new(Vec::new()));
    let sink = observed.clone();
    let doc = concrete.clone();
    let subscription = document.file_name_changed(Rc::new(move || {
        let name = doc.file_name();
        sink.borrow_mut().push(name.clone());
        if name.as_deref() == Some("first.rs") {
            doc.set_file_name(Some("second.rs".into()));
        }
    }));
    concrete.set_is_read_only(true);
    assert!(concrete.set_file_name(Some("first.rs".into())));
    assert_eq!(document.file_name().as_deref(), Some("second.rs"));
    assert!(!concrete.set_file_name(Some("second.rs".into())));
    assert!(concrete.set_file_name(None));
    assert_eq!(
        *observed.borrow(),
        [Some("first.rs".into()), Some("second.rs".into()), None]
    );
    assert_eq!(document.text(), "source");
    assert_eq!(concrete.version(), version);
    assert_eq!(concrete.undo_count(), 0);
    assert!(!concrete.is_modified());
    subscription.dispose();
    concrete.set_file_name(Some("third.rs".into()));
    assert_eq!(observed.borrow().len(), 3);
}

#[test]
fn version_interfaces_translate_custom_maps_in_both_directions_and_across_threads() {
    let concrete = TextDocument::from_text("abcdef");
    let source: &dyn ITextSource = &concrete;
    let before = source.version().unwrap();
    concrete
        .replace_with_mapping(1, 3, "XX", OffsetChangeMappingType::CharacterReplace)
        .unwrap();
    concrete.insert(0, "Q").unwrap();
    let after = source.version().unwrap();
    assert!(before.belongs_to_same_document_as(after.as_ref()));
    assert_eq!(before.compare_age(after.as_ref()).unwrap(), Ordering::Less);
    assert_eq!(before.get_changes_to(after.as_ref()).unwrap().len(), 2);
    assert_eq!(
        before
            .move_offset_to(after.as_ref(), 2, AnchorMovementType::Default)
            .unwrap(),
        3
    );
    assert_eq!(
        after
            .move_offset_to(before.as_ref(), 3, AnchorMovementType::Default)
            .unwrap(),
        2
    );
    let other = TextDocument::new().version().as_text_source_version();
    assert!(!before.belongs_to_same_document_as(other.as_ref()));
    assert_eq!(
        before.compare_age(other.as_ref()),
        Err(DocumentError::DifferentDocument)
    );
    drop(concrete);
    let count = std::thread::spawn(move || {
        assert_eq!(
            after.compare_age(before.as_ref()).unwrap(),
            Ordering::Greater
        );
        before.get_changes_to(after.as_ref()).unwrap().len()
    })
    .join()
    .unwrap();
    assert_eq!(count, 2);
}

#[test]
fn source_interface_snapshot_ranges_do_not_leak_mutable_state_or_version_identity() {
    let concrete = TextDocument::from_utf16(&[13, 10, 0xd800, 0xdc00, 0xdfff]);
    let source: &dyn ITextSource = &concrete;
    let slice = source.create_snapshot_range(2, 3).unwrap();
    assert_eq!(slice.to_utf16(), [0xd800, 0xdc00, 0xdfff]);
    assert!(ITextSource::version(&slice).is_none());
    assert!(source.create_snapshot_range(4, 2).is_err());
    concrete.set_text("").unwrap();
    assert_eq!(slice.to_utf16(), [0xd800, 0xdc00, 0xdfff]);
}
