use crate::{model::select_entries, *};
use roxmltree::{Document, Node, ParsingOptions};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub(crate) fn import(
    content: &str,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
    path: Option<&str>,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    // Standard doctypes and bounded internal entities are accepted. roxmltree never
    // resolves network/file resources and rejects recursive/excessive entity expansion.
    let document = Document::parse_with_options(
        content,
        ParsingOptions {
            allow_dtd: true,
            nodes_limit: 1_000_000,
        },
    )
    .map_err(|e| e.to_string())?;
    let root = document.root_element();
    match format {
        TranslationFileFormat::Resx => import_resx(root, options, path, result),
        TranslationFileFormat::AndroidResources => import_android(root, options, path, result),
        TranslationFileFormat::AppleStringsdict => import_stringsdict(root, options, path, result),
        _ => import_xliff(root, options, result),
    }
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}
fn text(node: Node<'_, '_>) -> String {
    node.descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect()
}
fn inline_warning(node: Node<'_, '_>, key: &str, result: &mut TranslationFormatResult) {
    if node.children().any(|n| n.is_element()) {
        result.diagnostics.push(TranslationFormatDiagnostic { key: Some(key.into()), ..TranslationFormatDiagnostic::new(Severity::Warning,"PTF181","Inline XML markup was flattened to text; review inline codes and formatting before export.") });
    }
}

fn import_resx(
    root: Node<'_, '_>,
    options: &TranslationFormatOptions,
    path: Option<&str>,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    if root.tag_name().name() != "root" {
        return Err("RESX requires a root element.".into());
    }
    let culture = options
        .culture
        .clone()
        .or_else(|| infer_culture(path, ".resx"));
    for data in root.children().filter(|n| n.has_tag_name("data")) {
        let key = data.attribute("name").filter(|key| !key.trim().is_empty());
        let value = child(data, "value");
        let (Some(key), Some(value)) = (key, value) else {
            result.diagnostics.push(TranslationFormatDiagnostic::new(
                Severity::Warning,
                "PTF130",
                "Ignored RESX data without a name or value.",
            ));
            continue;
        };
        if data.attribute("mimetype").is_some()
            || data
                .attribute("type")
                .is_some_and(|kind| kind.split(',').next().unwrap_or("").trim() != "System.String")
        {
            result.diagnostics.push(TranslationFormatDiagnostic { key: Some(key.into()), culture: culture.clone(), ..TranslationFormatDiagnostic::new(Severity::Warning,"PTF131","Ignored non-string RESX resource; binary/file/type conversion resources are not localized strings.") });
            continue;
        }
        inline_warning(value, key, result);
        let entry = result.catalog.add(key, culture.as_deref(), text(value));
        entry.comment = child(data, "comment").map(text);
    }
    Ok(())
}

fn android_culture(path: Option<&str>) -> Option<String> {
    let path = path?;
    for part in path.split(['/', '\\']) {
        if let Some(locale) = part.strip_prefix("values-b+") {
            let locale = locale.split('-').next()?;
            return Some(TranslationCatalog::normalize_culture_name(Some(
                &locale.replace('+', "-"),
            )));
        }
        if let Some(locale) = part.strip_prefix("values-") {
            let mut pieces = locale.split('-');
            let lang = pieces.next()?;
            if !(lang.len() == 2 || lang.len() == 3)
                || !lang.bytes().all(|b| b.is_ascii_lowercase())
            {
                continue;
            }
            let region = pieces.next().and_then(|part| part.strip_prefix('r'));
            return Some(match region {
                Some(region) => format!("{lang}-{region}"),
                None => lang.into(),
            });
        }
    }
    None
}

fn import_android(
    root: Node<'_, '_>,
    options: &TranslationFormatOptions,
    path: Option<&str>,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    if root.tag_name().name() != "resources" {
        return Err("Android strings.xml requires a resources root.".into());
    }
    let culture = options.culture.clone().or_else(|| android_culture(path));
    for node in root.children().filter(|n| n.is_element()) {
        let Some(name) = node
            .attribute("name")
            .filter(|name| !name.trim().is_empty())
        else {
            continue;
        };
        if node.attribute("translatable") == Some("false") {
            result.diagnostics.push(TranslationFormatDiagnostic::new(
                Severity::Info,
                "PTF141",
                format!("Resource '{name}' is marked non-translatable; flag retained in metadata."),
            ));
        }
        match node.tag_name().name() {
            "string" => {
                inline_warning(node, name, result);
                let value = decode_android(&text(node))?;
                let entry = result.catalog.add(name, culture.as_deref(), value);
                for attr in ["translatable", "formatted"] {
                    copy_android_flag(entry, node, attr)?;
                }
            }
            "plurals" => {
                for item in node.children().filter(|n| n.has_tag_name("item")) {
                    let Some(quantity) = item.attribute("quantity") else {
                        result.diagnostics.push(TranslationFormatDiagnostic::new(
                            Severity::Warning,
                            "PTF142",
                            format!("Plural item '{name}' has no quantity."),
                        ));
                        continue;
                    };
                    inline_warning(item, name, result);
                    let entry = result.catalog.add(
                        format!("{name}.{quantity}"),
                        culture.as_deref(),
                        decode_android(&text(item))?,
                    );
                    entry.plural_category = Some(quantity.into());
                    for attr in ["translatable", "formatted"] {
                        copy_android_flag(entry, node, attr)?;
                    }
                }
            }
            "string-array" => {
                for (index, item) in node
                    .children()
                    .filter(|n| n.has_tag_name("item"))
                    .enumerate()
                {
                    inline_warning(item, name, result);
                    let entry = result.catalog.add(
                        format!("{name}[{index}]"),
                        culture.as_deref(),
                        decode_android(&text(item))?,
                    );
                    entry.metadata.insert("android.array".into(), name.into());
                    entry
                        .metadata
                        .insert("android.index".into(), index.to_string());
                    for attr in ["translatable", "formatted"] {
                        copy_android_flag(entry, node, attr)?;
                    }
                }
            }
            other => result.diagnostics.push(TranslationFormatDiagnostic::new(
                Severity::Info,
                "PTF140",
                format!("Ignored Android resource '{other}' ({name})."),
            )),
        }
    }
    Ok(())
}

fn copy_android_flag(
    entry: &mut TranslationCatalogEntry,
    node: Node<'_, '_>,
    attr: &str,
) -> Result<(), String> {
    if let Some(value) = node.attribute(attr) {
        if !matches!(value, "true" | "false") {
            return Err(format!("Android {attr} must be true or false."));
        }
        entry
            .metadata
            .insert(format!("android.{attr}"), value.into());
    }
    Ok(())
}

