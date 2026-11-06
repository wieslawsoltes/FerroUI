use ferroboard_core::{keys::*, layouts::*, text::Utf16Text};
use serde_json::Value;
use std::{cell::Cell, rc::Rc};
fn leaf(code: i32) -> Key {
    Rc::new(TextKeyData::from_fields(TextKeyDataFields {
        code,
        ..Default::default()
    }))
}
#[test]
fn unchanged_csharp_selectors_and_code_boundaries() {
    for line in include_str!("../../../tests/fixtures/keys.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        match row["kind"].as_str().unwrap() {
            "shift" | "variation" => {
                let mask = row["mask"].as_i64().unwrap();
                let state = row["state"].as_u64().unwrap() as usize;
                let get = |bit| {
                    if mask & (1i64 << bit) != 0i64 {
                        Some(leaf(100 + bit))
                    } else {
                        None
                    }
                };
                let eval = KeyEvaluator::default();
                let key: Key = if row["kind"] == "shift" {
                    eval.set_shift_state(
                        [
                            ShiftState::Unshifted,
                            ShiftState::ShiftedManual,
                            ShiftState::ShiftedAutomatic,
                            ShiftState::CapsLock,
                        ][state],
                    );
                    Rc::new(ShiftStateSelector::from_parts(
                        get(0),
                        get(1),
                        get(2),
                        get(3),
                        get(4),
                        get(5),
                    ))
                } else {
                    eval.set_variation(
                        [
                            KeyVariation::All,
                            KeyVariation::Normal,
                            KeyVariation::EmailAddress,
                            KeyVariation::Uri,
                            KeyVariation::Password,
                        ][state],
                    );
                    Rc::new(VariationSelector::from_parts(
                        get(0),
                        get(1),
                        get(2),
                        get(3),
                        get(4),
                    ))
                };
                assert!(key.as_string(true).unwrap().unwrap().is_empty());
                let actual = key
                    .compute(&eval)
                    .unwrap()
                    .map(|key| key.data().code() as i64);
                assert_eq!(actual, row["code"].as_i64(), "{line}");
            }
            "text" => {
                let code = row["code"].as_i64().unwrap() as i32;
                let key = TextKeyData::from_fields(TextKeyDataFields {
                    code,
                    label: Some("label".into()),
                    ..Default::default()
                });
                assert_eq!(key.produces_text(), row["produces"].as_bool().unwrap());
                assert_eq!(key.is_space(), row["space"].as_bool().unwrap());
                assert_eq!(
                    KeyCode::is_character(code),
                    row["character"].as_bool().unwrap()
                );
                assert_eq!(
                    KeyCode::is_internal(code),
                    row["internalCode"].as_bool().unwrap()
                );
                assert_eq!(
                    KeyCode::is_currency_slot(code),
                    row["currency"].as_bool().unwrap()
                );
                assert_eq!(KeyCode::get_name(code), row["name"].as_str());
                let text = key.as_string(false);
                if row["error"].is_null() {
                    assert_eq!(
                        text.unwrap()
                            .unwrap()
                            .units()
                            .iter()
                            .map(u16::to_string)
                            .collect::<Vec<_>>()
                            .join(","),
                        row["text"].as_str().unwrap()
                    );
                } else {
                    assert!(text.is_err());
                }
            }
            "constant" => {}
            kind => panic!("Unexpected fixture kind {kind}"),
        }
    }
}
#[test]
fn shared_identity_multi_text_with_and_mutable_codepoints() {
    let key = TextKeyData::char("a".into(), None).unwrap();
    let eval = KeyEvaluator::default();
    let original: TextKey = key.clone();
    assert!(Rc::ptr_eq(&original, &key.compute(&eval).unwrap().unwrap()));
    assert!(Rc::ptr_eq(
        &TextKeyData::space_key(),
        &TextKeyData::space_key()
    ));
    assert!(Rc::ptr_eq(&PopupSet::empty(), &PopupSet::empty()));
    let popup = Rc::new(PopupSet::from_parts(
        Some(leaf(97)),
        ResourceReadOnlyList::<Key>::default(),
    ));
    let points = CodePointArray::new(vec![0x1f600, 0x301]);
    let key = MultiTextKeyData::from_parts(
        TextKeyData::from_fields(TextKeyDataFields {
            code: 999,
            popup: Some(popup.clone()),
            ..Default::default()
        }),
        Some(points.clone()),
    );
    let copy = key.with(KeyChanges {
        code: Some(65),
        label: Some("new".into()),
        ..Default::default()
    });
    assert_eq!(copy.data().code(), KeyCode::MultipleCodePoints);
    assert!(Rc::ptr_eq(copy.data().popup().as_ref().unwrap(), &popup));
    assert_eq!(
        copy.as_string(false).unwrap().unwrap().to_utf8().unwrap(),
        "😀\u{301}"
    );
    points.set(0, 98).unwrap();
    assert_eq!(
        copy.as_string(true).unwrap().unwrap().to_utf8().unwrap(),
        "b\u{301}"
    );
    points.set(0, 0xd800).unwrap();
    assert!(copy.as_string(true).is_err());
}
#[test]
fn character_factory_utf16_and_function_label_rules() {
    assert_eq!(
        TextKeyData::char("😀abc".into(), None).unwrap().code(),
        0x1f600
    );
    assert!(TextKeyData::char("".into(), None).is_err());
    for units in [vec![0xd800], vec![0xdc00], vec![0xd800, 65]] {
        assert!(TextKeyData::char(Utf16Text::from_units(units), None).is_err());
    }
    assert_eq!(
        TextKeyData::char("ab".into(), None)
            .unwrap()
            .as_string(false)
            .unwrap()
            .unwrap()
            .to_utf8()
            .unwrap(),
        "a"
    );
    let key = TextKeyData::function(KeyCode::CapsLock, KeyType::Function, None);
    assert_eq!(key.label().as_ref().unwrap().to_utf8().unwrap(), "capslock");
    assert!(key.as_string(false).unwrap().unwrap().is_empty());
    assert_eq!(KeyCode::try_get_code("caps_lock"), Some(KeyCode::CapsLock));
    assert_eq!(KeyCode::try_get_code("CapsLock"), None);
}
struct Custom {
    base: TextKeyData,
    calls: Cell<usize>,
    hide: bool,
}
impl AbstractKeyData for Custom {
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        self.calls.set(self.calls.get() + 1);
        if self.hide { Ok(None) } else { Ok(Some(self)) }
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        Ok(Some("custom".into()))
    }
}
impl ITextKeyData for Custom {
    fn data(&self) -> &TextKeyData {
        &self.base
    }
    fn produces_text(&self) -> Result<bool, KeyDataError> {
        Ok(true)
    }
}
#[test]
fn custom_dispatch_and_null_computation_does_not_trigger_fallback() {
    let custom = Rc::new(Custom {
        base: TextKeyData::default(),
        calls: Cell::new(0),
        hide: false,
    });
    let expected: TextKey = custom.clone();
    let key = Rc::new(CaseSelector::from_parts(
        Some(custom.clone()),
        Some(leaf(65)),
    ));
    let actual = key.compute(&KeyEvaluator::default()).unwrap().unwrap();
    assert!(Rc::ptr_eq(&expected, &actual));
    assert_eq!(custom.calls.get(), 1);
    assert!(actual.produces_text().unwrap());
    assert_eq!(
        actual.as_string(false).unwrap().unwrap().to_utf8().unwrap(),
        "custom"
    );
    let hide = Rc::new(Custom {
        base: TextKeyData::default(),
        calls: Cell::new(0),
        hide: true,
    });
    let key = Rc::new(VariationSelector::from_parts(
        Some(leaf(99)),
        None,
        None,
        Some(hide.clone()),
        None,
    ));
    assert!(key.compute(&KeyEvaluator::default()).unwrap().is_none());
    assert_eq!(hide.calls.get(), 1);
}
#[test]
fn directional_width_and_kana_selectors() {
    for active in [false, true] {
        let eval = KeyEvaluator::from_values(KeyEvaluatorValues {
            is_char_half_width: active,
            is_kana_kata: active,
            direction: if active {
                LayoutDirection::Rtl
            } else {
                LayoutDirection::Ltr
            },
            ..Default::default()
        });
        let keys: Vec<Key> = vec![
            Rc::new(LayoutDirectionSelector::from_parts(
                Some(leaf(1)),
                Some(leaf(2)),
            )),
            Rc::new(CharWidthSelector::from_parts(Some(leaf(1)), Some(leaf(2)))),
            Rc::new(KanaSelector::from_parts(Some(leaf(1)), Some(leaf(2)))),
        ];
        for key in keys {
            assert_eq!(
                key.compute(&eval).unwrap().unwrap().data().code(),
                if active { 2 } else { 1 }
            );
        }
    }
}

