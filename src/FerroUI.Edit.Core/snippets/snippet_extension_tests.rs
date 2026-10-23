use super::*;
use crate::document::{
    AnchorMovementType, AnchorSegment, DocumentError, SimpleSegment, TextDocument,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn formatting_expands_tabs_before_normalizing_cross_boundary_crlf() {
    let options = SnippetInsertionOptions {
        indentation: "  ".into(),
        tab: "\r".into(),
        line_terminator: "\n".into(),
    };
    assert_eq!(
        options.format_text("a\t\nb\r\nc\rd\ne").unwrap(),
        "a\n  b\n  c\n  d\n  e"
    );
    let document = TextDocument::from_text("  old\r\nlast");
    let options = SnippetInsertionOptions::from_document(&document, 2, "    ").unwrap();
    let prepared = Snippet::parse("if {\n\t${1:😀}\n}$0")
        .unwrap()
        .prepare_with_options("", options)
        .unwrap();
    assert_eq!(prepared.text(), "if {\r\n      😀\r\n  }");
    document.replace(2, 3, prepared.text()).unwrap();
    let session = prepared.activate(document.clone(), 2).unwrap();
    assert_eq!(session.active_segment(), Some(SimpleSegment::new(14, 2)));
    assert_eq!(session.caret_offset(), Some(21));
    session.replace_active("field").unwrap();
    assert_eq!(document.text(), "  if {\r\n      field\r\n  }\r\nlast");
}

#[test]
fn selection_indentation_and_conditional_caret_follow_captured_context() {
    let snippet = Snippet {
        elements: vec![
            SnippetElement::CaretIfSelection,
            SnippetElement::Text("[".into()),
            SnippetElement::SelectionWithIndentation { indentation: 2 },
            SnippetElement::Text("]".into()),
        ],
    };
    let options = SnippetInsertionOptions {
        indentation: "  ".into(),
        tab: "  ".into(),
        line_terminator: "\r\n".into(),
    };
    let selected = snippet
        .prepare_with_options(" \tfirst\r\n  second", options.clone())
        .unwrap();
    assert_eq!(selected.text(), "[first\r\n      second]");
    let document = TextDocument::new();
    document.insert(0, selected.text()).unwrap();
    assert_eq!(
        selected.activate(document, 0).unwrap().caret_offset(),
        Some(0)
    );
    let empty = snippet.prepare_with_options("", options).unwrap();
    let document = TextDocument::from_text(empty.text());
    assert_eq!(empty.activate(document, 0).unwrap().caret_offset(), Some(1));
}

struct FieldNode {
    label: &'static str,
    text: &'static str,
    events: Rc<RefCell<Vec<String>>>,
    fail_completion: bool,
}
struct CustomField {
    range: AnchorSegment,
    label: &'static str,
    events: Rc<RefCell<Vec<String>>>,
    fail_completion: bool,
    session: WeakSnippetSession,
}
impl IActiveElement for CustomField {
    fn on_insertion_completed(&self, context: &SnippetSession) -> Result<(), SnippetError> {
        self.events.borrow_mut().push(format!(
            "complete {} {}",
            self.label,
            context.active_elements().len()
        ));
        if self.fail_completion {
            return Err(SnippetError::Extension("completion failed".into()));
        }
        Ok(())
    }
    fn deactivate(&self, args: &SnippetEventArgs) {
        if let Some(session) = self.session.upgrade() {
            assert!(session.is_closed());
        }
        self.events
            .borrow_mut()
            .push(format!("end {} {:?}", self.label, args.reason));
    }
    fn is_editable(&self) -> bool {
        true
    }
    fn segment(&self) -> Option<SimpleSegment> {
        self.range.segment().ok()
    }
}
impl ISnippetElement for FieldNode {
    fn insert(&self, context: &mut SnippetInsertionContext) -> Result<(), SnippetError> {
        let start = context.insertion_position();
        context.insert_text(self.text)?;
        let length = context.insertion_position() - start;
        let label = self.label;
        let events = self.events.clone();
        let fail_completion = self.fail_completion;
        context.register_active_element(move |session| {
            events.borrow_mut().push(format!("factory {label}"));
            let range = AnchorSegment::new(
                &session.document(),
                session.start_position().unwrap() + start,
                length,
            )?;
            range
                .start
                .set_movement_type(AnchorMovementType::BeforeInsertion);
            range
                .end
                .set_movement_type(AnchorMovementType::AfterInsertion);
            Ok(Rc::new(CustomField {
                range,
                label,
                events: events.clone(),
                fail_completion,
                session: session.downgrade(),
            }))
        })
    }
}
fn custom(
    label: &'static str,
    text: &'static str,
    events: &Rc<RefCell<Vec<String>>>,
) -> SnippetElementRef {
    SnippetElementRef::new(FieldNode {
        label,
        text,
        events: events.clone(),
        fail_completion: false,
    })
}

#[test]
fn custom_elements_activate_after_all_factories_and_participate_in_native_traversal() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let first = custom("first", "😀", &events);
    let second = custom("second", "xyz", &events);
    let snippet = Snippet {
        elements: vec![
            SnippetElement::Text("[".into()),
            SnippetElement::Custom(first.clone()),
            SnippetElement::Text("/".into()),
            SnippetElement::Custom(second.clone()),
            SnippetElement::Text("]".into()),
            SnippetElement::Caret,
        ],
    };
    let document = TextDocument::new();
    let session = snippet.insert(document.clone(), 0, "").unwrap();
    assert_eq!(
        *events.borrow(),
        [
            "factory first",
            "factory second",
            "complete first 2",
            "complete second 2"
        ]
    );
    assert_eq!(
        session.get_active_element(&first).unwrap().segment(),
        Some(SimpleSegment::new(1, 2))
    );
    assert_eq!(session.active_placeholder(), None);
    session.replace_active("one").unwrap();
    assert_eq!(document.text(), "[one/xyz]");
    assert_eq!(session.next_element(false), Some(SimpleSegment::new(5, 3)));
    session.replace_active("z").unwrap();
    assert_eq!(document.text(), "[one/z]");
    assert!(session.next_element(false).is_none());
    assert_eq!(
        &events.borrow()[4..],
        ["end first Completed", "end second Completed"]
    );
    session.finish();
    assert_eq!(events.borrow().len(), 6);
}

