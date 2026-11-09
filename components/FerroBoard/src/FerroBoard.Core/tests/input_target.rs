use ferroboard_core::{editor::*, keys::*, text::Utf16Text};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
fn units(value: &str) -> Utf16Text {
    Utf16Text::from_units(if value.is_empty() {
        vec![]
    } else {
        value
            .split(',')
            .map(|n| n.parse::<u16>().unwrap())
            .collect::<Vec<_>>()
    })
}
#[test]
fn unchanged_csharp_target_corpus() {
    for row in include_str!("../../../tests/fixtures/input-target.tsv").lines() {
        let c: Vec<_> = row.split('\t').collect();
        let target = TextBufferTarget::new(units(c[0]), None);
        let events = Rc::new(RefCell::new(String::new()));
        let log = events.clone();
        target
            .edited()
            .add(Rc::new(move |_| log.borrow_mut().push('E')));
        let log = events.clone();
        target
            .changed()
            .add(Rc::new(move |_| log.borrow_mut().push('C')));
        target.select(c[1].parse().unwrap(), c[2].parse().unwrap());
        target
            .replace(
                c[1].parse().unwrap(),
                c[2].parse().unwrap(),
                Some(&units("57343,90")),
            )
            .unwrap();
        target.set_text(Some(target.text()));
        assert_eq!(target.text(), units(c[3]), "{row}");
        assert_eq!(target.selection_start(), c[4].parse::<i32>().unwrap());
        assert_eq!(target.selection_length(), c[5].parse::<i32>().unwrap());
        assert_eq!(*events.borrow(), c[6]);
        assert_eq!(target.to_text(), units(c[7]));
    }
}
#[test]
fn reentrant_changes_and_subscription_snapshot() {
    let target = Rc::new(TextBufferTarget::default());
    let calls = Rc::new(RefCell::new(vec![]));
    let second = Rc::new(Cell::new(0));
    let weak = Rc::downgrade(&target);
    let log = calls.clone();
    let token = second.clone();
    target.changed().add(Rc::new(move |sender| {
        log.borrow_mut().push(sender.text().to_utf8().unwrap());
        let t = weak.upgrade().unwrap();
        if sender.text() == Utf16Text::from("outer") {
            t.changed().remove(token.get());
            t.set_text(Some("inner".into()));
        }
    }));
    let log = calls.clone();
    second.set(
        target
            .changed()
            .add(Rc::new(move |_| log.borrow_mut().push("second".into()))),
    );
    target.set_text(Some("outer".into()));
    assert_eq!(*calls.borrow(), ["outer", "inner", "second"]);
}
#[test]
fn shared_actions_handler_order_and_default_identity() {
    let target = Rc::new(TextBufferTarget::default());
    assert!(Arc::ptr_eq(
        &target.attributes(),
        &TextBufferTarget::default().attributes()
    ));
    let actions = target.performed_actions();
    assert!(Rc::ptr_eq(&actions, &target.performed_actions()));
    let weak = Rc::downgrade(&target);
    target.set_enter_action_handler(Some(Rc::new(move |action| {
        let t = weak.upgrade().unwrap();
        assert_eq!(t.performed_actions().borrow().last(), Some(&action));
        t.set_enter_action_handler(None);
        t.select(-1, 99);
        true
    })));
    assert!(target.perform_enter_action(EnterAction::Go));
    assert!(!target.perform_enter_action(EnterAction::Done));
    assert_eq!(*actions.borrow(), [EnterAction::Go, EnterAction::Done]);
}
#[test]
fn external_clamp_null_text_and_notification_distinction() {
    let target = TextBufferTarget::new("abcdef".into(), None);
    target.select(2, 4);
    target.set_text(Some("abc".into()));
    assert_eq!(
        (target.selection_start(), target.selection_length()),
        (2, 1)
    );
    assert_eq!(target.to_text().to_utf8().unwrap(), "ab[c]");
    target.move_caret_externally(i32::MAX);
    assert_eq!(
        (target.selection_start(), target.selection_length()),
        (3, 0)
    );
    target.set_text(None);
    assert_eq!(target.to_text().to_utf8().unwrap(), "|");
}
#[test]
fn field_policy_preserves_numeric_password_and_multiline_precedence() {
    let attrs = InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
        kind: InputKind::NumericPassword,
        ..Default::default()
    });
    assert!(attrs.is_password());
    assert!(!attrs.supports_nlp());
    assert_eq!(attrs.variation(), KeyVariation::Normal);
    assert_eq!(attrs.initial_mode(), KeyboardMode::Numeric);
    for kind in [InputKind::Search, InputKind::Uri, InputKind::Chat] {
        let attrs = InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            kind,
            is_multiline: true,
            ..Default::default()
        });
        assert_eq!(attrs.effective_enter_action(), EnterAction::NewLine);
        let attrs = InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            enter_action: EnterAction::Next,
            ..(attrs).values()
        });
        assert_eq!(attrs.effective_enter_action(), EnterAction::Next);
    }
    // Privacy and autocorrect are separate policies in upstream SupportsNlp.
    assert!(
        InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            is_private: true,
            allow_auto_correct: false,
            ..Default::default()
        })
        .supports_nlp()
    );
}
#[test]
fn enum_parsing_is_case_sensitive_and_preserves_unknown_fallback() {
    assert_eq!(KeyType::parse_key_type(None), KeyType::Character);
    assert_eq!(KeyType::parse_key_type(Some("")), KeyType::Character);
    for key in [
        KeyType::Character,
        KeyType::EnterEditing,
        KeyType::Function,
        KeyType::Lock,
        KeyType::Modifier,
        KeyType::Navigation,
        KeyType::SystemGui,
        KeyType::Numeric,
        KeyType::Placeholder,
        KeyType::Unspecified,
    ] {
        assert_eq!(KeyType::parse_key_type(Some(key.to_json_string())), key);
    }
    assert_eq!(
        KeyType::parse_key_type(Some("Character")),
        KeyType::Unspecified
    );
}
