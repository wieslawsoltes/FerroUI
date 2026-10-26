//! Loss-aware translation catalog conversion, independent of any UI framework.
//!
//! Ported from the MIT-licensed ProTranslate.Formats library. Runtime translation
//! lookup belongs to `ferroui-protranslate-core`; this crate is optional tooling.
//! Parsers report malformed input as diagnostics rather than panicking. Exports
//! are deterministic and report metadata or cultures that cannot be represented.

mod json;
mod model;
mod text;
mod xcstrings;
mod xml;
pub use model::*;

/// Stateless conversion entry points matching the source library's organization.
pub struct TranslationCatalogConverter;

impl TranslationCatalogConverter {
    pub fn detect_format(path: &str) -> TranslationFileFormat {
        detect_format(path)
    }
    pub fn import(
        content: &str,
        format: TranslationFileFormat,
        options: &TranslationFormatOptions,
        path: Option<&str>,
    ) -> TranslationFormatResult {
        import(content, format, options, path)
    }
    pub fn export(
        catalog: &TranslationCatalog,
        format: TranslationFileFormat,
        options: &TranslationFormatOptions,
    ) -> TranslationExportResult {
        export(catalog, format, options)
    }
}

/// Detect by file name. XLIFF version is read from its root during import.
pub fn detect_format(path: &str) -> TranslationFileFormat {
    use TranslationFileFormat::*;
    let path = path.to_ascii_lowercase();
    for (suffix, format) in [
        (".xcstrings", AppleXcstrings),
        (".stringsdict", AppleStringsdict),
        (".strings", AppleStrings),
        (".arb", FlutterArb),
        (".resx", Resx),
        (".po", GettextPo),
        (".pot", GettextPo),
        (".xlf", Xliff12),
        (".xliff", Xliff12),
        ("strings.xml", AndroidResources),
        (".tsv", Tsv),
        (".csv", Csv),
        (".i18next.json", I18NextJson),
        (".i18n.json", I18NextJson),
    ] {
        if path.ends_with(suffix) {
            return format;
        }
    }
    ProTranslateJson
}

pub fn import(
    content: &str,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
    path: Option<&str>,
) -> TranslationFormatResult {
    let mut result = TranslationFormatResult {
        catalog: TranslationCatalog {
            name: options.name.clone().or_else(|| {
                path.and_then(|p| std::path::Path::new(p).file_stem())
                    .map(|p| p.to_string_lossy().into_owned())
            }),
            source_culture: options.source_culture.clone(),
            entries: Vec::new(),
            metadata: Default::default(),
        },
        diagnostics: Vec::new(),
    };
    use TranslationFileFormat::*;
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let parsed = match format {
        ProTranslateJson | I18NextJson | FlutterArb | AppleXcstrings => {
            json::import(content, format, options, &mut result)
        }
        Resx | AndroidResources | AppleStringsdict | Xliff12 | Xliff20 => {
            xml::import(content, format, options, path, &mut result)
        }
        AppleStrings | GettextPo | Csv | Tsv => {
            text::import(content, format, options, path, &mut result)
        }
    };
    if let Err(message) = parsed {
        result.catalog.entries.clear();
        result.diagnostics.push(TranslationFormatDiagnostic::new(
            Severity::Error,
            "PTF001",
            message,
        ));
    }
    result.diagnostics.extend(result.catalog.validate());
    result
}

pub fn export(
    catalog: &TranslationCatalog,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
) -> TranslationExportResult {
    use TranslationFileFormat::*;
    let mut result = TranslationExportResult {
        content: String::new(),
        diagnostics: catalog.validate(),
    };
    if result.has_errors() {
        return result;
    }
    diagnose_losses(catalog, format, options, &mut result.diagnostics);
    let output = match format {
        ProTranslateJson | I18NextJson | FlutterArb | AppleXcstrings => {
            json::export(catalog, format, options)
        }
        Resx | AndroidResources | AppleStringsdict | Xliff12 | Xliff20 => {
            xml::export(catalog, format, options)
        }
        AppleStrings | GettextPo | Csv | Tsv => text::export(catalog, format, options),
    };
    match output {
        Ok(content) => result.content = content,
        Err(message) => result.diagnostics.push(TranslationFormatDiagnostic::new(
            Severity::Error,
            "PTF201",
            message,
        )),
    }
    result
}