#[test]
fn repeated_custom_registration_is_rejected_before_inserting_anything() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let node = custom("field", "x", &events);
    let snippet = Snippet {
        elements: vec![
            SnippetElement::Custom(node.clone()),
            SnippetElement::Custom(node),
        ],
    };
    let document = TextDocument::from_text("keep");
    assert!(matches!(
        snippet.insert(document.clone(), 0, ""),
        Err(SnippetError::InvalidExpansion(_))
    ));
    assert_eq!(document.text(), "keep");
    assert!(events.borrow().is_empty());
}

#[test]
fn custom_completion_failure_and_dropped_session_both_release_activated_elements_once() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let first = SnippetElementRef::new(FieldNode {
        label: "first",
        text: "x",
        events: events.clone(),
        fail_completion: true,
    });
    let second = custom("second", "y", &events);
    let snippet = Snippet {
        elements: vec![
            SnippetElement::Custom(first),
            SnippetElement::Custom(second),
        ],
    };
    let document = TextDocument::new();
    assert!(matches!(
        snippet.insert(document.clone(), 0, ""),
        Err(SnippetError::Extension(_))
    ));
    assert_eq!(document.text(), "xy");
    assert_eq!(
        *events.borrow(),
        [
            "factory first",
            "factory second",
            "complete first 2",
            "end first Error",
            "end second Error",
        ]
    );
    events.borrow_mut().clear();
    let snippet = Snippet {
        elements: vec![SnippetElement::Custom(custom("retained", "x", &events))],
    };
    let session = snippet.insert(document, 0, "").unwrap();
    let weak = session.downgrade();
    drop(session);
    assert!(weak.upgrade().is_none());
    assert_eq!(&events.borrow()[2..], ["end retained InputHandlerDetached"]);
}