fn decode_android(value: &str) -> Result<String, String> {
    // XML entities are decoded first. Quotes can delimit any region of an Android
    // string, preserving whitespace only in that region; they are not literal text.
    let mut input = value.chars().peekable();
    let mut output = String::new();
    let mut quoted = false;
    let mut space = false;
    while let Some(c) = input.next() {
        if !quoted && c.is_whitespace() {
            if !space {
                output.push(' ');
            }
            space = true;
            continue;
        }
        space = false;
        match c {
            '"' => quoted = !quoted,
            '\'' if !quoted => return Err("Unescaped apostrophe in Android string.".into()),
            '\\' => {
                let next = input
                    .next()
                    .ok_or("Trailing backslash in Android string.")?;
                match next {
                    'n' => output.push('\n'),
                    't' => output.push('\t'),
                    'u' => {
                        let mut escaped = String::from("\\u");
                        escaped.extend(input.by_ref().take(4));
                        if escaped.len() == 6
                            && u16::from_str_radix(&escaped[2..], 16)
                                .ok()
                                .is_some_and(|code| (0xd800..=0xdbff).contains(&code))
                        {
                            escaped.extend(input.by_ref().take(6));
                        }
                        output.push_str(&crate::text::decode_escapes(&escaped, true)?);
                    }
                    _ => output.push(next),
                }
            }
            _ => output.push(c),
        }
    }
    Ok(output)
}
fn encode_android(value: &str) -> String {
    let mut result = String::new();
    for c in value.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\u000D"),
            '\t' => result.push_str("\\t"),
            '\'' => result.push_str("\\'"),
            '"' => result.push_str("\\\""),
            _ => result.push(c),
        }
    }
    // Quoting preserves leading/trailing and repeated whitespace in the platform parser.
    format!("\"{result}\"")
}

fn plist_dict<'a, 'input>(
    node: Node<'a, 'input>,
) -> Result<BTreeMap<String, Node<'a, 'input>>, String> {
    if node.tag_name().name() != "dict" {
        return Err("Expected a plist dictionary.".into());
    }
    let mut children = node.children().filter(|n| n.is_element());
    let mut map = BTreeMap::new();
    while let Some(key) = children.next() {
        if key.tag_name().name() != "key" {
            return Err("Plist dictionaries must alternate keys and values.".into());
        }
        let value = children.next().ok_or("Plist key has no value.")?;
        if map.insert(text(key), value).is_some() {
            return Err("Plist dictionary contains a duplicate key.".into());
        }
    }
    Ok(map)
}

fn import_stringsdict(
    root: Node<'_, '_>,
    options: &TranslationFormatOptions,
    path: Option<&str>,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    if root.tag_name().name() != "plist" {
        return Err(".stringsdict requires a plist root.".into());
    }
    let top = plist_dict(child(root, "dict").ok_or(".stringsdict requires a plist/dict root.")?)?;
    let culture = options
        .culture
        .clone()
        .or_else(|| infer_apple_culture(path));
    for (key, node) in top {
        let fields = plist_dict(node)?;
        let native_rules: Vec<_> = [
            "NSStringDeviceSpecificRuleType",
            "NSStringVariableWidthRuleType",
        ]
        .into_iter()
        .filter_map(|rule| fields.get(rule).map(|node| (rule, *node)))
        .collect();
        if !native_rules.is_empty() {
            if native_rules.len() > 1 || fields.contains_key("NSStringLocalizedFormatKey") {
                return Err(format!(
                    "Stringsdict '{key}' mixes incompatible variation rules."
                ));
            }
            let (rule, variants) = native_rules[0];
            let axis = if rule == "NSStringDeviceSpecificRuleType" {
                "device"
            } else {
                "width"
            };
            for (variant, value) in plist_dict(variants)? {
                if !value.has_tag_name("string") {
                    return Err(format!(
                        "Stringsdict '{key}.{axis}.{variant}' must be a string."
                    ));
                }
                let entry = result.catalog.add(
                    format!("{key}.{axis}.{variant}"),
                    culture.as_deref(),
                    text(value),
                );
                entry.metadata.insert("apple.key".into(), key.clone());
                entry.metadata.insert("apple.rule".into(), rule.into());
                entry.metadata.insert("apple.variant".into(), variant);
            }
            if fields.len() > 1 {
                result.diagnostics.push(TranslationFormatDiagnostic::new(
                    Severity::Warning,
                    "PTF163",
                    format!("Additional stringsdict fields in '{key}' were skipped."),
                ));
            }
            continue;
        }
        if fields
            .get("NSStringLocalizedFormatKey")
            .is_some_and(|node| !node.has_tag_name("string"))
        {
            return Err(format!(
                "Stringsdict '{key}' localized format must be a string."
            ));
        }
        let format = fields.get("NSStringLocalizedFormatKey").map(|n| text(*n));
        let variables: Vec<_> = fields
            .iter()
            .filter(|(_, v)| v.has_tag_name("dict"))
            .collect();
        for (variable, dict) in &variables {
            let plural = plist_dict(**dict)?;
            if plural
                .get("NSStringFormatValueTypeKey")
                .is_some_and(|node| !node.has_tag_name("string"))
            {
                return Err(format!(
                    "Stringsdict '{key}.{variable}' value type must be a string."
                ));
            }
            if plural
                .get("NSStringFormatSpecTypeKey")
                .map(|n| text(*n))
                .as_deref()
                != Some("NSStringPluralRuleType")
            {
                result.diagnostics.push(TranslationFormatDiagnostic::new(
                    Severity::Warning,
                    "PTF161",
                    format!("Unsupported stringsdict rule for '{key}.{variable}' was skipped."),
                ));
                continue;
            }
            if variables.len() > 1 {
                result.diagnostics.push(TranslationFormatDiagnostic::new(
                    Severity::Warning,
                    "PTF162",
                    format!(
                        "Multiple variables in '{key}'; variable name retained in normalized key."
                    ),
                ));
            }
            for category in ["zero", "one", "two", "few", "many", "other"] {
                if let Some(value) = plural.get(category) {
                    if !value.has_tag_name("string") {
                        return Err(format!("Plural '{key}.{category}' must be a string."));
                    }
                    let entry_key = if variables.len() > 1 {
                        format!("{key}.{variable}.{category}")
                    } else {
                        format!("{key}.{category}")
                    };
                    let entry = result
                        .catalog
                        .add(entry_key, culture.as_deref(), text(*value));
                    entry.plural_category = Some(category.into());
                    entry
                        .metadata
                        .insert("apple.variable".into(), (*variable).clone());
                    entry.metadata.insert("apple.key".into(), key.clone());
                    if let Some(format) = &format {
                        entry
                            .metadata
                            .insert("apple.localized_format".into(), format.clone());
                    }
                    if let Some(kind) = plural.get("NSStringFormatValueTypeKey") {
                        entry
                            .metadata
                            .insert("apple.value_type".into(), text(*kind));
                    }
                }
            }
            if plural.keys().any(|k| {
                !matches!(
                    k.as_str(),
                    "NSStringFormatSpecTypeKey"
                        | "NSStringFormatValueTypeKey"
                        | "zero"
                        | "one"
                        | "two"
                        | "few"
                        | "many"
                        | "other"
                )
            }) {
                result.diagnostics.push(TranslationFormatDiagnostic::new(
                    Severity::Warning,
                    "PTF163",
                    format!("Additional stringsdict fields in '{key}.{variable}' were skipped."),
                ));
            }
        }
    }
    if result.catalog.entries.is_empty() {
        result.diagnostics.push(TranslationFormatDiagnostic::new(
            Severity::Warning,
            "PTF160",
            "No supported plural entries were found in .stringsdict.",
        ));
    }
    Ok(())
}

