#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
#[path = "../../../tests/support/fixture_duration.rs"]
mod fixture_duration;
use ferroboard_core::{editor::*, layouts::PunctuationRule, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn text(v: &Value) -> Utf16Text {
    Utf16Text::from_units(v.as_array().map_or_else(Vec::new, |a| {
        a.iter()
            .map(|c| c.as_u64().unwrap() as u16)
            .collect::<Vec<_>>()
    }))
}
fn snapshot(e: &EditorController) -> Value {
    let caps = [
        CapitalizationMode::None,
        CapitalizationMode::Characters,
        CapitalizationMode::Words,
        CapitalizationMode::Sentences,
    ]
    .iter()
    .map(|m| {
        e.should_auto_capitalize(*m, &PunctuationRule::shared_default())
            .unwrap()
    })
    .collect::<Vec<_>>();
    json!([
        e.text().units(),
        e.selection_start(),
        e.selection_length().unwrap(),
        e.selection_end().unwrap(),
        e.has_selection().unwrap(),
        e.selected_text().unwrap().units(),
        e.get_text_before_cursor(i32::MAX).unwrap().units(),
        e.get_text_after_cursor(i32::MAX).unwrap().units(),
        e.current_word().unwrap().units(),
        e.is_composing_word().unwrap(),
        e.selection_anchor(),
        e.is_selecting(),
        e.can_undo(),
        e.can_redo(),
        caps
    ])
}
fn apply(
    e: &EditorController,
    t: &TextBufferTarget,
    clock: &Cell<i64>,
    op: &Value,
) -> EditorResult<Value> {
    let name = op["Name"].as_str().unwrap();
    let arg = op["Arg"].as_i64().unwrap() as i32;
    let txt = text(&op["Text"]);
    match name {
        "commit" => e.commit_text(&txt)?,
        "replaceBefore" => e.replace_before_cursor(arg, &txt)?,
        "replaceWord" => e.replace_current_word(&txt)?,
        "backward" => return Ok(json!(e.delete_backward()?)),
        "forward" => return Ok(json!(e.delete_forward()?)),
        "wordBackward" => return Ok(json!(e.delete_word_backward()?)),
        "wordForward" => return Ok(json!(e.delete_word_forward()?)),
        "deleteChars" => e.delete_characters_before(arg)?,
        "horizontal" => e.move_horizontal(arg)?,
        "word" => e.move_word(arg)?,
        "vertical" => e.move_vertical(arg)?,
        "lineStart" => e.move_line_start()?,
        "lineEnd" => e.move_line_end()?,
        "documentStart" => e.move_document_start(),
        "documentEnd" => e.move_document_end(),
        "caret" => e.set_caret(arg),
        "selectAll" => e.select_all(),
        "toggle" => e.toggle_selection_mode()?,
        "endSelect" => e.end_selection_mode(),
        "break" => e.break_undo_group(),
        "undo" => return Ok(json!(e.undo()?)),
        "redo" => return Ok(json!(e.redo()?)),
        "clear" => e.clear_history(),
        "external" => t.set_text(Some(txt)),
        "select" => t.select(arg, op["Second"].as_i64().unwrap() as i32),
        "clock" => clock.set(op["Ticks"].as_i64().unwrap()),
        "before" => return Ok(json!(e.get_text_before_cursor(arg)?.units())),
        "after" => return Ok(json!(e.get_text_after_cursor(arg)?.units())),
        "max" => e.set_max_undo_steps(arg),
        "timeout" => {
            e.set_undo_group_timeout(fixture_duration::duration(op["Ticks"].as_i64().unwrap()))
        }
        _ => panic!("unknown {name}"),
    }
    Ok(Value::Null)
}
fn error_name(e: EditorError) -> &'static str {
    match e {
        EditorError::Target(e) => panic!("unexpected target error in original nonnull corpus: {e}"),
        EditorError::NullReference => "NullReferenceException",
        EditorError::SubstringOutOfRange => "ArgumentOutOfRangeException",
        EditorError::InvalidClampRange => "ArgumentException",
        EditorError::AbsoluteValueOverflow => "OverflowException",
    }
}
#[test]
fn unchanged_source_edit_sequences_and_callback_states() {
    let mut retired_negative_spans = 0;
    for (n, line) in include_str!("../../../tests/fixtures/editor.jsonl")
        .lines()
        .enumerate()
    {
        let v: Value = serde_json::from_str(line).unwrap();
        // Native Duration has no negative values. Retain these C# profiles unchanged
        // as reference data, outside the migrated public timeout contract.
        if v["timeout"].as_i64().unwrap() < 0 {
            retired_negative_spans += 1;
            continue;
        }
        let t = Rc::new(TextBufferTarget::new(text(&v["initial"]), None));
        let e = Rc::new(EditorController::new(t.clone()));
        let now = Rc::new(Cell::new(0));
        e.set_clock(Rc::new({
            let now = now.clone();
            move || fixture_clock::time(now.get())
        }));
        e.set_max_undo_steps(v["max"].as_i64().unwrap() as i32);
        e.set_undo_group_timeout(fixture_duration::duration(v["timeout"].as_i64().unwrap()));
        let events = Rc::new(RefCell::new(Vec::<Value>::new()));
        let edited = t.edited().add(Rc::new({
            let e = Rc::downgrade(&e);
            let events = events.clone();
            move |_| {
                events
                    .borrow_mut()
                    .push(json!(["edited", snapshot(&e.upgrade().unwrap())]))
            }
        }));
        let changed = t.changed().add(Rc::new({
            let e = Rc::downgrade(&e);
            let events = events.clone();
            move |_| {
                events
                    .borrow_mut()
                    .push(json!(["changed", snapshot(&e.upgrade().unwrap())]))
            }
        }));
        for (i, op) in v["ops"].as_array().unwrap().iter().enumerate() {
            events.borrow_mut().clear();
            let (result, error) = match apply(&e, &t, &now, op) {
                Ok(value) => (value, Value::Null),
                Err(e) => (Value::Null, json!(error_name(e))),
            };
            assert_eq!(
                json!([result, error, snapshot(&e), *events.borrow()]),
                v["results"][i],
                "trace {n}, step {i}, op {op}"
            );
        }
        assert!(t.edited().remove(edited));
        assert!(t.changed().remove(changed));
        assert_eq!(t.edited().handler_count(), 0);
        assert_eq!(t.changed().handler_count(), 0);
    }
    assert_eq!(retired_negative_spans, 8);
}
#[test]
fn source_vertical_navigation_keeps_column() {
    let target = Rc::new(TextBufferTarget::new("abcdef\nab\nabcdef".into(), None));
    let editor = EditorController::new(target.clone());
    editor.set_caret(5);
    editor.move_vertical(1).unwrap();
    assert_eq!(target.selection_start(), 9);
    editor.move_vertical(1).unwrap();
    assert_eq!(target.selection_start(), 15);
    editor.move_vertical(-1).unwrap();
    editor.move_vertical(-1).unwrap();
    assert_eq!(target.selection_start(), 5);
}
#[test]
fn controller_ownership_and_native_defaults() {
    let target = Rc::new(TextBufferTarget::default());
    let weak = Rc::downgrade(&target);
    let editor = EditorController::new(target.clone());
    assert!(Rc::ptr_eq(
        &editor.target(),
        &(target.clone() as Rc<dyn ITextInputTarget>)
    ));
    drop(target);
    assert!(weak.upgrade().is_some());
    assert_eq!(editor.max_undo_steps(), 100);
    assert_eq!(
        editor.undo_group_timeout(),
        std::time::Duration::from_millis(1500)
    );
    let now = editor.clock().unwrap()();
    assert!(now > std::time::UNIX_EPOCH);
    drop(editor);
    assert!(weak.upgrade().is_none());
}
#[test]
fn unchanged_source_reentrant_edit_and_clock_callbacks() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/editor-reentrant.json"
    ))
    .unwrap();
    for expected in corpus.as_array().unwrap() {
        let mode = expected["mode"].as_str().unwrap().to_owned();
        let t = Rc::new(TextBufferTarget::new("abc".into(), None));
        let e = Rc::new(EditorController::new(t.clone()));
        e.set_clock(Rc::new(|| fixture_clock::time(30_000_000)));
        e.set_caret(1);
        e.toggle_selection_mode().unwrap();
        let once = Rc::new(Cell::new(true));
        let log = Rc::new(RefCell::new(Vec::<Value>::new()));
        if mode == "clock" {
            e.set_clock(Rc::new({
                let e = Rc::downgrade(&e);
                let once = once.clone();
                let log = log.clone();
                move || {
                    let e = e.upgrade().unwrap();
                    log.borrow_mut().push(json!(["clock", snapshot(&e)]));
                    if once.replace(false) {
                        e.commit_text(&"inner".into()).unwrap();
                        log.borrow_mut().push(json!(["after nested", snapshot(&e)]));
                    }
                    fixture_clock::time(30_000_000)
                }
            }));
        }
        let first = t.edited().add(Rc::new({
            let e = Rc::downgrade(&e);
            let once = once.clone();
            let log = log.clone();
            let mode = mode.clone();
            move |_| {
                let e = e.upgrade().unwrap();
                log.borrow_mut().push(json!(["edited", snapshot(&e)]));
                if once.get() && mode != "clock" {
                    once.set(false);
                    match mode.as_str() {
                        "commit" => e.commit_text(&"Y".into()).unwrap(),
                        "undo" => {
                            e.undo().unwrap();
                        }
                        "selection" => e.select_all(),
                        _ => unreachable!(),
                    };
                    log.borrow_mut().push(json!(["after nested", snapshot(&e)]));
                }
            }
        }));
        let second = t.edited().add(Rc::new({
            let e = Rc::downgrade(&e);
            let log = log.clone();
            move |_| {
                log.borrow_mut()
                    .push(json!(["tail", snapshot(&e.upgrade().unwrap())]))
            }
        }));
        e.commit_text(&"X".into()).unwrap();
        log.borrow_mut().push(json!(["after outer", snapshot(&e)]));
        e.undo().unwrap();
        log.borrow_mut().push(json!(["after undo", snapshot(&e)]));
        e.redo().unwrap();
        log.borrow_mut().push(json!(["after redo", snapshot(&e)]));
        assert_eq!(json!(*log.borrow()), expected["log"], "{mode}");
        t.edited().remove(first);
        t.edited().remove(second);
    }
}
struct ProbeTarget {
    texts: Vec<Utf16Text>,
    index: Cell<usize>,
    start: i32,
    length: i32,
    calls: RefCell<Vec<Value>>,
    changed: ferroui_system::events::Event<dyn ITextInputTarget>,
}
impl ITextInputTarget for ProbeTarget {
    fn attributes(&self) -> std::sync::Arc<InputAttributes> {
        InputAttributes::shared_default()
    }
    fn text(&self) -> Utf16Text {
        self.calls.borrow_mut().push(json!("text"));
        let i = self.index.get();
        self.index.set(i + 1);
        self.texts[i.min(self.texts.len() - 1)].clone()
    }
    fn selection_start(&self) -> i32 {
        self.calls.borrow_mut().push(json!("start"));
        self.start
    }
    fn selection_length(&self) -> i32 {
        self.calls.borrow_mut().push(json!("length"));
        self.length
    }
    fn replace(
        &self,
        start: i32,
        length: i32,
        text: Option<&Utf16Text>,
    ) -> ferroboard_core::editor::TargetReplaceResult {
        self.calls.borrow_mut().push(json!([
            "replace",
            start,
            length,
            text.map(Utf16Text::units)
        ]));
        Ok(())
    }
    fn select(&self, start: i32, length: i32) {
        self.calls
            .borrow_mut()
            .push(json!(["select", start, length]));
    }
    fn perform_enter_action(&self, _: EnterAction) -> bool {
        false
    }
    fn changed(&self) -> &ferroui_system::events::Event<dyn ITextInputTarget> {
        &self.changed
    }
}
#[test]
fn unchanged_source_custom_target_getter_order_and_invalid_selection() {
    let corpus: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/editor-targets.json")).unwrap();
    for v in corpus.as_array().unwrap() {
        let t = Rc::new(ProbeTarget {
            texts: v["texts"].as_array().unwrap().iter().map(text).collect(),
            index: Cell::new(0),
            start: v["start"].as_i64().unwrap() as i32,
            length: v["length"].as_i64().unwrap() as i32,
            calls: RefCell::new(Vec::new()),
            changed: Default::default(),
        });
        let e = EditorController::new(t.clone());
        e.set_clock(Rc::new(|| fixture_clock::time(30)));
        let result: EditorResult<Value> = (|| {
            Ok(match v["operation"].as_str().unwrap() {
                "start" => json!(e.selection_start()),
                "length" => json!(e.selection_length()?),
                "end" => json!(e.selection_end()?),
                "selected" => json!(e.selected_text()?.units()),
                "before" => json!(e.get_text_before_cursor(1)?.units()),
                "after" => json!(e.get_text_after_cursor(1)?.units()),
                "word" => json!(e.current_word()?.units()),
                "composing" => json!(e.is_composing_word()?),
                "commit" => {
                    e.commit_text(&"X".into())?;
                    Value::Null
                }
                "replaceBefore" => {
                    e.replace_before_cursor(2, &"X".into())?;
                    Value::Null
                }
                _ => unreachable!(),
            })
        })();
        let (result, error) = match result {
            Ok(value) => (value, Value::Null),
            Err(e) => (Value::Null, json!(error_name(e))),
        };
        assert_eq!(
            json!([result, error, *t.calls.borrow(), e.can_undo(), e.can_redo()]),
            json!([v["result"], v["error"], v["calls"], v["undo"], v["redo"]]),
            "{v}"
        );
    }
}
