use crate::{model::select_entries, *};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;

pub(crate) fn import(
    content: &str,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    // The source accepts trailing commas. The scanner only removes delimiter tokens outside
    // strings, and leaves all other syntax validation to serde_json.
    let normalized = without_trailing_commas(content);
    let root = serde_json::from_str::<CheckedJson>(&normalized)
        .map_err(|e| e.to_string())?
        .0;
    let root = root
        .as_object()
        .ok_or("JSON translation catalog root must be an object.")?;
    if format == TranslationFileFormat::AppleXcstrings {
        return import_xcstrings(root, result);
    }
    let arb = format == TranslationFileFormat::FlutterArb;
    let culture = if arb {
        root.get("@@locale")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| options.culture.clone())
    } else {
        options.culture.clone()
    };
    flatten(root, "", culture.as_deref(), arb, result);
    if arb {
        for entry in &mut result.catalog.entries {
            if let Some(metadata) = root
                .get(&format!("@{}", entry.key))
                .filter(|v| v.is_object())
            {
                entry.comment = metadata
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                entry.metadata.insert("arb".into(), metadata.to_string());
            }
        }
        if root
            .keys()
            .any(|key| key.starts_with("@@") && key != "@@locale")
        {
            result.diagnostics.push(TranslationFormatDiagnostic::new(
                Severity::Warning,
                "PTF102",
                "ARB global metadata other than @@locale is not represented by the catalog model.",
            ));
        }
    }
    Ok(())
}

fn without_trailing_commas(content: &str) -> String {
    let bytes = content.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let (mut quoted, mut escaped) = (false, false);
    for (index, &byte) in bytes.iter().enumerate() {
        if quoted {
            output.push(byte);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
            output.push(byte);
        } else if byte == b','
            && bytes[..index]
                .iter()
                .rev()
                .copied()
                .find(|b| !b.is_ascii_whitespace())
                .is_some_and(|b| !matches!(b, b'{' | b'[' | b',' | b':'))
            && bytes[index + 1..]
                .iter()
                .copied()
                .find(|b| !b.is_ascii_whitespace())
                .is_some_and(|b| b == b'}' || b == b']')
        {
        } else {
            output.push(byte);
        }
    }
    // Only ASCII comma bytes were removed from valid UTF-8.
    String::from_utf8(output).expect("removing ASCII punctuation preserves UTF-8")
}

fn flatten(
    object: &Map<String, Value>,
    prefix: &str,
    culture: Option<&str>,
    arb: bool,
    result: &mut TranslationFormatResult,
) {
    for (name, value) in object {
        if arb && name.starts_with('@') {
            continue;
        }
        let key = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        match value {
            Value::String(value) => {
                result.catalog.add(key, culture, value.clone());
            }
            Value::Object(object) => flatten(object, &key, culture, arb, result),
            _ => result.diagnostics.push(TranslationFormatDiagnostic {
                key: Some(key.clone()),
                culture: culture.map(str::to_owned),
                ..TranslationFormatDiagnostic::new(
                    Severity::Info,
                    "PTF101",
                    format!("Ignored non-string JSON value at '{key}'."),
                )
            }),
        }
    }
}