fn import_xliff(
    root: Node<'_, '_>,
    options: &TranslationFormatOptions,
    result: &mut TranslationFormatResult,
) -> Result<(), String> {
    if root.tag_name().name() != "xliff" {
        return Err("XLIFF requires an xliff root.".into());
    }
    let version = root
        .attribute("version")
        .ok_or("XLIFF version is required.")?;
    if !matches!(version, "1.0" | "1.1" | "1.2" | "2.0" | "2.1") {
        return Err(format!("Unsupported XLIFF version '{version}'."));
    }
    let old = version.starts_with("1.");
    if !old {
        result.catalog.source_culture = root
            .attribute("srcLang")
            .map(str::to_owned)
            .or(result.catalog.source_culture.clone());
    }
    for file in root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "file")
    {
        let culture = if old {
            file.attribute("target-language")
        } else {
            root.attribute("trgLang")
        }
        .or(options.culture.as_deref());
        if old {
            result.catalog.source_culture = result
                .catalog
                .source_culture
                .clone()
                .or_else(|| file.attribute("source-language").map(str::to_owned));
        }
        let unit_tag = if old { "trans-unit" } else { "unit" };
        for unit in file
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == unit_tag)
        {
            let Some(key) = unit
                .attribute(if old { "resname" } else { "name" })
                .or_else(|| unit.attribute("id"))
            else {
                result.diagnostics.push(TranslationFormatDiagnostic::new(
                    Severity::Warning,
                    "PTF182",
                    "Skipped XLIFF unit without a deterministic key (id/name).",
                ));
                continue;
            };
            let comment = unit
                .descendants()
                .find(|n| {
                    n.is_element()
                        && n.tag_name().name() == "note"
                        && n.attribute("category") != Some("context")
                })
                .map(text);
            let context = unit
                .descendants()
                .find(|n| {
                    n.is_element()
                        && (n.tag_name().name() == "context"
                            || (n.tag_name().name() == "note"
                                && n.attribute("category") == Some("context")))
                })
                .map(text);
            let notes: Vec<_> = if old {
                unit.children()
                    .filter(|node| {
                        node.is_element()
                            && matches!(node.tag_name().name(), "note" | "context-group")
                    })
                    .map(node_xml)
                    .collect()
            } else {
                child(unit, "notes")
                    .into_iter()
                    .flat_map(|notes| notes.children())
                    .filter(|node| node.is_element())
                    .map(node_xml)
                    .collect()
            };
            let extras: Vec<_> = unit
                .children()
                .filter(|node| {
                    node.is_element()
                        && !matches!(
                            node.tag_name().name(),
                            "notes" | "note" | "context-group" | "source" | "target" | "segment"
                        )
                })
                .map(node_xml)
                .collect();
            let mut preceding_segments = 0;
            let extra_positions: Vec<_> = unit
                .children()
                .filter(|node| node.is_element())
                .filter_map(|node| {
                    if node.tag_name().name() == "segment" {
                        preceding_segments += 1;
                        None
                    } else if !matches!(
                        node.tag_name().name(),
                        "notes" | "note" | "context-group" | "source" | "target"
                    ) {
                        Some(preceding_segments)
                    } else {
                        None
                    }
                })
                .collect();
            let segments: Vec<_> = if old {
                vec![unit]
            } else {
                unit.children()
                    .filter(|n| n.is_element() && n.tag_name().name() == "segment")
                    .collect()
            };
            let multiple = segments.len() > 1;
            let mut segment_ids = BTreeSet::new();
            if multiple {
                result.diagnostics.push(TranslationFormatDiagnostic::new(Severity::Warning,"PTF183",format!("Unit '{key}' has multiple segments; normalized keys include each segment identifier.")));
            }
            for (index, segment) in segments.into_iter().enumerate() {
                let source = child(segment, "source");
                let target = child(segment, "target");
                for node in [source, target].into_iter().flatten() {
                    if node.children().any(|node| node.is_element()) {
                        result.diagnostics.push(TranslationFormatDiagnostic::new(Severity::Info, "PTF181", format!("Inline XML for '{key}' is retained in metadata; normalized text excludes markup.")));
                    }
                }
                let source_xml = source.map(node_xml);
                let source = source.map(text);
                let entry_key = if multiple {
                    format!(
                        "{key}#{}",
                        segment
                            .attribute("id")
                            .map(str::to_owned)
                            .unwrap_or_else(|| (index + 1).to_string())
                    )
                } else {
                    key.into()
                };
                let entry = result.catalog.add(
                    entry_key,
                    culture,
                    target
                        .map(text)
                        .or_else(|| source.clone())
                        .unwrap_or_default(),
                );
                entry.source = source;
                entry.comment = comment.clone();
                entry.context = context.clone();
                entry.state = if old {
                    target.and_then(|n| n.attribute("state"))
                } else {
                    segment.attribute("state")
                }
                .map(str::to_owned);
                entry
                    .metadata
                    .insert("xliff.version".into(), if old { "1" } else { "2" }.into());
                let mut unit_attributes = String::new();
                write_node(
                    &mut unit_attributes,
                    unit,
                    Some((unit.id(), "")),
                    None,
                    None,
                );
                entry
                    .metadata
                    .insert("xliff.unit_attributes".into(), unit_attributes);
                if !old {
                    let mut segment_attributes = String::new();
                    write_node(
                        &mut segment_attributes,
                        segment,
                        Some((segment.id(), "")),
                        None,
                        None,
                    );
                    entry
                        .metadata
                        .insert("xliff.segment_attributes".into(), segment_attributes);
                }
                if let Some(id) = unit.attribute("id") {
                    entry.metadata.insert("xliff.unit_id".into(), id.into());
                }
                if let Some(source_xml) = source_xml {
                    entry.metadata.insert("xliff.source_xml".into(), source_xml);
                }
                if let Some(target) = target {
                    entry
                        .metadata
                        .insert("xliff.target_xml".into(), node_xml(target));
                } else {
                    entry
                        .metadata
                        .insert("xliff.untranslated".into(), "true".into());
                }
                if !notes.is_empty() {
                    entry.metadata.insert(
                        "xliff.notes".into(),
                        serde_json::to_string(&notes).map_err(|e| e.to_string())?,
                    );
                }
                if !extras.is_empty() {
                    entry.metadata.insert(
                        "xliff.extras".into(),
                        serde_json::to_string(&extras).map_err(|e| e.to_string())?,
                    );
                    entry.metadata.insert(
                        "xliff.extra_positions".into(),
                        serde_json::to_string(&extra_positions).map_err(|e| e.to_string())?,
                    );
                }
                if multiple {
                    entry.metadata.insert("xliff.unit".into(), key.into());
                    entry.metadata.insert(
                        "xliff.segment".into(),
                        segment
                            .attribute("id")
                            .map(str::to_owned)
                            .unwrap_or_else(|| (index + 1).to_string()),
                    );
                    entry
                        .metadata
                        .insert("xliff.segment_order".into(), index.to_string());
                    if !segment_ids.insert(entry.metadata["xliff.segment"].clone()) {
                        return Err(format!(
                            "XLIFF unit '{key}' has duplicate segment identifiers."
                        ));
                    }
                } else if !old {
                    if let Some(id) = segment.attribute("id") {
                        entry.metadata.insert("xliff.segment_id".into(), id.into());
                    }
                }
            }
        }
    }
    if root.descendants().any(|n| {
        n.is_element() && matches!(n.tag_name().name(), "skl" | "skeleton" | "phase-group")
    }) {
        result.diagnostics.push(TranslationFormatDiagnostic::new(Severity::Warning,"PTF184","XLIFF file skeleton or phase information is not represented in the normalized catalog."));
    }
    if result.catalog.entries.is_empty() {
        result.diagnostics.push(TranslationFormatDiagnostic::new(
            Severity::Warning,
            "PTF180",
            "No XLIFF units were imported.",
        ));
    }
    Ok(())
}

