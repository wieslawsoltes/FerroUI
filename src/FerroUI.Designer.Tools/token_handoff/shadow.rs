use super::{
    references::Tokens,
    scalars::{color, dimension, hex_color, literal_number},
    ValueResult,
};
use serde_json::{json, Value};

const DIMENSIONS: [&str; 4] = ["offsetX", "offsetY", "blur", "spread"];

pub(super) fn import<'a>(
    tokens: &Tokens<'a>,
    value: &'a Value,
    pixels_per_rem: Option<f64>,
    note: &mut Option<String>,
) -> ValueResult<String> {
    let layers = value
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_else(|| std::slice::from_ref(value));
    if layers.is_empty() {
        return Ok("none".into());
    }
    layers.iter().enumerate().map(|(index, layer)| {
        let convert = |note: &mut Option<String>| {
            let layer = tokens.typed_property(layer, "shadow")?;
            if !layer.is_object() {
                return Err("Expected a single shadow object. Array references cannot expand other arrays".into());
            }
            let mut geometry = [0.0; 4];
            for (length, name) in geometry.iter_mut().zip(DIMENSIONS) {
                let value = tokens.typed_property(&layer[name], "dimension")?;
                *length = dimension(tokens, value, pixels_per_rem, note)?;
            }
            let [offset_x, offset_y, blur, spread] = geometry;
            let inset = match layer.get("inset") {
                Some(value) => tokens.typed_property(value, "boolean")?.as_bool().ok_or("The inset flag must be true or false")?,
                None => false,
            };
            let color = color(tokens, tokens.typed_property(&layer["color"], "color")?, note)?;
            let prefix = if inset { "inset " } else { "" };
            Ok(format!("{prefix}{offset_x} {offset_y} {blur} {spread} {color}"))
        };
        convert(note).map_err(|message: String| format!("Shadow layer {}: {message}", index + 1))
    }).collect::<ValueResult<Vec<_>>>().map(|layers| layers.join(", "))
}

pub(super) fn export(value: &str) -> ValueResult<Value> {
    if value == "none" {
        return Ok(json!([]));
    }
    let layers = value
        .split(',')
        .enumerate()
        .map(|(index, value)| {
            export_layer(value).map_err(|message| format!("Shadow layer {}: {message}", index + 1))
        })
        .collect::<ValueResult<Vec<_>>>()?;
    match layers.as_slice() {
        [layer] => Ok(layer.clone()),
        _ => Ok(Value::Array(layers)),
    }
}

fn export_layer(source: &str) -> ValueResult<Value> {
    let mut words = source.split_whitespace().collect::<Vec<_>>();
    let inset = words.first() == Some(&"inset");
    let tint = words
        .pop()
        .ok_or("A shadow needs offsets and a literal hexadecimal color")?;
    let geometry = &words[usize::from(inset && !words.is_empty())..];
    if !(2..=4).contains(&geometry.len()) {
        return Err(
            "A shadow needs two offsets, optional blur and spread, and a literal hexadecimal color"
                .into(),
        );
    }
    let mut numbers = [0.0; 4];
    for (number, word) in numbers.iter_mut().zip(geometry) {
        *number = literal_number(word)?;
    }
    let mut layer = json!({"color":hex_color(tint)?});
    for (name, number) in DIMENSIONS.into_iter().zip(numbers) {
        layer[name] = json!({"value":number,"unit":"px"});
    }
    if inset {
        layer["inset"] = true.into();
    }
    Ok(layer)
}
