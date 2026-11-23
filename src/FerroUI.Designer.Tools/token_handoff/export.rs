use super::scalars::{duration_ms, hex_color, literal_number};
use super::GRADIENT_PROPERTIES;
use super::{
    bezier, font, gradient, gradient_geometry, native_type, shadow, size, typography,
    valid_token_name, Conversion, Issue, Options, Plan, ValueResult, EXTENSION,
};
use crate::components::{framework, key_attribute, literal_key, XAML};
use crate::{Result, ToolError};
use ferroui_designer_core::{Document, Node, NodeKind};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;

/// Export direct literal entries. Included dictionaries and dynamic values remain source-owned.
pub fn plan_export(source: &str, options: &Options) -> Result<Plan> {
    size(source)?;
    let document = Document::new(source).map_err(crate::error)?;
    if !document.is_valid() {
        return Err(ToolError(
            "Repair the XAML dictionary before exporting its tokens".into(),
        ));
    }
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
                let (mapping, resource) = conversion(&document, node, &key, options)?;
                let mut identity = json!({"key":key,"xamlType":mapping.xaml_type});
                if let Some(properties) = resource.gradient_properties {
                    identity[GRADIENT_PROPERTIES] = properties;
                }
                install(
                    &mut output,
                    &mapping.token,
                    json!({"$type":mapping.kind, "$value":resource.value,
                    "$extensions":{EXTENSION:identity}}),
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

struct ExportedResource {
    kind: String,
    xaml_type: &'static str,
    value: Value,
    display: String,
    note: Option<String>,
    gradient_properties: Option<Value>,
}

fn conversion(
    document: &Document,
    node: &Node,
    key: &str,
    options: &Options,
) -> ValueResult<(Conversion, ExportedResource)> {
    let mapping = options.mappings.get(key);
    let token = mapping
        .and_then(|mapping| mapping.token.as_deref())
        .unwrap_or(key);
    let name = node.local_name();
    let resource = if native_type(node, "FerroUI.Media") && gradient_geometry::TYPES.contains(&name)
    {
        let brush = gradient::export(document, node)?;
        let display = brush.summary();
        ExportedResource {
            kind: "gradient".into(),
            xaml_type: brush.kind,
            value: Value::Array(brush.stops),
            display,
            note: Some("Retained FerroUI gradient properties in the token extensions".into()),
            gradient_properties: Some(json!(brush.properties)),
        }
    } else if name == "ControlTheme" && framework(node) {
        let style = typography::export(document, node)?;
        ExportedResource {
            kind: "typography".into(),
            xaml_type: "ControlTheme",
            value: style.token_value()?,
            display: style.summary(),
            note: Some("Converted XAML LineHeight to a multiplier of FontSize".into()),
            gradient_properties: None,
        }
    } else {
        scalar_resource(
            document,
            node,
            mapping.and_then(|mapping| mapping.kind.as_deref()),
        )?
    };
    Ok((
        Conversion {
            token: token.into(),
            key: key.into(),
            kind: resource.kind.clone(),
            xaml_type: resource.xaml_type.into(),
            value: resource.display.clone(),
            note: resource.note.clone(),
        },
        resource,
    ))
}

fn scalar_resource(
    document: &Document,
    node: &Node,
    numeric_kind: Option<&str>,
) -> ValueResult<ExportedResource> {
    let extra = node
        .attributes
        .iter()
        .filter(|attr| !attr.is_namespace() && Some(*attr) != key_attribute(node));
    let plain = extra.clone().next().is_none();
    let name = node.local_name();
    let namespace = node.namespace_uri.as_deref();
    let content = literal_content(document, node)?;
    let media = native_type(node, "FerroUI.Media");
    let (kind, xaml_type, value, display) = match name {
        "SolidColorBrush"
            if media && content.is_empty() && extra.clone().all(|attr| attr.name == "Color") =>
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
        "Color" if media && plain => ("color", "Color", hex_color(&content)?, content),
        "Double" if plain && matches!(namespace, Some(XAML | "using:System")) => {
            let number = literal_number(&content)?;
            let kind = numeric_kind.unwrap_or("number");
            let value = match kind {
                "number" => json!(number),
                "dimension" => json!({"value":number,"unit":"px"}),
                _ => return Err("Map a Double to number or dimension".into()),
            };
            (kind, "x:Double", value, content)
        }
        "TimeSpan" if plain && namespace == Some("using:System") => (
            "duration",
            "sys:TimeSpan",
            json!({"value":duration_ms(&content)?,"unit":"ms"}),
            content,
        ),
        "FontFamily" if media && plain => {
            ("fontFamily", "FontFamily", font::export_family(&content)?, content)
        }
        "FontWeight" if media && plain => {
            ("fontWeight", "FontWeight", json!(font::export_weight(&content)?), content)
        }
        "BoxShadows" if media && plain => {
            ("shadow", "BoxShadows", shadow::export(&content)?, content)
        }
        "KeySpline" if native_type(node, "FerroUI.Animation") => {
            let curve = if plain {
                content
            } else if content.is_empty()
                && extra
                    .clone()
                    .all(|attr| bezier::PROPERTIES.contains(&attr.name.as_str()))
            {
                bezier::PROPERTIES
                    .iter()
                    .map(|&name| {
                        node.attribute(name)
                            .ok_or("Define all four literal KeySpline control-point properties")
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?
                    .join(",")
            } else {
                return Err(
                    "Use a literal KeySpline value or four literal control-point properties".into(),
                );
            };
            let (points, display) = bezier::export(&curve)?;
            ("cubicBezier", "KeySpline", json!(points), display)
        }
        _ => return Err(format!("No literal token mapping for {}", node.name)),
    };
    let note = (kind == "number").then(|| "Map to dimension to export as logical pixels".into());
    Ok(ExportedResource {
        kind: kind.into(),
        xaml_type,
        value,
        display,
        note,
        gradient_properties: None,
    })
}

fn literal_content(document: &Document, element: &Node) -> ValueResult<String> {
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

fn install(output: &mut Map<String, Value>, path: &str, token: Value) -> ValueResult<()> {
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