fn escape(value: &str) -> String {
    let mut output = String::new();
    for c in value.chars() {
        match c {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            '\r' => output.push_str("&#13;"),
            '\n' => output.push_str("&#10;"),
            '\t' => output.push_str("&#9;"),
            _ => output.push(c),
        }
    }
    output
}
fn valid_xml(value: &str) -> bool {
    value.chars().all(|c| matches!(c,'\t'|'\n'|'\r'|'\u{20}'..='\u{d7ff}'|'\u{e000}'..='\u{fffd}'|'\u{10000}'..='\u{10ffff}'))
}
fn element(output: &mut String, name: &str, value: &str) {
    writeln!(output, "<{name}>{}</{name}>", escape(value)).unwrap();
}

fn node_xml(node: Node<'_, '_>) -> String {
    let mut output = String::new();
    write_node(&mut output, node, None, None, None);
    output
}

fn qualified_name(
    node: Node<'_, '_>,
    namespace: Option<&str>,
    name: &str,
    attribute: bool,
) -> String {
    let Some(namespace) = namespace else {
        return name.into();
    };
    if namespace == "http://www.w3.org/XML/1998/namespace" {
        return format!("xml:{name}");
    }
    match node
        .namespaces()
        .find(|item| item.uri() == namespace && (!attribute || item.name().is_some()))
        .and_then(|item| item.name())
    {
        Some(prefix) => format!("{prefix}:{name}"),
        None => name.into(),
    }
}

fn write_node(
    output: &mut String,
    node: Node<'_, '_>,
    text_override: Option<(roxmltree::NodeId, &str)>,
    root_attribute: Option<(&str, Option<&str>)>,
    xliff_namespace: Option<&str>,
) {
    if node.is_text() {
        output.push_str(&escape(node.text().unwrap_or("")));
    } else if node.is_comment() {
        write!(output, "<!--{}-->", node.text().unwrap_or("")).unwrap();
    } else if let Some(pi) = node.pi() {
        write!(
            output,
            "<?{}{}?>",
            pi.target,
            pi.value
                .map(|value| format!(" {value}"))
                .unwrap_or_default()
        )
        .unwrap();
    } else if node.is_element() {
        let native = xliff_namespace.is_some()
            && node.tag_name().namespace().is_none_or(|namespace| {
                namespace.is_empty() || namespace.starts_with("urn:oasis:names:tc:xliff:document:")
            });
        let name = if native {
            node.tag_name().name().into()
        } else {
            qualified_name(
                node,
                node.tag_name().namespace(),
                node.tag_name().name(),
                false,
            )
        };
        write!(output, "<{name}").unwrap();
        for namespace in node
            .namespaces()
            .filter(|namespace| namespace.name() != Some("xml"))
        {
            let uri = if namespace
                .uri()
                .starts_with("urn:oasis:names:tc:xliff:document:")
            {
                xliff_namespace.unwrap_or(namespace.uri())
            } else {
                namespace.uri()
            };
            match namespace.name() {
                Some(prefix) => write!(output, " xmlns:{prefix}=\"{}\"", escape(uri)).unwrap(),
                None if !native => write!(output, " xmlns=\"{}\"", escape(uri)).unwrap(),
                None => {}
            }
        }
        // A fragment with no default namespace must remain unqualified when embedded.
        if native {
            write!(output, " xmlns=\"{}\"", xliff_namespace.unwrap()).unwrap();
        } else if node.tag_name().namespace().is_none()
            && !node
                .namespaces()
                .any(|namespace| namespace.name().is_none())
        {
            output.push_str(" xmlns=\"\"");
        }
        for attribute in node.attributes() {
            if root_attribute.is_some_and(|(name, _)| {
                attribute.namespace().is_none() && attribute.name() == name
            }) {
                continue;
            }
            let name = qualified_name(node, attribute.namespace(), attribute.name(), true);
            write!(output, " {name}=\"{}\"", escape(attribute.value())).unwrap();
        }
        if let Some((name, Some(value))) = root_attribute {
            write!(output, " {name}=\"{}\"", escape(value)).unwrap();
        }
        output.push('>');
        if let Some((_, value)) = text_override.filter(|(id, _)| *id == node.id()) {
            output.push_str(&escape(value));
        } else {
            for child in node.children() {
                write_node(output, child, text_override, None, xliff_namespace);
            }
        }
        write!(output, "</{name}>").unwrap();
    }
}

