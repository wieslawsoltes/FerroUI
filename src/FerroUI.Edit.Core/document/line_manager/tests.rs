use super::*;
use crate::document::{TextDocument, WeakLineTracker};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn validate(document: &TextDocument) {
    let live = document.live_lines();
    let snapshot = document.create_snapshot();
    live[0].manager.tree.borrow().validate(&snapshot);
    for (index, line) in live.iter().enumerate() {
        assert_eq!(
            line.snapshot().unwrap(),
            snapshot.get_line_by_number(index + 1).unwrap()
        );
        assert_eq!(
            line.previous_line().as_ref(),
            index.checked_sub(1).map(|i| &live[i])
        );
        assert_eq!(line.next_line().as_ref(), live.get(index + 1));
    }
    for offset in 0..=snapshot.text_length() {
        assert_eq!(
            document
                .get_live_line_by_offset(offset)
                .unwrap()
                .snapshot()
                .unwrap(),
            snapshot.get_line_by_offset(offset).unwrap()
        );
    }
}

#[test]
fn stable_line_identity_survives_splits_merges_and_suffix_shifts() {
    let document = TextDocument::from_text("aa\r\nbb\ncc");
    let lines = document.live_lines();
    document.insert(0, "\n").unwrap();
    assert_eq!(document.get_live_line_by_number(1).unwrap(), lines[0]);
    assert_eq!(document.get_live_line_by_number(3).unwrap(), lines[1]);
    assert_eq!(lines[1].offset().unwrap(), 5);
    let split = document.get_live_line_by_number(2).unwrap();
    document.remove(0, 1).unwrap();
    assert!(split.is_deleted());
    assert_eq!(split.offset(), Err(DocumentError::DeletedLine));
    assert!(split.next_line().is_none());
    assert_eq!(split.length(), 2);
    assert_eq!(document.live_lines(), lines);
    document.remove(2, 2).unwrap();
    assert!(lines[1].is_deleted());
    assert_eq!(document.get_live_line_by_number(1).unwrap(), lines[0]);
    assert_eq!(document.get_live_line_by_number(2).unwrap(), lines[2]);
    assert_eq!(lines[0].length(), 4);
    assert_eq!(lines[2].offset().unwrap(), 5);
    validate(&document);
}

#[test]
fn crlf_edits_preserve_and_delete_the_expected_line_identities() {
    let document = TextDocument::from_text("a\r\nb");
    let original = document.live_lines();
    document.remove(2, 1).unwrap();
    assert_eq!(document.live_lines(), original);
    assert_eq!(original[0].delimiter_length(), 1);
    document.insert(2, "\n").unwrap();
    // Joining an inserted LF to a preceding CR consumes the old following line
    // and creates its text suffix as a new line identity.
    assert_eq!(document.get_live_line_by_number(1).unwrap(), original[0]);
    assert!(original[1].is_deleted());
    let following = document.get_live_line_by_number(2).unwrap();
    assert_eq!(original[0].delimiter_length(), 2);
    document.insert(2, "x").unwrap();
    assert_eq!(document.line_count(), 3);
    let middle = document.get_live_line_by_number(2).unwrap();
    assert_eq!(document.get_live_line_by_number(3).unwrap(), following);
    document.remove(2, 1).unwrap();
    assert!(middle.is_deleted());
    assert_eq!(document.live_lines(), [original[0].clone(), following]);
    validate(&document);

    let document = TextDocument::from_text("a\r\r\n");
    let original = document.live_lines();
    document.subscribe_line_events(|event| {
        if let LineEvent::BeforeRemove(line) = event {
            assert!(!line.is_deleted());
            let _ = line.snapshot().unwrap();
            let _ = line.length();
        }
    });
    document.remove(2, 1).unwrap();
    assert!(original[1].is_deleted());
    assert_eq!(
        document.live_lines(),
        [original[0].clone(), original[2].clone()]
    );
    validate(&document);
}

#[test]
fn replacing_the_document_rebuilds_all_but_the_first_line() {
    let document = TextDocument::from_text("a\nb\n");
    let original = document.live_lines();
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    document.subscribe_line_events(move |event| {
        sink.borrow_mut().push(match event {
            LineEvent::Rebuild => "rebuild",
            LineEvent::Complete(_) => "complete",
            _ => "unexpected",
        })
    });
    document.set_text("😀\r\nx").unwrap();
    assert_eq!(*events.borrow(), ["rebuild", "complete"]);
    assert_eq!(document.get_live_line_by_number(1).unwrap(), original[0]);
    assert!(original[1].is_deleted());
    assert!(original[2].is_deleted());
    document.set_text("").unwrap();
    assert_eq!(document.live_lines(), [original[0].clone()]);
    assert_eq!(original[0].length(), 0);
    validate(&document);
}

#[test]
fn retained_live_line_keeps_the_index_after_document_release() {
    let line = {
        let document = TextDocument::from_text("a\nb\n");
        document.get_live_line_by_number(2).unwrap()
    };
    assert_eq!(line.offset().unwrap(), 2);
    assert_eq!(line.line_number().unwrap(), 2);
    assert_eq!(line.previous_line().unwrap().offset().unwrap(), 0);
    assert_eq!(line.next_line().unwrap().length(), 0);
}

