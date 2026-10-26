use crate::{model::select_entries, *};
use serde_json::{Map, Value};

pub(crate) fn import(
    content: &str,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    // The source accepts trailing commas. The scanner only removes delimiter tokens outside
    // strings, and leaves all other syntax validation to serde_json.
    let normalized = without_trailing_commas(content);
    let root = parse_checked(&normalized)?;
    let root = root
        .as_object()
        .ok_or("JSON translation catalog root must be an object.")?;
    if format == TranslationFileFormat::AppleXcstrings {
        return crate::xcstrings::import(root, result);
    }
    let arb = format == TranslationFileFormat::FlutterArb;
    if arb
        && root
            .get("@@locale")
            .is_some_and(|locale| !locale.is_string())
    {
        return Err("ARB @@locale must be a string.".into());
    }
    let culture = if arb {
        root.get("@@locale")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| options.culture.clone())
    } else {
        options.culture.clone()
    };
    flatten(root, None, culture.as_deref(), arb, result);
    if arb {
        let globals: Map<_, _> = root
            .iter()
            .filter(|(key, _)| key.starts_with("@@") && *key != "@@locale")
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        if !globals.is_empty() {
            result
                .catalog
                .metadata
                .insert("arb.globals".into(), Value::Object(globals).to_string());
        }
        let mut orphan_metadata = Map::new();
        for (key, value) in root
            .iter()
            .filter(|(key, _)| key.starts_with('@') && !key.starts_with("@@"))
        {
            if !value.is_object() {
                return Err(format!("ARB message metadata '{key}' must be an object."));
            }
            if !result
                .catalog
                .entries
                .iter()
                .any(|entry| key[1..] == entry.key)
            {
                orphan_metadata.insert(key.clone(), value.clone());
            }
        }
        if !orphan_metadata.is_empty() {
            result.catalog.metadata.insert(
                "arb.orphan_metadata".into(),
                Value::Object(orphan_metadata).to_string(),
            );
        }
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
    prefix: Option<&str>,
    culture: Option<&str>,
    arb: bool,
    result: &mut TranslationFormatResult,
) {
    for (name, value) in object {
        if arb && name.starts_with('@') {
            continue;
        }
        let key = prefix.map_or_else(|| name.clone(), |prefix| format!("{prefix}.{name}"));
        match value {
            Value::String(value) => {
                result.catalog.add(key, culture, value.clone());
            }
            Value::Object(object) => flatten(object, Some(&key), culture, arb, result),
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

pub(crate) fn export(
    catalog: &TranslationCatalog,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
) -> Result<String, String> {
    if format == TranslationFileFormat::AppleXcstrings {
        return crate::xcstrings::export(catalog, options);
    }
    let culture = options
        .culture
        .clone()
        .or_else(|| catalog.cultures().into_iter().next());
    let arb = format == TranslationFileFormat::FlutterArb;
    let mut root = Map::new();
    if arb {
        for (metadata_key, global) in [("arb.globals", true), ("arb.orphan_metadata", false)] {
            if let Some(value) = catalog.metadata.get(metadata_key) {
                let object = parse_object(value)?;
                for (key, value) in object {
                    if key == "@@locale"
                        || if global {
                            !key.starts_with("@@")
                        } else {
                            !key.starts_with('@') || key.starts_with("@@") || !value.is_object()
                        }
                    {
                        return Err(format!("Invalid reserved ARB metadata key '{key}'."));
                    }
                    root.insert(key, value);
                }
            }
        }
        if let Some(culture) = &culture {
            root.insert("@@locale".into(), Value::String(culture.clone()));
        }
    }
    for entry in select_entries(catalog, culture.as_deref()) {
        if arb && entry.key.starts_with('@') {
            return Err(format!(
                "ARB translation key '{}' uses the reserved @ metadata prefix.",
                entry.key
            ));
        }
        root.insert(entry.key.clone(), Value::String(entry.value.clone()));
        if arb {
            let mut metadata = match entry.metadata.get("arb") {
                Some(metadata) => parse_object(metadata).map_err(|e| {
                    format!("Invalid retained ARB metadata for '{}': {e}", entry.key)
                })?,
                None => Map::new(),
            };
            if options.include_comments {
                if let Some(comment) = &entry.comment {
                    metadata.insert("description".into(), Value::String(comment.clone()));
                } else {
                    metadata.remove("description");
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

pub(crate) fn parse_checked(content: &str) -> Result<Value, String> {
    serde_json::from_str::<CheckedJson>(content)
        .map(|value| value.0)
        .map_err(|e| e.to_string())
}

pub(crate) fn parse_object(content: &str) -> Result<Map<String, Value>, String> {
    parse_checked(content)?
        .as_object()
        .cloned()
        .ok_or_else(|| "Expected a JSON object.".into())
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
