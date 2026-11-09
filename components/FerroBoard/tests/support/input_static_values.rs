//! Oracle input/output transport only; no property or keyboard behavior is modeled here.
use ferroboard_core::{
    editor::{CapitalizationMode, EnterAction, InputKind},
    text::Utf16Text,
};
use ferroui_base::BoxedValue;
use serde_json::{Value, json};
use std::rc::Rc;
#[derive(Clone)]
pub struct Token {
    pub id: i32,
}
impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
pub fn boxed(v: &Value) -> Option<BoxedValue> {
    if v.is_null() {
        return None;
    }
    let raw = || v["raw"].as_i64().unwrap();
    Some(match v["type"].as_str().unwrap() {
        "String" => {
            let units = v["units"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>();
            match String::from_utf16(&units) {
                Ok(v) => Rc::new(v) as BoxedValue,
                Err(_) => Rc::new(Utf16Text::from_units(units)),
            }
        }
        "InputKind" => Rc::new(InputKind(raw() as i32)),
        "EnterAction" => Rc::new(EnterAction(raw() as i32)),
        "CapitalizationMode" => Rc::new(CapitalizationMode(raw() as i32)),
        "Int32" => Rc::new(raw() as i32),
        "Int64" => Rc::new(raw()),
        "Boolean" => Rc::new(v["value"].as_bool().unwrap()),
        "Token" => Rc::new(Token {
            id: v["id"].as_i64().unwrap() as i32,
        }),
        _ => panic!("unmapped input {v}"),
    })
}
pub fn describe(value: Option<&BoxedValue>) -> Value {
    let Some(v) = value else {
        return Value::Null;
    };
    if let Some(v) = v.downcast_ref::<String>() {
        return json!({"type":"String","units":v.encode_utf16().collect::<Vec<_>>()});
    }
    if let Some(v) = v.downcast_ref::<Utf16Text>() {
        return json!({"type":"String","units":v.units()});
    }
    if let Some(v) = v.downcast_ref::<InputKind>() {
        return json!({"type":"InputKind","raw":v.0});
    }
    if let Some(v) = v.downcast_ref::<EnterAction>() {
        return json!({"type":"EnterAction","raw":v.0});
    }
    if let Some(v) = v.downcast_ref::<CapitalizationMode>() {
        return json!({"type":"CapitalizationMode","raw":v.0});
    }
    if let Some(v) = v.downcast_ref::<i32>() {
        return json!({"type":"Int32","raw":v});
    }
    if let Some(v) = v.downcast_ref::<i64>() {
        return json!({"type":"Int64","raw":v});
    }
    if let Some(v) = v.downcast_ref::<bool>() {
        return json!({"type":"Boolean","value":v});
    }
    if let Some(v) = v.downcast_ref::<Token>() {
        return json!({"type":"Token","id":v.id});
    }
    panic!("unmapped output")
}
pub fn output(name: &str, value: Option<BoxedValue>, supplied: &Option<BoxedValue>) -> Value {
    let same = if matches!(name, "InputKind" | "EnterAction" | "Capitalization") {
        Some(match (&value, supplied) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        })
    } else {
        None
    };
    json!({"value":describe(value.as_ref()),"same":same})
}
pub const NAMES: [&str; 6] = [
    "InputKind",
    "EnterAction",
    "Capitalization",
    "SuggestionsEnabled",
    "IsPrivate",
    "IsEnabled",
];
