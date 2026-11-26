//! Strict schemas shared by the catalog and runtime validation for designer extension tools.
use serde_json::{json, Map, Value};

pub(super) fn object(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
pub(super) fn string() -> Value {
    json!({"type":"string","maxLength":1048576})
}
pub(super) fn id() -> Value {
    json!({"type":"integer","minimum":1})
}
pub(super) fn revision() -> Value {
    json!({"type":"integer","minimum":0})
}
pub(super) fn number() -> Value {
    json!({"type":"number"})
}
pub(super) fn time() -> Value {
    json!({"type":"number","minimum":0})
}
pub(super) fn preview_range() -> Value {
    object(json!({"start":time(),"end":time(),"loop":{"type":"boolean"}}), &["start","end","loop"])
}
pub(super) fn nullable(schema: Value) -> Value {
    json!({"anyOf":[schema,{"type":"null"}]})
}
pub(super) fn array(items: Value, maximum: usize) -> Value {
    json!({"type":"array","items":items,"maxItems":maximum})
}
pub(super) fn dictionary(value: Value) -> Value {
    json!({"type":"object","additionalProperties":value,"maxProperties":4096})
}
pub(super) fn tool(name: &str, description: &str, schema: Value, write: bool) -> Value {
    json!({"name":name,"description":description,"inputSchema":schema,"annotations":{"readOnlyHint":!write,"destructiveHint":write,"openWorldHint":false}})
}
pub(super) fn action(action: &str, mut properties: Value, required: &[&str], write: bool) -> Value {
    properties["action"] = json!({"const":action});
    let mut required: Vec<_> = required.iter().map(|name| Value::from(*name)).collect();
    required.push(json!("action"));
    if write {
        properties["expected_revision"] = revision();
        required.push(json!("expected_revision"));
    }
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
pub(super) fn actions(name: &str, description: &str, branches: Vec<Value>, write: bool) -> Value {
    let mut properties = Map::new();
    for branch in &branches {
        for (key, value) in branch["properties"].as_object().unwrap() {
            if key != "action" {
                properties.insert(key.clone(), value.clone());
            }
        }
    }
    properties.insert("action".into(), json!({"enum":branches.iter().map(|branch|branch["properties"]["action"]["const"].clone()).collect::<Vec<_>>()}));
    let required = if write {
        vec!["action", "expected_revision"]
    } else {
        vec!["action"]
    };
    let mut schema = object(Value::Object(properties), &required);
    schema["oneOf"] = json!(branches);
    tool(name, description, schema, write)
}
pub(super) fn animation_value() -> Value {
    let components = |names: &[&str], color: bool| {
        let mut values = Map::new();
        for name in names {
            values.insert(
                (*name).into(),
                if color {
                    json!({"type":"number","minimum":0,"maximum":1})
                } else {
                    number()
                },
            );
        }
        object(Value::Object(values), names)
    };
    let value = |kind: &str, schema: Value| {
        object(
            json!({"kind":{"const":kind},"value":schema}),
            &["kind", "value"],
        )
    };
    let effect_form = json!({"enum":["literal","object","property_element"]});
    let argb = json!({"type":"array","minItems":4,"maxItems":4,"items":{"type":"integer","minimum":0,"maximum":255}});
    let box_shadow = object(json!({"offset_x":number(),"offset_y":number(),"blur":number(),"spread":number(),"argb":argb,"inset":{"type":"boolean"}}),
        &["offset_x","offset_y","blur","spread","argb","inset"]);
    let coefficients = |count| json!({"type":"array","minItems":count,"maxItems":count,"items":number()});
    let transform_operation = json!({"oneOf":[
        value("translate",coefficients(2)), value("rotate",number()), value("scale",coefficients(2)),
        value("skew",coefficients(2)), value("matrix",coefficients(6))
    ]});
    let transform = object(json!({"operations":array(transform_operation,100_000)}), &["operations"]);
    let point = object(json!({"x":number(),"y":number(),"unit":{"enum":["Absolute","Relative"]}}), &["x","y","unit"]);
    let distance = object(json!({"scalar":number(),"unit":{"enum":["Absolute","Relative"]}}), &["scalar","unit"]);
    let shapes = json!({"oneOf":[
        object(json!({"kind":{"const":"linear"},"start":point,"end":point}), &["kind","start","end"]),
        object(json!({"kind":{"const":"radial"},"center":point,"origin":point,"radius_x":distance,"radius_y":distance}),
            &["kind","center","origin","radius_x","radius_y"]),
        object(json!({"kind":{"const":"conic"},"center":point,"angle":number()}), &["kind","center","angle"])
    ]});
    let stop = object(json!({"argb":argb,"offset":number()}), &["argb","offset"]);
    let optional_transform = json!({"anyOf":[transform,{"type":"null"}]});
    let gradient = object(json!({"shape":shapes,"stops":array(stop,100_000),"opacity":number(),
        "spread":{"enum":["Pad","Reflect","Repeat"]},"transform_origin":point,
        "transform":optional_transform,"relative_transform":optional_transform
    }), &["shape","stops","opacity","spread","transform_origin"]);
    let brush = json!({"oneOf":[
        object(json!({"Gradient":object(json!({"value":gradient,"wrapped":{"type":"boolean"}}), &["value"])}), &["Gradient"]),
        object(json!({"Solid":object(json!({"argb":argb,"opacity":number(),"form":effect_form}), &["argb","opacity"])}), &["Solid"])
    ]});
    json!({"oneOf":[
        value("Number",number()), value("Color",components(&["a","r","g","b"],true)),
        value("Point",components(&["x","y"],false)),
        value("Vector",components(&["x","y"],false)),
        value("Size",components(&["width","height"],false)),
        value("Rect",components(&["x","y","width","height"],false)),
        value("Thickness",components(&["left","top","right","bottom"],false)),
        value("CornerRadius",components(&["top_left","top_right","bottom_right","bottom_left"],false)),
        value("RelativePoint",object(json!({"x":number(),"y":number(),"unit":{"enum":["Absolute","Relative"]}}), &["x","y","unit"])),
        value("RelativeScalar",object(json!({"scalar":number(),"unit":{"enum":["Absolute","Relative"]}}), &["scalar","unit"])),
        value("BoxShadows",object(json!({"layers":{"type":"array","items":box_shadow}}), &["layers"])),
        value("TransformList",transform), value("Brush",brush),
        value("Effect",json!({"oneOf":[
            object(json!({"kind":{"const":"blur"},"radius":number(),"form":effect_form}), &["kind","radius"]),
            object(json!({"kind":{"const":"drop_shadow"},"offset_x":number(),"offset_y":number(),"blur_radius":number(),
                "argb":argb,"opacity":number(),"form":effect_form}), &["kind","offset_x","offset_y","blur_radius","argb"]),
            object(json!({"kind":{"const":"directional_drop_shadow"},"direction":number(),"shadow_depth":number(),"blur_radius":number(),
                "argb":argb,"opacity":number(),"form":{"enum":["object","property_element"]}}),
                &["kind","direction","shadow_depth","blur_radius","argb","opacity","form"]),
            object(json!({"kind":{"const":"registered"},"source":string()}), &["kind","source"])
        ]})),
        value("Discrete",string())
    ]})
}
pub(super) fn easing() -> Value {
    json!({"oneOf":[
        object(json!({"kind":{"const":"Linear"}}), &["kind"]),
        object(json!({"kind":{"const":"Named"},"value":string()}), &["kind","value"]),
        object(json!({"kind":{"const":"CubicBezier"},"value":object(json!({"x1":{"type":"number","minimum":0,"maximum":1},"y1":number(),"x2":{"type":"number","minimum":0,"maximum":1},"y2":number()}), &["x1","y1","x2","y2"])}), &["kind","value"]),
        object(json!({"kind":{"const":"Spring"},"value":object(json!({"mass":{"type":"number","exclusiveMinimum":0},"stiffness":{"type":"number","exclusiveMinimum":0},"damping":time(),"initial_velocity":number()}), &["mass","stiffness","damping","initial_velocity"])}), &["kind","value"])
    ]})
}
fn keyframe() -> Value {
    object(
        json!({"time":time(),"value":animation_value(),"easing":easing()}),
        &["time", "value", "easing"],
    )
}
pub(super) fn keyframe_paste() -> Value {
    let track = object(json!({"property":string(),"keyframes":array(keyframe(),4096)}), &["property","keyframes"]);
    let clipboard = object(json!({"format":{"const":"ferroui.keyframes.v1"},"tracks":array(track,4096)}), &["format","tracks"]);
    let destination = object(json!({"target":id(),"property":string()}), &["target","property"]);
    object(json!({"clipboard":clipboard,"destinations":array(destination,4096),"time":time(),"reverse":{"type":"boolean"}}),
        &["clipboard","destinations","time"])
}
pub(super) fn timeline() -> Value {
    let track = object(
        json!({"target":id(),"property":string(),"keyframes":array(keyframe(),100000),"base_value":nullable(animation_value()),"muted":{"type":"boolean"}}),
        &["target", "property", "keyframes", "muted"],
    );
    let mut properties = timing_properties();
    properties["tracks"] = array(track, 4096);
    object(
        properties,
        &[
            "duration",
            "delay",
            "speed_ratio",
            "repeat",
            "playback_direction",
            "fill",
            "easing",
            "tracks",
        ],
    )
}
fn timing_properties() -> Value {
    let repeat = json!({"oneOf":[{"const":"Infinite"},object(json!({"Count":{"type":"integer","minimum":1}}), &["Count"])]});
    json!({
        "duration":{"type":"number","exclusiveMinimum":0},"delay":time(),"delay_between_iterations":time(),"playback_behavior":{"enum":["Auto","Always","OnlyIfVisible"]},
        "speed_ratio":{"type":"number","exclusiveMinimum":0},"repeat":repeat,"playback_direction":{"enum":["Normal","Reverse","Alternate","AlternateReverse"]},
        "fill":{"enum":["None","Forward","Backward","Both"]},"easing":easing()
    })
}
pub(super) fn timing_edit() -> Value {
    let mut properties = timing_properties();
    properties["repeat"] = json!({"type":"string","description":"Repeat count as a decimal integer, or Infinite."});
    properties["easing"] = json!({"type":"string","description":"Native FerroUI easing name, cubic Bézier coordinates, or spring settings."});
    object(properties, &[])
}
pub(super) fn state_group() -> Value {
    let setter = object(
        json!({"target":string(),"property":string(),"value":string()}),
        &["target", "property", "value"],
    );
    let state = object(
        json!({"name":string(),"setters":array(setter,4096)}),
        &["name", "setters"],
    );
    let transition = object(
        json!({"from":nullable(string()),"to":string(),"duration":time(),"easing":easing()}),
        &["to", "duration", "easing"],
    );
    object(
        json!({"owner":id(),"name":string(),"states":array(state,1024),"transitions":array(transition,4096)}),
        &["owner", "name", "states", "transitions"],
    )
}
fn variant(name: &str, fields: Value, required: &[&str]) -> Value {
    let mut properties = Map::new();
    properties.insert(name.into(), object(fields, required));
    object(Value::Object(properties), &[name])
}
pub(super) fn behavior_action() -> Value {
    json!({"oneOf":[
        variant("ChangeProperty",json!({"target":nullable(string()),"property":string(),"value":string()}), &["property","value"]),
        variant("InvokeCommand",json!({"command":string(),"parameter":nullable(string())}), &["command"]),
        variant("CallMethod",json!({"target":nullable(string()),"method":string()}), &["method"]),
        variant("AddClass",json!({"target":nullable(string()),"class":string()}), &["class"]),
        variant("RemoveClass",json!({"target":nullable(string()),"class":string()}), &["class"]),
        variant("Custom",json!({"type_name":string(),"properties":dictionary(string()),"children":string()}), &["type_name","properties","children"]),
        variant("Markup",json!({"source":string()}), &["source"])
    ]})
}
pub(super) fn behavior_trigger() -> Value {
    json!({"oneOf":[
        variant("Event",json!({"event":string(),"source":nullable(string())}), &["event"]),
        variant("Data",json!({"binding":string(),"comparison":string(),"value":string(),"revert_on_false":{"type":"boolean"}}), &["binding","comparison","value","revert_on_false"]),
        variant("Custom",json!({"type_name":string(),"properties":dictionary(string())}), &["type_name","properties"])
    ]})
}
pub(super) fn behavior() -> Value {
    object(
        json!({"id":nullable(id()),"owner":id(),"trigger":behavior_trigger(),"actions":array(behavior_action(),4096)}),
        &["owner", "trigger", "actions"],
    )
}
pub(super) fn formatting() -> Value {
    object(
        json!({
            "tabSize":{"type":"integer","minimum":1,"maximum":16},
            "insertSpaces":{"type":"boolean"},
            "trimTrailingWhitespace":{"type":"boolean"},
            "insertFinalNewline":{"type":"boolean"},
            "trimFinalNewlines":{"type":"boolean"},
            "preserveBlankLines":{"type":"boolean"},
            "attributeLayout":{"enum":["preserve","inline","onePerLine","wrap"]},
            "wrapColumn":{"type":"integer","minimum":20,"maximum":240},
            "closingBracketOnNewLine":{"type":"boolean"},
            "selfClosingSpace":{"enum":["preserve","space","none"]}
        }),
        &[],
    )
}
pub(super) fn source_range() -> Value {
    let position = object(
        json!({"line":revision(),"character":revision()}),
        &["line", "character"],
    );
    object(json!({"start":position,"end":position}), &["start", "end"])
}
pub(super) fn workspace_edit() -> Value {
    let edit = object(
        json!({"range":source_range(),"newText":string()}),
        &["range", "newText"],
    );
    let document = object(
        json!({"uri":string(),"version":{"type":"integer","minimum":-1},"edits":array(edit,4096)}),
        &["uri", "version", "edits"],
    );
    object(
        json!({"document_changes":array(document,4096)}),
        &["document_changes"],
    )
}

/// Validate the exact subset used in these generated schemas, including nested unknown fields.
/// Semantic rules (native ranges, name scopes, type registration) remain with the model APIs.
pub(super) fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    if value.to_string().len() > 1_048_576 {
        return Err("Tool arguments exceed 1 MiB".into());
    }
    check(schema, value, "arguments", 0)
}
fn check(schema: &Value, value: &Value, path: &str, depth: usize) -> Result<(), String> {
    if depth > 64 {
        return Err(format!("{path} exceeds schema nesting limit"));
    }
    if let Some(expected) = schema.get("const") {
        if value != expected {
            return Err(format!("{path} must equal {expected}"));
        }
    }
    if let Some(values) = schema["enum"].as_array() {
        if !values.contains(value) {
            return Err(format!("{path} is outside the allowed values"));
        }
    }
    if let Some(clauses) = schema["allOf"].as_array() {
        for clause in clauses {
            check(clause, value, path, depth + 1)?;
        }
    }
    for key in ["oneOf", "anyOf"] {
        if let Some(choices) = schema[key].as_array() {
            let count = choices
                .iter()
                .filter(|choice| check(choice, value, path, depth + 1).is_ok())
                .count();
            if count == 0 || key == "oneOf" && count != 1 {
                return Err(format!("{path} does not match an allowed {key} schema"));
            }
        }
    }
    if let Some(kind) = schema["type"].as_str() {
        let matches = match kind {
            "object" => value.is_object(),
            "array" => value.is_array(),
            "string" => value.is_string(),
            "integer" => value.is_i64() || value.is_u64(),
            "number" => value.as_f64().is_some_and(f64::is_finite),
            "boolean" => value.is_boolean(),
            "null" => value.is_null(),
            _ => false,
        };
        if !matches {
            return Err(format!("{path} must be {kind}"));
        }
    }
    if let Some(number) = value.as_f64() {
        for (key, valid) in [
            (
                "minimum",
                schema["minimum"]
                    .as_f64()
                    .is_none_or(|limit| number >= limit),
            ),
            (
                "maximum",
                schema["maximum"]
                    .as_f64()
                    .is_none_or(|limit| number <= limit),
            ),
            (
                "exclusiveMinimum",
                schema["exclusiveMinimum"]
                    .as_f64()
                    .is_none_or(|limit| number > limit),
            ),
        ] {
            if !valid {
                return Err(format!("{path} violates {key}"));
            }
        }
    }
    if let Some(text) = value.as_str() {
        let length = text.chars().count() as u64;
        if schema["maxLength"]
            .as_u64()
            .is_some_and(|limit| length > limit)
            || schema["minLength"]
                .as_u64()
                .is_some_and(|limit| length < limit)
        {
            return Err(format!("{path} has invalid string length"));
        }
    }
    if let Some(items) = value.as_array() {
        if schema["maxItems"]
            .as_u64()
            .is_some_and(|limit| items.len() as u64 > limit)
            || schema["minItems"]
                .as_u64()
                .is_some_and(|limit| (items.len() as u64) < limit)
        {
            return Err(format!("{path} has invalid array length"));
        }
        if let Some(item_schema) = schema.get("items") {
            for (index, item) in items.iter().enumerate() {
                check(item_schema, item, &format!("{path}[{index}]"), depth + 1)?;
            }
        }
    }
    if let Some(fields) = value.as_object() {
        if schema["maxProperties"]
            .as_u64()
            .is_some_and(|limit| fields.len() as u64 > limit)
        {
            return Err(format!("{path} has too many fields"));
        }
        if let Some(required) = schema["required"].as_array() {
            for name in required.iter().filter_map(Value::as_str) {
                if !fields.contains_key(name) {
                    return Err(format!("{path}.{name} is required"));
                }
            }
        }
        for (name, item) in fields {
            if let Some(field_schema) = schema["properties"].get(name) {
                check(field_schema, item, &format!("{path}.{name}"), depth + 1)?;
            } else if schema["additionalProperties"] == false {
                return Err(format!("{path}.{name} is not supported"));
            } else if schema["additionalProperties"].is_object() {
                check(
                    &schema["additionalProperties"],
                    item,
                    &format!("{path}.{name}"),
                    depth + 1,
                )?;
            }
        }
    }
    Ok(())
}
