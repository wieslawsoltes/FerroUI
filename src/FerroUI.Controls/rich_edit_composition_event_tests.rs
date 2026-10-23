//! Routed WinUI composition ranges are the entire UTF-16 composition, not the
//! insertion caret. Text payloads are the accepted transaction result.
use super::*;
use crate::rich_text::EditorSelection;
use crate::{RichEditCharacterCasing, RichEditCompositionEventArgs};
use ferroui_base::input::text_input::{TextInputMethodEdit, TextSelection};

#[path = "rich_edit_composition_start_tests.rs"]
mod start_tests;

#[derive(Debug, PartialEq, Eq)]
struct ObservedComposition {
    stage: &'static str,
    text: String,
    start: i32,
    length: i32,
    selection: EditorSelection,
    committed: bool,
}

fn observe(owner: &Ref<RichEditBox>) -> Rc<RefCell<Vec<ObservedComposition>>> {
    let observed = Rc::new(RefCell::new(Vec::new()));
    for (stage, event) in [
        ("start", RichEditBox::text_composition_started_event()),
        ("change", RichEditBox::text_composition_changed_event()),
        ("end", RichEditBox::text_composition_ended_event()),
    ] {
        let observed = observed.clone();
        owner.add_handler(event, move |_, args| {
            observed.borrow_mut().push(ObservedComposition {
                stage,
                text: args.text.clone(),
                start: args.start_index(),
                length: args.length(),
                selection: args.selection,
                committed: args.committed,
            });
        });
    }
    observed
}

fn edit(start: i32, end: i32, text: &str, anchor: i32, active: i32) -> TextInputMethodEdit {
    TextInputMethodEdit::new(
        TextSelection::new(start, end),
        text,
        TextSelection::new(anchor, active),
    )
}

#[test]
fn rich_composition_event_default_range_uses_the_native_unavailable_sentinel() {
    let args = RichEditCompositionEventArgs::default();
    assert_eq!(args.start_index(), -1);
    assert_eq!(args.length(), 0);
}

#[test]
fn rich_composition_events_report_whole_utf16_extent_and_skip_cursor_only_changes() {
    let f = Fixture::new(true);
    f.owner.set_text("😀ab").unwrap();
    f.owner.select(2, 3);
    let observed = observe(&f.owner);
    let session = f.connection().begin_composition().unwrap();
    session.update("x😀z", Some(3)).unwrap();
    session.update("x😀z", Some(1)).unwrap();
    assert_eq!(
        observed.borrow().len(),
        2,
        "a caret move is not composed text"
    );
    session.update("😀", Some(0)).unwrap();
    assert!(session.cancel().unwrap());
    assert_eq!(f.owner.text(), "😀ab");
    assert_eq!(
        &*observed.borrow(),
        &[
            ObservedComposition {
                stage: "start",
                text: "".into(),
                start: 0,
                length: 0,
                selection: EditorSelection::new(2, 3),
                committed: false,
            },
            ObservedComposition {
                stage: "change",
                text: "x😀z".into(),
                start: 2,
                length: 4,
                selection: EditorSelection::caret(5),
                committed: false,
            },
            ObservedComposition {
                stage: "change",
                text: "😀".into(),
                start: 2,
                length: 2,
                selection: EditorSelection::caret(2),
                committed: false,
            },
            ObservedComposition {
                stage: "end",
                text: "".into(),
                start: 2,
                length: 0,
                selection: EditorSelection::new(2, 3),
                committed: false,
            },
        ]
    );
}

#[test]
fn rich_composition_commit_events_use_accepted_filtered_and_utf16_limited_text() {
    for through_native_connection in [false, true] {
        let f = Fixture::new(true);
        let before = f.owner.document().snapshot();
        f.owner.set_accepts_return(false);
        f.owner.set_character_casing(RichEditCharacterCasing::Upper);
        f.owner.set_max_length(5);
        let observed = observe(&f.owner);
        let session = f.connection().begin_composition().unwrap();
        session.update("x😀z", Some(1)).unwrap();
        let changed = if through_native_connection {
            session.commit("ß😀\n", 1).unwrap()
        } else {
            f.owner.commit_composition("ß😀\n").unwrap()
        };
        assert!(changed);
        assert_eq!(f.owner.text(), "aSSd");
        assert_eq!(
            observed.borrow().last(),
            Some(&ObservedComposition {
                stage: "end",
                text: "SS".into(),
                start: 1,
                length: 2,
                selection: EditorSelection::caret(3),
                committed: true,
            })
        );
        assert!(!session.is_current());
        assert!(f.owner.undo().unwrap());
        assert_eq!(f.owner.document().snapshot(), before);
    }
}

