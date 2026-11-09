//! Original key factories and runtime init accessors, over live objects and actual With dispatch.
use ferroboard_core::{
    keys::*,
    layouts::*,
    text::{TextCulture, Utf16Text},
};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
pub fn labels() -> Vec<Option<Utf16Text>> {
    vec![
        None,
        Some("".into()),
        Some("a".into()),
        Some("a\u{301}".into()),
        Some("😀".into()),
        Some("\0".into()),
        Some(Utf16Text::from_units(vec![0xd800])),
        Some(Utf16Text::from_units(vec![0xdfff])),
        Some(Utf16Text::from_units(vec![0xd800, 120])),
        Some(Utf16Text::from_units(vec![0xd800, 0xdfff])),
        Some(Utf16Text::from_units(vec![97, 0xd800])),
        Some(Utf16Text::from_units(vec![0xd800, 0xd800, 0xdfff])),
        Some("ß".into()),
        Some("\r\n".into()),
    ]
}
pub fn popups() -> Vec<Option<Rc<PopupSet>>> {
    vec![
        None,
        Some(Rc::new(PopupSet::from_parts(
            Some(TextKeyData::char("p".into(), None).unwrap()),
            ResourceReadOnlyList::<Key>::default(),
        ))),
        Some(Rc::new(PopupSet::from_parts(
            Some(TextKeyData::char("q".into(), None).unwrap()),
            ResourceReadOnlyList::<Key>::default(),
        ))),
    ]
}
#[derive(Debug, PartialEq, Eq)]
pub struct Failure {
    pub name: &'static str,
    pub param: &'static str,
}
pub trait Driver {
    fn char(text: Option<Utf16Text>, popup: Option<Rc<PopupSet>>) -> Result<TextKey, Failure>;
    fn function(code: i32, kind: KeyType, label: Option<Utf16Text>) -> TextKey;
    fn init(key: &TextKey, field: &str, value: Field);
    fn get(key: &TextKey, field: &str) -> Field;
    fn with(key: &TextKey, changes: KeyChanges) -> TextKey;
    fn text(key: &TextKey) -> Option<Utf16Text>;
}
#[derive(Clone)]
pub enum Field {
    Type(KeyType),
    Code(i32),
    Label(Option<Utf16Text>),
    Group(i32),
    Popup(Option<Rc<PopupSet>>),
    Bool(bool),
}
pub struct Core;
impl Driver for Core {
    fn char(text: Option<Utf16Text>, popup: Option<Rc<PopupSet>>) -> Result<TextKey, Failure> {
        TextKeyData::char_nullable(text, popup)
            .map(|k| k as TextKey)
            .map_err(|e| match e {
                KeyDataError::NullCharacter => Failure {
                    name: "ArgumentNullException",
                    param: "s",
                },
                KeyDataError::EmptyCharacter => Failure {
                    name: "ArgumentOutOfRangeException",
                    param: "index",
                },
                KeyDataError::InvalidSurrogate(_) => Failure {
                    name: "ArgumentException",
                    param: "s",
                },
                e => panic!("unexpected {e:?}"),
            })
    }
    fn function(code: i32, kind: KeyType, label: Option<Utf16Text>) -> TextKey {
        TextKeyData::function(code, kind, label)
    }
    fn init(key: &TextKey, _: &str, value: Field) {
        let d = key.data();
        match value {
            Field::Type(v) => d.set_type(v),
            Field::Code(v) => d.set_code(v),
            Field::Label(v) => d.set_label(v),
            Field::Group(v) => d.set_group_id(v),
            Field::Popup(v) => d.set_popup(v),
            Field::Bool(_) => unreachable!(),
        }
    }
    fn get(key: &TextKey, field: &str) -> Field {
        let d = key.data();
        match field {
            "Type" => Field::Type(d.key_type()),
            "Code" => Field::Code(d.code()),
            "Label" => Field::Label(d.label()),
            "GroupId" => Field::Group(d.group_id()),
            "Popup" => Field::Popup(d.popup()),
            "PopupLookupLabel" => Field::Label(d.popup_lookup_label()),
            "ProducesText" => Field::Bool(key.produces_text().unwrap()),
            "IsSpace" => Field::Bool(d.is_space()),
            _ => unreachable!(),
        }
    }
    fn with(key: &TextKey, changes: KeyChanges) -> TextKey {
        key.with(changes)
    }
    fn text(key: &TextKey) -> Option<Utf16Text> {
        key.to_text().unwrap()
    }
}
pub fn units(v: Option<Utf16Text>) -> Value {
    json!(v.as_ref().map(Utf16Text::units))
}
fn same_text(a: Option<Utf16Text>, b: Option<Utf16Text>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.ptr_eq(&b),
        _ => false,
    }
}
fn same_popup(a: Option<Rc<PopupSet>>, b: Option<Rc<PopupSet>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
        _ => false,
    }
}
fn output(key: &TextKey, display: bool) -> Value {
    match key.as_string(display) {
        Ok(v) => json!({"value":units(v),"error":null}),
        Err(KeyDataError::InvalidCodePoint(_)) => {
            json!({"value":null,"error":"ArgumentOutOfRangeException"})
        }
        Err(e) => panic!("unexpected {e:?}"),
    }
}
pub fn state<D: Driver>(k: &TextKey, popups: &[Option<Rc<PopupSet>>]) -> Value {
    let Field::Code(code) = D::get(k, "Code") else {
        panic!()
    };
    let Field::Type(kind) = D::get(k, "Type") else {
        panic!()
    };
    let Field::Label(label) = D::get(k, "Label") else {
        panic!()
    };
    let Field::Group(group) = D::get(k, "GroupId") else {
        panic!()
    };
    let Field::Popup(popup) = D::get(k, "Popup") else {
        panic!()
    };
    let Field::Bool(produces) = D::get(k, "ProducesText") else {
        panic!()
    };
    let Field::Bool(space) = D::get(k, "IsSpace") else {
        panic!()
    };
    let Field::Label(lookup) = D::get(k, "PopupLookupLabel") else {
        panic!()
    };
    let id = if popup.is_none() {
        0
    } else {
        popups
            .iter()
            .position(|p| same_popup(popup.clone(), p.clone()))
            .map_or(-1, |v| v as i32)
    };
    json!({"code":code,"type":kind.0,"label":units(label),"group":group,"popup":id,"produces":produces,"space":space,"lookup":units(lookup),"output":output(k,false),"display":output(k,true),"text":units(D::text(k))})
}
fn make(kind: &str) -> TextKey {
    match kind {
        "text" => Rc::new(TextKeyData::default()),
        "auto" => Rc::new(AutoTextKeyData {
            base: TextKeyData::default(),
        }),
        "multi" => Rc::new(MultiTextKeyData::from_parts(
            MultiTextKeyData::default().base,
            Some(CodePointArray::new(vec![97, 0x301])),
        )),
        _ => unreachable!(),
    }
}
pub fn verify<D: Driver>(fixture: &str) -> (usize, usize, usize) {
    let labels = labels();
    let popups = popups();
    let (mut behavior, mut facts, mut reentrant) = (0, 0, 0);
    let mut current: Option<(String, TextKey, TextKey, TextKey)> = None;
    for line in fixture.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let tag = row["tag"].as_str().unwrap();
        let index = |name: &str| row[name].as_u64().unwrap() as usize;
        match tag {
            "char" => {
                let text = labels[index("v")].clone();
                let popup = popups[index("p")].clone();
                let result = D::char(text.clone(), popup.clone());
                match result {
                    Ok(key) => {
                        assert!(row["error"].is_null());
                        assert_eq!(state::<D>(&key, &popups), row["state"]);
                        let Field::Label(label) = D::get(&key, "Label") else {
                            panic!()
                        };
                        let Field::Popup(p) = D::get(&key, "Popup") else {
                            panic!()
                        };
                        assert_eq!(json!(same_text(text, label)), row["labelSame"]);
                        assert_eq!(json!(same_popup(popup, p)), row["popupSame"]);
                    }
                    Err(e) => {
                        assert_eq!(e.name, row["error"].as_str().unwrap());
                        assert_eq!(e.param, row["param"].as_str().unwrap());
                        assert!(row["state"].is_null());
                    }
                }
            }
            "function" => {
                let code = row["code"].as_i64().unwrap() as i32;
                let kind = KeyType(row["type"].as_i64().unwrap() as i32);
                let label = labels[index("v")].clone();
                let key = D::function(code, kind, label.clone());
                assert_eq!(state::<D>(&key, &popups), row["state"], "{row}");
                assert_eq!(
                    json!(!Rc::ptr_eq(&key, &D::function(code, kind, label.clone()))),
                    row["fresh"]
                );
                if row["arity"] == 3 && !row["labelSame"].is_null() {
                    let Field::Label(v) = D::get(&key, "Label") else {
                        panic!()
                    };
                    assert_eq!(json!(same_text(label, v)), row["labelSame"]);
                }
            }
            "init" => {
                let kind = row["kind"].as_str().unwrap();
                if current.as_ref().is_none_or(|v| v.0 != kind) {
                    let key = make(kind);
                    current = Some((
                        kind.into(),
                        key.clone(),
                        key.clone(),
                        D::with(&key, KeyChanges::default()),
                    ));
                }
                let (_, key, alias, copy) = current.as_ref().unwrap();
                let field = row["field"].as_str().unwrap();
                let i = index("v");
                let value = match field {
                    "Type" => Field::Type(KeyType([-1, 0, 2, 7, 9, 17][i])),
                    "Code" => Field::Code(
                        [
                            i32::MIN,
                            -1100,
                            -16,
                            -1,
                            0,
                            9,
                            10,
                            32,
                            97,
                            0x1f600,
                            i32::MAX,
                        ][i],
                    ),
                    "Label" => Field::Label(labels[i].clone()),
                    "GroupId" => Field::Group([-1, 0, 1, 2, 3, 97, 123][i]),
                    "Popup" => Field::Popup(popups[i].clone()),
                    _ => unreachable!(),
                };
                D::init(key, field, value);
                let modified = D::with(
                    key,
                    KeyChanges {
                        code: Some(98),
                        ..Default::default()
                    },
                );
                let array_same = match (key.serialization(), modified.serialization()) {
                    (KeySerialization::Multi(a), KeySerialization::Multi(b)) => {
                        Some(a.code_points().unwrap().ptr_eq(&b.code_points().unwrap()))
                    }
                    _ => None,
                };
                let actual = json!({"tag":"init","kind":kind,"field":field,"v":i,"state":state::<D>(key,&popups),"aliasSame":Rc::ptr_eq(key,alias),"aliasState":state::<D>(alias,&popups),"oldCopy":state::<D>(copy,&popups),"modified":state::<D>(&modified,&popups),"modifiedFresh":!Rc::ptr_eq(key,&modified),"popupShared":same_popup(key.data().popup(),modified.data().popup()),"labelShared":same_text(key.data().label(),modified.data().label()),"arrayShared":array_same});
                assert_eq!(actual, row, "{row}");
            }
            "custom-text" => {
                let key: TextKey = match row["kind"].as_str().unwrap() {
                    "NamedDisplayKey" => Rc::new(NamedDisplayKey(TextKeyData::default())),
                    "NullDisplayKey" => Rc::new(NullDisplayKey(TextKeyData::default())),
                    _ => unreachable!(),
                };
                assert_eq!(units(D::text(&key)), row["value"]);
            }
            "auto-reentrant" => {
                verify_reentrant::<D>(&row, &labels, &popups);
                reentrant += 1;
                continue;
            }
            "property" | "method" => {
                facts += 1;
                continue;
            }
            _ => panic!("unexpected {tag}"),
        }
        behavior += 1;
    }
    (behavior, facts, reentrant)
}
struct ReentrantEvaluator {
    shift: ShiftState,
    trace: Rc<RefCell<Vec<String>>>,
    action: Box<dyn Fn()>,
}
impl IKeyEvaluator for ReentrantEvaluator {
    fn mode(&self) -> KeyboardMode {
        KeyboardMode::Characters
    }
    fn shift_state(&self) -> ShiftState {
        self.trace.borrow_mut().push("shift".into());
        self.shift
    }
    fn variation(&self) -> KeyVariation {
        KeyVariation::Normal
    }
    fn direction(&self) -> LayoutDirection {
        LayoutDirection::Ltr
    }
    fn is_char_half_width(&self) -> bool {
        false
    }
    fn is_kana_kata(&self) -> bool {
        false
    }
    fn culture(&self) -> Option<TextCulture> {
        self.trace.borrow_mut().push("culture".into());
        (self.action)();
        Some(TextCulture::invariant())
    }
}
fn verify_reentrant<D: Driver>(
    row: &Value,
    labels: &[Option<Utf16Text>],
    popups: &[Option<Rc<PopupSet>>],
) {
    let key: TextKey = Rc::new(AutoTextKeyData {
        base: TextKeyData::from_fields(TextKeyDataFields {
            code: 97,
            label: labels[row["v"].as_u64().unwrap() as usize].clone(),
            ..Default::default()
        }),
    });
    let original = key.clone();
    let target = key.clone();
    let mutation = row["mutation"].as_str().unwrap().to_owned();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let popup = popups[1].clone();
    let evaluator = ReentrantEvaluator {
        shift: ShiftState(row["shift"].as_i64().unwrap() as i32),
        trace: trace.clone(),
        action: Box::new(move || match mutation.as_str() {
            "label-null" => D::init(&target, "Label", Field::Label(None)),
            "label-upper" => D::init(&target, "Label", Field::Label(Some("A".into()))),
            "fields" => {
                D::init(&target, "Code", Field::Code(32));
                D::init(&target, "Type", Field::Type(KeyType(17)));
                D::init(&target, "GroupId", Field::Group(97));
                D::init(&target, "Popup", Field::Popup(popup.clone()));
            }
            "none" => {}
            _ => unreachable!(),
        }),
    };
    let (computed, error) = match key.compute(&evaluator) {
        Ok(v) => (v, None),
        Err(KeyDataError::NullReference) => (None, Some("NullReferenceException")),
        Err(e) => panic!("unexpected {e:?}"),
    };
    let actual = json!({"tag":"auto-reentrant","v":row["v"],"mutation":row["mutation"],"shift":row["shift"],"trace":*trace.borrow(),"error":error,"state":state::<D>(&original,popups),"computed":computed.as_ref().map(|k|state::<D>(k,popups)),"same":computed.is_some_and(|k|Rc::ptr_eq(&k,&original))});
    assert_eq!(actual, *row, "{row}");
}

struct NamedDisplayKey(TextKeyData);
struct NullDisplayKey(TextKeyData);
macro_rules! custom_output {
    ($name:ident) => {
        impl AbstractKeyData for $name {
            fn into_text(self: Rc<Self>) -> Option<TextKey> {
                Some(self)
            }
            fn compute_nullable(
                self: Rc<Self>,
                _: Option<&dyn IKeyEvaluator>,
            ) -> Result<Option<TextKey>, KeyDataError> {
                Ok(Some(self))
            }
            fn as_string(&self, display: bool) -> Result<Option<Utf16Text>, KeyDataError> {
                self.0.as_string(display)
            }
        }
    };
}
custom_output!(NamedDisplayKey);
custom_output!(NullDisplayKey);
impl ITextKeyData for NamedDisplayKey {
    fn data(&self) -> &TextKeyData {
        &self.0
    }
    fn diagnostic_name(&self) -> &'static str {
        "NamedDisplayKey"
    }
}
impl ITextKeyData for NullDisplayKey {
    fn data(&self) -> &TextKeyData {
        &self.0
    }
    fn to_text(&self) -> Result<Option<Utf16Text>, KeyDataError> {
        Ok(None)
    }
}