#[test]
fn random_line_edits_match_persistent_text_metrics_and_replay() {
    let document = TextDocument::from_text("\r\n\ra\n");
    let first = document.get_live_line_by_number(1).unwrap();
    let mut model = document.create_snapshot().to_utf16();
    let mut random = 0x43cafebabeu64;
    let mut next = || {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        random as usize
    };
    let alphabet = [13, 10, 13, 10, 0xd800, 0xdc00, b'a' as u16, b'b' as u16];
    for step in 0..2500 {
        let offset = next() % (model.len() + 1);
        let length = next() % (model.len() - offset + 1);
        let inserted: Vec<_> = (0..next() % 15)
            .map(|_| alphabet[next() % alphabet.len()])
            .collect();
        let old = model.clone();
        document.replace_utf16(offset, length, &inserted).unwrap();
        model.splice(offset..offset + length, inserted.iter().copied());
        assert_eq!(document.create_snapshot().to_utf16(), model, "step {step}");
        assert_eq!(document.get_live_line_by_number(1).unwrap(), first);
        validate(&document);
        if step % 17 == 0 && (length > 0 || !inserted.is_empty()) {
            document.undo().unwrap();
            assert_eq!(document.create_snapshot().to_utf16(), old);
            validate(&document);
            document.redo().unwrap();
            assert_eq!(document.create_snapshot().to_utf16(), model);
            validate(&document);
        }
    }
}

#[test]
fn tracker_order_and_reentrant_reads_respect_the_change_boundary() {
    let document = TextDocument::from_text("aa\nbb\ncc");
    let anchor = document.create_anchor(6).unwrap();
    let lines = document.live_lines();
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let doc = document.clone();
    let removed = lines[1].clone();
    let subscription = document.subscribe_line_events(move |event| {
        assert_eq!(doc.text(), "aacc");
        assert_eq!(
            doc.insert(0, "forbidden"),
            Err(DocumentError::ReentrantChange)
        );
        match event {
            LineEvent::BeforeRemove(line) => {
                assert!(!line.is_deleted());
                line.snapshot().unwrap();
                sink.borrow_mut().push("remove");
            }
            LineEvent::SetLength(line, new_length) => {
                assert_ne!(line.total_length(), *new_length);
                line.snapshot().unwrap();
                sink.borrow_mut().push("length");
            }
            LineEvent::Complete(change) => {
                assert_eq!(change.offset, 2);
                assert!(removed.is_deleted());
                assert_eq!(anchor.offset().unwrap(), 2);
                validate(&doc);
                sink.borrow_mut().push("complete");
            }
            _ => panic!("unexpected line notification"),
        }
    });
    let sink = events.clone();
    let changed = document.changed(move |_| sink.borrow_mut().push("changed"));
    document.remove(2, 4).unwrap();
    assert_eq!(
        *events.borrow(),
        ["remove", "remove", "length", "complete", "changed"]
    );
    subscription.dispose();
    changed.dispose();
}

struct CountingTracker(Cell<usize>);
impl ILineTracker for CountingTracker {
    fn before_remove_line(&self, _: &LiveDocumentLine) {
        self.0.set(self.0.get() + 1);
    }
    fn set_line_length(&self, _: &LiveDocumentLine, _: usize) {
        self.0.set(self.0.get() + 1);
    }
    fn line_inserted(&self, _: &LiveDocumentLine, _: &LiveDocumentLine) {
        self.0.set(self.0.get() + 1);
    }
    fn rebuild_document(&self) {
        self.0.set(self.0.get() + 1);
    }
    fn change_complete(&self, _: &DocumentChangeEventArgs) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn weak_trackers_deregister_expired_targets_and_support_explicit_dispose() {
    let document = TextDocument::new();
    let concrete = Rc::new(CountingTracker(Cell::new(0)));
    let tracker: Rc<dyn ILineTracker> = concrete.clone();
    let weak = WeakLineTracker::register(&document, &tracker);
    document.insert(0, "a\nb").unwrap();
    assert!(concrete.0.get() > 0);
    assert!(weak.is_registered());
    weak.deregister();
    assert!(!weak.is_registered());
    let count = concrete.0.get();
    document.insert(0, "x").unwrap();
    assert_eq!(concrete.0.get(), count);
    let weak = WeakLineTracker::register(&document, &tracker);
    drop(tracker);
    drop(concrete);
    document.insert(0, "y").unwrap();
    assert!(!weak.is_registered());
}

#[test]
fn invalid_and_read_only_edits_do_not_publish_line_changes() {
    let document = TextDocument::from_text("a\nb");
    let concrete = Rc::new(CountingTracker(Cell::new(0)));
    let subscription = document.add_line_tracker(concrete.clone());
    assert!(document.get_live_line_by_number(0).is_err());
    assert!(document.get_live_line_by_offset(4).is_err());
    assert!(document.replace(3, 1, "x").is_err());
    document.set_is_read_only(true);
    assert_eq!(document.insert(0, "x"), Err(DocumentError::ReadOnly));
    assert_eq!(concrete.0.get(), 0);
    validate(&document);
    subscription.dispose();
}

#[test]
fn tracking_enabled_from_changing_observes_the_in_flight_edit() {
    let document = TextDocument::from_text("a\nb");
    let captured = Rc::new(RefCell::new(None));
    let slot = captured.clone();
    let doc = document.clone();
    let subscription = document.changing(move |_| {
        *slot.borrow_mut() = Some(doc.get_live_line_by_number(2).unwrap());
    });
    document.remove(1, 1).unwrap();
    assert!(captured.borrow().as_ref().unwrap().is_deleted());
    validate(&document);
    subscription.dispose();
}
