//! Original public ComputedKey.PopupKeys and KeyPopupState.Keys collection contracts.
#[path = "support/scheduler.rs"]
mod scheduler;
use ferroboard_core::{editor::*, engine::*, input::*, keys::*, layouts::*, text::Utf16Text};
use scheduler::ManualScheduler;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

struct Probe(Rc<Cell<&'static str>>);
impl AbstractKeyData for Probe {
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        if self.0.get().starts_with("hidden") {
            return Ok(None);
        }
        Ok(Some(TextKeyData::char(
            "a".into(),
            Some(Rc::new(PopupSet::from_parts(
                None,
                (ResourceObjectList::<Key>::new(if self.0.get() == "empty" {
                    vec![]
                } else {
                    vec![
                        TextKeyData::char("x".into(), None)?,
                        TextKeyData::char("y".into(), None)?,
                    ]
                }))
                .into(),
            ))),
        )?))
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        Ok(Some("a".into()))
    }
}
fn setup(mode: Rc<Cell<&'static str>>) -> (Rc<KeyboardResources>, Arc<Subtype>) {
    let resources = Rc::new(KeyboardResources::create_empty());
    resources.add_layout(Rc::new(KeyboardLayout::new(
        LayoutType::Characters,
        "probe".into(),
        "main".into(),
        Rc::new(LayoutArrangement::from_rows(vec![vec![
            Rc::new(Probe(mode)) as Key,
            TextKeyData::char("b".into(), None).unwrap(),
        ]])),
    )));
    let subtype = Arc::new(Subtype::from_values(
        ferroboard_core::layouts::SubtypeValues {
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    characters: ComponentName::new("probe", "main"),
                    ..(*Subtype::shared_default().layouts().unwrap()).values()
                },
            ))),
            ..(*Subtype::shared_default()).values()
        },
    ));
    (resources, subtype)
}
fn text(k: TextKey) -> String {
    k.as_string(false).unwrap().unwrap().to_utf8().unwrap()
}
fn texts(list: &PopupKeys) -> Vec<String> {
    list.snapshot().unwrap().into_iter().map(text).collect()
}
fn observe<T: serde::Serialize>(result: Result<T, ResourceTextListError>) -> Value {
    match result {
        Ok(v) => json!({"value":v}),
        Err(e) => json!({"error":match e {
                            ResourceTextListError::IndexOutOfRange=>"ArgumentOutOfRangeException",
                            ResourceTextListError::NotSupported=>"NotSupportedException",
                ResourceTextListError::NullReference=>"NullReferenceException",
                ResourceTextListError::ArgumentNull=>"ArgumentNullException",
        ResourceTextListError::KeyNotFound=>"KeyNotFoundException",
        ResourceTextListError::DuplicateKey=>"ArgumentException",
                            ResourceTextListError::Modified=>"InvalidOperationException",
                        }}),
    }
}
fn edit(list: &PopupKeys, mutation: &str) -> Value {
    observe((|| match mutation {
        "none" => Ok(()),
        "add" => {
            list.push(TextKeyData::char("z".into(), None).unwrap());
            Ok(())
        }
        "clear" => {
            list.clear();
            Ok(())
        }
        "set-same" => list.set(0, list.get(0)?),
        "replace" => list.set(0, TextKeyData::char("z".into(), None).unwrap()),
        "remove-last" => list
            .remove(
                list.len()
                    .checked_sub(1)
                    .ok_or(ResourceTextListError::IndexOutOfRange)?,
            )
            .map(|_| ()),
        _ => unreachable!(),
    })())
}
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/computed-popup-list.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
const MUTATIONS: [&str; 6] = ["none", "add", "clear", "set-same", "replace", "remove-last"];

#[test]
fn native_computed_popup_values_mutations_and_replacement_retain_source_outcomes() {
    let expected = rows();
    let mut retained = 0;
    for (group, mutation) in MUTATIONS.into_iter().enumerate() {
        let mode = Rc::new(Cell::new("initial"));
        let (resources, subtype) = setup(mode.clone());
        let keyboard = LayoutComputer::new(resources)
            .compute(KeyboardMode::Characters, subtype, false)
            .unwrap();
        let first = keyboard.keys().next().unwrap().unwrap().unwrap();
        let second = keyboard.keys().last().unwrap().unwrap().unwrap();
        let initial = first.popup_keys();
        assert!(!initial.ptr_eq(&second.popup_keys()));
        let mut previous = None::<PopupKeys>;
        for (phase_index, phase) in [
            "initial",
            "populated",
            "empty",
            "hidden",
            "hidden-again",
            "recover",
        ]
        .into_iter()
        .enumerate()
        {
            mode.set(phase);
            if phase != "initial" {
                keyboard
                    .compute(Rc::new(KeyComputeContext::from_values(
                        KeyComputeContextValues {
                            show_number_hints: false,
                            show_symbol_hints: false,
                            ..Default::default()
                        },
                    )))
                    .unwrap();
            }
            let list = first.popup_keys();
            if let Some(previous) = &previous {
                assert!(!list.ptr_eq(previous));
            }
            let edited = edit(&list, mutation);
            if matches!(phase, "populated" | "empty" | "recover") {
                let source = &expected[group * 6 + phase_index];
                let mut actual = json!({"hasPopup":first.has_popup(),"defaultIndex":first.popup_default_index(),"visible":first.is_visible(),"text":texts(&list),"edit":edited});
                let mut projected = json!({"hasPopup":source["hasPopup"],"defaultIndex":source["defaultIndex"],"visible":source["visible"],"text":source["text"],"edit":source["edit"]});
                if phase == "empty" {
                    actual["previousText"] = json!(previous.as_ref().map(texts));
                    projected["previousText"] = source["previousText"].clone();
                }
                assert_eq!(actual, projected, "{mutation}/{phase}");
                retained += 1;
            }
            previous = Some(list);
        }
    }
    assert_eq!(retained, 18);
}

