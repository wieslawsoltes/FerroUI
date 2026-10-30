# ProTranslate catalog formats

`ferroui-protranslate-formats` ports the optional import/export layer of
[ProTranslate](https://github.com/wieslawsoltes/ProTranslate) to Rust. It has no UI
or runtime translation-service dependency. Applications can convert catalogs at
build time and use the generated provider for runtime lookup.

```rust
use ferroui_protranslate_formats::{
    TranslationCatalogConverter, TranslationFileFormat, TranslationFormatOptions,
};

let options = TranslationFormatOptions {
    culture: Some("pl-PL".into()),
    ..Default::default()
};
let result = TranslationCatalogConverter::import(
    r#"{"Shell":{"Title":"Tłumaczenia"}}"#,
    TranslationFileFormat::I18NextJson,
    &options,
    Some("Strings.pl-PL.i18next.json"),
);
assert!(!result.has_errors());
assert_eq!(result.catalog.entries[0].key, "Shell.Title");

let exported = TranslationCatalogConverter::export(
    &result.catalog,
    TranslationFileFormat::Xliff20,
    &options,
);
assert!(!exported.has_errors());
// Show/export diagnostics before accepting a lossy conversion.
println!("{}", exported.content);
```

## Model and behavior

`TranslationCatalog` contains ordered `TranslationCatalogEntry` values. Each
record contains the key, optional culture, localized value, source, comment,
context, workflow state, plural category, format, and an ordered metadata map.
The catalog has its own ordered metadata map for document-level information.
Catalogs, entries, options, formats, diagnostics and conversion results implement
Serde serialization with camelCase names. Catalogs, entries, options and results
default omitted fields, so complete editing sessions retain normalized data.
`to_dictionary` selects an exact culture case-insensitively; repeated keys use
the last record and are reported by `validate`. Export selection overlays exact
culture records on neutral fallbacks independently of source order.

Import returns a `TranslationFormatResult`. A malformed document produces an
error diagnostic and an empty record collection, so a partial parse cannot be
mistaken for a successful import. Export returns a `TranslationExportResult`;
errors leave `content` empty, and representational losses produce warnings.
Callers decide whether warnings require review or prevent saving.

Output ordering is deterministic. CSV/TSV and Apple string catalogs preserve
multiple cultures; XLIFF 1.2 exports one file element per culture. Other formats
have one language per file. Pass `culture` explicitly for those formats; omitting
it for a multi-language catalog produces a diagnostic.

## Format mapping

| Format | Implemented behavior | Explicit boundaries |
| --- | --- | --- |
| ProTranslate JSON / i18next JSON | Nested object import, flat-key deterministic export, Unicode strings, trailing commas compatible with the source parser | Non-string values are diagnosed. Duplicate JSON object members fail import. Interpolation expressions remain text. |
| Flutter ARB | `@@locale`, values, descriptions, placeholders, global metadata and metadata for untranslated messages | ICU message expressions remain text. Translation keys using the reserved `@` prefix fail export. Clearing an entry comment removes its description. |
| RESX | String resources, comments, filename culture inference, XML escaping | Binary, file-reference and typed non-string resources are skipped with diagnostics. |
| Android resources | Strings, plural quantities, string arrays, quoted whitespace regions, escape layer, array order, `translatable` and `formatted` on all resource groups, Android locale-folder inference | Styled/inline XML is flattened with warnings. Conflicting group flags fail export. Plural selection is not performed by the converter. |
| Apple `.strings` | Quoted Unicode strings, C/octal/UTF-16 escapes, comments, locale-folder inference | Missing delimiters and malformed escapes fail import. |
| Apple `.stringsdict` | Plural variables, categories, localized format and value type, device and width variants, standard plist doctype | Ordinary singular records need `.strings`; device/width variants retain their native rule metadata. Conflicting shared formats, value types or plural categories fail export. Unknown rules are diagnosed. |
| Apple `.xcstrings` | Multiple languages, comments, states, nested device/plural variations, pluralized substitutions, root/record/string-unit metadata, untranslated records | Runtime device selection and substitution evaluation belong to platform consumers. Shared comments and structural metadata must agree; ambiguous manual singular/plural mixtures fail export. |
| gettext PO/POT | Complete headers, C byte escapes, multiline literals, contexts, source text, single-form and multi-form plurals, fuzzy flags, previous source messages, comments, references and obsolete records | Plural formulas are retained, not evaluated; the declared form count is validated. Indices 0/1 retain the source's `one`/`other` naming; additional indices use `indexN` and preserve `po.index`. Empty targets preserve untranslated status. Clearing `state = "fuzzy"` removes the fuzzy flag. Conflicting shared plural metadata fails export. |
| XLIFF 1.2 | Source/target languages, unit identity/attributes, source/target text and inline XML, state, all notes/context, segmentation and unit metadata | Normalized values contain plain text. Changing text with retained inline codes requires updating its XML or explicitly removing that metadata. File skeleton/phase data is diagnosed. |
| XLIFF 2.0 / 2.1 | Unit/segment reconstruction, segment order and attributes, inline XML, originalData/ignorable content, all notes/context, untranslated-target status; export writes version 2.1, matching upstream `Xliff20` | Multiple segments receive distinct `unit#segment` keys. Native unit metadata is restored when exporting the same XLIFF family; conversions across families report metadata loss. XLIFF 2 workflow states must be initial/translated/reviewed/final. |
| CSV / TSV | Required key/value columns; culture/source/comment/context/state/plural/format; optional JSON metadata and catalogMetadata columns; quoting, embedded CR/LF; custom CSV delimiter | Document name, source culture and metadata are retained, including empty catalogs. Duplicate headers/JSON metadata keys, inconsistent row sizes and malformed quoting fail import. Formula-looking text is retained and diagnosed. |

The format layer never evaluates plural expressions, executes spreadsheet
formulas, resolves external XML resources, or mutates runtime culture state.
XML internal entities use the parser's bounded expansion and recursion checks;
external entities are never resolved, and parsed XML has a bounded node count.
XML export rejects characters that XML 1.0 cannot represent.

## Normalized metadata

- ARB: entry `arb` contains the per-message JSON object. Catalog `arb.globals`
  and `arb.orphan_metadata` retain document fields and unattached message metadata.
- Android: `android.array`, `android.index`, `android.translatable`,
  `android.formatted` preserve resource structure and flags.
- Apple plurals: `apple.key`, `apple.variable`, `apple.localized_format`,
  `apple.value_type` preserve dictionary reconstruction. `apple.rule` and
  `apple.variant` identify width/device variants.
- Apple string catalogs: `xcstrings.path`, `xcstrings.localization`,
  `xcstrings.record`, and `xcstrings.unit` retain variation and substitution
  structure. Catalog `xcstrings.root` and `xcstrings.untranslated` preserve
  document fields and records without translated string units. Device variants
  normalize to `key.device.variant`; substitutions to
  `key.substitutions.variable.category`. Ordinary plurals remain `key.category`.
- gettext: entry `po.index`, `po.msgid_plural`, `po.plural_forms`, `po.flags`,
  `po.flags_prefix`, `po.previous`, `po.references`, `po.translator_comment` and
  `po.untranslated` preserve message semantics. Catalog `po.header_entry` and
  `po.obsolete` preserve header information and obsolete records.
- XLIFF: `xliff.version`, `xliff.unit_id`, `xliff.unit_attributes`,
  `xliff.segment`, `xliff.segment_id`, `xliff.segment_order`,
  `xliff.segment_attributes`, `xliff.source_xml`, `xliff.target_xml`,
  `xliff.notes`, `xliff.extras`, `xliff.extra_positions` and
  `xliff.untranslated` retain unit structure. `xliff.unit` records the original
  group name for multi-segment records. XML fragments retain namespace bindings;
  they are parsed and validated before export.

CSV/TSV metadata cells round-trip arbitrary string metadata. Other exporters
report metadata they cannot represent. Culture name normalization in this crate
only standardizes common tag spelling; locale validation and localization data
belong to `ferroui-protranslate-core`.

PO octal and hexadecimal escapes are decoded as bytes across continuation
literals, then validated as UTF-8. As specified by the
[GNU PO grammar](https://www.gnu.org/software/gettext/manual/html_node/More-Details.html),
universal `\u`/`\U` escapes are rejected; literal Unicode is supported. The exporter
writes UTF-8 headers. Android escaping and quoted whitespace regions follow the
[Android string resource rules](https://developer.android.com/guide/topics/resources/string-resource).

## Validation

Run `cargo test -p ferroui-protranslate-formats` for import/export coverage across
all thirteen enum variants, malformed document handling, Unicode and escaping,
plural/context/metadata preservation, deterministic ordering and loss diagnostics.

Source baseline: `b088669a22c8979d54dd81098991f8d16f880156`.