pub(crate) fn export(
    catalog: &TranslationCatalog,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
) -> Result<String, String> {
    for value in [
        catalog.name.as_deref(),
        catalog.source_culture.as_deref(),
        options.name.as_deref(),
        options.culture.as_deref(),
        options.source_culture.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !valid_xml(value) {
            return Err("Catalog metadata contains characters forbidden by XML 1.0.".into());
        }
    }
    for entry in &catalog.entries {
        for value in [
            Some(entry.key.as_str()),
            Some(entry.value.as_str()),
            entry.source.as_deref(),
            entry.comment.as_deref(),
            entry.context.as_deref(),
            entry.state.as_deref(),
            entry.culture.as_deref(),
            entry.plural_category.as_deref(),
        ]
        .into_iter()
        .flatten()
        .chain(entry.metadata.values().map(String::as_str))
        {
            if !valid_xml(value) {
                return Err(format!(
                    "Record '{}' contains characters forbidden by XML 1.0.",
                    entry.key
                ));
            }
        }
    }
    let mut output = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let culture = options
        .culture
        .clone()
        .or_else(|| catalog.cultures().into_iter().next());
    let entries = select_entries(catalog, culture.as_deref());
    match format {
        TranslationFileFormat::Resx => {
            output.push_str("<root>\n<resheader name=\"resmimetype\"><value>text/microsoft-resx</value></resheader>\n<resheader name=\"version\"><value>2.0</value></resheader>\n");
            for entry in entries {
                writeln!(
                    output,
                    "<data name=\"{}\" xml:space=\"preserve\">",
                    escape(&entry.key)
                )
                .unwrap();
                element(&mut output, "value", &entry.value);
                if options.include_comments {
                    if let Some(comment) = &entry.comment {
                        element(&mut output, "comment", comment);
                    }
                }
                output.push_str("</data>\n");
            }
            output.push_str("</root>\n");
        }
        TranslationFileFormat::AndroidResources => export_android(entries, &mut output)?,
        TranslationFileFormat::AppleStringsdict => export_stringsdict(entries, &mut output)?,
        TranslationFileFormat::Xliff12 => export_xliff(catalog, options, true, &mut output)?,
        TranslationFileFormat::Xliff20 => export_xliff(catalog, options, false, &mut output)?,
        _ => unreachable!(),
    }
    Ok(output)
}

fn export_android(
    entries: Vec<&TranslationCatalogEntry>,
    output: &mut String,
) -> Result<(), String> {
    output.push_str("<resources>\n");
    let mut plurals = BTreeMap::<&str, Vec<&TranslationCatalogEntry>>::new();
    let mut arrays = BTreeMap::<&str, BTreeMap<usize, &TranslationCatalogEntry>>::new();
    for entry in entries {
        if entry.plural_category.is_some() {
            plurals.entry(entry.base_key()).or_default().push(entry);
        } else if let Some(name) = entry.metadata.get("android.array") {
            let index = entry
                .metadata
                .get("android.index")
                .ok_or("Android array index is missing.")?
                .parse::<usize>()
                .map_err(|_| "Invalid Android array index.")?;
            if arrays
                .entry(name)
                .or_default()
                .insert(index, entry)
                .is_some()
            {
                return Err(format!("Duplicate Android array index '{name}[{index}]'."));
            }
        } else {
            write!(output, "<string name=\"{}\"", escape(&entry.key)).unwrap();
            android_flags(output, &[entry])?;
            writeln!(
                output,
                ">{}</string>",
                escape(&encode_android(&entry.value))
            )
            .unwrap();
        }
    }
    for (name, items) in arrays {
        write!(output, "<string-array name=\"{}\"", escape(name)).unwrap();
        android_flags(output, &items.values().copied().collect::<Vec<_>>())?;
        output.push_str(">\n");
        for (expected, (index, entry)) in items.into_iter().enumerate() {
            if expected != index {
                return Err(format!(
                    "Android array '{name}' has a gap at index {expected}."
                ));
            }
            element(output, "item", &encode_android(&entry.value));
        }
        output.push_str("</string-array>\n");
    }
    for (name, entries) in plurals {
        write!(output, "<plurals name=\"{}\"", escape(name)).unwrap();
        android_flags(output, &entries)?;
        output.push_str(">\n");
        for entry in entries {
            let category = entry.plural_category.as_deref().unwrap();
            if !matches!(category, "zero" | "one" | "two" | "few" | "many" | "other") {
                return Err(format!("Unsupported Android plural category '{category}'."));
            }
            writeln!(
                output,
                "<item quantity=\"{}\">{}</item>",
                escape(category),
                escape(&encode_android(&entry.value))
            )
            .unwrap();
        }
        output.push_str("</plurals>\n");
    }
    output.push_str("</resources>\n");
    Ok(())
}

fn android_flags(output: &mut String, entries: &[&TranslationCatalogEntry]) -> Result<(), String> {
    for attr in ["translatable", "formatted"] {
        let key = format!("android.{attr}");
        let value = entries[0].metadata.get(&key);
        if entries
            .iter()
            .any(|entry| entry.metadata.get(&key) != value)
        {
            return Err(format!(
                "Android resource '{}' has conflicting {attr} flags.",
                entries[0].base_key()
            ));
        }
        if let Some(value) = value {
            if !matches!(value.as_str(), "true" | "false") {
                return Err(format!("Android {attr} must be true or false."));
            }
            write!(output, " {attr}=\"{value}\"").unwrap();
        }
    }
    Ok(())
}