#[test]
fn all_source_key_constants() {
    let actual = [
        ("CharactersMin", KeyCode::CharactersMin),
        ("CharactersMax", KeyCode::CharactersMax),
        ("InternalMin", KeyCode::InternalMin),
        ("InternalMax", KeyCode::InternalMax),
        ("Unspecified", KeyCode::Unspecified),
        ("PhoneWait", KeyCode::PhoneWait),
        ("PhonePause", KeyCode::PhonePause),
        ("Space", KeyCode::Space),
        ("Escape", KeyCode::Escape),
        ("Enter", KeyCode::Enter),
        ("Tab", KeyCode::Tab),
        ("Ctrl", KeyCode::Ctrl),
        ("CtrlLock", KeyCode::CtrlLock),
        ("Alt", KeyCode::Alt),
        ("AltLock", KeyCode::AltLock),
        ("Fn", KeyCode::Fn),
        ("FnLock", KeyCode::FnLock),
        ("Delete", KeyCode::Delete),
        ("DeleteWord", KeyCode::DeleteWord),
        ("ForwardDelete", KeyCode::ForwardDelete),
        ("ForwardDeleteWord", KeyCode::ForwardDeleteWord),
        ("Shift", KeyCode::Shift),
        ("CapsLock", KeyCode::CapsLock),
        ("ArrowLeft", KeyCode::ArrowLeft),
        ("ArrowRight", KeyCode::ArrowRight),
        ("ArrowUp", KeyCode::ArrowUp),
        ("ArrowDown", KeyCode::ArrowDown),
        ("MoveStartOfPage", KeyCode::MoveStartOfPage),
        ("MoveEndOfPage", KeyCode::MoveEndOfPage),
        ("MoveStartOfLine", KeyCode::MoveStartOfLine),
        ("MoveEndOfLine", KeyCode::MoveEndOfLine),
        ("ClipboardCopy", KeyCode::ClipboardCopy),
        ("ClipboardCut", KeyCode::ClipboardCut),
        ("ClipboardPaste", KeyCode::ClipboardPaste),
        ("ClipboardSelect", KeyCode::ClipboardSelect),
        ("ClipboardSelectAll", KeyCode::ClipboardSelectAll),
        ("ClipboardClearHistory", KeyCode::ClipboardClearHistory),
        (
            "ClipboardClearFullHistory",
            KeyCode::ClipboardClearFullHistory,
        ),
        (
            "ClipboardClearPrimaryClip",
            KeyCode::ClipboardClearPrimaryClip,
        ),
        ("ToggleFloatingWindow", KeyCode::ToggleFloatingWindow),
        ("ToggleCompactLayout", KeyCode::ToggleCompactLayout),
        ("CompactLayoutToLeft", KeyCode::CompactLayoutToLeft),
        ("CompactLayoutToRight", KeyCode::CompactLayoutToRight),
        ("SplitLayout", KeyCode::SplitLayout),
        ("MergeLayout", KeyCode::MergeLayout),
        ("ToggleResizeMode", KeyCode::ToggleResizeMode),
        ("Undo", KeyCode::Undo),
        ("Redo", KeyCode::Redo),
        ("ViewCharacters", KeyCode::ViewCharacters),
        ("ViewSymbols", KeyCode::ViewSymbols),
        ("ViewSymbols2", KeyCode::ViewSymbols2),
        ("ViewNumeric", KeyCode::ViewNumeric),
        ("ViewNumericAdvanced", KeyCode::ViewNumericAdvanced),
        ("ViewPhone", KeyCode::ViewPhone),
        ("ViewPhone2", KeyCode::ViewPhone2),
        ("ImeUiModeText", KeyCode::ImeUiModeText),
        ("ImeUiModeMedia", KeyCode::ImeUiModeMedia),
        ("ImeUiModeClipboard", KeyCode::ImeUiModeClipboard),
        ("ImeUiModeEditing", KeyCode::ImeUiModeEditing),
        ("SystemInputMethodPicker", KeyCode::SystemInputMethodPicker),
        ("SystemPrevInputMethod", KeyCode::SystemPrevInputMethod),
        ("SystemNextInputMethod", KeyCode::SystemNextInputMethod),
        ("ImeSubtypePicker", KeyCode::ImeSubtypePicker),
        ("ImePrevSubtype", KeyCode::ImePrevSubtype),
        ("ImeNextSubtype", KeyCode::ImeNextSubtype),
        ("LanguageSwitch", KeyCode::LanguageSwitch),
        ("ShowSubtypePicker", KeyCode::ShowSubtypePicker),
        ("ImeShowUi", KeyCode::ImeShowUi),
        ("ImeHideUi", KeyCode::ImeHideUi),
        ("VoiceInput", KeyCode::VoiceInput),
        (
            "ToggleSmartbarVisibility",
            KeyCode::ToggleSmartbarVisibility,
        ),
        ("ToggleActionsOverflow", KeyCode::ToggleActionsOverflow),
        ("ToggleActionsEditor", KeyCode::ToggleActionsEditor),
        ("ToggleIncognitoMode", KeyCode::ToggleIncognitoMode),
        ("ToggleAutocorrect", KeyCode::ToggleAutocorrect),
        ("UriComponentTld", KeyCode::UriComponentTld),
        ("Settings", KeyCode::Settings),
        ("CurrencySlot1", KeyCode::CurrencySlot1),
        ("CurrencySlot2", KeyCode::CurrencySlot2),
        ("CurrencySlot3", KeyCode::CurrencySlot3),
        ("CurrencySlot4", KeyCode::CurrencySlot4),
        ("CurrencySlot5", KeyCode::CurrencySlot5),
        ("CurrencySlot6", KeyCode::CurrencySlot6),
        ("MultipleCodePoints", KeyCode::MultipleCodePoints),
        ("DragMarker", KeyCode::DragMarker),
        ("Noop", KeyCode::Noop),
        ("CharWidthSwitcher", KeyCode::CharWidthSwitcher),
        ("CharWidthFull", KeyCode::CharWidthFull),
        ("CharWidthHalf", KeyCode::CharWidthHalf),
        ("KanaSmall", KeyCode::KanaSmall),
        ("KanaSwitcher", KeyCode::KanaSwitcher),
        ("KanaHira", KeyCode::KanaHira),
        ("KanaKata", KeyCode::KanaKata),
        ("KanaHalfKata", KeyCode::KanaHalfKata),
        ("Kashida", KeyCode::Kashida),
        ("HalfSpace", KeyCode::HalfSpace),
        ("CjkSpace", KeyCode::CjkSpace),
    ];
    for line in include_str!("../../../tests/fixtures/keys.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        if row["kind"] != "constant" {
            continue;
        }
        let name = row["field"].as_str().unwrap();
        let code = actual.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(code as i64, row["code"].as_i64().unwrap());
        assert_eq!(KeyCode::get_name(code), row["name"].as_str());
    }
}
