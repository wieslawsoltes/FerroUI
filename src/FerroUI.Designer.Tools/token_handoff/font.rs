use super::{scalars::finite, ValueResult};
use serde_json::{json, Value};

// DTCG uses case-sensitive aliases. XAML also accepts FerroUI's native names
// case-insensitively. The value is shared even where the spellings differ.
const WEIGHTS: &[(u16, &[&str], &[&str])] = &[
    (100, &["thin", "hairline"], &["Thin"]),
    (
        200,
        &["extra-light", "ultra-light"],
        &["ExtraLight", "UltraLight"],
    ),
    (300, &["light"], &["Light"]),
    (350, &[], &["SemiLight"]),
    (400, &["normal", "regular", "book"], &["Normal", "Regular"]),
    (500, &["medium"], &["Medium"]),
    (600, &["semi-bold", "demi-bold"], &["SemiBold", "DemiBold"]),
    (700, &["bold"], &["Bold"]),
    (
        800,
        &["extra-bold", "ultra-bold"],
        &["ExtraBold", "UltraBold"],
    ),
    (900, &["black", "heavy"], &["Black", "Heavy", "Solid"]),
    (
        950,
        &["extra-black", "ultra-black"],
        &["ExtraBlack", "UltraBlack"],
    ),
];

pub(super) fn import_family(value: &Value) -> ValueResult<String> {
    if let Some(name) = value.as_str() {
        return Ok(family_name(name)?.into());
    }
    let names = value
        .as_array()
        .filter(|names| !names.is_empty())
        .ok_or("Expected a font family name or a nonempty list of fallback names")?;
    names
        .iter()
        .map(|name| family_name(name.as_str().ok_or("Expected a font family name")?))
        .collect::<ValueResult<Vec<_>>>()
        .map(|names| names.join(", "))
}

pub(super) fn export_family(value: &str) -> ValueResult<Value> {
    let names = value
        .split(',')
        .map(|name| family_name(name.trim()))
        .collect::<ValueResult<Vec<_>>>()?;
    match names.as_slice() {
        [name] => Ok(json!(name)),
        _ => Ok(json!(names)),
    }
}

fn family_name(name: &str) -> ValueResult<&str> {
    if name.is_empty()
        || name != name.trim()
        || name.contains([',', '#'])
        || name.starts_with('{')
        || name.ends_with('}')
        || name.chars().any(char::is_control)
        || name == "$Default"
    {
        return Err("Use literal font family names without XAML sources or placeholders".into());
    }
    Ok(name)
}

pub(super) fn import_weight(value: &Value) -> ValueResult<u16> {
    if let Some(alias) = value.as_str() {
        return WEIGHTS
            .iter()
            .find(|(_, aliases, _)| aliases.contains(&alias))
            .map(|(weight, _, _)| *weight)
            .ok_or_else(|| "Unknown DTCG font-weight alias".into());
    }
    let weight = finite(value)?;
    if (1.0..=1000.0).contains(&weight) && weight.fract() == 0.0 {
        return Ok(weight as u16);
    }
    Err("FerroUI FontWeight requires a whole number from 1 to 1000".into())
}

pub(super) fn export_weight(value: &str) -> ValueResult<u16> {
    let weight = value.parse::<u16>().ok().or_else(|| {
        WEIGHTS
            .iter()
            .find(|(_, _, names)| names.iter().any(|name| name.eq_ignore_ascii_case(value)))
            .map(|(weight, _, _)| *weight)
    });
    weight
        .filter(|weight| (1..=1000).contains(weight))
        .ok_or_else(|| "Use a literal FerroUI FontWeight from 1 to 1000".into())
}
