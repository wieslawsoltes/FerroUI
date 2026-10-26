use crate::{model::select_entries, *};
use roxmltree::{Document, Node, ParsingOptions};
use std::collections::BTreeMap;
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
        _ => import_xliff(root, result),
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
        let format = fields.get("NSStringLocalizedFormatKey").map(|n| text(*n));
        let variables: Vec<_> = fields
            .iter()
            .filter(|(_, v)| v.has_tag_name("dict"))
            .collect();
        for (variable, dict) in &variables {
            let plural = plist_dict(**dict)?;
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

fn import_xliff(root: Node<'_, '_>, result: &mut TranslationFormatResult) -> Result<(), String> {
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
        };
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
            let segments: Vec<_> = if old {
                vec![unit]
            } else {
                unit.children()
                    .filter(|n| n.is_element() && n.tag_name().name() == "segment")
                    .collect()
            };
            let multiple = segments.len() > 1;
            if multiple {
                result.diagnostics.push(TranslationFormatDiagnostic::new(Severity::Warning,"PTF183",format!("Unit '{key}' has multiple segments; normalized keys include each segment identifier.")));
            }
            for (index, segment) in segments.into_iter().enumerate() {
                let source = child(segment, "source");
                let target = child(segment, "target");
                for node in [source, target].into_iter().flatten() {
                    inline_warning(node, key, result);
                }
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
                if multiple {
                    entry.metadata.insert("xliff.unit".into(), key.into());
                    entry.metadata.insert(
                        "xliff.segment".into(),
                        segment
                            .attribute("id")
                            .map(str::to_owned)
                            .unwrap_or_else(|| (index + 1).to_string()),
                    );
                }
            }
        }
    }
    if root.descendants().any(|n| {
        n.is_element()
            && matches!(
                n.tag_name().name(),
                "skl" | "skeleton" | "phase-group" | "ignorable"
            )
    }) {
        result.diagnostics.push(TranslationFormatDiagnostic::new(Severity::Warning,"PTF184","XLIFF skeleton, phase, or ignorable content is not represented in the normalized catalog."));
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
        TranslationFileFormat::Xliff12 => export_xliff(catalog, options, true, &mut output),
        TranslationFileFormat::Xliff20 => export_xliff(catalog, options, false, &mut output),
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
    for entry in entries {
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
        groups
            .entry(key)
            .or_default()
            .entry(variable)
            .or_default()
            .push(entry);
    }
    output.push_str("<plist version=\"1.0\"><dict>\n");
    for (key, variables) in groups {
        element(output, "key", key);
        output.push_str("<dict>\n");
        element(output, "key", "NSStringLocalizedFormatKey");
        let first = variables
            .values()
            .flatten()
            .next()
            .expect("group is not empty");
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

fn export_xliff(
    catalog: &TranslationCatalog,
    options: &TranslationFormatOptions,
    old: bool,
    output: &mut String,
) {
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
        writeln!(output,"<xliff xmlns=\"urn:oasis:names:tc:xliff:document:2.0\" version=\"2.1\" srcLang=\"{}\" trgLang=\"{}\">",escape(source),escape(&cultures[0])).unwrap();
    }
    for (file_index, culture) in cultures.iter().enumerate() {
        if old {
            writeln!(output,"<file source-language=\"{}\" target-language=\"{}\" datatype=\"plaintext\" original=\"{}\"><body>",escape(source),escape(culture),escape(catalog.name.as_deref().unwrap_or("ProTranslate"))).unwrap();
        } else {
            writeln!(
                output,
                "<file id=\"f{}\" original=\"{}\">",
                file_index + 1,
                escape(catalog.name.as_deref().unwrap_or("ProTranslate"))
            )
            .unwrap();
        }
        for (unit_index, entry) in select_entries(catalog, Some(culture)).iter().enumerate() {
            let unit_tag = if old { "trans-unit" } else { "unit" };
            let key_attr = if old { "resname" } else { "name" };
            writeln!(
                output,
                "<{unit_tag} id=\"u{}\" {key_attr}=\"{}\">",
                unit_index + 1,
                escape(&entry.key)
            )
            .unwrap();
            let comment = if options.include_comments {
                entry.comment.as_deref()
            } else {
                None
            };
            if !old && (comment.is_some() || entry.context.is_some()) {
                output.push_str("<notes>\n");
            }
            if let Some(comment) = comment {
                element(output, "note", comment);
            }
            if let Some(context) = &entry.context {
                if old {
                    output.push_str("<context-group><context context-type=\"x-protranslate\">");
                    output.push_str(&escape(context));
                    output.push_str("</context></context-group>\n");
                } else {
                    output.push_str("<note category=\"context\">");
                    output.push_str(&escape(context));
                    output.push_str("</note>\n");
                }
            }
            if !old && (comment.is_some() || entry.context.is_some()) {
                output.push_str("</notes>\n");
            }
            if !old {
                write!(output, "<segment").unwrap();
                if let Some(state) = &entry.state {
                    write!(output, " state=\"{}\"", escape(state)).unwrap();
                }
                output.push_str(">\n");
            }
            element(
                output,
                "source",
                entry.source.as_deref().unwrap_or(&entry.key),
            );
            output.push_str("<target");
            if old {
                if let Some(state) = &entry.state {
                    write!(output, " state=\"{}\"", escape(state)).unwrap();
                }
            }
            writeln!(output, ">{}</target>", escape(&entry.value)).unwrap();
            if !old {
                output.push_str("</segment>\n");
            }
            writeln!(output, "</{unit_tag}>").unwrap();
        }
        if old {
            output.push_str("</body>");
        }
        output.push_str("</file>\n");
    }
    output.push_str("</xliff>\n");
}
