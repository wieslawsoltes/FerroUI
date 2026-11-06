#[path = "../../../tests/support/fixture_clipboard_time.rs"]
mod fixture_clipboard_time;
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
use ferroboard_core::{
    editor::*, engine::*, keys::*, layouts::KeyboardResources, settings::KeyboardSettings, text::*,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/engine-target.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn bits(n: f64) -> String {
    format!("{:016x}", n.to_bits())
}
fn units(t: &Utf16Text) -> Value {
    json!(t.units())
}
fn suggestions(e: &KeyboardEngine) -> Value {
    json!(e.suggestions().snapshot().unwrap().into_iter().map(|v|v.expect("ordinary source fixture has nonnull candidates")).map(|s|json!({"text":s.text().as_ref().map(units),"kind":s.kind().0,"confidence":bits(s.confidence()),"auto":s.is_auto_commit(),"description":s.description().as_ref().map(units),"removable":s.is_removable()})).collect::<Vec<_>>())
}
fn state(e: &KeyboardEngine) -> Value {
    let target = e.target();
    let editor = e.editor();
    let context = e.create_compute_context().unwrap();
    json!({"attached":target.is_some(),"text":units(&target.as_ref().map(|t| t.text()).unwrap_or_default()),"start":target.as_ref().map_or(0,|t|t.selection_start()),"length":target.as_ref().map_or(0,|t|t.selection_length()),"mode":e.mode().0,"shift":e.shift_state().0,"panel":e.ui_mode().0,"version":e.keyboard().unwrap().version(),"selecting":editor.as_ref().is_some_and(|v|v.is_selecting()),"undo":editor.as_ref().is_some_and(|v|v.can_undo()),"redo":editor.as_ref().is_some_and(|v|v.can_redo()),"incognito":e.is_incognito(),"variation":context.variation().0,"enter":context.enter_action().0,"suggestions":suggestions(e)})
}
struct Provider(Rc<RefCell<Vec<Value>>>);
impl PartialEq for Provider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ISuggestionProvider for Provider {
    fn suggest(&self, r: Option<&SuggestionRequest>) -> SuggestionResult<Option<SuggestionList>> {
        let result = {
            let r = r.expect("engine constructs a request");
            self.0.borrow_mut().push(json!({"word":units(&r.word().unwrap()),"before":units(&r.preceding_text().unwrap()),"max":r.max_count(),"allow":r.allow_auto_correct(),"culture":r.culture().unwrap().name(),"near":r.key_proximity().as_ref().map(|f|bits(f(101,114))),"far":r.key_proximity().as_ref().map(|f|bits(f(113,109)))}));
            if r.word() == Some(Utf16Text::from("throw")) {
                return Err(SuggestionError::Custom("custom provider".into()));
            }
            Ok([
                ("first", SuggestionKind::Completion, 0.9, false),
                ("second", SuggestionKind::Correction, 0.8, true),
                ("third", SuggestionKind::Typed, 0.7, false),
                ("fourth", SuggestionKind::Prediction, 0.6, false),
            ]
            .into_iter()
            .map(|(text, kind, confidence, auto)| {
                Rc::new(Suggestion::from_values(SuggestionValues {
                    confidence,
                    is_auto_commit: auto,
                    ..SuggestionValues::new(text.into(), kind)
                }))
            })
            .collect())
        };
        result.map(|values| Some(SuggestionList::new(values)))
    }
    fn learn(&self, _: Option<&Utf16Text>, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
    fn forget(&self, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
}
fn input_kind(n: i64) -> InputKind {
    match n {
        0 => InputKind::Text,
        1 => InputKind::Number,
        2 => InputKind::Decimal,
        3 => InputKind::Phone,
        4 => InputKind::DateTime,
        5 => InputKind::Email,
        6 => InputKind::Uri,
        7 => InputKind::Password,
        8 => InputKind::NumericPassword,
        9 => InputKind::Search,
        10 => InputKind::Chat,
        _ => panic!("fixture kind"),
    }
}
#[test]
fn unchanged_engine_target_input_suggestions_privacy_and_event_traces() {
    let rows = rows();
    let mut names = Vec::new();
    for row in &rows {
        if row["kind"] == "op" && !names.contains(&row["name"]) {
            names.push(row["name"].clone());
        }
    }
    for name in names {
        let cases = rows
            .iter()
            .filter(|r| r["kind"] == "op" && r["name"] == name)
            .collect::<Vec<_>>();
        let first = cases[0];
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_glide_typing(false);
        let e = KeyboardEngine::new_explicit(
            settings.clone(),
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        e.set_clock(Rc::new(|| fixture_clock::time(10_000_000_000)));
        let target = Rc::new(TextBufferTarget::new(
            Utf16Text::default(),
            Some(Arc::new(InputAttributes::from_values(
                ferroboard_core::editor::InputAttributesValues {
                    kind: input_kind(first["attrs"]["kind"].as_i64().unwrap()),
                    is_private: first["attrs"]["privacy"].as_bool().unwrap(),
                    ..(InputAttributes::default()).values()
                },
            ))),
        ));
        let log = Rc::new(RefCell::new(Vec::<Value>::new()));
        let requests = Rc::new(RefCell::new(Vec::<Value>::new()));
        let factories = Rc::new(Cell::new(0));
        if first["custom"] == true {
            let count = factories.clone();
            let requests = requests.clone();
            e.set_suggestion_provider_factory(Some(Rc::new(move |_| {
                count.set(count.get() + 1);
                Ok(Some(Rc::new(Provider(requests.clone()))))
            })));
        }
        for (kind, event) in [
            ("keyboard", e.keyboard_changed()),
            ("state", e.state_changed()),
            ("suggestions", e.suggestions_changed()),
        ] {
            let log = log.clone();
            event.add(Rc::new(move |e| {
                log.borrow_mut().push(json!({"kind":kind,"state":state(e)}));
            }));
        }
        let weak = Rc::downgrade(&e);
        let commits = log.clone();
        e.text_committed().add(Rc::new(move |text| {
            let e = weak.upgrade().unwrap();
            commits
                .borrow_mut()
                .push(json!({"kind":"committed","text":units(text),"state":state(&e)}));
        }));
        for row in cases {
            log.borrow_mut().clear();
            requests.borrow_mut().clear();
            let op = row["op"].as_array().unwrap();
            match op[0].as_str().unwrap() {
                "attach" => e.attach(target.clone()).unwrap(),
                "detach" => e.detach().unwrap(),
                "input" => e.input_text(&op[1].as_str().unwrap().into()).unwrap(),
                "external" => target.set_text(Some(op[1].as_str().unwrap().into())),
                "caret" => target.move_caret_externally(op[1].as_i64().unwrap() as i32),
                "layout" => e.update_layout(800.0, 240.0, None).unwrap(),
                "panel" => e.set_ui_mode(KeyboardUiMode::Media).unwrap(),
                "mode" => e.set_mode(KeyboardMode::Characters).unwrap(),
                "selecting" => e.editor().unwrap().toggle_selection_mode().unwrap(),
                "refresh" => e.update_suggestions().unwrap(),
                "clipboard" => {
                    let fresh = op[2].as_bool().unwrap();
                    e.clipboard_history().set_clock(Rc::new(move || {
                        fixture_clipboard_time::timestamp(
                            if fresh { 3_155_378_975_999_999_999 } else { 0 },
                            0,
                        )
                        .unwrap()
                    }));
                    e.clipboard_history()
                        .add(Some(&op[1].as_str().unwrap().into()))
                        .unwrap();
                }
                "incognito" => settings.set_incognito_mode(op[1].as_bool().unwrap()),
                "emoji" => settings.set_emoji_suggestions(op[1].as_bool().unwrap()),
                "show" => settings.set_show_suggestions(op[1].as_bool().unwrap()),
                "max" => settings.set_suggestion_count(op[1].as_i64().unwrap() as i32),
                _ => panic!("unknown op {op:?}"),
            }
            assert_eq!(state(&e), row["state"], "{name} {op:?}");
            assert_eq!(json!(*log.borrow()), row["events"], "events {name} {op:?}");
            assert_eq!(
                json!(*requests.borrow()),
                row["requests"],
                "requests {name} {op:?}"
            );
            assert_eq!(
                json!(factories.get()),
                row["factories"],
                "factories {name} {op:?}"
            );
        }
        e.dispose().unwrap();
    }
}
#[test]
fn target_identity_clock_snapshot_disposal_and_empty_suggestion_identity() {
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    settings.set_show_suggestions(false);
    settings.set_emoji_suggestions(false);
    let e = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    let target = Rc::new(TextBufferTarget::default());
    let first: EditorClock = Rc::new(|| fixture_clock::time(123));
    let second: EditorClock = Rc::new(|| fixture_clock::time(456));
    e.set_clock(first.clone());
    e.attach(target.clone()).unwrap();
    assert!(Rc::ptr_eq(&e.editor().unwrap().clock().unwrap(), &first));
    let editor = e.editor().unwrap();
    e.set_clock(second.clone());
    e.attach(target.clone()).unwrap();
    assert!(Rc::ptr_eq(&editor, &e.editor().unwrap()));
    assert!(Rc::ptr_eq(&editor.clock().unwrap(), &first));
    let suggestions = e.suggestions();
    e.update_suggestions().unwrap();
    assert!(suggestions.ptr_eq(&e.suggestions()));
    assert_eq!(target.changed().handler_count(), 1);
    e.detach().unwrap();
    assert_eq!(target.changed().handler_count(), 0);
    e.attach(target.clone()).unwrap();
    assert!(Rc::ptr_eq(&e.editor().unwrap().clock().unwrap(), &second));
    let weak = Rc::downgrade(&e);
    e.dispose().unwrap();
    assert_eq!(target.changed().handler_count(), 0);
    drop(e);
    assert!(weak.upgrade().is_none());
}

use ferroui_system::events::Event;
type ReplaceHook = Rc<dyn Fn()>;
struct EchoTarget {
    buffer: TextBufferTarget,
    changed: Event<dyn ITextInputTarget>,
    on_replace: RefCell<Option<ReplaceHook>>,
}
impl EchoTarget {
    fn new() -> Rc<Self> {
        let t = Rc::new(Self {
            buffer: TextBufferTarget::default(),
            changed: Event::new(),
            on_replace: RefCell::new(None),
        });
        let weak = Rc::downgrade(&t);
        t.buffer.changed().add(Rc::new(move |_| {
            let t = weak.upgrade().unwrap();
            t.changed.raise(t.as_ref());
        }));
        t
    }
}
impl ITextInputTarget for EchoTarget {
    fn attributes(&self) -> Arc<InputAttributes> {
        self.buffer.attributes()
    }
    fn text(&self) -> Utf16Text {
        self.buffer.text()
    }
    fn selection_start(&self) -> i32 {
        self.buffer.selection_start()
    }
    fn selection_length(&self) -> i32 {
        self.buffer.selection_length()
    }
    fn replace(
        &self,
        start: i32,
        length: i32,
        text: Option<&Utf16Text>,
    ) -> ferroboard_core::editor::TargetReplaceResult {
        self.buffer.replace(start, length, text)?;
        self.changed.raise(self);
        let hook = self.on_replace.borrow().clone();
        if let Some(hook) = hook {
            hook();
        }
        Ok(())
    }
    fn select(&self, start: i32, length: i32) {
        self.buffer.select(start, length);
        self.changed.raise(self);
    }
    fn perform_enter_action(&self, _: EnterAction) -> bool {
        false
    }
    fn changed(&self) -> &Event<dyn ITextInputTarget> {
        &self.changed
    }
}
struct DetachingProvider(std::rc::Weak<KeyboardEngine>);
impl PartialEq for DetachingProvider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ISuggestionProvider for DetachingProvider {
    fn suggest(&self, _: Option<&SuggestionRequest>) -> SuggestionResult<Option<SuggestionList>> {
        let result = {
            self.0.upgrade().unwrap().detach().unwrap();
            Ok(vec![Rc::new(Suggestion::new(
                "detached".into(),
                SuggestionKind::Typed,
            ))])
        };
        result.map(|values| Some(SuggestionList::new(values)))
    }
    fn learn(&self, _: Option<&Utf16Text>, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
    fn forget(&self, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
}
#[test]
fn source_reentrant_target_and_commit_callbacks_and_provider_failure_boundary() {
    for row in rows().iter().filter(|r| r["kind"] == "reentrant") {
        let mode = row["mode"].as_str().unwrap();
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_glide_typing(false);
        settings.set_show_suggestions(false);
        settings.set_emoji_suggestions(false);
        let e = KeyboardEngine::new_explicit(
            settings.clone(),
            Rc::new(KeyboardResources::create_default().unwrap()),
            LanguageModels::new(),
        )
        .unwrap();
        e.set_clock(Rc::new(|| fixture_clock::time(10_000_000_000)));
        let t = EchoTarget::new();
        e.attach(t.clone()).unwrap();
        let log = Rc::new(RefCell::new(Vec::<Value>::new()));
        for (kind, event) in [
            ("keyboard", e.keyboard_changed()),
            ("state", e.state_changed()),
            ("suggestions", e.suggestions_changed()),
        ] {
            let log = log.clone();
            event.add(Rc::new(move |e| {
                log.borrow_mut().push(json!({"kind":kind,"state":state(e)}));
            }));
        }
        let weak = Rc::downgrade(&e);
        let commits = log.clone();
        e.text_committed().add(Rc::new(move |text| {
            let e = weak.upgrade().unwrap();
            commits
                .borrow_mut()
                .push(json!({"kind":"committed","text":units(text),"state":state(&e)}));
        }));
        match mode {
            "nested" => {
                let once = Cell::new(true);
                let engine = Rc::downgrade(&e);
                let target = Rc::downgrade(&t);
                *t.on_replace.borrow_mut() = Some(Rc::new(move || {
                    if once.replace(false) {
                        engine.upgrade().unwrap().input_text(&"b".into()).unwrap();
                        target.upgrade().unwrap().buffer.set_text(Some("".into()));
                    }
                }));
            }
            "committed-detach" => {
                let weak = Rc::downgrade(&e);
                e.text_committed()
                    .add(Rc::new(move |_| weak.upgrade().unwrap().detach().unwrap()));
            }
            "provider-detach" => {
                settings.set_show_suggestions(true);
                let weak = Rc::downgrade(&e);
                e.set_suggestion_provider_factory(Some(Rc::new(move |_| {
                    Ok(Some(Rc::new(DetachingProvider(weak.clone()))))
                })));
                log.borrow_mut().clear();
            }
            "factory-fail" => {
                settings.set_show_suggestions(true);
                e.set_suggestion_provider_factory(Some(Rc::new(|_| {
                    Err(EngineError::Suggestion(SuggestionError::Custom(
                        "factory".into(),
                    )))
                })));
                log.borrow_mut().clear();
            }
            "clock-fail" => {
                let once = Cell::new(true);
                e.editor().unwrap().set_clock(Rc::new(move || {
                    assert!(!once.replace(false), "clock");
                    fixture_clock::time(10_000_000_000)
                }));
            }
            "long-word" => {
                settings.set_show_suggestions(true);
                log.borrow_mut().clear();
            }
            _ => unreachable!(),
        }
        let error = if mode == "clock-fail" {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                e.input_text(&"a".into())
            }));
            t.buffer.set_text(Some("hello".into()));
            assert!(result.is_err());
            true
        } else {
            let result = if matches!(mode, "provider-detach" | "factory-fail") {
                e.update_suggestions()
            } else {
                e.input_text(&if mode == "long-word" {
                    "a".repeat(65).as_str().into()
                } else {
                    "a".into()
                })
            };
            result.is_err()
        };
        assert_eq!(error, row["error"], "{row}");
        assert_eq!(state(&e), row["state"], "{row}");
        assert_eq!(json!(*log.borrow()), row["events"], "{row}");
        e.dispose().unwrap();
    }
}