fn export_stringsdict(
    entries: Vec<&TranslationCatalogEntry>,
    output: &mut String,
) -> Result<(), String> {
    let mut groups = BTreeMap::<&str, BTreeMap<&str, Vec<&TranslationCatalogEntry>>>::new();
    let mut variants = BTreeMap::<&str, (&str, BTreeMap<&str, &TranslationCatalogEntry>)>::new();
    for entry in entries {
        if let Some(rule) = entry.metadata.get("apple.rule") {
            if entry.plural_category.is_some()
                || !matches!(
                    rule.as_str(),
                    "NSStringDeviceSpecificRuleType" | "NSStringVariableWidthRuleType"
                )
            {
                return Err(format!(
                    "Unsupported or conflicting stringsdict rule for '{}'.",
                    entry.key
                ));
            }
            let key = entry
                .metadata
                .get("apple.key")
                .ok_or("Stringsdict variant is missing apple.key.")?;
            let variant = entry
                .metadata
                .get("apple.variant")
                .ok_or("Stringsdict variant is missing apple.variant.")?;
            let group = variants
                .entry(key)
                .or_insert_with(|| (rule, BTreeMap::new()));
            if group.0 != rule || group.1.insert(variant, entry).is_some() {
                return Err(format!("Conflicting stringsdict variants for '{key}'."));
            }
            continue;
        }
        if entry.plural_category.is_none() {
            return Err(format!(
                ".stringsdict cannot represent singular record '{}'; export it as .strings.",
                entry.key
            ));
        }
        let key = entry
            .metadata
            .get("apple.key")
            .map(String::as_str)
            .unwrap_or_else(|| entry.base_key());
        let variable = entry
            .metadata
            .get("apple.variable")
            .map(String::as_str)
            .unwrap_or("value");
        if variable == "NSStringLocalizedFormatKey" || variable.is_empty() {
            return Err("Stringsdict plural variable uses a reserved or empty name.".into());
        }
        groups
            .entry(key)
            .or_default()
            .entry(variable)
            .or_default()
            .push(entry);
    }
    if groups.keys().any(|key| variants.contains_key(key)) {
        return Err("A stringsdict key mixes plural and device/width rules.".into());
    }
    output.push_str("<plist version=\"1.0\"><dict>\n");
    for (key, (rule, variants)) in variants {
        element(output, "key", key);
        output.push_str("<dict>\n");
        element(output, "key", rule);
        output.push_str("<dict>\n");
        for (variant, entry) in variants {
            element(output, "key", variant);
            element(output, "string", &entry.value);
        }
        output.push_str("</dict></dict>\n");
    }
    for (key, variables) in groups {
        element(output, "key", key);
        output.push_str("<dict>\n");
        element(output, "key", "NSStringLocalizedFormatKey");
        let first = variables
            .values()
            .flatten()
            .next()
            .expect("group is not empty");
        if variables.values().flatten().any(|entry| {
            entry.metadata.get("apple.localized_format")
                != first.metadata.get("apple.localized_format")
        }) {
            return Err(format!(
                "Stringsdict '{key}' has conflicting localized format strings."
            ));
        }
        let default_format = variables
            .keys()
            .map(|variable| format!("%#@{variable}@"))
            .collect::<Vec<_>>()
            .join(" ");
        element(
            output,
            "string",
            first
                .metadata
                .get("apple.localized_format")
                .map(String::as_str)
                .unwrap_or(&default_format),
        );
        for (variable, entries) in variables {
            if entries.iter().any(|entry| {
                entry.metadata.get("apple.value_type")
                    != entries[0].metadata.get("apple.value_type")
            }) {
                return Err(format!(
                    "Stringsdict '{key}.{variable}' has conflicting value types."
                ));
            }
            let mut categories = BTreeSet::new();
            for entry in &entries {
                let category = entry.plural_category.as_deref().unwrap();
                if !matches!(category, "zero" | "one" | "two" | "few" | "many" | "other")
                    || !categories.insert(category)
                {
                    return Err(format!("Stringsdict '{key}.{variable}' has an invalid or duplicate plural category."));
                }
            }
            element(output, "key", variable);
            output.push_str("<dict>\n");
            element(output, "key", "NSStringFormatSpecTypeKey");
            element(output, "string", "NSStringPluralRuleType");
            element(output, "key", "NSStringFormatValueTypeKey");
            element(
                output,
                "string",
                entries[0]
                    .metadata
                    .get("apple.value_type")
                    .map(String::as_str)
                    .unwrap_or("d"),
            );
            for entry in entries {
                element(output, "key", entry.plural_category.as_deref().unwrap());
                element(output, "string", &entry.value);
            }
            output.push_str("</dict>\n");
        }
        output.push_str("</dict>\n");
    }
    output.push_str("</dict></plist>\n");
    Ok(())
}