fn import_xcstrings(
    root: &Map<String, Value>,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    let strings = root
        .get("strings")
        .and_then(Value::as_object)
        .ok_or("Apple .xcstrings root must contain a 'strings' object.")?;
    if let Some(source) = root.get("sourceLanguage").and_then(Value::as_str) {
        result.catalog.source_culture = Some(source.into());
    }
    for (key, record) in strings {
        let Some(record) = record.as_object() else {
            result.diagnostics.push(TranslationFormatDiagnostic::new(
                Severity::Warning,
                "PTF120",
                format!("Skipped non-object .xcstrings record '{key}'."),
            ));
            continue;
        };
        let comment = record.get("comment").and_then(Value::as_str);
        if record
            .keys()
            .any(|key| !matches!(key.as_str(), "localizations" | "comment"))
        {
            result.diagnostics.push(TranslationFormatDiagnostic::new(
                Severity::Warning,
                "PTF121",
                format!("Additional .xcstrings metadata for '{key}' is not represented."),
            ));
        }
        let Some(localizations) = record.get("localizations").and_then(Value::as_object) else {
            continue;
        };
        for (culture, localization) in localizations {
            if localization.as_object().is_some_and(|object| {
                object
                    .keys()
                    .any(|key| !matches!(key.as_str(), "stringUnit" | "variations"))
            }) || localization
                .get("variations")
                .and_then(Value::as_object)
                .is_some_and(|object| object.keys().any(|key| key != "plural"))
            {
                result.diagnostics.push(TranslationFormatDiagnostic::new(Severity::Warning,"PTF121",format!("Additional localization variations or substitution metadata for '{key}' ({culture}) are not represented.")));
            }
            if let Some((value, state)) = string_unit(localization) {
                let entry = result.catalog.add(key, Some(culture), value);
                entry.state = state.map(str::to_owned);
                entry.comment = comment.map(str::to_owned);
            } else if let Some(plurals) = localization
                .pointer("/variations/plural")
                .and_then(Value::as_object)
            {
                for (category, unit) in plurals {
                    if let Some((value, state)) = string_unit(unit) {
                        let entry =
                            result
                                .catalog
                                .add(format!("{key}.{category}"), Some(culture), value);
                        entry.state = state.map(str::to_owned);
                        entry.comment = comment.map(str::to_owned);
                        entry.plural_category = Some(category.into());
                    } else {
                        result.diagnostics.push(TranslationFormatDiagnostic::new(Severity::Warning,"PTF120",format!("Unsupported plural variation '{key}.{category}' ({culture}) was skipped.")));
                    }
                }
            } else {
                result.diagnostics.push(TranslationFormatDiagnostic::new(
                    Severity::Warning,
                    "PTF120",
                    format!("Unsupported .xcstrings variation '{key}' ({culture}) was skipped."),
                ));
            }
        }
    }
    Ok(())
}

fn string_unit(value: &Value) -> Option<(&str, Option<&str>)> {
    let unit = value.get("stringUnit")?;
    Some((
        unit.get("value")?.as_str()?,
        unit.get("state").and_then(Value::as_str),
    ))
}

pub(crate) fn export(
    catalog: &TranslationCatalog,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
) -> Result<String, String> {
    if format == TranslationFileFormat::AppleXcstrings {
        return export_xcstrings(catalog, options);
    }
    let culture = options
        .culture
        .clone()
        .or_else(|| catalog.cultures().into_iter().next());
    let arb = format == TranslationFileFormat::FlutterArb;
    let mut root = Map::new();
    if arb {
        if let Some(culture) = &culture {
            root.insert("@@locale".into(), Value::String(culture.clone()));
        }
    }
    for entry in select_entries(catalog, culture.as_deref()) {
        root.insert(entry.key.clone(), Value::String(entry.value.clone()));
        if arb {
            let mut metadata = match entry.metadata.get("arb") {
                Some(metadata) => serde_json::from_str::<Value>(metadata)
                    .map_err(|e| format!("Invalid retained ARB metadata for '{}': {e}", entry.key))?
                    .as_object()
                    .cloned()
                    .ok_or_else(|| {
                        format!("ARB metadata for '{}' must be an object.", entry.key)
                    })?,
                None => Map::new(),
            };
            if options.include_comments {
                if let Some(comment) = &entry.comment {
                    metadata.insert("description".into(), Value::String(comment.clone()));
                }
            } else {
                metadata.remove("description");
            }
            if !metadata.is_empty() {
                root.insert(format!("@{}", entry.key), Value::Object(metadata));
            }
        }
    }
    serde_json::to_string_pretty(&Value::Object(root)).map_err(|e| e.to_string())
}

