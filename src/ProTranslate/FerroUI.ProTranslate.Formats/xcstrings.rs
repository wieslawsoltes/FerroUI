//! String-catalog tree reconstruction retains variation axes and substitution metadata.
use crate::{json::parse_object, *};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

type Unit<'a> = (Vec<String>, &'a Map<String, Value>);

fn units<'a>(
    node: &'a Value,
    path: &mut Vec<String>,
    output: &mut Vec<Unit<'a>>,
) -> Result<(), String> {
    let object = node
        .as_object()
        .ok_or("A .xcstrings localization or variation must be an object.")?;
    if let Some(unit) = object.get("stringUnit") {
        output.push((
            path.clone(),
            unit.as_object()
                .ok_or("A .xcstrings stringUnit must be an object.")?,
        ));
    }
    if let Some(variations) = object.get("variations") {
        for (kind, variants) in variations
            .as_object()
            .ok_or(".xcstrings variations must be an object.")?
        {
            for (name, value) in variants
                .as_object()
                .ok_or("A .xcstrings variation axis must be an object.")?
            {
                if kind == "plural" && !is_plural(name) {
                    return Err(format!("Invalid .xcstrings plural category '{name}'."));
                }
                path.extend(["variations".into(), kind.clone(), name.clone()]);
                units(value, path, output)?;
                path.truncate(path.len() - 3);
            }
        }
    }
    if let Some(substitutions) = object.get("substitutions") {
        for (name, value) in substitutions
            .as_object()
            .ok_or(".xcstrings substitutions must be an object.")?
        {
            path.extend(["substitutions".into(), name.clone()]);
            units(value, path, output)?;
            path.truncate(path.len() - 2);
        }
    }
    Ok(())
}

fn is_plural(category: &str) -> bool {
    matches!(category, "zero" | "one" | "two" | "few" | "many" | "other")
}

fn path_key(path: &[String]) -> Result<(String, Option<String>), String> {
    let mut suffix = String::new();
    let mut plural = None;
    let mut index = 0;
    while index < path.len() {
        match path[index].as_str() {
            "variations" if index + 2 < path.len() => {
                if path[index + 1] == "plural" {
                    if !is_plural(&path[index + 2]) {
                        return Err("Invalid retained .xcstrings plural category.".into());
                    }
                    plural = Some(path[index + 2].clone());
                } else {
                    suffix.push('.');
                    suffix.push_str(&path[index + 1]);
                }
                suffix.push('.');
                suffix.push_str(&path[index + 2]);
                index += 3;
            }
            "substitutions" if index + 1 < path.len() => {
                suffix.push_str(".substitutions.");
                suffix.push_str(&path[index + 1]);
                index += 2;
            }
            _ => return Err("Invalid retained .xcstrings variation path.".into()),
        }
    }
    Ok((suffix, plural))
}

fn map_units_mut(node: &mut Value, retain_marker: bool) {
    let Some(object) = node.as_object_mut() else {
        return;
    };
    if retain_marker {
        if let Some(unit) = object.get_mut("stringUnit").and_then(Value::as_object_mut) {
            unit.remove("value");
            unit.remove("state");
        }
    } else {
        object.remove("stringUnit");
    }
    if let Some(variations) = object.get_mut("variations").and_then(Value::as_object_mut) {
        for variants in variations.values_mut().filter_map(Value::as_object_mut) {
            for child in variants.values_mut() {
                map_units_mut(child, retain_marker);
            }
        }
    }
    if let Some(substitutions) = object
        .get_mut("substitutions")
        .and_then(Value::as_object_mut)
    {
        for child in substitutions.values_mut() {
            map_units_mut(child, retain_marker);
        }
    }
}