#[test]
fn named_anchor_replacement_tracks_utf16_and_checks_host_policy() {
    let snippet = Snippet {
        elements: vec![
            SnippetElement::Text("before".into()),
            SnippetElement::Anchor {
                name: "body".into(),
            },
            SnippetElement::Text("after".into()),
        ],
    };
    let document = TextDocument::new();
    let session = snippet.insert(document.clone(), 0, "").unwrap();
    assert!(!session.is_closed()); // A named active element keeps the context alive.
    let anchor = session.named_anchor("body").unwrap();
    assert_eq!(anchor.segment(), Some(SimpleSegment::new(6, 0)));
    anchor.set_text("😀").unwrap();
    assert_eq!(anchor.text().unwrap(), "😀");
    assert_eq!(anchor.segment(), Some(SimpleSegment::new(6, 2)));
    anchor.set_text("replacement").unwrap();
    assert_eq!(document.text(), "beforereplacementafter");
    session.set_edit_validator(Some(Rc::new(|_| Err(DocumentError::ReadOnly))));
    assert!(anchor.set_text("denied").is_err());
    assert_eq!(document.text(), "beforereplacementafter");
    let duplicate = Snippet {
        elements: vec![
            SnippetElement::Anchor {
                name: "same".into(),
            },
            SnippetElement::Anchor {
                name: "same".into(),
            },
        ],
    };
    assert!(duplicate.prepare("").is_err());
}

#[test]
fn transformed_bindings_update_all_changed_fields_and_publish_after_atomic_mirrors() {
    let snippet = Snippet {
        elements: vec![
            SnippetElement::Replaceable {
                id: 1,
                text: "a".into(),
            },
            SnippetElement::Text("/".into()),
            SnippetElement::Replaceable {
                id: 2,
                text: "b".into(),
            },
            SnippetElement::Text("=".into()),
            SnippetElement::TransformedBound {
                id: 1,
                transform: SnippetTextTransform::new(str::to_uppercase),
            },
            SnippetElement::Text("+".into()),
            SnippetElement::TransformedBound {
                id: 2,
                transform: SnippetTextTransform::new(|text| {
                    text.encode_utf16().count().to_string()
                }),
            },
        ],
    };
    let document = TextDocument::new();
    let session = snippet.insert(document.clone(), 0, "").unwrap();
    let first = session.replaceable_element(1).unwrap();
    let second = session.replaceable_element(2).unwrap();
    let seen = Rc::new(RefCell::new(Vec::new()));
    for (id, field) in [(1, first.clone()), (2, second.clone())] {
        let document = document.clone();
        let seen = seen.clone();
        field.text_changed(Rc::new(move || {
            seen.borrow_mut().push((id, document.text()))
        }));
    }
    let update = document.begin_update().unwrap();
    let range = second.segment().unwrap(); // Deliberately edit an inactive field first.
    document.replace(range.offset, range.length, "😀z").unwrap();
    let range = first.segment().unwrap();
    document.replace(range.offset, range.length, "ß").unwrap();
    drop(update);
    assert_eq!(document.text(), "ß/😀z=SS+3");
    assert_eq!(
        *seen.borrow(),
        [(1, "ß/😀z=SS+3".into()), (2, "ß/😀z=SS+3".into())]
    );
    assert_eq!(session.active_placeholder(), Some(1));
    document.undo().unwrap();
    assert_eq!(document.text(), "a/b=A+1");
    document.redo().unwrap();
    assert_eq!(document.text(), "ß/😀z=SS+3");
}

