use super::{size, valid_token_name, Conversion, Issue, Options, Plan, EXTENSION};
use crate::components::{framework, key_attribute, literal_key, FRAMEWORK, XAML};
use crate::{Result, ToolError};
use ferroui_designer_core::{Document, Node, NodeKind};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

/// Export direct literal entries. Included dictionaries and dynamic values remain source-owned.
pub fn plan_export(source: &str, options: &Options) -> Result<Plan> {
    size(source)?;
    let document = Document::new(source).map_err(crate::error)?;
    let root = document.ast().root();
    if !document
        .ast()
        .node(root)
        .is_some_and(|node| framework(node) && node.local_name() == "ResourceDictionary")
    {
        return Err(ToolError(
            "Export tokens from a standalone FerroUI ResourceDictionary".into(),
        ));
    }
    let mut output = Map::new();
    let mut mappings = Vec::new();
    let mut issues = Vec::new();
    let mut seen = BTreeSet::new();
    for child in document.ast().element_children(root) {
        let node = document
            .ast()
            .node(child)
            .expect("Indexed dictionary entry");
        let path = literal_key(node).unwrap_or_else(|| node.name.clone());
        let result = literal_key(node)
            .ok_or("Only literal keyed resources are exportable".to_string())
            .and_then(|key| {
                if !seen.insert(key.clone()) {
                    return Err("Duplicate XAML resource key".into());
                }
                let (mapping, value) = conversion(&document, node, &key, options)?;
                install(
                    &mut output,
                    &mapping.token,
                    json!({"$type":mapping.kind, "$value":value,
                "$extensions":{EXTENSION:{"key":key,"xamlType":mapping.xaml_type}}}),
                )?;
                Ok(mapping)
            });
        match result {
            Ok(mapping) => mappings.push(mapping),
            Err(message) => issues.push(Issue { path, message }),
        }
    }
    for key in options.mappings.keys().filter(|key| !seen.contains(*key)) {
        issues.push(Issue {
            path: key.clone(),
            message: "Mapping does not match an authored resource key".into(),
        });
    }
    let source = serde_json::to_string_pretty(&output).map_err(crate::error)? + "\n";
    size(&source)?;
    Ok(Plan {
        source,
        mappings,
        issues,
    })
}

type Export<T> = std::result::Result<T, String>;

fn conversion(
    document: &Document,
    node: &Node,
    key: &str,
    options: &Options,
) -> Export<(Conversion, Value)> {
    let mapping = options.mappings.get(key);
    let token = mapping
        .and_then(|mapping| mapping.token.as_deref())
        .unwrap_or(key);
    let extra = node
        .attributes
        .iter()
        .filter(|attr| !attr.is_namespace() && Some(*attr) != key_attribute(node));
    let plain = extra.clone().next().is_none();
    let name = node.local_name();
    let namespace = node.namespace_uri.as_deref();
    let content = literal_content(document, node)?;
    let (kind, xaml_type, value, display) = match (namespace, name) {
        (Some(FRAMEWORK), "SolidColorBrush")
            if content.is_empty() && extra.clone().all(|attr| attr.name == "Color") =>
        {
            let color = node
                .attribute("Color")
                .ok_or("SolidColorBrush requires a literal Color")?;
            (
                "color",
                "SolidColorBrush",
                hex_color(color)?,
                color.to_string(),
            )
        }
        (Some(FRAMEWORK), "Color") if plain => ("color", "Color", hex_color(&content)?, content),
        (Some(XAML | "using:System"), "Double") if plain => {
            let number = content
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or("Expected a finite literal Double")?;
            let kind = mapping
                .and_then(|mapping| mapping.kind.as_deref())
                .unwrap_or("number");
            let value = match kind {
                "number" => json!(number),
                "dimension" => json!({"value":number,"unit":"px"}),
                _ => return Err("Map a Double to number or dimension".into()),
            };
            (kind, "x:Double", value, content)
        }
        (Some("using:System"), "TimeSpan") if plain => (
            "duration",
            "sys:TimeSpan",
            json!({"value":duration_ms(&content)?,"unit":"ms"}),
            content,
        ),
        _ => return Err(format!("No literal token mapping for {}", node.name)),
    };
    let note = (kind == "number").then(|| "Map to dimension to export as logical pixels".into());
    Ok((
        Conversion {
            token: token.into(),
            key: key.into(),
            kind: kind.into(),
            xaml_type: xaml_type.into(),
            value: display,
            note,
        },
        value,
    ))
}