fn diagnose_losses(
    catalog: &TranslationCatalog,
    format: TranslationFileFormat,
    options: &TranslationFormatOptions,
    diagnostics: &mut Vec<TranslationFormatDiagnostic>,
) {
    use TranslationFileFormat::*;
    let multi = matches!(format, AppleXcstrings | Csv | Tsv | Xliff12);
    if !multi && options.culture.is_none() && catalog.cultures().len() > 1 {
        diagnostics.push(TranslationFormatDiagnostic::new(Severity::Warning,"PTF200","The target has one language per file; pass an explicit culture to avoid ambiguous selection."));
    }
    let lost_document_metadata: Vec<_> = catalog
        .metadata
        .keys()
        .filter(|key| match format {
            Csv | Tsv => false,
            FlutterArb => !matches!(key.as_str(), "arb.globals" | "arb.orphan_metadata"),
            GettextPo => !matches!(key.as_str(), "po.header_entry" | "po.obsolete"),
            AppleXcstrings => !matches!(key.as_str(), "xcstrings.root" | "xcstrings.untranslated"),
            _ => true,
        })
        .map(String::as_str)
        .collect();
    if !lost_document_metadata.is_empty() {
        diagnostics.push(TranslationFormatDiagnostic::new(
            Severity::Warning,
            "PTF204",
            format!(
                "Target cannot preserve document metadata: {}.",
                lost_document_metadata.join(", ")
            ),
        ));
    }
    let selected = options.culture.clone().or_else(|| {
        (!multi)
            .then(|| catalog.cultures().into_iter().next())
            .flatten()
    });
    let entries: Vec<_> = if multi {
        catalog
            .entries
            .iter()
            .filter(|entry| {
                selected.as_deref().is_none_or(|culture| {
                    entry
                        .culture
                        .as_deref()
                        .is_none_or(|tag| tag.eq_ignore_ascii_case(culture))
                })
            })
            .collect()
    } else {
        model::select_entries(catalog, selected.as_deref())
    };
    for entry in entries {
        let mut losses = Vec::new();
        if entry.comment.is_some()
            && (!options.include_comments
                || matches!(
                    format,
                    ProTranslateJson | I18NextJson | AndroidResources | AppleStringsdict
                ))
        {
            losses.push("comment");
        }
        if entry.source.is_some() && !matches!(format, GettextPo | Xliff12 | Xliff20 | Csv | Tsv) {
            losses.push("source");
        }
        if entry.context.is_some() && !matches!(format, GettextPo | Csv | Tsv | Xliff12 | Xliff20) {
            losses.push("context");
        }
        if entry.state.is_some()
            && (!matches!(
                format,
                GettextPo | AppleXcstrings | Csv | Tsv | Xliff12 | Xliff20
            ) || (format == GettextPo && entry.state.as_deref() != Some("fuzzy")))
        {
            losses.push("state");
        }
        if entry.plural_category.is_some()
            && matches!(
                format,
                ProTranslateJson
                    | I18NextJson
                    | FlutterArb
                    | Resx
                    | AppleStrings
                    | Xliff12
                    | Xliff20
            )
        {
            losses.push("plural category (value retained under its flattened key)");
        }
        if entry.format.is_some() && !matches!(format, Csv | Tsv) {
            losses.push("format");
        }
        let unknown_metadata = entry.metadata.keys().any(|key| match format {
            Csv | Tsv => false,
            FlutterArb => key != "arb",
            AppleXcstrings => !matches!(
                key.as_str(),
                "xcstrings.path" | "xcstrings.localization" | "xcstrings.record" | "xcstrings.unit"
            ),
            Xliff12 | Xliff20 => {
                let compatible = entry
                    .metadata
                    .get("xliff.version")
                    .is_some_and(|version| version == if format == Xliff12 { "1" } else { "2" });
                !compatible
                    || !matches!(
                        key.as_str(),
                        "xliff.version"
                            | "xliff.unit"
                            | "xliff.unit_id"
                            | "xliff.unit_attributes"
                            | "xliff.segment"
                            | "xliff.segment_id"
                            | "xliff.segment_order"
                            | "xliff.segment_attributes"
                            | "xliff.source_xml"
                            | "xliff.target_xml"
                            | "xliff.notes"
                            | "xliff.extras"
                            | "xliff.extra_positions"
                            | "xliff.untranslated"
                    )
            }
            GettextPo => !matches!(
                key.as_str(),
                "po.index"
                    | "po.msgid_plural"
                    | "po.plural_forms"
                    | "po.flags"
                    | "po.flags_prefix"
                    | "po.previous"
                    | "po.references"
                    | "po.translator_comment"
                    | "po.untranslated"
            ),
            AppleStringsdict => !matches!(
                key.as_str(),
                "apple.key"
                    | "apple.variable"
                    | "apple.localized_format"
                    | "apple.value_type"
                    | "apple.rule"
                    | "apple.variant"
            ),
            AndroidResources => !matches!(
                key.as_str(),
                "android.array" | "android.index" | "android.translatable" | "android.formatted"
            ),
            _ => true,
        });
        if unknown_metadata {
            losses.push("metadata");
        }
        if format == AppleXcstrings && entry.culture.is_none() {
            diagnostics.push(TranslationFormatDiagnostic::entry(
                Severity::Warning,
                "PTF203",
                "A .xcstrings localization requires a culture; this neutral record is omitted.",
                entry,
            ));
        }
        if matches!(format, Csv | Tsv)
            && [
                entry.key.as_str(),
                entry.value.as_str(),
                entry.comment.as_deref().unwrap_or(""),
            ]
            .iter()
            .any(|value| value.starts_with(['=', '+', '-', '@', '\t', '\r']))
        {
            diagnostics.push(TranslationFormatDiagnostic::entry(Severity::Warning,"PTF192","Formula-looking spreadsheet cell retained as text; review spreadsheet import settings.",entry));
        }
        if !losses.is_empty() {
            diagnostics.push(TranslationFormatDiagnostic::entry(
                Severity::Warning,
                "PTF202",
                format!("Target cannot preserve: {}.", losses.join(", ")),
                entry,
            ));
        }
    }
}