#[test]
fn unchanged_converted_mirrors_do_not_write_or_fail_read_only_validation() {
    let document = TextDocument::new();
    let snippet = Snippet {
        elements: vec![
            SnippetElement::Replaceable {
                id: 1,
                text: "a".into(),
            },
            SnippetElement::Text("=".into()),
            SnippetElement::TransformedBound {
                id: 1,
                transform: SnippetTextTransform::new(|_| "constant".into()),
            },
        ],
    };
    let session = snippet.insert(document.clone(), 0, "").unwrap();
    let changes = Rc::new(Cell::new(0));
    let captured = changes.clone();
    document.changed(move |_| captured.set(captured.get() + 1));
    session.set_edit_validator(Some(Rc::new(|_| Err(DocumentError::ReadOnly))));
    document.replace(0, 1, "b").unwrap();
    assert_eq!(document.text(), "b=constant");
    assert_eq!(changes.get(), 1);
    assert!(!session.is_closed());
}

#[test]
fn converter_failures_and_reentrant_changes_cannot_partially_write_mirrors() {
    let document = TextDocument::new();
    let enabled = Rc::new(Cell::new(false));
    let captured_document = document.clone();
    let captured_enabled = enabled.clone();
    let snippet = Snippet {
        elements: vec![
            SnippetElement::Replaceable {
                id: 1,
                text: "a".into(),
            },
            SnippetElement::Text("=".into()),
            SnippetElement::TransformedBound {
                id: 1,
                transform: SnippetTextTransform::new(move |_| {
                    if captured_enabled.get() {
                        captured_document
                            .insert(captured_document.text_length(), "!")
                            .unwrap();
                    }
                    "X".into()
                }),
            },
            SnippetElement::Text("/".into()),
            SnippetElement::Bound { id: 1 },
        ],
    };
    let session = snippet.insert(document.clone(), 0, "").unwrap();
    enabled.set(true);
    document.replace(0, 1, "b").unwrap();
    assert_eq!(document.text(), "b=X/a!");
    assert_eq!(session.deactivation_reason(), Some(DeactivateReason::Error));
    assert_eq!(
        session.error(),
        Some(DocumentError::ReentrantChange.to_string())
    );

    let snippet = Snippet {
        elements: vec![
            SnippetElement::Replaceable {
                id: 1,
                text: "a".into(),
            },
            SnippetElement::Text("=".into()),
            SnippetElement::Bound { id: 1 },
            SnippetElement::Text("/".into()),
            SnippetElement::TransformedBound {
                id: 1,
                transform: SnippetTextTransform::try_new(|text| {
                    if text == "bad" {
                        Err(SnippetError::Extension("rejected value".into()))
                    } else {
                        Ok(text.into())
                    }
                }),
            },
        ],
    };
    let document = TextDocument::new();
    let session = snippet.insert(document.clone(), 0, "").unwrap();
    document.replace(0, 1, "bad").unwrap();
    assert_eq!(document.text(), "bad=a/a");
    assert_eq!(session.error().as_deref(), Some("rejected value"));
}

#[test]
fn linking_existing_ranges_has_no_initial_write_and_uses_grouped_history() {
    let document = TextDocument::from_text("a + A + a");
    let session = SnippetSession::link_with_transforms(
        document.clone(),
        SimpleSegment::new(0, 1),
        &[
            (
                SimpleSegment::new(4, 1),
                Some(SnippetTextTransform::new(str::to_uppercase)),
            ),
            (SimpleSegment::new(8, 1), None),
        ],
    )
    .unwrap();
    assert_eq!(document.undo_count(), 0);
    session.replace_active("ß").unwrap();
    assert_eq!(document.text(), "ß + SS + ß");
    document.undo().unwrap();
    assert_eq!(document.text(), "a + A + a");
    assert!(
        SnippetSession::link(
            document.clone(),
            SimpleSegment::new(0, 3),
            &[SimpleSegment::new(1, 1)]
        )
        .is_err()
    );
    assert!(SnippetSession::link(document, SimpleSegment::new(usize::MAX, 1), &[]).is_err());
}