#[test]
fn native_default_popup_buffers_are_mutable_independent_and_detached_on_compute() {
    let (resources, subtype) = setup(Rc::new(Cell::new("populated")));
    let keyboard = LayoutComputer::new(resources)
        .compute(KeyboardMode::Characters, subtype, false)
        .unwrap();
    let first = keyboard.keys().next().unwrap().unwrap().unwrap();
    let second = keyboard.keys().last().unwrap().unwrap().unwrap();
    let original = first.popup_keys();
    assert!(original.is_empty());
    original.push(TextKeyData::char("z".into(), None).unwrap());
    assert!(second.popup_keys().is_empty());
    assert!(first.popup_keys().ptr_eq(&original));
    keyboard
        .compute(Rc::new(KeyComputeContext::default()))
        .unwrap();
    assert!(!first.popup_keys().ptr_eq(&original));
    assert_eq!(texts(&original), ["z"]);
    assert!(!first.popup_keys().is_empty());
}

#[test]
fn original_open_popup_retains_list_after_compute_and_observes_open_close_callback_edits() {
    let expected = rows();
    let mut count = 36;
    for timing in ["opened", "closed"] {
        for mutation in MUTATIONS {
            let (resources, subtype) = setup(Rc::new(Cell::new("populated")));
            let settings = Rc::new(ferroboard_core::settings::KeyboardSettings::default());
            settings.set_glide_typing(false);
            settings.set_show_suggestions(false);
            settings.set_auto_capitalization(false);
            let engine =
                KeyboardEngine::new_explicit(settings, resources, LanguageModels::new()).unwrap();
            engine.set_subtypes([subtype]).unwrap();
            let target = Rc::new(TextBufferTarget::new("".into(), None));
            engine.attach(target.clone()).unwrap();
            engine.update_layout(600., 240., None).unwrap();
            let scheduler = ManualScheduler::new();
            let touch = KeyboardTouchProcessor::new(engine.clone(), scheduler.clone());
            let key = engine
                .keyboard()
                .unwrap()
                .keys()
                .map(|key| key.unwrap().unwrap())
                .next()
                .unwrap();
            let retained = key.popup_keys();
            let observed = Rc::new(RefCell::new(None::<Rc<KeyPopupState>>));
            let events = Rc::new(RefCell::new(Vec::<Value>::new()));
            let (o, ev, r) = (observed.clone(), events.clone(), retained.clone());
            let subscription = touch.popup_changed().add(Rc::new(move |sender| {
                let popup = sender.popup();
                let mut edited = Value::Null;
                if timing == "opened"
                    && let Some(p) = &popup
                {
                    edited = edit(&p.keys(), mutation);
                }
                if timing == "closed"
                    && popup.is_none()
                    && let Some(p) = o.borrow().as_ref()
                {
                    edited = edit(&p.keys(), mutation);
                }
                ev.borrow_mut()
                    .push(json!({"open":popup.is_some(),"edit":edited,"retainedText":texts(&r)}));
            }));
            let b = key.touch_bounds();
            touch.pointer_down(1, b.center_x(), b.center_y()).unwrap();
            scheduler.advance(engine.settings().long_press_delay() as i64 * 10000);
            assert_eq!(
                scheduler.now(),
                engine.settings().long_press_delay() as i64 * 10000
            );
            let popup = touch.popup().unwrap();
            observed.replace(Some(popup.clone()));
            let opened_same = popup.keys().ptr_eq(&retained);
            engine.recompute_keyboard().unwrap();
            let after_compute_same = popup.keys().ptr_eq(&key.popup_keys());
            touch.pointer_up(1, b.center_x(), b.center_y()).unwrap();
            let actual = json!({"kind":"touch","timing":timing,"mutation":mutation,"openedSame":opened_same,"afterComputeSame":after_compute_same,"popupCleared":touch.popup().is_none(),"selected":popup.selected_index(),"bounds":popup.bounds().borrow().len(),"text":target.text().to_utf8().unwrap(),"retainedText":texts(&retained),"currentText":texts(&key.popup_keys()),"events":*events.borrow()});
            assert_eq!(actual, expected[count], "{timing}/{mutation}");
            count += 1;
            touch.popup_changed().remove(subscription);
            touch.cancel_all().unwrap();
            engine.dispose().unwrap();
        }
    }
    assert_eq!(count, 48);
}