fn xliff_metadata(entry: &TranslationCatalogEntry, old: bool) -> bool {
    entry
        .metadata
        .get("xliff.version")
        .is_some_and(|version| version == if old { "1" } else { "2" })
}
fn xliff_namespace(old: bool) -> &'static str {
    if old {
        "urn:oasis:names:tc:xliff:document:1.2"
    } else {
        "urn:oasis:names:tc:xliff:document:2.0"
    }
}
fn retained_list(
    entry: &TranslationCatalogEntry,
    key: &str,
    old: bool,
) -> Result<Vec<String>, String> {
    if !xliff_metadata(entry, old) {
        return Ok(Vec::new());
    }
    entry
        .metadata
        .get(key)
        .map(|value| {
            serde_json::from_value(crate::json::parse_checked(value)?)
                .map_err(|e| format!("Invalid {key} metadata: {e}"))
        })
        .transpose()
        .map(|value| value.unwrap_or_default())
}
fn write_xliff_field(
    output: &mut String,
    entry: &TranslationCatalogEntry,
    field: &str,
    value: &str,
    old: bool,
) -> Result<(), String> {
    if field == "target"
        && xliff_metadata(entry, old)
        && entry
            .metadata
            .get("xliff.untranslated")
            .is_some_and(|value| value == "true")
        && entry.value == entry.source.as_deref().unwrap_or("")
    {
        return Ok(());
    }
    let state = (old && field == "target").then_some(("state", entry.state.as_deref()));
    if let Some(retained) = entry
        .metadata
        .get(&format!("xliff.{field}_xml"))
        .filter(|_| xliff_metadata(entry, old))
    {
        let document = Document::parse(retained)
            .map_err(|e| format!("Invalid retained XLIFF {field}: {e}"))?;
        let node = document.root_element();
        if node.tag_name().name() != field {
            return Err(format!(
                "Retained XLIFF {field} has the wrong root element."
            ));
        }
        let changed = text(node) != value;
        if changed && node.children().any(|node| node.is_element()) {
            return Err(format!("XLIFF {field} for '{}' contains inline codes. Update its retained XML together with the text, or remove xliff.{field}_xml to explicitly flatten it.", entry.key));
        }
        write_node(
            output,
            node,
            changed.then_some((node.id(), value)),
            state,
            Some(xliff_namespace(old)),
        );
        output.push('\n');
    } else {
        write!(output, "<{field}").unwrap();
        if let Some((name, Some(value))) = state {
            write!(output, " {name}=\"{}\"", escape(value)).unwrap();
        }
        writeln!(output, ">{}</{field}>", escape(value)).unwrap();
    }
    Ok(())
}
fn write_xliff_notes(
    output: &mut String,
    entry: &TranslationCatalogEntry,
    old: bool,
    options: &TranslationFormatOptions,
) -> Result<(), String> {
    let mut notes = String::new();
    let mut comment_written = false;
    let mut context_written = false;
    for retained in retained_list(entry, "xliff.notes", old)? {
        let document =
            Document::parse(&retained).map_err(|e| format!("Invalid retained XLIFF note: {e}"))?;
        let node = document.root_element();
        let context = node.tag_name().name() == "context-group"
            || node.attribute("category") == Some("context");
        let value = if context {
            entry.context.as_deref()
        } else {
            options
                .include_comments
                .then_some(entry.comment.as_deref())
                .flatten()
        };
        let Some(value) = value else {
            continue;
        };
        let written = if context {
            &mut context_written
        } else {
            &mut comment_written
        };
        let target = if context && node.tag_name().name() == "context-group" {
            node.descendants()
                .find(|node| node.is_element() && node.tag_name().name() == "context")
                .unwrap_or(node)
        } else {
            node
        };
        let changed = !*written && text(target) != value;
        if changed && target.children().any(|node| node.is_element()) {
            return Err(format!("A retained XLIFF note for '{}' contains inline content; update or remove xliff.notes before replacing it.", entry.key));
        }
        write_node(
            &mut notes,
            node,
            changed.then_some((target.id(), value)),
            None,
            Some(xliff_namespace(old)),
        );
        notes.push('\n');
        *written = true;
    }
    if !comment_written && options.include_comments {
        if let Some(comment) = &entry.comment {
            element(&mut notes, "note", comment);
        }
    }
    if !context_written {
        if let Some(context) = &entry.context {
            if old {
                writeln!(notes, "<context-group><context context-type=\"x-protranslate\">{}</context></context-group>", escape(context)).unwrap();
            } else {
                writeln!(
                    notes,
                    "<note category=\"context\">{}</note>",
                    escape(context)
                )
                .unwrap();
            }
        }
    }
    if !old && !notes.is_empty() {
        output.push_str("<notes>\n");
    }
    output.push_str(&notes);
    if !old && !notes.is_empty() {
        output.push_str("</notes>\n");
    }
    Ok(())
}
fn write_xliff_extra(output: &mut String, extra: &str, old: bool) -> Result<(), String> {
    let document =
        Document::parse(extra).map_err(|e| format!("Invalid retained XLIFF unit metadata: {e}"))?;
    write_node(
        output,
        document.root_element(),
        None,
        None,
        Some(xliff_namespace(old)),
    );
    output.push('\n');
    Ok(())
}

fn write_xliff_attributes(
    output: &mut String,
    entry: &TranslationCatalogEntry,
    metadata_key: &str,
    excluded: &[&str],
    old: bool,
) -> Result<(), String> {
    if !xliff_metadata(entry, old) {
        return Ok(());
    }
    if let Some(value) = entry.metadata.get(metadata_key) {
        let document = Document::parse(value)
            .map_err(|e| format!("Invalid retained XLIFF attributes: {e}"))?;
        let node = document.root_element();
        for namespace in node.namespaces() {
            if let Some(prefix) = namespace.name().filter(|prefix| *prefix != "xml") {
                let uri = if namespace
                    .uri()
                    .starts_with("urn:oasis:names:tc:xliff:document:")
                {
                    xliff_namespace(old)
                } else {
                    namespace.uri()
                };
                write!(output, " xmlns:{prefix}=\"{}\"", escape(uri)).unwrap();
            }
        }
        for attribute in node.attributes() {
            if attribute.namespace().is_none() && excluded.contains(&attribute.name()) {
                continue;
            }
            let name = qualified_name(node, attribute.namespace(), attribute.name(), true);
            write!(output, " {name}=\"{}\"", escape(attribute.value())).unwrap();
        }
    }
    Ok(())
}