pub(crate) fn infer_culture(path: Option<&str>, suffix: &str) -> Option<String> {
    let file = path?.rsplit(['/', '\\']).next()?;
    let split = file.len().checked_sub(suffix.len())?;
    if !file.get(split..)?.eq_ignore_ascii_case(suffix) {
        return None;
    }
    let stem = &file[..split];
    let candidate = stem.rsplit('.').next()?;
    if candidate == stem
        && !candidate.contains('-')
        && !candidate.contains('_')
        && candidate.len() > 3
    {
        return None;
    }
    if candidate.split(['-', '_']).next()?.len() > 3 {
        return None;
    }
    Some(TranslationCatalog::normalize_culture_name(Some(candidate)))
}

pub(crate) fn infer_apple_culture(path: Option<&str>) -> Option<String> {
    let path = path?;
    path.split(['/', '\\'])
        .find_map(|part| {
            let split = part.len().checked_sub(6)?;
            part.get(split..)?
                .eq_ignore_ascii_case(".lproj")
                .then(|| &part[..split])
        })
        .map(|tag| TranslationCatalog::normalize_culture_name(Some(tag)))
        .or_else(|| infer_culture(Some(path), ".strings"))
        .or_else(|| infer_culture(Some(path), ".stringsdict"))
}

#[cfg(test)]
mod tests;