#[test]
fn rich_composition_exact_events_keep_the_full_range_across_disjoint_edits() {
    let f = Fixture::new(true);
    let observed = observe(&f.owner);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(0, 1, "😀", 2, 2)).unwrap();
    session.apply_edit(&edit(4, 5, "XY", 4, 4)).unwrap();
    session.select_exact(TextSelection::new(5, 2)).unwrap();
    assert_eq!(f.owner.text(), "😀bcXY");
    assert_eq!(observed.borrow().len(), 3);
    assert!(session.complete(0).unwrap());
    assert_eq!(
        observed.borrow().last(),
        Some(&ObservedComposition {
            stage: "end",
            text: "😀bcXY".into(),
            start: 0,
            length: 6,
            selection: EditorSelection::new(5, 2),
            committed: true,
        })
    );
    let observed = observed.borrow();
    assert_eq!((observed[0].start, observed[0].length), (0, 0));
    assert_eq!((observed[1].start, observed[1].length), (0, 2));
    assert_eq!((observed[2].start, observed[2].length), (0, 6));
    assert_eq!(observed[2].text, "😀bcXY");
}

#[test]
fn rich_composition_exact_empty_deletion_reports_its_actual_zero_length_range() {
    let f = Fixture::new(true);
    let observed = observe(&f.owner);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "", 1, 1)).unwrap();
    assert!(session.complete(0).unwrap());
    assert_eq!(f.owner.text(), "ad");
    assert_eq!(observed.borrow().len(), 3);
    assert_eq!(
        (observed.borrow()[0].start, observed.borrow()[0].length),
        (0, 0)
    );
    for event in observed.borrow().iter().skip(1) {
        assert_eq!((event.start, event.length), (1, 0));
        assert_eq!(event.text, "");
    }
    assert!(observed.borrow().last().unwrap().committed);
}

#[test]
fn rich_composition_rejected_payload_ends_with_an_empty_range_and_no_undo() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let observed = observe(&f.owner);
    f.owner.set_preedit_text(Some("preview"), None).unwrap();
    // There is no available insertion space after the retained prefix/suffix.
    f.owner.set_max_length(1);
    assert!(!f.owner.commit_composition("unaccepted").unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
    assert_eq!(
        observed.borrow().last(),
        Some(&ObservedComposition {
            stage: "end",
            text: "".into(),
            start: 1,
            length: 0,
            selection: EditorSelection::new(1, 3),
            committed: true,
        })
    );
}

#[test]
fn rich_composition_reentrant_model_changes_do_not_publish_a_successful_end_event() {
    for exact in [false, true] {
        let f = Fixture::new(true);
        let session = f.connection().begin_composition().unwrap();
        if exact {
            session.apply_edit(&edit(1, 3, "preview", 4, 4)).unwrap();
        } else {
            session.update("preview", None).unwrap();
        }
        let observed = observe(&f.owner);
        let replaced = Rc::new(Cell::new(false));
        let once = replaced.clone();
        let weak = f.owner.downgrade();
        f.owner
            .add_handler(RichEditBox::text_changing_event(), move |_, _| {
                if !once.replace(true) {
                    weak.upgrade().unwrap().set_text("external").unwrap();
                }
            });
        let result = if exact {
            session.complete(1)
        } else {
            session.commit("accepted", 1)
        };
        assert_eq!(result, Err(TextInputMethodError::ReentrantEdit));
        assert!(replaced.get());
        assert_eq!(f.owner.text(), "external");
        assert!(observed.borrow().is_empty());
        assert!(!session.is_current());
        assert!(!f.owner.can_undo());
    }
}

#[test]
fn rich_composition_end_callback_changes_reject_the_older_native_acknowledgement() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.update("preview", None).unwrap();
    let weak = f.owner.downgrade();
    f.owner.add_handler(
        RichEditBox::text_composition_ended_event(),
        move |_, args| {
            assert_eq!(args.text, "accepted");
            assert_eq!((args.start_index(), args.length()), (1, 8));
            weak.upgrade().unwrap().set_text("external").unwrap();
        },
    );
    assert_eq!(
        session.commit("accepted", 1),
        Err(TextInputMethodError::ReentrantEdit)
    );
    assert_eq!(f.owner.text(), "external");
    assert!(!session.is_current());
}

#[test]
fn rich_composition_callback_caret_round_trips_cannot_be_acknowledged_as_unchanged() {
    for exact in [false, true] {
        for during_start in [false, true] {
            let f = Fixture::new(true);
            let session = if during_start {
                None
            } else {
                let session = f.connection().begin_composition().unwrap();
                if exact {
                    session.apply_edit(&edit(1, 3, "preview", 4, 4)).unwrap();
                } else {
                    session.update("preview", None).unwrap();
                }
                Some(session)
            };
            let event = if during_start {
                RichEditBox::text_composition_started_event()
            } else {
                RichEditBox::text_composition_ended_event()
            };
            let weak = f.owner.downgrade();
            f.owner.add_handler(event, move |_, _| {
                let owner = weak.upgrade().unwrap();
                let selection = (owner.selection_start(), owner.selection_end());
                owner.select(0, 0);
                owner.select(selection.0, selection.1);
            });
            let result = if during_start {
                f.connection().begin_composition().map(|_| ())
            } else if exact {
                session.as_ref().unwrap().complete(0).map(|_| ())
            } else {
                session.as_ref().unwrap().commit("preview", 0).map(|_| ())
            };
            assert_eq!(result, Err(TextInputMethodError::ReentrantEdit));
            assert_eq!(
                f.owner.text(),
                if during_start { "abcd" } else { "apreviewd" }
            );
            if let Some(session) = session {
                assert!(!session.is_current());
            }
        }
    }
}
