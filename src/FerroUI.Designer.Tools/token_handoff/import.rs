use super::references::{Resolved, Token, Tokens};
use super::{simple_key, size, valid_key, Conversion, Issue, Options, Plan, EXTENSION};
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
    for (path, token) in &tokens.entries {
        match conversion(&tokens, path, *token, options) {
            Ok(entry) => entries.push(entry),
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
    for entry in &entries {
        *counts.entry(entry.key.clone()).or_insert(0) += 1;
    }
    entries.retain(|entry| {
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
    let mut source = format!("<ResourceDictionary xmlns=\"{FRAMEWORK}\"\n                    xmlns:x=\"{XAML}\"\n                    xmlns:sys=\"using:System\">\n");
    for entry in &entries {
        let key = escape(&entry.key);
        let value = &entry.value;
        let element = &entry.xaml_type;
        source.push_str(&if element == "SolidColorBrush" {
            format!("    <{element} x:Key=\"{key}\" Color=\"{value}\"/>\n")
        } else {
            format!("    <{element} x:Key=\"{key}\">{value}</{element}>\n")
        });
    }
    source.push_str("</ResourceDictionary>\n");
    size(&source)?;
    Document::new(&source).map_err(crate::error)?;
    Ok(Plan {
        source,
        mappings: entries,
        issues,
    })
}

fn conversion(
    tokens: &Tokens<'_>,
    path: &str,
    token: Token<'_>,
    options: &Options,
) -> std::result::Result<Conversion, String> {
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
    let (xaml_type, value) = match kind {
        "color" => {
            let target = mapping
                .and_then(|mapping| mapping.xaml_type.as_deref())
                .or_else(|| identity["xamlType"].as_str())
                .unwrap_or("SolidColorBrush");
            if !matches!(target, "Color" | "SolidColorBrush") {
                return Err("Map a color to Color or SolidColorBrush".into());
            }
            (target, color(tokens, &resolved, &mut note)?)
        }
        "number" => ("x:Double", finite(resolved.value)?.to_string()),
        "dimension" => (
            "x:Double",
            dimension(tokens, &resolved, options.pixels_per_rem, &mut note)?,
        ),
        "duration" => ("sys:TimeSpan", duration(tokens, resolved.value)?),
        _ => return Err(format!("No XAML mapping for token type {kind}")),
    };
    Ok(Conversion {
        token: path.into(),
        key,
        kind: kind.into(),
        xaml_type: xaml_type.into(),
        value,
        note,
    })
}

fn finite(value: &Value) -> std::result::Result<f64, String> {
    value
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| "Expected a finite number".into())
}

fn dimension(
    tokens: &Tokens<'_>,
    resolved: &Resolved<'_>,
    pixels_per_rem: Option<f64>,
    note: &mut Option<String>,
) -> std::result::Result<String, String> {
    let value = finite(tokens.property(&resolved.value["value"])?)?;
    let pixels = match tokens.property(&resolved.value["unit"])?.as_str() {
        Some("px") => value,
        Some("rem") => {
            let scale =
                pixels_per_rem.ok_or("Supply pixelsPerRem to convert relative dimensions")?;
            append_note(
                note,
                &format!("{value} rem at {scale} logical pixels per rem"),
            );
            value * scale
        }
        _ => return Err("Expected dimension unit px or rem".into()),
    };
    if !pixels.is_finite() {
        return Err("Dimension is outside the finite XAML range".into());
    }
    Ok(pixels.to_string())
}

fn color(
    tokens: &Tokens<'_>,
    resolved: &Resolved<'_>,
    note: &mut Option<String>,
) -> std::result::Result<String, String> {
    if tokens.property(&resolved.value["colorSpace"])?.as_str() != Some("srgb") {
        return Err("Only sRGB colors can be emitted as XAML colors".into());
    }
    let components = tokens.property(&resolved.value["components"])?;
    let rgb = components
        .as_array()
        .filter(|values| values.len() == 3)
        .ok_or("Expected three sRGB components")?;
    let mut channels = [255; 4];
    for (byte, component) in channels[1..].iter_mut().zip(rgb) {
        *byte = color_channel(tokens.property(component)?, note)?;
    }
    if let Some(alpha) = resolved.value.get("alpha") {
        channels[0] = color_channel(tokens.property(alpha)?, note)?;
    }
    if channels[0] == 255 {
        Ok(format!(
            "#{:02X}{:02X}{:02X}",
            channels[1], channels[2], channels[3]
        ))
    } else {
        Ok(format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            channels[0], channels[1], channels[2], channels[3]
        ))
    }
}

fn color_channel(value: &Value, note: &mut Option<String>) -> std::result::Result<u8, String> {
    let channel = finite(value)?;
    if !(0.0..=1.0).contains(&channel) {
        return Err("Color channels must be between zero and one".into());
    }
    let scaled = channel * 255.0;
    if (scaled - scaled.round()).abs() > 1e-8
        && !note
            .as_deref()
            .is_some_and(|text| text.contains("8-bit channels"))
    {
        append_note(note, "Rounded to 8-bit channels");
    }
    Ok(scaled.round() as u8)
}

fn duration(tokens: &Tokens<'_>, value: &Value) -> std::result::Result<String, String> {
    let scalar = finite(tokens.property(&value["value"])?)?;
    let ticks = match tokens.property(&value["unit"])?.as_str() {
        Some("ms") => scalar * 10_000.0,
        Some("s") => scalar * 10_000_000.0,
        _ => return Err("Expected duration unit ms or s".into()),
    }
    .round();
    if ticks < 0.0 || ticks >= i64::MAX as f64 {
        return Err("Duration is outside the positive TimeSpan range".into());
    }
    let ticks = ticks as i64;
    let seconds = ticks / 10_000_000;
    Ok(format!(
        "{}.{:02}:{:02}:{:02}.{:07}",
        seconds / 86_400,
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60,
        ticks % 10_000_000
    ))
}

fn append_note(note: &mut Option<String>, detail: &str) {
    match note {
        Some(note) => {
            note.push_str(" · ");
            note.push_str(detail);
        }
        None => *note = Some(detail.into()),
    }
}
