//! Selected original runtime-null key contracts; native output dispatch is injectable.
use ferroboard_core::{keys::*, layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::rc::Rc;
pub fn labels() -> Vec<Option<Utf16Text>> {
    vec![
        None,
        Some("".into()),
        Some("a".into()),
        Some("ß".into()),
        Some(Utf16Text::from_units(vec![0xd800, 120, 0xdfff])),
        Some("😀".into()),
        Some(Utf16Text::from_units(vec![0xd800])),
        Some(Utf16Text::from_units(vec![0xdfff])),
        Some(Utf16Text::from_units(vec![0xd800, 0xd800, 0xdfff, 0xdfff])),
        Some(Utf16Text::from_units(vec![0xd800, 0xdfff])),
        Some("\"\r\n\0".into()),
        Some("\u{fffd}".into()),
    ]
}
pub fn make(kind: &str, label: Option<Utf16Text>, code: i32, group: i32) -> TextKey {
    let base = TextKeyData::from_fields(TextKeyDataFields {
        code,
        label,
        group_id: group,
        ..Default::default()
    });
    match kind {
        "text" => Rc::new(base),
        "auto" => Rc::new(AutoTextKeyData { base }),
        "multi" => Rc::new(MultiTextKeyData::from_parts(
            base,
            Some(CodePointArray::new(vec![97, 0x301])),
        )),
        _ => panic!("unknown source key {kind}"),
    }
}
pub trait Driver {
    fn output(key: Key, display: bool) -> Result<Option<Utf16Text>, KeyDataError>;
}
pub struct Core;
impl Driver for Core {
    fn output(key: Key, display: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        key.as_string(display)
    }
}
fn units(value: Option<Utf16Text>) -> Value {
    json!(value.as_ref().map(Utf16Text::units))
}
fn state(k: &TextKey) -> Value {
    let d = k.data();
    json!({"code":d.code(),"label":units(d.label().clone()),"type":d.key_type().0,"group":d.group_id(),"multi":k.is_multi_text(),"produces":k.produces_text().unwrap()})
}
fn result(value: Result<Value, KeyDataError>) -> Value {
    match value {
        Ok(value) => json!({"value":value,"error":null}),
        Err(KeyDataError::NullReference) => json!({"value":null,"error":"NullReferenceException"}),
        Err(KeyDataError::InvalidCodePoint(_) | KeyDataError::InvalidSurrogate(_)) => {
            json!({"value":null,"error":"ArgumentOutOfRangeException"})
        }
        Err(e) => panic!("unexpected source error {e:?}"),
    }
}
pub fn verify<D: Driver>(fixture: &str) -> (usize, usize, usize) {
    let labels = labels();
    let (mut checked, mut native_output, source_only) = (0, 0, 0);
    for line in fixture.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let kind = row["kind"].as_str().unwrap();
        let index = row["label"].as_u64().unwrap() as usize;
        let code = row["code"].as_i64().unwrap() as i32;
        let group = row["group"].as_i64().unwrap() as i32;
        let key = make(kind, labels[index].clone(), code, group);
        let op = row["op"].as_str().unwrap();
        let actual = match op {
            "output" | "display-output" => {
                native_output += 1;
                result(D::output(key.clone(), op == "display-output").map(units))
            }
            "lookup" => result(Ok(units(key.data().popup_lookup_label()))),
            "with-null" | "with-empty" => {
                let copy = key.with(KeyChanges {
                    code: (op == "with-null").then_some(98),
                    label: (op == "with-empty").then(|| "".into()),
                    ..Default::default()
                });
                let array_same = if let (KeySerialization::Multi(a), KeySerialization::Multi(b)) =
                    (key.serialization(), copy.serialization())
                {
                    Some(a.code_points().unwrap().ptr_eq(&b.code_points().unwrap()))
                } else {
                    None
                };
                let popup_same = match (&key.data().popup(), &copy.data().popup()) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                    _ => false,
                };
                result(Ok(
                    json!({"state":state(&copy),"same":Rc::ptr_eq(&key,&copy),"popupSame":popup_same,"arraySame":array_same}),
                ))
            }
            "face" => result(
                KeyLabels::get_display(key.as_ref(), &KeyComputeContext::default())
                    .map(|(label, icon)| json!({"label":units(label),"icon":icon.0})),
            ),
            "json" => {
                let output = LayoutJson::write_arrangement(
                    &LayoutArrangement::from_rows(vec![vec![key.clone()]]),
                    false,
                )
                .unwrap();
                json!({"value":serde_json::from_str::<Value>(&output).unwrap(),"error":null})
            }
            _ if op.starts_with("compute:") => {
                let shift: i32 = op[8..].parse().unwrap();
                let eval = KeyEvaluator::from_values(KeyEvaluatorValues {
                    shift_state: ShiftState(shift),
                    ..Default::default()
                });
                let original = key.clone();
                result(key.compute(&eval).map(|computed|json!({"state":computed.as_ref().map(state),"same":computed.is_some_and(|k|Rc::ptr_eq(&original,&k))})))
            }
            _ => panic!("unexpected source operation {op}"),
        };
        assert_eq!(actual, row["result"], "{row}");
        checked += 1;
    }
    (checked, native_output, source_only)
}
