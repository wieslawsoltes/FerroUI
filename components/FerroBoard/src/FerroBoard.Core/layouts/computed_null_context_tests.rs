//! Exact unchanged-source state/callback traces around nullable ComputedKeyboard.Compute.
use super::*;
use crate::{keys::*, settings::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    rc::{Rc, Weak},
};
#[derive(Debug)]
struct Marker;
struct Probe {
    name: &'static str,
    kind: String,
    warm: TextKey,
    chosen: TextKey,
    phase: Rc<Cell<u8>>,
    fired: Rc<Cell<bool>>,
    owner: Rc<RefCell<Option<Weak<ComputedKeyboard>>>>,
    trace: Rc<RefCell<Vec<String>>>,
    reentrant: Rc<KeyComputeContext>,
}
fn mutate(board: &ComputedKeyboard, kind: &str) {
    let rows = board.rows();
    match kind {
        "outer-clear" => rows.clear(),
        "outer-set-same" => rows.set(0, rows.get(0).unwrap()).unwrap(),
        "outer-remove-last" => {
            rows.remove(rows.len() - 1).unwrap();
        }
        "outer-add" => rows.push(ComputedRow::new(vec![rows.get(1).unwrap().get(0).unwrap()])),
        "inner-clear" => rows.get(0).unwrap().clear(),
        "inner-set-same" => {
            let row = rows.get(0).unwrap();
            row.set(0, row.get(0).unwrap()).unwrap();
        }
        "inner-remove-last" => {
            let row = rows.get(0).unwrap();
            row.remove(row.len() - 1).unwrap();
        }
        _ => panic!("{kind}"),
    }
}
impl AbstractKeyData for Probe {
    fn compute_nullable(
        self: Rc<Self>,
        e: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        self.trace.borrow_mut().push(format!(
            "{}:{}",
            self.name,
            e.map_or_else(
                || "null".into(),
                |e| format!("{}:{}", e.mode().0, e.shift_state().0)
            )
        ));
        if self.kind == "always-none" {
            return Ok(None);
        }
        if e.is_some() {
            return Ok(Some(self.warm.clone()));
        }
        if self.name == "a" && self.phase.get() == 1 && !self.fired.replace(true) {
            if self.kind == "reenter" {
                self.trace.borrow_mut().push("mutate:reenter".into());
                let board = self.owner.borrow().as_ref().unwrap().upgrade().unwrap();
                board.compute(self.reentrant.clone())?;
            } else if self.kind.starts_with("outer-") || self.kind.starts_with("inner-") {
                self.trace
                    .borrow_mut()
                    .push(format!("mutate:{}", self.kind));
                let board = self.owner.borrow().as_ref().unwrap().upgrade().unwrap();
                mutate(&board, &self.kind);
            }
        }
        if self.kind == "throw" {
            resume_unwind(Box::new(Marker))
        }
        if self.kind == "none"
            || self.kind.starts_with("outer-")
            || self.kind.starts_with("inner-")
            || self.kind == "reenter"
        {
            Ok(None)
        } else {
            Ok(Some(self.chosen.clone()))
        }
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        Ok(Some(self.name.into()))
    }
}
fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
fn rect(v: KeyRect) -> Value {
    json!([bits(v.x), bits(v.y), bits(v.width), bits(v.height)])
}
fn units(v: Option<Utf16Text>) -> Value {
    json!(v.map(|v| v.units().to_vec()))
}
fn error(v: KeyDataError) -> &'static str {
    match v {
        KeyDataError::NullReference
        | KeyDataError::Collection(ResourceTextListError::NullReference) => {
            "NullReferenceException"
        }
        KeyDataError::Collection(ResourceTextListError::Modified) => "InvalidOperationException",
        v => panic!("unexpected source error {v:?}"),
    }
}
fn outcome<T>(v: Result<T, KeyDataError>, to_json: impl FnOnce(T) -> Value) -> Value {
    match v {
        Ok(v) => json!({"value":to_json(v),"error":null}),
        Err(v) => json!({"value":null,"error":error(v)}),
    }
}
fn key_state(k: &ComputedKey, warm: &TextKey, popups: &PopupKeys) -> Value {
    let p = k.popup_keys();
    let d = k.data();
    json!({"row":k.row(),"column":k.column(),"code":k.code(),"type":k.key_type().0,"dataLabel":units(d.data().label()),"group":d.data().group_id(),"dataSameWarm":Rc::ptr_eq(&d,warm),"label":units(k.label()),"icon":k.icon().0,"hint":units(k.hint_label()),"sub":units(k.sub_label()),"hintCode":k.hint_key().map(|v|v.data().code()),"popups":p.iter().map(|v|v.unwrap().data().code()).collect::<Vec<_>>(),"popupsSameWarm":p.ptr_eq(popups),"popupDefault":k.popup_default_index(),"visible":k.is_visible(),"enabled":k.is_enabled(),"width":bits(k.width_factor()),"grow":bits(k.grow()),"shrink":bits(k.shrink()),"touch":rect(k.touch_bounds()),"bounds":rect(k.visible_bounds()),"hasPopup":k.has_popup(),"character":{"value":k.is_character_key().unwrap(),"error":null},"text":outcome(k.output(),units)})
}
fn chosen(kind: &str) -> TextKey {
    match kind {
        "placeholder-null-label" => Rc::new(TextKeyData::from_fields(TextKeyDataFields {
            key_type: KeyType::Placeholder,
            code: 0,
            label: None,
            ..Default::default()
        })),
        "placeholder" => Rc::new(TextKeyData::from_fields(TextKeyDataFields {
            key_type: KeyType::Placeholder,
            code: 55,
            label: Some("p".into()),
            ..Default::default()
        })),
        "language" => TextKeyData::function(KeyCode::LanguageSwitch, KeyType::Function, None),
        "emoji" => TextKeyData::function(KeyCode::ImeUiModeMedia, KeyType::Function, None),
        "currency" => TextKeyData::function(KeyCode::CurrencySlot1, KeyType::Function, None),
        "null-label" => Rc::new(TextKeyData::from_fields(TextKeyDataFields {
            code: 122,
            label: None,
            ..Default::default()
        })),
        "unspecified-null-label" => Rc::new(TextKeyData::from_fields(TextKeyDataFields {
            code: 0,
            label: None,
            ..Default::default()
        })),
        "zero-empty" => Rc::new(TextKeyData::from_fields(TextKeyDataFields {
            code: 0,
            label: Some("".into()),
            ..Default::default()
        })),
        "unspecified-type" => Rc::new(TextKeyData::from_fields(TextKeyDataFields {
            key_type: KeyType(99),
            code: 122,
            label: Some("z".into()),
            ..Default::default()
        })),
        _ => TextKeyData::char("z".into(), None).unwrap(),
    }
}
#[test]
fn source_nullable_compute_full_state_callbacks_mutation_reentry_and_recovery() {
    let rows = include_str!("../../../tests/fixtures/computed-null-context.jsonl")
        .lines()
        .map(|s| serde_json::from_str::<Value>(s).unwrap())
        .collect::<Vec<_>>();
    let mut count = 0;
    let (groups, remainder) = rows.as_chunks::<3>();
    assert!(remainder.is_empty());
    for group in groups {
        let warm = group[0]["warm"].as_str().unwrap();
        let kind = group[0]["kind"].as_str().unwrap();
        let phase = Rc::new(Cell::new(0));
        let fired = Rc::new(Cell::new(false));
        let owner = Rc::new(RefCell::new(None));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let popup = Rc::new(PopupSet::from_parts(
            Some(TextKeyData::char("é".into(), None).unwrap() as Key),
            ResourceObjectList::new(vec![TextKeyData::char("ê".into(), None).unwrap() as Key])
                .into(),
        ));
        let warm_data: TextKey = if warm == "phone" {
            Rc::new(TextKeyData::from_fields(TextKeyDataFields {
                key_type: KeyType::Numeric,
                code: 50,
                label: Some("2".into()),
                popup: Some(popup.clone()),
                ..Default::default()
            }))
        } else {
            TextKeyData::char("a".into(), Some(popup.clone())).unwrap()
        };
        let warm_context = Rc::new(KeyComputeContext::from_values(KeyComputeContextValues {
            mode: if warm == "phone" {
                KeyboardMode::Phone
            } else {
                KeyboardMode::Characters
            },
            shift_state: ShiftState::CapsLock,
            hint_mode: KeyHintMode::HintPriority,
            ..Default::default()
        }));
        let recovery = Rc::new((*warm_context).clone());
        recovery.set_shift_state(ShiftState::Unshifted);
        let reentrant = Rc::new((*warm_context).clone());
        reentrant.set_mode(KeyboardMode::NumericAdvanced);
        reentrant.set_shift_state(ShiftState::ShiftedManual);
        let mut nested = Vec::new();
        if kind != "empty" {
            for (r, names) in [vec!["a", "b"], vec!["c"]].into_iter().enumerate() {
                let mut keys = Vec::new();
                for (c, name) in names.into_iter().enumerate() {
                    let source: Key = match kind {
                        "text" => warm_data.clone() as Key,
                        "auto" => Rc::new(AutoTextKeyData {
                            base: TextKeyData::from_fields(TextKeyDataFields {
                                code: 97,
                                label: Some("a".into()),
                                popup: Some(popup.clone()),
                                ..Default::default()
                            }),
                        }),
                        _ => Rc::new(Probe {
                            name,
                            kind: kind.into(),
                            warm: warm_data.clone(),
                            chosen: chosen(kind),
                            phase: phase.clone(),
                            fired: fired.clone(),
                            owner: owner.clone(),
                            trace: trace.clone(),
                            reentrant: reentrant.clone(),
                        }),
                    };
                    let key = Rc::new(ComputedKey::new(
                        if kind == "null-source" {
                            None
                        } else {
                            Some(source)
                        },
                        r as i32,
                        c as i32,
                    ));
                    *key.number_hint_source.borrow_mut() =
                        Some(Rc::new(TextKeyData::from_fields(TextKeyDataFields {
                            key_type: KeyType::Numeric,
                            code: 49,
                            label: Some("1".into()),
                            ..Default::default()
                        })));
                    keys.push(key);
                }
                nested.push(keys);
            }
        }
        let board = Rc::new(ComputedKeyboard::new(
            KeyboardMode::Characters,
            Subtype::shared_default(),
            LayoutDirection::Ltr,
            nested,
            None,
            None,
        ));
        *owner.borrow_mut() = Some(Rc::downgrade(&board));
        let originals = board
            .keys()
            .map(|key| key.unwrap().unwrap())
            .collect::<Vec<_>>();
        let mut warm_keys = Vec::new();
        let mut warm_popups = Vec::new();
        for (expected, p) in group.iter().zip(0..3) {
            phase.set(p);
            trace.borrow_mut().clear();
            let result = catch_unwind(AssertUnwindSafe(|| {
                if p == 0 {
                    board.compute(warm_context.clone())?;
                    board
                        .layout(
                            400.5,
                            120.25,
                            Some(&KeyboardGeometryOptions::from_values(
                                KeyboardGeometryOptionsValues {
                                    key_spacing_horizontal: 2.5,
                                    key_spacing_vertical: 3.25,
                                    split_gap: 40.5,
                                    units_per_row: 10.25,
                                },
                            )),
                        )
                        .unwrap();
                } else {
                    board.compute_nullable(if p == 1 { None } else { Some(recovery.clone()) })?;
                }
                Ok::<_, KeyDataError>(())
            }));
            let failure = match result {
                Ok(Ok(())) => None,
                Ok(Err(e)) => Some(error(e)),
                Err(e) => {
                    if e.downcast_ref::<Marker>().is_some() {
                        Some("MarkerException")
                    } else {
                        resume_unwind(e)
                    }
                }
            };
            if p == 0 {
                warm_keys = originals.iter().map(|k| k.data()).collect();
                warm_popups = originals.iter().map(|k| k.popup_keys()).collect();
            }
            let context = board.context().map_or("null", |v| {
                if Rc::ptr_eq(&v, &warm_context) {
                    "warm"
                } else if Rc::ptr_eq(&v, &recovery) {
                    "recover"
                } else if Rc::ptr_eq(&v, &reentrant) {
                    "reenter"
                } else {
                    "other"
                }
            });
            let actual = json!({"warm":warm,"kind":kind,"phase":(["warm","null","recover"][p as usize]),"error":failure,"trace":*trace.borrow(),"version":board.version(),"context":context,"width":bits(board.width()),"height":bits(board.height()),"rows":board.rows().iter().map(|row|row.unwrap().iter().map(|key|{let key=key.unwrap();originals.iter().position(|k|Rc::ptr_eq(k,&key)).unwrap()}).collect::<Vec<_>>()).collect::<Vec<_>>(),"state":originals.iter().enumerate().map(|(i,k)|key_state(k,&warm_keys[i],&warm_popups[i])).collect::<Vec<_>>()});
            let mut actual = actual;
            let mut expected = expected.clone();
            let warm_phase = expected["phase"] == "warm";
            for (a, e) in actual["state"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .zip(expected["state"].as_array_mut().unwrap())
            {
                let was_shared =
                    e.as_object_mut().unwrap().remove("sharedEmpty") == Some(json!(true));
                if was_shared && !warm_phase && e["popupsSameWarm"] == true {
                    e.as_object_mut().unwrap().remove("popupsSameWarm");
                    a.as_object_mut().unwrap().remove("popupsSameWarm");
                }
            }
            assert_eq!(actual, expected, "{warm}/{kind}/phase{p}");
            count += 1;
        }
    }
    assert_eq!(count, 150);
}
