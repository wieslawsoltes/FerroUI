//! Unchanged source DOM/read algorithms. JSON output comparisons are semantic ordered-input
//! observations, not a claim of exact Utf8JsonWriter bytes or complete CLR exception objects.
use ferroboard_core::{layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::rc::Rc;
pub trait Driver {
    const CORE: bool;
    fn resource(json: &str) -> Result<Rc<KeyboardLayout>, LayoutJsonError>;
}
pub struct Core;
impl Driver for Core {
    const CORE: bool = true;
    fn resource(json: &str) -> Result<Rc<KeyboardLayout>, LayoutJsonError> {
        KeyboardResources::create_empty()
            .add_layout_json_utf16(Some(&Utf16Text::from(json)), Some("custom".into()))
            .map_err(|e| match e {
                ResourceError::Json(e) => e,
                e => panic!("unexpected registry error {e}"),
            })
    }
}
fn units(v: Option<Utf16Text>) -> Value {
    v.map(|s| json!(s.units())).unwrap_or(Value::Null)
}
fn kind(v: &JsonElement) -> i32 {
    match v {
        JsonElement::Object(_) | JsonElement::DeferredObject(_) => 1,
        JsonElement::Array(_) => 2,
        JsonElement::String(_) | JsonElement::DeferredString(_) => 3,
        JsonElement::Number(_) | JsonElement::NumberLexeme(_) => 4,
        JsonElement::Bool(true) => 5,
        JsonElement::Bool(false) => 6,
        JsonElement::Null => 7,
    }
}
fn category(error: &LayoutJsonError) -> &'static str {
    match error.0.as_str() {
        "Expected a JSON array" | "Unexpected JSON value type" | "Invalid JSON string encoding" => {
            "InvalidOperationException"
        }
        "Invalid or out-of-range JSON number" => "FormatException",
        text if text.starts_with("Invalid JSON at byte ") => "JsonReaderException",
        "Required layout data is missing" => "NullReferenceException",
        text if text.starts_with("Layout arrangement must be")
            || text == "Invalid key definition"
            || text.starts_with("Unknown layout type")
            || text.starts_with("Unknown key data type")
            || text.starts_with("Layout document requires an 'id'")
            || text.starts_with("Missing '") =>
        {
            "FormatException"
        }
        text => panic!("unknown source error transport {text}"),
    }
}
fn output(r: Result<Value, LayoutJsonError>) -> Value {
    match r {
        Ok(v) => json!({"value":v,"error":null}),
        Err(e) => json!({"value":null,"error":category(&e)}),
    }
}
fn semantic(text: String) -> Value {
    serde_json::from_str(&text).unwrap()
}
pub fn verify<D: Driver>(fixture: &str) -> usize {
    let mut checked = 0;
    for line in fixture.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let input = row["json"].as_str().unwrap();
        let tag = row["tag"].as_str().unwrap();
        if !D::CORE && !(tag == "layout" && row["action"] == "resource") {
            continue;
        }
        let actual: Result<Value, LayoutJsonError> = (|| {
            Ok(match tag {
                "dom" => {
                    let node = LayoutJson::parse(input)?;
                    match row["operation"].as_str().unwrap() {
                        "string" => units(node.string()?),
                        "int" => json!(node.int32()?),
                        "names" => json!(
                            node.properties()?
                                .map(|(name, _)| name
                                    .text()
                                    .map(|v| units(Some(v.as_str().into()))))
                                .collect::<Result<Vec<_>, _>>()?
                        ),
                        "get" => match node.get(row["query"].as_str().unwrap())? {
                            None => Value::Null,
                            Some(v) => {
                                json!({"kind":kind(v),"value":if v.is_string(){units(v.string()?)}else{Value::Null}})
                            }
                        },
                        _ => unreachable!(),
                    }
                }
                "layout" => {
                    let l = if row["action"] == "resource" {
                        D::resource(input)?
                    } else {
                        Rc::new(LayoutJson::read_layout_document(input, "custom".into())?)
                    };
                    semantic(LayoutJson::write_layout_document(&l, false)?)
                }
                "arrangement" => semantic(LayoutJson::write_arrangement(
                    LayoutJson::read_arrangement(input)?.as_ref(),
                    false,
                )?),
                "subtype" => {
                    let v = LayoutJson::read_subtype(input)?;
                    json!({"language":units(v.language_tag()),"display":units(v.display_name()),"characters":units(Some(v.layouts().unwrap().characters().to_text()))})
                }
                "manifest" => {
                    let v = LayoutJson::read_extension_manifest(input)?;
                    json!({"id":units(Some(v.id)),"title":units(Some(v.title)),"layouts":v.layouts.borrow().len()})
                }
                "popup" => {
                    let v = LayoutJson::read_popup_mapping(input, "id".into(), "ext".into(), None)?;
                    let mut values = Vec::new();
                    for entry in v.mappings().as_ref().unwrap().iter().unwrap() {
                        let entry = entry.unwrap().unwrap();
                        for item in entry.value.unwrap().iter().unwrap() {
                            let item = item.unwrap().unwrap();
                            values.push(
                                json!({"variation":entry.key.unwrap().0,"label":units(item.key)}),
                            );
                        }
                    }
                    json!(values)
                }
                _ => panic!("unknown tag {tag}"),
            })
        })();
        let actual = output(actual);
        let mut expected = row["result"].clone();
        if expected["error"].is_null() && matches!(tag, "layout" | "arrangement") {
            expected["value"] = semantic(expected["value"].as_str().unwrap().to_owned());
        }
        assert_eq!(actual, expected, "{row}");
        checked += 1;
    }
    checked
}