pub(crate) fn import(
    root: &Map<String, Value>,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    let strings = root
        .get("strings")
        .and_then(Value::as_object)
        .ok_or("Apple .xcstrings root must contain a 'strings' object.")?;
    if let Some(source) = root.get("sourceLanguage") {
        result.catalog.source_culture = Some(
            source
                .as_str()
                .ok_or(".xcstrings sourceLanguage must be a string.")?
                .into(),
        );
    }
    let mut root_metadata = root.clone();
    root_metadata.remove("strings");
    root_metadata.remove("sourceLanguage");
    if !root_metadata.is_empty() {
        result.catalog.metadata.insert(
            "xcstrings.root".into(),
            Value::Object(root_metadata).to_string(),
        );
    }
    let mut untranslated = Map::new();
    for (key, record) in strings {
        let record = record
            .as_object()
            .ok_or("A .xcstrings string record must be an object.")?;
        let comment = record
            .get("comment")
            .map(|value| {
                value
                    .as_str()
                    .ok_or("A .xcstrings comment must be a string.")
            })
            .transpose()?;
        let mut record_metadata = record.clone();
        record_metadata.remove("comment");
        record_metadata.remove("localizations");
        let Some(localizations) = record.get("localizations") else {
            untranslated.insert(key.clone(), Value::Object(record.clone()));
            continue;
        };
        let localizations = localizations
            .as_object()
            .ok_or(".xcstrings localizations must be an object.")?;
        let mut untranslated_localizations = Map::new();
        for (culture, localization) in localizations {
            let mut localized_units = Vec::new();
            units(localization, &mut Vec::new(), &mut localized_units)?;
            if localized_units.is_empty() {
                untranslated_localizations.insert(culture.clone(), localization.clone());
                result.diagnostics.push(TranslationFormatDiagnostic::new(Severity::Info, "PTF120", format!("Localization '{key}' ({culture}) has no string units; retained as document metadata.")));
                continue;
            }
            let mut skeleton = localization.clone();
            map_units_mut(&mut skeleton, true);
            for (path, unit) in localized_units {
                let value = unit
                    .get("value")
                    .and_then(Value::as_str)
                    .ok_or("A .xcstrings stringUnit requires a string value.")?;
                let state = unit
                    .get("state")
                    .map(|value| {
                        value
                            .as_str()
                            .ok_or("A .xcstrings stringUnit state must be a string.")
                    })
                    .transpose()?;
                let (suffix, plural) = path_key(&path)?;
                let entry = result
                    .catalog
                    .add(format!("{key}{suffix}"), Some(culture), value);
                entry.comment = comment.map(str::to_owned);
                entry.state = state.map(str::to_owned);
                entry.plural_category = plural;
                entry.metadata.insert(
                    "xcstrings.path".into(),
                    serde_json::to_string(&path).map_err(|e| e.to_string())?,
                );
                entry
                    .metadata
                    .insert("xcstrings.localization".into(), skeleton.to_string());
                if !record_metadata.is_empty() {
                    entry.metadata.insert(
                        "xcstrings.record".into(),
                        Value::Object(record_metadata.clone()).to_string(),
                    );
                }
                let mut unit_metadata = unit.clone();
                unit_metadata.remove("value");
                unit_metadata.remove("state");
                entry.metadata.insert(
                    "xcstrings.unit".into(),
                    Value::Object(unit_metadata).to_string(),
                );
            }
        }
        if localizations.is_empty() || !untranslated_localizations.is_empty() {
            let mut preserved = record.clone();
            preserved.insert(
                "localizations".into(),
                Value::Object(untranslated_localizations),
            );
            untranslated.insert(key.clone(), Value::Object(preserved));
        }
    }
    if !untranslated.is_empty() {
        result.catalog.metadata.insert(
            "xcstrings.untranslated".into(),
            Value::Object(untranslated).to_string(),
        );
    }
    Ok(())
}

fn shared_metadata(
    entries: &[&TranslationCatalogEntry],
    key: &str,
) -> Result<Map<String, Value>, String> {
    let mut result = None;
    for entry in entries {
        if let Some(value) = entry.metadata.get(key) {
            let value = parse_object(value)?;
            if result.as_ref().is_some_and(|previous| previous != &value) {
                return Err(format!(
                    ".xcstrings entries disagree on shared '{key}' metadata."
                ));
            }
            result = Some(value);
        }
    }
    Ok(result.unwrap_or_default())
}