fn literal_content(document: &Document, element: &Node) -> Export<String> {
    let mut content = String::new();
    for child in element
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
    {
        match child.kind {
            NodeKind::Text | NodeKind::CData => {
                content.push_str(child.text.as_deref().unwrap_or_default())
            }
            NodeKind::Comment => {}
            _ => return Err("Only scalar resource values are exportable".into()),
        }
    }
    Ok(content.trim().to_string())
}

fn hex_color(value: &str) -> Export<Value> {
    let digits = value
        .trim()
        .strip_prefix('#')
        .ok_or("Use a literal hexadecimal sRGB color")?;
    let hex = u32::from_str_radix(digits, 16).map_err(|_| "Invalid hexadecimal sRGB color")?;
    let (alpha, red, green, blue) = match digits.len() {
        3 => (
            255,
            (hex >> 8 & 15) * 17,
            (hex >> 4 & 15) * 17,
            (hex & 15) * 17,
        ),
        4 => (
            (hex >> 12 & 15) * 17,
            (hex >> 8 & 15) * 17,
            (hex >> 4 & 15) * 17,
            (hex & 15) * 17,
        ),
        6 => (255, hex >> 16 & 255, hex >> 8 & 255, hex & 255),
        8 => (hex >> 24 & 255, hex >> 16 & 255, hex >> 8 & 255, hex & 255),
        _ => return Err("Invalid hexadecimal sRGB color".into()),
    };
    let normalize = |value: u32| f64::from(value) / 255.0;
    Ok(
        json!({"colorSpace":"srgb","components":[normalize(red),normalize(green),normalize(blue)],
        "alpha":normalize(alpha),"hex":format!("#{red:02X}{green:02X}{blue:02X}")}),
    )
}

fn duration_ms(value: &str) -> Export<f64> {
    let components = value.split(':').collect::<Vec<_>>();
    let [first, minutes, seconds] = components.as_slice() else {
        return Err("Expected a literal TimeSpan with hours, minutes and seconds".into());
    };
    let (days, hours) = first.split_once('.').unwrap_or(("0", first));
    let days = days.parse::<u32>().map_err(|_| "Invalid TimeSpan days")?;
    let hours = hours.parse::<u32>().map_err(|_| "Invalid TimeSpan hours")?;
    let minutes = minutes
        .parse::<u32>()
        .map_err(|_| "Invalid TimeSpan minutes")?;
    let seconds = seconds
        .parse::<f64>()
        .map_err(|_| "Invalid TimeSpan seconds")?;
    let total = (f64::from(days) * 86_400.0
        + f64::from(hours) * 3600.0
        + f64::from(minutes) * 60.0
        + seconds)
        * 1000.0;
    if hours >= 24
        || minutes >= 60
        || !(0.0..60.0).contains(&seconds)
        || total >= (i64::MAX as f64) / 10_000.0
    {
        return Err("TimeSpan is outside the supported duration range".into());
    }
    Ok(total)
}

fn install(output: &mut Map<String, Value>, path: &str, token: Value) -> Export<()> {
    let steps: Vec<_> = path.split('.').collect();
    let (name, groups) = steps.split_last().expect("A path has one or more segments");
    if groups.iter().any(|group| !valid_token_name(group))
        || (!valid_token_name(name) && (*name != "$root" || groups.is_empty()))
    {
        return Err("Choose a valid dotted DTCG token path".into());
    }
    let mut parent = output;
    for group in groups {
        parent = parent
            .entry(*group)
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .expect("Token or group object");
        if parent.contains_key("$value") {
            return Err(
                "Token path overlaps a token. Map the parent value to its $root token".into(),
            );
        }
    }
    if parent.contains_key(*name) {
        return Err(
            "Token path already exists. Choose a distinct path or use $root for a group value"
                .into(),
        );
    }
    parent.insert((*name).into(), token);
    Ok(())
}