#[test]
fn positional_traversal_cycles_by_document_position_independently_of_numeric_order() {
    let document = TextDocument::new();
    let session = Snippet::parse("${3:third} ${1:first} ${2:second}")
        .unwrap()
        .insert(document, 0, "")
        .unwrap();
    assert_eq!(session.active_placeholder(), Some(1));
    assert_eq!(
        session.next_element_from_offset(11, false),
        Some(SimpleSegment::new(12, 6))
    );
    assert_eq!(session.active_placeholder(), Some(2));
    assert_eq!(
        session.next_element_from_offset(18, false),
        Some(SimpleSegment::new(0, 5))
    );
    assert_eq!(session.active_placeholder(), Some(3));
    assert_eq!(
        session.next_element_from_offset(0, true),
        Some(SimpleSegment::new(12, 6))
    );
    assert!(!session.is_closed());
}

#[test]
fn stale_custom_preparation_does_not_apply_an_edit_to_obsolete_coordinates() {
    struct MutatingNode(TextDocument);
    impl ISnippetElement for MutatingNode {
        fn insert(&self, context: &mut SnippetInsertionContext) -> Result<(), SnippetError> {
            self.0.insert(0, "prefix")?;
            context.insert_text("snippet")
        }
    }
    let document = TextDocument::from_text("keep");
    let snippet = Snippet {
        elements: vec![SnippetElement::Custom(SnippetElementRef::new(
            MutatingNode(document.clone()),
        ))],
    };
    assert!(matches!(
        snippet.insert(document.clone(), 2, ""),
        Err(SnippetError::Document(DocumentError::ReentrantChange))
    ));
    assert_eq!(document.text(), "prefixkeep");
}

#[test]
fn edit_policy_is_installed_before_custom_completion_observers_write_named_anchors() {
    let denied = Rc::new(Cell::new(false));
    let mut prepared = Snippet {
        elements: vec![
            SnippetElement::Text("[]".into()),
            SnippetElement::Anchor {
                name: "body".into(),
            },
        ],
    }
    .prepare("")
    .unwrap();
    let captured = denied.clone();
    prepared.on_insertion_completed(move |session| {
        captured.set(matches!(
            session.named_anchor("body").unwrap().set_text("forbidden"),
            Err(SnippetError::Document(DocumentError::ReadOnly))
        ));
    });
    let document = TextDocument::from_text(prepared.text());
    let session = prepared
        .activate_with_validator(
            document.clone(),
            0,
            Some(Rc::new(|_| Err(DocumentError::ReadOnly))),
        )
        .unwrap();
    assert!(denied.get());
    assert_eq!(document.text(), "[]");
    assert!(!session.is_closed());
}

#[test]
fn undoing_atomic_selection_replacement_ends_the_insertion_but_field_undo_does_not() {
    for template in ["${1:name}", "{\n${1:name}\n}$0"] {
        let document = TextDocument::from_text("old");
        let prepared = Snippet::parse(template).unwrap().prepare("old").unwrap();
        let update = document.begin_update().unwrap();
        document.replace(0, 3, prepared.text()).unwrap();
        let session = prepared.activate(document.clone(), 0).unwrap();
        drop(update);
        assert_eq!(document.undo_count(), 1);
        let expansion = document.text();
        session.replace_active("value").unwrap();
        assert_eq!(document.undo_count(), 2);
        document.undo().unwrap();
        assert_eq!(document.text(), expansion);
        assert!(!session.is_closed());
        document.undo().unwrap();
        assert_eq!(document.text(), "old");
        assert_eq!(
            session.deactivation_reason(),
            Some(DeactivateReason::Deleted)
        );
        document.redo().unwrap();
        assert_eq!(document.text(), expansion);
        assert!(session.is_closed());
    }
}