fn remove_path(root: &mut Value, path: &[String]) {
    if let Some((last, parents)) = path.split_last() {
        let mut node = root;
        for part in parents {
            let Some(next) = node.get_mut(part) else {
                return;
            };
            node = next;
        }
        if let Some(object) = node.as_object_mut() {
            object.remove(last);
        }
    }
}

fn set_unit(root: &mut Value, path: &[String], unit: Value) -> Result<(), String> {
    let mut node = root;
    for part in path {
        node = node
            .as_object_mut()
            .ok_or("Conflicting .xcstrings variation structure.")?
            .entry(part.clone())
            .or_insert_with(|| Value::Object(Map::new()));
    }
    node.as_object_mut()
        .ok_or("Conflicting .xcstrings string unit.")?
        .insert("stringUnit".into(), unit);
    Ok(())
}

fn clean_empty_containers(node: &mut Value) {
    let Some(object) = node.as_object_mut() else {
        return;
    };
    if let Some(variations) = object.get_mut("variations").and_then(Value::as_object_mut) {
        for variants in variations.values_mut().filter_map(Value::as_object_mut) {
            for child in variants.values_mut() {
                clean_empty_containers(child);
            }
            variants.retain(|_, value| value.as_object().is_none_or(|object| !object.is_empty()));
        }
        variations.retain(|_, value| value.as_object().is_none_or(|object| !object.is_empty()));
        if variations.is_empty() {
            object.remove("variations");
        }
    }
    if let Some(substitutions) = object
        .get_mut("substitutions")
        .and_then(Value::as_object_mut)
    {
        for child in substitutions.values_mut() {
            clean_empty_containers(child);
        }
        substitutions.retain(|_, value| value.as_object().is_none_or(|object| !object.is_empty()));
        if substitutions.is_empty() {
            object.remove("substitutions");
        }
    }
}