fn export_xcstrings(
    catalog: &TranslationCatalog,
    options: &TranslationFormatOptions,
) -> Result<String, String> {
    let mut groups = BTreeMap::<&str, BTreeMap<&str, Vec<&TranslationCatalogEntry>>>::new();
    for entry in &catalog.entries {
        let Some(culture) = entry.culture.as_deref() else {
            continue;
        };
        if options
            .culture
            .as_deref()
            .is_some_and(|selected| !selected.eq_ignore_ascii_case(culture))
        {
            continue;
        }
        groups
            .entry(entry.base_key())
            .or_default()
            .entry(culture)
            .or_default()
            .push(entry);
    }
    let mut strings = Map::new();
    for (key, cultures) in groups {
        let comment = cultures
            .values()
            .flatten()
            .find_map(|entry| entry.comment.as_ref());
        let mut record = Map::new();
        if options.include_comments {
            if let Some(comment) = comment {
                record.insert("comment".into(), Value::String(comment.clone()));
            }
        }
        let mut localizations = Map::new();
        for (culture, entries) in &cultures {
            let plurals: Map<String, Value> = entries
                .iter()
                .filter_map(|entry| {
                    entry
                        .plural_category
                        .as_ref()
                        .map(|category| (category.clone(), unit(entry)))
                })
                .collect();
            if !plurals.is_empty() && entries.iter().any(|entry| entry.plural_category.is_none()) {
                return Err(format!(
                    ".xcstrings key '{key}' ({culture}) contains both singular and plural records."
                ));
            }
            let value = if !plurals.is_empty() {
                json!({"variations":{"plural":plurals}})
            } else {
                unit(entries.last().expect("group is nonempty"))
            };
            localizations.insert((*culture).into(), value);
        }
        record.insert("localizations".into(), Value::Object(localizations));
        strings.insert(key.into(), Value::Object(record));
    }
    serde_json::to_string_pretty(&json!({"sourceLanguage":options.source_culture.as_deref().or(catalog.source_culture.as_deref()).unwrap_or("en"),"strings":strings,"version":"1.0"})).map_err(|e| e.to_string())
}
fn unit(entry: &TranslationCatalogEntry) -> Value {
    json!({"stringUnit":{"state":entry.state.as_deref().unwrap_or("translated"),"value":entry.value}})
}

// Reject duplicate JSON object members instead of silently losing earlier values in a map.
struct CheckedJson(Value);
impl<'de> serde::Deserialize<'de> for CheckedJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = CheckedJson;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON value with unique object keys")
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<CheckedJson, E> {
                Ok(CheckedJson(Value::Bool(value)))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<CheckedJson, E> {
                Ok(CheckedJson(Value::Number(value.into())))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<CheckedJson, E> {
                Ok(CheckedJson(Value::Number(value.into())))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<CheckedJson, E> {
                serde_json::Number::from_f64(value)
                    .map(|n| CheckedJson(Value::Number(n)))
                    .ok_or_else(|| E::custom("Invalid JSON number"))
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<CheckedJson, E> {
                Ok(CheckedJson(Value::String(value.into())))
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<CheckedJson, E> {
                Ok(CheckedJson(Value::String(value)))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<CheckedJson, E> {
                Ok(CheckedJson(Value::Null))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<CheckedJson, E> {
                Ok(CheckedJson(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<CheckedJson, A::Error> {
                let mut values = Vec::new();
                while let Some(CheckedJson(value)) = seq.next_element::<CheckedJson>()? {
                    values.push(value);
                }
                Ok(CheckedJson(Value::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<CheckedJson, A::Error> {
                let mut values = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom(format!(
                            "Duplicate JSON key '{key}'"
                        )));
                    }
                    values.insert(key, map.next_value::<CheckedJson>()?.0);
                }
                Ok(CheckedJson(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}