fn export_xliff(
    catalog: &TranslationCatalog,
    options: &TranslationFormatOptions,
    old: bool,
    output: &mut String,
) -> Result<(), String> {
    let source = options
        .source_culture
        .as_deref()
        .or(catalog.source_culture.as_deref())
        .unwrap_or("en");
    let cultures = if let Some(culture) = &options.culture {
        vec![culture.clone()]
    } else if old {
        let cultures = catalog.cultures();
        if cultures.is_empty() {
            vec!["und".into()]
        } else {
            cultures
        }
    } else {
        vec![catalog
            .cultures()
            .into_iter()
            .next()
            .unwrap_or("und".into())]
    };
    if old {
        output
            .push_str("<xliff xmlns=\"urn:oasis:names:tc:xliff:document:1.2\" version=\"1.2\">\n");
    } else {
        writeln!(output, "<xliff xmlns=\"urn:oasis:names:tc:xliff:document:2.0\" version=\"2.1\" srcLang=\"{}\" trgLang=\"{}\">", escape(source), escape(&cultures[0])).unwrap();
    }
    for (file_index, culture) in cultures.iter().enumerate() {
        if old {
            writeln!(output, "<file source-language=\"{}\" target-language=\"{}\" datatype=\"plaintext\" original=\"{}\"><body>", escape(source), escape(culture), escape(options.name.as_deref().or(catalog.name.as_deref()).unwrap_or("ProTranslate"))).unwrap();
        } else {
            writeln!(
                output,
                "<file id=\"f{}\" original=\"{}\">",
                file_index + 1,
                escape(
                    options
                        .name
                        .as_deref()
                        .or(catalog.name.as_deref())
                        .unwrap_or("ProTranslate")
                )
            )
            .unwrap();
        }
        let mut units = BTreeMap::<&str, Vec<&TranslationCatalogEntry>>::new();
        for entry in select_entries(catalog, Some(culture)) {
            let key = if !old && xliff_metadata(entry, false) {
                entry
                    .metadata
                    .get("xliff.segment")
                    .and_then(|id| entry.key.strip_suffix(&format!("#{id}")))
                    .unwrap_or(&entry.key)
            } else {
                &entry.key
            };
            units.entry(key).or_default().push(entry);
        }
        let mut used_ids = BTreeSet::new();
        for group in units.values() {
            if let Some(id) = group[0]
                .metadata
                .get("xliff.unit_id")
                .filter(|_| xliff_metadata(group[0], old))
            {
                if id.is_empty() || !used_ids.insert(id.clone()) {
                    return Err("XLIFF file contains empty or duplicate unit identifiers.".into());
                }
            }
        }
        let mut generated_id = 1;
        for (key, mut entries) in units {
            let first = entries[0];
            if entries.iter().any(|entry| {
                entry.comment != first.comment
                    || entry.context != first.context
                    || [
                        "xliff.unit_id",
                        "xliff.unit_attributes",
                        "xliff.notes",
                        "xliff.extras",
                        "xliff.extra_positions",
                    ]
                    .iter()
                    .any(|key| entry.metadata.get(*key) != first.metadata.get(*key))
            }) {
                return Err(format!(
                    "XLIFF unit '{key}' has conflicting shared notes or metadata."
                ));
            }
            let mut orders = BTreeSet::new();
            for entry in &entries {
                if let Some(order) = entry.metadata.get("xliff.segment_order") {
                    let order = order
                        .parse::<usize>()
                        .map_err(|_| "Invalid retained XLIFF segment order.")?;
                    if !orders.insert(order) {
                        return Err(format!(
                            "XLIFF unit '{key}' has duplicate segment positions."
                        ));
                    }
                }
            }
            entries.sort_by_key(|entry| {
                entry
                    .metadata
                    .get("xliff.segment_order")
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(0)
            });
            let id = if let Some(id) = first
                .metadata
                .get("xliff.unit_id")
                .filter(|_| xliff_metadata(first, old))
            {
                id.clone()
            } else {
                loop {
                    let id = format!("u{generated_id}");
                    generated_id += 1;
                    if used_ids.insert(id.clone()) {
                        break id;
                    }
                }
            };
            let unit_tag = if old { "trans-unit" } else { "unit" };
            let key_attribute = if old { "resname" } else { "name" };
            write!(
                output,
                "<{unit_tag} id=\"{}\" {key_attribute}=\"{}\"",
                escape(&id),
                escape(key)
            )
            .unwrap();
            write_xliff_attributes(
                output,
                first,
                "xliff.unit_attributes",
                &["id", "name", "resname"],
                old,
            )?;
            output.push_str(">\n");
            let extras = retained_list(first, "xliff.extras", old)?;
            if old {
                write_xliff_field(
                    output,
                    first,
                    "source",
                    first.source.as_deref().unwrap_or(key),
                    true,
                )?;
                for extra in &extras {
                    let document = Document::parse(extra).map_err(|e| e.to_string())?;
                    if document.root_element().tag_name().name() == "seg-source" {
                        if first
                            .metadata
                            .get("xliff.source_xml")
                            .is_some_and(|source| {
                                Document::parse(source).ok().is_some_and(|source| {
                                    text(source.root_element())
                                        != first.source.as_deref().unwrap_or(key)
                                })
                            })
                        {
                            return Err(format!("XLIFF unit '{key}' source changed while retained segmentation still describes the previous source."));
                        }
                        write_xliff_extra(output, extra, true)?;
                    }
                }
                write_xliff_field(output, first, "target", &first.value, true)?;
                write_xliff_notes(output, first, true, options)?;
                for extra in &extras {
                    let document = Document::parse(extra).map_err(|e| e.to_string())?;
                    if document.root_element().tag_name().name() != "seg-source" {
                        write_xliff_extra(output, extra, true)?;
                    }
                }
            } else {
                write_xliff_notes(output, first, false, options)?;
                let positions: Vec<usize> = first
                    .metadata
                    .get("xliff.extra_positions")
                    .map(|value| {
                        serde_json::from_value(crate::json::parse_checked(value)?)
                            .map_err(|e| format!("Invalid XLIFF extra positions: {e}"))
                    })
                    .transpose()?
                    .unwrap_or_else(|| vec![0; extras.len()]);
                if positions.len() != extras.len()
                    || positions.windows(2).any(|pair| pair[0] > pair[1])
                {
                    return Err("Invalid XLIFF unit metadata ordering.".into());
                }
                let mut extra_index = 0;
                let mut segment_ids = BTreeSet::new();
                for entry in entries {
                    let order = entry
                        .metadata
                        .get("xliff.segment_order")
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(0);
                    while extra_index < extras.len() && positions[extra_index] <= order {
                        write_xliff_extra(output, &extras[extra_index], false)?;
                        extra_index += 1;
                    }
                    output.push_str("<segment");
                    write_xliff_attributes(
                        output,
                        entry,
                        "xliff.segment_attributes",
                        &["id", "state"],
                        false,
                    )?;
                    if let Some(id) = entry
                        .metadata
                        .get("xliff.segment")
                        .or_else(|| entry.metadata.get("xliff.segment_id"))
                    {
                        if id.is_empty() || !segment_ids.insert(id) {
                            return Err(format!(
                                "XLIFF unit '{key}' has empty or duplicate segment identifiers."
                            ));
                        }
                        write!(output, " id=\"{}\"", escape(id)).unwrap();
                    }
                    if let Some(state) = &entry.state {
                        if !matches!(
                            state.as_str(),
                            "initial" | "translated" | "reviewed" | "final"
                        ) {
                            return Err(format!(
                                "State '{state}' is not representable as an XLIFF 2 segment state."
                            ));
                        }
                        write!(output, " state=\"{}\"", escape(state)).unwrap();
                    }
                    output.push_str(">\n");
                    write_xliff_field(
                        output,
                        entry,
                        "source",
                        entry.source.as_deref().unwrap_or(&entry.key),
                        false,
                    )?;
                    write_xliff_field(output, entry, "target", &entry.value, false)?;
                    output.push_str("</segment>\n");
                }
                while extra_index < extras.len() {
                    write_xliff_extra(output, &extras[extra_index], false)?;
                    extra_index += 1;
                }
            }
            writeln!(output, "</{unit_tag}>").unwrap();
        }
        if old {
            output.push_str("</body>");
        }
        output.push_str("</file>\n");
    }
    output.push_str("</xliff>\n");
    Ok(())
}
