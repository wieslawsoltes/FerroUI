//! Diagnostic fixtures preserve UTF-16/custom dispatch; numeric diagnostics use Rust formatting.
use super::*;
use serde_json::{Value, json};
fn text(value: &Value) -> Option<Utf16Text> {
    value.as_array().map(|v| {
        Utf16Text::from_units(
            v.iter()
                .map(|n| n.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn result(key: &ComputedKey) -> Value {
    match key.to_text() {
        Ok(v) => json!({"value":v.units(),"error":null,"param":null}),
        Err(KeyDataError::InvalidCodePoint(_)) => {
            json!({"value":null,"error":"ArgumentOutOfRangeException","param":"utf32"})
        }
        Err(e) => panic!("unexpected {e:?}"),
    }
}
struct CustomData {
    base: TextKeyData,
    mode: String,
    trace: Rc<RefCell<Vec<String>>>,
    on_format: Option<Rc<dyn Fn()>>,
}
impl AbstractKeyData for CustomData {
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        Ok(Some(self))
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        Ok(Some("".into()))
    }
}
impl ITextKeyData for CustomData {
    fn data(&self) -> &TextKeyData {
        &self.base
    }
    fn to_text(&self) -> Result<Option<Utf16Text>, KeyDataError> {
        self.trace.borrow_mut().push("text".into());
        if let Some(on_format) = &self.on_format {
            on_format();
        }
        match self.mode.as_str() {
            "null" => Ok(None),
            "throw" => Err(KeyDataError::InvalidCodePoint(0)),
            _ => Ok(Some(Utf16Text::from_units(vec![
                99, 117, 115, 116, 111, 109, 0xd800, 0xd83d, 0xde00,
            ]))),
        }
    }
}

#[test]
fn source_invariant_diagnostics_preserve_utf16_and_custom_dispatch() {
    let mut matched = 0;
    let mut retired_ambient_rows = 0;
    for line in include_str!("../../../tests/fixtures/computed-key-text.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        if row["tag"] == "reentrant" || row.get("sign").is_some_and(|sign| sign != &json!([45])) {
            // Ambient NumberFormatInfo callback/sign behavior is not a native diagnostic API.
            // Keep the source oracle unchanged and account for every excluded row.
            retired_ambient_rows += 1;
            continue;
        }
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        if row["tag"] == "bundled" {
            let keyboard =
                LayoutComputer::new(Rc::new(KeyboardResources::create_default().unwrap()))
                    .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
                    .unwrap();
            keyboard
                .compute(Rc::new(KeyComputeContext::default()))
                .unwrap();
            assert_eq!(
                result(&keyboard.find_key(KeyCode::Delete).unwrap().unwrap()),
                row["result"]
            );
            matched += 1;
            continue;
        }
        let key = match row["tag"].as_str().unwrap() {
            "initial" => ComputedKey::new(
                None,
                row["row"].as_i64().unwrap() as i32,
                row["col"].as_i64().unwrap() as i32,
            ),
            "label" => ComputedKey::new(None, 2, 3),
            "data" => ComputedKey::new(None, i32::MIN, -2),
            _ => unreachable!(),
        };
        match row["tag"].as_str().unwrap() {
            "label" => {
                let mut state = key.state.borrow_mut();
                state.label = text(&row["label"]);
                state.icon = KeyIcon(row["icon"].as_i64().unwrap() as i32);
            }
            "data" => {
                let base = TextKeyData::from_fields(TextKeyDataFields {
                    code: -3,
                    label: Some(Utf16Text::from_units(vec![97, 0xd800])),
                    key_type: KeyType(17),
                    ..Default::default()
                });
                let data: TextKey = match row["profile"].as_str().unwrap() {
                    "base" => Rc::new(base),
                    "auto" => Rc::new(AutoTextKeyData { base }),
                    "multi" => Rc::new(MultiTextKeyData::from_parts(
                        base,
                        Some(CodePointArray::new(vec![0x1f600])),
                    )),
                    mode => Rc::new(CustomData {
                        base,
                        mode: mode.into(),
                        trace: trace.clone(),
                        on_format: None,
                    }),
                };
                let mut state = key.state.borrow_mut();
                state.data = data;
                state.label = None;
                state.icon = KeyIcon(-7);
            }
            _ => {}
        }
        assert_eq!(result(&key), row["result"], "{row}");
        if row["tag"] == "data" {
            assert_eq!(json!(*trace.borrow()), row["trace"], "{row}");
        }
        matched += 1;
    }
    assert_eq!(matched, 103);
    assert_eq!(retired_ambient_rows, 35);
}
#[test]
fn custom_diagnostic_formatting_can_reenter_the_key_without_a_borrow_guard() {
    let key = Rc::new(ComputedKey::new(None, -1, -2));
    let weak = Rc::downgrade(&key);
    let trace = Rc::new(RefCell::new(Vec::new()));
    let data: TextKey = Rc::new(CustomData {
        base: TextKeyData::default(),
        mode: "null".into(),
        trace: trace.clone(),
        on_format: Some(Rc::new(move || {
            weak.upgrade().unwrap().state.borrow_mut().label = Some("after".into());
        })),
    });
    {
        let mut state = key.state.borrow_mut();
        state.label = Some("before".into());
        state.data = data;
    }
    assert_eq!(
        key.to_text().unwrap(),
        Utf16Text::from("Key[-1,-2] before ()")
    );
    assert_eq!(key.label(), Some(Utf16Text::from("after")));
    assert_eq!(
        key.to_text().unwrap(),
        Utf16Text::from("Key[-1,-2] after ()")
    );
    assert_eq!(*trace.borrow(), ["text", "text"]);
}
#[test]
fn computed_key_equality_retains_reference_identity() {
    let original = Rc::new(ComputedKey::new(None, 0, 0));
    let alias = original.clone();
    let other = Rc::new(ComputedKey::new(None, 0, 0));
    assert!(original == alias);
    assert!(original != other);
}
