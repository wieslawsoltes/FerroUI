//! Corresponding assertions from all 34 upstream EngineTests.cs tests.
//! Explicit dependency injection and test Drop substitute for the source process defaults/CLR GC.
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
#[path = "../../../tests/support/local_future.rs"]
mod native_future;
use ferroboard_core::{editor::*, engine::*, keys::*, layouts::*, settings::*, text::*};
use std::{cell::Cell, rc::Rc, sync::Arc};
struct Harness {
    e: Rc<KeyboardEngine>,
    t: Rc<TextBufferTarget>,
    now: Rc<Cell<i64>>,
}
impl Harness {
    fn new(text: &str, attrs: InputAttributes, configure: impl FnOnce(&KeyboardSettings)) -> Self {
        let settings = Rc::new(KeyboardSettings::default());
        configure(&settings);
        Self::with_resources(
            text,
            attrs,
            settings,
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
    }
    fn with_resources(
        text: &str,
        attrs: InputAttributes,
        settings: Rc<KeyboardSettings>,
        resources: Rc<KeyboardResources>,
        models: Arc<LanguageModels>,
    ) -> Self {
        let e = KeyboardEngine::new_explicit(settings, resources, models).unwrap();
        let now = Rc::new(Cell::new(639028656000000000));
        let clock = now.clone();
        e.set_clock(Rc::new(move || fixture_clock::time(clock.get())));
        let t = Rc::new(TextBufferTarget::new(text.into(), Some(Arc::new(attrs))));
        e.attach(t.clone()).unwrap();
        e.update_layout(1000.0, 240.0, None).unwrap();
        Self { e, t, now }
    }
    fn plain() -> Self {
        Self::new("", InputAttributes::default(), |_| {})
    }
    fn text(&self) -> Utf16Text {
        self.t.text()
    }
    fn advance(&self, ms: i64) {
        self.now.set(self.now.get() + ms * 10000);
    }
    fn type_each(&self, text: &str) {
        for c in text.encode_utf16() {
            self.advance(120);
            self.e.type_text(&Utf16Text::from_units(vec![c])).unwrap();
        }
    }
    fn key(&self, code: i32) {
        self.e.input_key(key(code).as_ref()).unwrap();
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.e.dispose().unwrap();
    }
}
fn key(code: i32) -> TextKey {
    match code {
        KeyCode::Space => TextKeyData::space_key(),
        KeyCode::Enter | KeyCode::Delete => {
            TextKeyData::function(code, KeyType::EnterEditing, None)
        }
        KeyCode::Shift => TextKeyData::function(code, KeyType::Modifier, None),
        _ if KeyCode::is_character(code) => TextKeyData::char(
            char::from_u32(code as u32)
                .unwrap()
                .to_string()
                .as_str()
                .into(),
            None,
        )
        .unwrap(),
        _ => TextKeyData::function(code, KeyType::Function, None),
    }
}
#[test]
fn auto_capitalizes_start_of_sentence() {
    let h = Harness::plain();
    assert_eq!(h.e.shift_state(), ShiftState::ShiftedAutomatic);
    h.type_each("hi");
    assert_eq!(h.text(), Utf16Text::from("Hi"));
    assert_eq!(h.e.shift_state(), ShiftState::Unshifted);
}
#[test]
fn auto_capitalization_can_be_disabled() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_capitalization(false)
    });
    h.type_each("hi");
    assert_eq!(h.text(), Utf16Text::from("hi"));
}
#[test]
fn capitalizes_after_sentence_terminator() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_correct(false)
    });
    h.type_each("ok. yes");
    assert_eq!(h.text(), Utf16Text::from("Ok. Yes"));
}
#[test]
fn shift_toggles_and_double_tap_enables_caps_lock() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_capitalization(false)
    });
    h.key(KeyCode::Shift);
    assert_eq!(h.e.shift_state(), ShiftState::ShiftedManual);
    h.advance(100);
    h.key(KeyCode::Shift);
    assert_eq!(h.e.shift_state(), ShiftState::CapsLock);
    h.type_each("abc");
    assert_eq!(h.text(), Utf16Text::from("ABC"));
    assert_eq!(h.e.shift_state(), ShiftState::CapsLock);
    h.key(KeyCode::Shift);
    assert_eq!(h.e.shift_state(), ShiftState::Unshifted);
}
#[test]
fn shift_chording_resets_after_release() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_capitalization(false)
    });
    let shift = key(KeyCode::Shift);
    h.e.on_key_down(shift.as_ref()).unwrap();
    for c in [97, 98] {
        let auto = Rc::new(AutoTextKeyData {
            base: TextKeyData::from_fields(TextKeyDataFields {
                code: c,
                label: Some(
                    char::from_u32(c as u32)
                        .unwrap()
                        .to_string()
                        .as_str()
                        .into(),
                ),
                ..Default::default()
            }),
        });
        let k = auto
            .compute(&h.e.create_compute_context().unwrap())
            .unwrap()
            .unwrap();
        h.e.input_key(k.as_ref()).unwrap();
    }
    h.e.on_key_up(shift.as_ref()).unwrap();
    assert_eq!(h.text(), Utf16Text::from("AB"));
    assert_eq!(h.e.shift_state(), ShiftState::Unshifted);
}
#[test]
fn double_space_inserts_period() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_correct(false)
    });
    h.type_each("hello  ");
    assert_eq!(h.text(), Utf16Text::from("Hello. "));
    assert_eq!(h.e.shift_state(), ShiftState::ShiftedAutomatic);
}
#[test]
fn double_space_requires_quick_succession() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_correct(false)
    });
    h.type_each("hello ");
    h.advance(2000);
    h.e.type_text(&" ".into()).unwrap();
    assert_eq!(h.text(), Utf16Text::from("Hello  "));
}
#[test]
fn autocorrects_and_backspace_reverts() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_capitalization(false)
    });
    h.type_each("i like teh");
    assert!(
        h.e.suggestions()
            .snapshot()
            .unwrap()
            .into_iter()
            .map(|s| s.expect("ordinary source fixture has nonnull candidates"))
            .any(|s| s.text() == Some(Utf16Text::from("the")) && s.is_auto_commit())
    );
    h.type_each(" ");
    assert_eq!(h.text(), Utf16Text::from("i like the "));
    h.key(KeyCode::Delete);
    assert_eq!(h.text(), Utf16Text::from("i like teh"));
    h.type_each(" ");
    assert_eq!(h.text(), Utf16Text::from("i like teh "));
}
#[test]
fn no_autocorrect_in_password_fields() {
    let h = Harness::new(
        "",
        InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            kind: InputKind::Password,
            ..Default::default()
        }),
        |_| {},
    );
    h.type_each("teh ");
    assert_eq!(h.text(), Utf16Text::from("teh "));
    assert!(h.e.suggestions().len().unwrap() == 0);
}
#[test]
fn suggestions_complete_prefix() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_capitalization(false)
    });
    h.type_each("tha");
    assert!(
        h.e.suggestions()
            .snapshot()
            .unwrap()
            .into_iter()
            .map(|v| v.expect("ordinary source fixture has nonnull candidates"))
            .any(|s| {
                ["thank", "that", "thanks"]
                    .iter()
                    .any(|t| s.text() == Some(Utf16Text::from(*t)))
            })
    );
}
#[test]
fn committing_suggestion_replaces_word_and_adds_space_then_punctuation_swaps() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_capitalization(false)
    });
    h.type_each("hell");
    let suggestion =
        h.e.suggestions()
            .snapshot()
            .unwrap()
            .into_iter()
            .map(|s| s.expect("ordinary source fixture has nonnull candidates"))
            .find(|s| s.kind() == SuggestionKind::Completion)
            .unwrap()
            .clone();
    h.e.commit_suggestion(&suggestion).unwrap();
    assert_eq!(
        h.text().to_utf8().unwrap(),
        format!(
            "{} ",
            suggestion
                .text()
                .expect("source case candidate has nonnull text")
                .to_utf8()
                .unwrap()
        )
    );
    h.type_each(".");
    assert_eq!(
        h.text().to_utf8().unwrap(),
        format!(
            "{}. ",
            suggestion
                .text()
                .expect("source case candidate has nonnull text")
                .to_utf8()
                .unwrap()
        )
    );
}
#[test]
fn learns_words_and_predicts_next_word() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_correct(false);
        s.set_auto_capitalization(false);
    });
    for _ in 0..3 {
        h.type_each("zorblax quux ");
    }
    h.type_each("zorb");
    assert!(
        h.e.suggestions()
            .snapshot()
            .unwrap()
            .into_iter()
            .map(|s| s.expect("ordinary source fixture has nonnull candidates"))
            .any(|s| s.text() == Some(Utf16Text::from("zorblax")))
    );
    h.t.set_text(None);
    h.t.move_caret_externally(0);
    h.type_each("zorblax ");
    assert!(
        h.e.suggestions()
            .snapshot()
            .unwrap()
            .into_iter()
            .map(|s| s.expect("ordinary source fixture has nonnull candidates"))
            .any(|s| s.kind() == SuggestionKind::Prediction
                && s.text() == Some(Utf16Text::from("quux")))
    );
}
#[test]
fn incognito_does_not_learn() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_correct(false);
        s.set_incognito_mode(true);
    });
    h.e.type_text(&"zorblax zorblax zorblax ".into()).unwrap();
    assert!(
        h.e.language_models()
            .get_user_dictionary(&"en".into())
            .words()
            .is_empty()
    );
}
#[test]
fn delete_removes_whole_grapheme() {
    let h = Harness::new("family 👨‍👩‍👧 é́", InputAttributes::default(), |_| {});
    h.key(KeyCode::Delete);
    assert_eq!(h.text(), Utf16Text::from("family 👨‍👩‍👧 "));
    h.key(KeyCode::Delete);
    h.key(KeyCode::Delete);
    assert_eq!(h.text(), Utf16Text::from("family "));
}
#[test]
fn enter_inserts_newline_in_multiline_and_performs_action_otherwise() {
    let h = Harness::new(
        "",
        InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            is_multiline: true,
            ..Default::default()
        }),
        |_| {},
    );
    h.key(KeyCode::Enter);
    assert_eq!(h.text(), Utf16Text::from("\n"));
    let h = Harness::new(
        "",
        InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            kind: InputKind::Search,
            ..Default::default()
        }),
        |_| {},
    );
    h.key(KeyCode::Enter);
    assert!(h.text().is_empty());
    assert_eq!(*h.t.performed_actions().borrow(), vec![EnterAction::Search]);
}
#[test]
fn input_kind_selects_initial_mode_and_variation() {
    let h = Harness::new(
        "",
        InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            kind: InputKind::Number,
            ..Default::default()
        }),
        |_| {},
    );
    assert_eq!(h.e.mode(), KeyboardMode::Numeric);
    let h = Harness::new(
        "",
        InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            kind: InputKind::Phone,
            ..Default::default()
        }),
        |_| {},
    );
    assert_eq!(h.e.mode(), KeyboardMode::Phone);
    let h = Harness::new(
        "",
        InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            kind: InputKind::Email,
            ..Default::default()
        }),
        |_| {},
    );
    assert!(
        h.e.keyboard()
            .unwrap()
            .keys()
            .map(|key| key.unwrap().unwrap())
            .any(|k| k.label() == Some("@".into()))
    );
    assert_eq!(h.e.shift_state(), ShiftState::Unshifted);
}
#[test]
fn mode_switching_and_space_returns_to_characters() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_capitalization(false)
    });
    h.key(KeyCode::ViewSymbols);
    assert_eq!(h.e.mode(), KeyboardMode::Symbols);
    h.type_each("1");
    h.key(KeyCode::Space);
    assert_eq!(h.e.mode(), KeyboardMode::Characters);
    assert_eq!(h.text(), Utf16Text::from("1 "));
}
#[test]
fn subtype_switching_changes_layout() {
    let h = Harness::plain();
    let de = h.e.find_subtype(&"de-DE".into()).unwrap();
    h.e.set_subtypes([Subtype::shared_default(), de]).unwrap();
    let k = h.e.keyboard().unwrap();
    assert_eq!(
        TextCulture::to_lower_invariant(
            k.rows()
                .get(0)
                .unwrap()
                .get(0)
                .unwrap()
                .data()
                .data()
                .label()
                .as_ref()
                .unwrap()
        ),
        Utf16Text::from("q")
    );
    h.e.next_subtype().unwrap();
    assert_eq!(
        h.e.active_subtype().language_tag().unwrap(),
        Utf16Text::from("de-DE")
    );
    let k = h.e.keyboard().unwrap();
    assert_eq!(
        TextCulture::to_lower_invariant(
            k.rows()
                .get(0)
                .unwrap()
                .get(5)
                .unwrap()
                .data()
                .data()
                .label()
                .as_ref()
                .unwrap()
        ),
        Utf16Text::from("z")
    );
    assert!(k.find_key(KeyCode::LanguageSwitch).unwrap().is_some());
}
#[test]
fn hangul_composer_builds_syllables() {
    let h = Harness::plain();
    h.e.set_subtypes([h.e.find_subtype(&"ko-KR".into()).unwrap()])
        .unwrap();
    h.e.type_text(&"ㅎㅏㄴ".into()).unwrap();
    assert_eq!(h.text(), Utf16Text::from("한"));
}
#[test]
fn telex_composer_applies_rules() {
    let r = KeyboardResources::create_default().unwrap();
    let c = r.get_composer(&ComponentName::new("org.florisboard.composers", "telex"));
    assert_eq!(c.get_actions(&"a".into(), &"a".into()), (1, "â".into()));
    assert_eq!(c.get_actions(&"A".into(), &"a".into()), (1, "Â".into()));
}
#[test]
fn kana_composer_applies_dakuten() {
    assert_eq!(
        KanaComposer.get_actions(&"か".into(), &"゙".into()),
        (1, "が".into())
    );
}
#[test]
fn undo_and_redo_restore_text() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_correct(false);
        s.set_auto_capitalization(false);
    });
    h.type_each("one ");
    h.advance(3000);
    h.type_each("two");
    h.key(KeyCode::Undo);
    assert_eq!(h.text(), Utf16Text::from("one "));
    h.key(KeyCode::Redo);
    assert_eq!(h.text(), Utf16Text::from("one two"));
}
#[test]
fn arrow_keys_move_caret_and_selection_mode_selects() {
    let h = Harness::new("hello", InputAttributes::default(), |_| {});
    h.key(KeyCode::ArrowLeft);
    assert_eq!(h.t.selection_start(), 4);
    h.key(KeyCode::ClipboardSelect);
    h.key(KeyCode::ArrowLeft);
    h.key(KeyCode::ArrowLeft);
    assert_eq!(h.t.selection_start(), 2);
    assert_eq!(h.t.selection_length(), 2);
}
#[test]
fn clipboard_copy_paste_and_history() {
    let h = Harness::new("copy me", InputAttributes::default(), |_| {});
    h.key(KeyCode::ClipboardSelectAll);
    native_future::ready(h.e.copy_async()).unwrap();
    assert_eq!(
        h.e.clipboard_history().primary().unwrap().text,
        Utf16Text::from("copy me")
    );
    h.key(KeyCode::ArrowRight);
    native_future::ready(h.e.paste_async()).unwrap();
    assert_eq!(h.text(), Utf16Text::from("copy mecopy me"));
}
#[test]
fn emoji_shortcode_suggests_emoji() {
    let h = Harness::new("", InputAttributes::default(), |s| {
        s.set_auto_capitalization(false)
    });
    h.type_each("yum :pizza");
    let emoji =
        h.e.suggestions()
            .snapshot()
            .unwrap()
            .into_iter()
            .map(|s| s.expect("ordinary source fixture has nonnull candidates"))
            .find(|s| s.kind() == SuggestionKind::Emoji)
            .unwrap()
            .clone();
    h.e.commit_suggestion(&emoji).unwrap();
    assert_eq!(h.text(), Utf16Text::from("yum 🍕"));
    assert!(
        h.e.emoji_history()
            .recent()
            .lock()
            .clone()
            .contains(&Some("🍕".into()))
    );
}
#[test]
fn swipe_actions_move_cursor() {
    let h = Harness::new("abcdef", InputAttributes::default(), |_| {});
    h.e.execute_swipe_action(SwipeAction::MoveCursorLeft, 3)
        .unwrap();
    assert_eq!(h.t.selection_start(), 3);
    h.e.execute_swipe_action(SwipeAction::MoveCursorStartOfLine, 1)
        .unwrap();
    assert_eq!(h.t.selection_start(), 0);
}
#[test]
fn precise_delete_removes_words() {
    let h = Harness::new("one two three", InputAttributes::default(), |_| {});
    h.e.begin_precise_selection().unwrap();
    h.e.update_precise_selection(2, true).unwrap();
    let text = h.text();
    let start = h.t.selection_start() as usize;
    let end = start + h.t.selection_length() as usize;
    assert_eq!(
        &text.units()[start..end],
        Utf16Text::from("two three").units()
    );
    h.e.end_precise_selection(true).unwrap();
    assert_eq!(h.text(), Utf16Text::from("one "));
}
#[test]
fn external_changes_update_state() {
    let h = Harness::new("Hello. ", InputAttributes::default(), |_| {});
    assert_eq!(h.e.shift_state(), ShiftState::ShiftedAutomatic);
    h.t.move_caret_externally(3);
    assert_eq!(h.e.shift_state(), ShiftState::Unshifted);
}
#[test]
fn settings_changes_rebuild_keyboard() {
    let h = Harness::plain();
    assert_eq!(h.e.keyboard().unwrap().row_count(), 4);
    h.e.settings().set_number_row(true);
    assert_eq!(h.e.keyboard().unwrap().row_count(), 5);
}
#[test]
fn readme_example_works() {
    let h = Harness::plain();
    h.e.type_text(&"i like teh ".into()).unwrap();
    assert_eq!(h.text(), Utf16Text::from("I like the "));
}
#[test]
fn phone_pad_has_letter_sub_labels_and_space_icon() {
    let h = Harness::new(
        "",
        InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            kind: InputKind::Phone,
            ..Default::default()
        }),
        |_| {},
    );
    let two =
        h.e.keyboard()
            .unwrap()
            .visible_keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.label() == Some("2".into()))
            .unwrap();
    assert_eq!(two.sub_label(), Some("ABC".into()));
    let h = Harness::new(
        "",
        InputAttributes::from_values(ferroboard_core::editor::InputAttributesValues {
            kind: InputKind::Number,
            ..Default::default()
        }),
        |_| {},
    );
    if let Some(space) = h.e.keyboard().unwrap().find_key(KeyCode::Space).unwrap() {
        assert_eq!(space.icon(), KeyIcon::Space);
    }
}
#[test]
fn floating_and_resize_keys() {
    let h = Harness::plain();
    let resize = Rc::new(Cell::new(false));
    let mark = resize.clone();
    h.e.resize_mode_requested()
        .add(Rc::new(move |_| mark.set(true)));
    h.key(KeyCode::ToggleFloatingWindow);
    assert!(h.e.settings().floating());
    h.key(KeyCode::ToggleResizeMode);
    assert!(resize.get());
}
#[test]
fn space_bar_has_no_hint() {
    let h = Harness::plain();
    assert!(
        h.e.keyboard()
            .unwrap()
            .find_key(KeyCode::Space)
            .unwrap()
            .unwrap()
            .hint_label()
            .is_none()
    );
}
#[test]
fn composer_rules_apply_via_custom_composer() {
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    let rules = Arc::new(parking_lot::RwLock::new(
        [("--".into(), "—".into())].into_iter().collect(),
    ));
    let composer = Arc::new(RulesComposer::new("dash".into(), "Dash".into(), rules));
    resources.add_composer(composer, Some("test".into()));
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    let h = Harness::with_resources(
        "",
        InputAttributes::default(),
        settings,
        resources,
        LanguageModels::new(),
    );
    h.e.set_subtypes([Arc::new(Subtype::from_values(
        ferroboard_core::layouts::SubtypeValues {
            composer: ComponentName::new("test", "dash"),
            ..(*Subtype::shared_default()).values()
        },
    ))])
    .unwrap();
    h.e.type_text(&"a--b".into()).unwrap();
    assert_eq!(h.text(), Utf16Text::from("a—b"));
}
