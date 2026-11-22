use super::references::{Token, Tokens};
use super::scalars::{color, dimension, duration, finite};
use super::{
    bezier, gradient, shadow, simple_key, size, valid_key, Conversion, Issue, Options, Plan,
    ValueResult, EXTENSION, MAX_SOURCE_BYTES,
};
use crate::components::{escape, FRAMEWORK, XAML};
use crate::{Result, ToolError};
use ferroui_designer_core::Document;
use serde_json::Value;
use std::collections::BTreeMap;

/// Plan a new dictionary. Unsupported or ambiguous rows remain visible as issues.
pub fn plan_import(source: &str, options: &Options) -> Result<Plan> {
    size(source)?;
    let tokens: Value =
        serde_json::from_str(source.trim_start_matches('\u{feff}')).map_err(crate::error)?;
    if !tokens.is_object() || tokens.get("$value").is_some() {
        return Err(ToolError(
            "A DTCG document must contain a root group".into(),
        ));
    }
    if options
        .pixels_per_rem
        .is_some_and(|scale| !scale.is_finite() || scale <= 0.0)
    {
        return Err(ToolError(
            "Pixels per rem must be positive and finite".into(),
        ));
    }
    let mut issues = Vec::new();
    let tokens = Tokens::read(&tokens, &mut issues);
    let mut entries = Vec::new();
    let mut generated = 0;
    for (path, token) in &tokens.entries {
        match conversion(&tokens, path, *token, options) {
            Ok((entry, xaml)) => {
                generated += xaml.len();
                if generated > MAX_SOURCE_BYTES {
                    return Err(ToolError("The generated dictionary exceeds 1 MiB".into()));
                }
                entries.push((entry, xaml));
            }
            Err(message) => issues.push(Issue {
                path: path.clone(),
                message,
            }),
        }
    }
    for path in options
        .mappings
        .keys()
        .filter(|path| !tokens.entries.contains_key(*path))
    {
        issues.push(Issue {
            path: path.clone(),
            message: "Mapping does not match a token".into(),
        });
    }
    let mut counts = BTreeMap::new();
    for (entry, _) in &entries {
        *counts.entry(entry.key.clone()).or_insert(0) += 1;
    }
    entries.retain(|(entry, _)| {
        if counts[&entry.key] == 1 {
            return true;
        }
        issues.push(Issue {
            path: entry.token.clone(),
            message: format!(
                "Duplicate resource key {}. Choose a distinct key.",
                entry.key
            ),
        });
        false
    });
    let mut source = format!(
        "<ResourceDictionary xmlns=\"{FRAMEWORK}\"\n                    xmlns:x=\"{XAML}\"\n                    xmlns:sys=\"using:System\">\n"
    );
    for (_, xaml) in &entries {
        source.push_str(xaml);
    }
    source.push_str("</ResourceDictionary>\n");
    size(&source)?;
    Document::new(&source).map_err(crate::error)?;
    Ok(Plan {
        source,
        mappings: entries.into_iter().map(|(entry, _)| entry).collect(),
        issues,
    })
}

fn conversion(
    tokens: &Tokens<'_>,
    path: &str,
    token: Token<'_>,
    options: &Options,
) -> ValueResult<(Conversion, String)> {
    let resolved = tokens.resolve(token)?;
    let kind = resolved
        .kind
        .ok_or("Token needs a $type or a typed reference")?;
    let mapping = options.mappings.get(path);
    let identity = &token.node["$extensions"][EXTENSION];
    let key = mapping
        .and_then(|mapping| mapping.key.clone())
        .or_else(|| identity["key"].as_str().map(str::to_owned))
        .unwrap_or_else(|| simple_key(path));
    if !valid_key(&key) {
        return Err(
            "Choose a resource key using letters, digits, dots, dashes or underscores".into(),
        );
    }
    let mut note = resolved
        .reference
        .map(|reference| format!("Resolved {reference}"));
    let target = mapping.and_then(|mapping| mapping.xaml_type.as_deref());
    let (xaml_type, value, xaml) = if kind == "gradient" {
        let brush = gradient::import(tokens, &resolved, target, &mut note)?;
        (brush.kind, brush.summary(), brush.xaml(&key))
    } else {
        let (element, value) = scalar_resource(
            kind,
            tokens,
            resolved.value,
            target.or_else(|| identity["xamlType"].as_str()),
            options,
            &mut note,
        )?;
        let escaped_key = escape(&key);
        let escaped_value = escape(&value);
        let xaml = if element == "SolidColorBrush" {
            format!("    <{element} x:Key=\"{escaped_key}\" Color=\"{escaped_value}\"/>\n")
        } else {
            format!("    <{element} x:Key=\"{escaped_key}\">{escaped_value}</{element}>\n")
        };
        (element, value, xaml)
    };
    Ok((
        Conversion {
            token: path.into(),
            key,
            kind: kind.into(),
            xaml_type: xaml_type.into(),
            value,
            note,
        },
        xaml,
    ))
}

fn scalar_resource<'a>(
    kind: &str,
    tokens: &Tokens<'a>,
    value: &'a Value,
    target: Option<&str>,
    options: &Options,
    note: &mut Option<String>,
) -> ValueResult<(&'static str, String)> {
    let resource = match kind {
        "color" => {
            let target = match target {
                None | Some("SolidColorBrush") => "SolidColorBrush",
                Some("Color") => "Color",
                _ => return Err("Map a color to Color or SolidColorBrush".into()),
            };
            (target, color(tokens, value, note)?)
        }
        "number" => ("x:Double", finite(value)?.to_string()),
        "dimension" => (
            "x:Double",
            dimension(tokens, value, options.pixels_per_rem, note)?.to_string(),
        ),
        "duration" => ("sys:TimeSpan", duration(tokens, value)?),
        "shadow" => (
            "BoxShadows",
            shadow::import(tokens, value, options.pixels_per_rem, note)?,
        ),
        "cubicBezier" => ("KeySpline", bezier::import(tokens, value)?),
        _ => return Err(format!("No XAML mapping for token type {kind}")),
    };
    Ok(resource)
}