pub(crate) fn export(
    catalog: &TranslationCatalog,
    options: &TranslationFormatOptions,
) -> Result<String, String> {
    let mut unique = BTreeMap::new();
    for entry in &catalog.entries {
        let Some(culture) = entry
            .culture
            .as_deref()
            .filter(|culture| !culture.is_empty())
        else {
            continue;
        };
        if options
            .culture
            .as_deref()
            .is_some_and(|selected| !selected.eq_ignore_ascii_case(culture))
        {
            continue;
        }
        unique.insert((culture.to_ascii_lowercase(), &entry.key), entry);
    }
    type LocalizedEntry<'a> = (&'a TranslationCatalogEntry, Vec<String>);
    let mut groups = BTreeMap::<String, BTreeMap<String, Vec<LocalizedEntry<'_>>>>::new();
    for ((culture, _), entry) in unique {
        let path: Vec<String> = match entry.metadata.get("xcstrings.path") {
            Some(path) => serde_json::from_value(crate::json::parse_checked(path)?)
                .map_err(|e| format!("Invalid .xcstrings path: {e}"))?,
            None => entry
                .plural_category
                .as_ref()
                .map(|category| vec!["variations".into(), "plural".into(), category.clone()])
                .unwrap_or_default(),
        };
        let (suffix, plural) = path_key(&path)?;
        if plural != entry.plural_category {
            return Err(format!(
                ".xcstrings plural metadata disagrees with the variation path for '{}'.",
                entry.key
            ));
        }
        let key = entry
            .key
            .strip_suffix(&suffix)
            .unwrap_or_else(|| entry.base_key());
        groups
            .entry(key.into())
            .or_default()
            .entry(culture)
            .or_default()
            .push((entry, path));
    }
    let mut strings = match catalog.metadata.get("xcstrings.untranslated") {
        Some(value) => parse_object(value)?,
        None => Map::new(),
    };
    for (key, cultures) in groups {
        let entries: Vec<_> = cultures
            .values()
            .flatten()
            .map(|(entry, _)| *entry)
            .collect();
        let comment = entries
            .iter()
            .filter_map(|entry| entry.comment.as_ref())
            .next();
        if options.include_comments
            && entries
                .iter()
                .filter_map(|entry| entry.comment.as_ref())
                .any(|value| Some(value) != comment)
        {
            return Err(format!(
                ".xcstrings key '{key}' has conflicting shared comments."
            ));
        }
        let mut record = strings
            .remove(&key)
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        record.extend(shared_metadata(&entries, "xcstrings.record")?);
        record.remove("comment");
        if options.include_comments {
            if let Some(comment) = comment {
                record.insert("comment".into(), Value::String(comment.clone()));
            }
        }
        let mut localizations = record
            .remove("localizations")
            .and_then(|value| value.as_object().cloned())
            .unwrap_or_default();
        for (culture, entries) in cultures {
            if entries.iter().any(|(_, path)| path.is_empty())
                && entries.iter().any(|(_, path)| !path.is_empty())
                && entries
                    .iter()
                    .any(|(entry, _)| !entry.metadata.contains_key("xcstrings.path"))
            {
                return Err(format!(".xcstrings key '{key}' ({culture}) contains ambiguous singular and plural records."));
            }
            let entry_refs: Vec<_> = entries.iter().map(|(entry, _)| *entry).collect();
            let mut localization =
                Value::Object(shared_metadata(&entry_refs, "xcstrings.localization")?);
            let mut original_units = Vec::new();
            units(&localization, &mut Vec::new(), &mut original_units)?;
            let mut original_paths: Vec<_> =
                original_units.into_iter().map(|(path, _)| path).collect();
            original_paths.sort_by_key(|path| std::cmp::Reverse(path.len()));
            map_units_mut(&mut localization, false);
            for path in original_paths {
                let mut length = 0;
                while length < path.len() {
                    length += if path[length] == "substitutions" {
                        2
                    } else {
                        3
                    };
                    let prefix = &path[..length];
                    if !entries.iter().any(|(_, active)| active.starts_with(prefix)) {
                        remove_path(&mut localization, prefix);
                        break;
                    }
                }
            }
            let mut paths = BTreeSet::new();
            for (entry, path) in &entries {
                if !paths.insert(path) {
                    return Err(format!(
                        "Duplicate .xcstrings variation path for '{key}' ({culture})."
                    ));
                }
                let mut unit = entry
                    .metadata
                    .get("xcstrings.unit")
                    .map(|value| parse_object(value))
                    .transpose()?
                    .unwrap_or_default();
                unit.insert("value".into(), Value::String(entry.value.clone()));
                unit.remove("state");
                if let Some(state) = &entry.state {
                    unit.insert("state".into(), Value::String(state.clone()));
                } else if !entry.metadata.contains_key("xcstrings.unit") {
                    unit.insert("state".into(), Value::String("translated".into()));
                }
                set_unit(&mut localization, path, Value::Object(unit))?;
            }
            clean_empty_containers(&mut localization);
            // Culture matching is case-insensitive, including duplicate spelling variants.
            localizations.retain(|tag, _| !tag.eq_ignore_ascii_case(&culture));
            let display = entries[0].0.culture.as_deref().unwrap_or(&culture);
            localizations.insert(display.into(), localization);
        }
        if let Some(culture) = &options.culture {
            localizations.retain(|tag, _| tag.eq_ignore_ascii_case(culture));
        }
        record.insert("localizations".into(), Value::Object(localizations));
        strings.insert(key, Value::Object(record));
    }
    for record in strings.values_mut().filter_map(Value::as_object_mut) {
        if !options.include_comments {
            record.remove("comment");
        }
        if let Some(culture) = &options.culture {
            if let Some(localizations) = record
                .get_mut("localizations")
                .and_then(Value::as_object_mut)
            {
                localizations.retain(|tag, _| tag.eq_ignore_ascii_case(culture));
            }
        }
    }
    let mut root = catalog
        .metadata
        .get("xcstrings.root")
        .map(|value| parse_object(value))
        .transpose()?
        .unwrap_or_default();
    root.insert(
        "sourceLanguage".into(),
        Value::String(
            options
                .source_culture
                .as_deref()
                .or(catalog.source_culture.as_deref())
                .unwrap_or("en")
                .into(),
        ),
    );
    root.entry("version")
        .or_insert_with(|| Value::String("1.0".into()));
    root.insert("strings".into(), Value::Object(strings));
    serde_json::to_string_pretty(&root).map_err(|e| e.to_string())
}
