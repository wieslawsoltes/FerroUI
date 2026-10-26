use super::*;

fn options(culture: &str) -> TranslationFormatOptions {
    TranslationFormatOptions {
        culture: Some(culture.into()),
        ..Default::default()
    }
}
fn read(content: &str, format: TranslationFileFormat, culture: &str) -> TranslationFormatResult {
    let result = import(content, format, &options(culture), None);
    assert!(!result.has_errors(), "{format:?}: {:?}", result.diagnostics);
    result
}
fn sample() -> TranslationCatalog {
    let mut catalog = TranslationCatalog {
        name: Some("Messages".into()),
        source_culture: Some("en".into()),
        entries: Vec::new(),
    };
    catalog
        .add(
            "Greeting",
            Some("pl-PL"),
            "Cześć, \"świat\"!\nSecond line\r\n\\ <> & ' \t🙂",
        )
        .comment = Some("Translator note".into());
    catalog.add("Empty", Some("pl-PL"), "");
    catalog
}
fn plurals() -> TranslationCatalog {
    let mut catalog = TranslationCatalog::default();
    let one = catalog.add("Count.one", Some("en"), "%d message");
    one.plural_category = Some("one".into());
    one.source = Some("Count".into());
    let other = catalog.add("Count.other", Some("en"), "%d messages");
    other.plural_category = Some("other".into());
    other.source = Some("Counts".into());
    catalog
}

#[test]
fn detects_all_file_names_case_insensitively() {
    use TranslationFileFormat::*;
    for (name, format) in [
        ("Strings.json", ProTranslateJson),
        ("pl.i18next.json", I18NextJson),
        ("pl.i18n.json", I18NextJson),
        ("EN.ARB", FlutterArb),
        ("Strings.pl.resx", Resx),
        ("res/values/strings.xml", AndroidResources),
        ("en.lproj/Localizable.strings", AppleStrings),
        ("Localizable.stringsdict", AppleStringsdict),
        ("Localizable.xcstrings", AppleXcstrings),
        ("pl.po", GettextPo),
        ("en.pot", GettextPo),
        ("translations.xlf", Xliff12),
        ("translations.xliff", Xliff12),
        ("strings.csv", Csv),
        ("strings.tsv", Tsv),
    ] {
        assert_eq!(detect_format(name), format, "{name}");
    }
    assert_eq!(TranslationFileFormat::ALL.len(), 13);
}

#[test]
fn imports_original_common_fixtures() {
    use TranslationFileFormat::*;
    for (content,format,culture) in [
        (r#"{"AppTitle":"ProTranslate"}"#,ProTranslateJson,"en-US"),
        (r#"{"Shell":{"Title":"ProTranslate"}}"#,I18NextJson,"en-US"),
        (r#"{"@@locale":"en-US","AppTitle":"ProTranslate","@AppTitle":{"description":"Application title"}}"#,FlutterArb,"en-US"),
        (r#"<root><data name="AppTitle"><value>ProTranslate</value><comment>Application title</comment></data></root>"#,Resx,"en-US"),
        (r#"<resources><string name="AppTitle">ProTranslate</string></resources>"#,AndroidResources,"en-US"),
        (r#"/* Application title */ "AppTitle" = "ProTranslate";"#,AppleStrings,"en-US"),
        (r#"{"sourceLanguage":"en","strings":{"AppTitle":{"localizations":{"en-US":{"stringUnit":{"state":"translated","value":"ProTranslate"}}}}}}"#,AppleXcstrings,"en-US"),
        ("msgid \"\"\nmsgstr \"\"\n\"Language: en-US\\n\"\n\nmsgid \"AppTitle\"\nmsgstr \"ProTranslate\"",GettextPo,"en-US"),
        (r#"<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2"><file source-language="en" target-language="en-US"><body><trans-unit id="AppTitle"><source>Title</source><target>ProTranslate</target></trans-unit></body></file></xliff>"#,Xliff12,"en-US"),
        (r#"<xliff version="2.1" xmlns="urn:oasis:names:tc:xliff:document:2.0" srcLang="en" trgLang="en-US"><file id="f"><unit id="AppTitle"><segment><source>Title</source><target>ProTranslate</target></segment></unit></file></xliff>"#,Xliff20,"en-US"),
        ("key,culture,value,comment\nAppTitle,en-US,ProTranslate,Application title",Csv,"en-US"),
        ("key\tculture\tvalue\nAppTitle\ten-US\tProTranslate",Tsv,"en-US"),
    ] {
        let result=read(content,format,culture);
        assert_eq!(result.catalog.entries.len(),1,"{format:?}");
        assert_eq!(result.catalog.entries[0].value,"ProTranslate","{format:?}");
        assert_eq!(result.catalog.entries[0].culture.as_deref(),Some(culture),"{format:?}");
    }
}

#[test]
fn round_trips_text_and_empty_values_through_twelve_formats() {
    for format in TranslationFileFormat::ALL
        .into_iter()
        .filter(|f| *f != TranslationFileFormat::AppleStringsdict)
    {
        let catalog = sample();
        let exported = export(&catalog, format, &options("pl-PL"));
        assert!(
            !exported.has_errors(),
            "{format:?}: {:?}",
            exported.diagnostics
        );
        let imported = read(&exported.content, format, "pl-PL");
        let expected = catalog.to_dictionary(Some("pl-PL"));
        let mut actual = imported.catalog.to_dictionary(Some("pl-PL"));
        // gettext explicitly marks untranslated empty values, matching the source behavior.
        if format == TranslationFileFormat::GettextPo {
            assert_eq!(actual.get("Empty").map(String::as_str), Some("Empty"));
            actual.insert("Empty".into(), String::new());
        }
        assert_eq!(actual, expected, "{format:?}: {}", exported.content);
    }
}

#[test]
fn round_trips_plural_categories_in_native_formats() {
    for format in [
        TranslationFileFormat::AndroidResources,
        TranslationFileFormat::AppleStringsdict,
        TranslationFileFormat::AppleXcstrings,
        TranslationFileFormat::GettextPo,
    ] {
        let catalog = plurals();
        let exported = export(&catalog, format, &options("en"));
        assert!(!exported.has_errors(), "{:?}", exported.diagnostics);
        let imported = read(&exported.content, format, "en");
        assert_eq!(imported.catalog.entries.len(), 2, "{format:?}");
        assert!(imported.catalog.entries.iter().any(|e| e.key == "Count.one"
            && e.plural_category.as_deref() == Some("one")
            && e.value == "%d message"));
        assert!(imported
            .catalog
            .entries
            .iter()
            .any(|e| e.key == "Count.other"
                && e.plural_category.as_deref() == Some("other")
                && e.value == "%d messages"));
    }
}

#[test]
fn malformed_input_is_transactional_for_every_format() {
    use TranslationFileFormat::*;
    for format in TranslationFileFormat::ALL {
        let content = match format {
            ProTranslateJson | I18NextJson | FlutterArb | AppleXcstrings => "{\"a\":\"b\",",
            Resx | AndroidResources | AppleStringsdict | Xliff12 | Xliff20 => {
                "<root><unterminated></root>"
            }
            AppleStrings => "\"a\" = \"b\"; \"missing\" = \"value\"",
            GettextPo => "msgid \"a\"\nmsgstr \"b\"\n\nmsgid \"bad\"\nmsgstr[nope] \"value\"",
            Csv => "key,value\na,b\n\"unterminated,value",
            Tsv => "key\tvalue\na\tb\n\"unterminated\tvalue",
        };
        let result = import(content, format, &Default::default(), None);
        assert!(result.has_errors(), "{format:?}: {:?}", result.diagnostics);
        assert!(result.catalog.entries.is_empty(), "{format:?}");
    }
}

#[test]
fn json_preserves_strings_when_accepting_trailing_commas() {
    let result = read(
        r#"{"a":{"b":"text ,} ] \\\"",},"ignored":true,}"#,
        TranslationFileFormat::I18NextJson,
        "en",
    );
    assert_eq!(result.catalog.entries[0].key, "a.b");
    assert_eq!(result.catalog.entries[0].value, "text ,} ] \\\"");
    assert!(result.diagnostics.iter().any(|d| d.code == "PTF101"));
}

#[test]
fn duplicate_json_members_are_rejected_and_flattening_collisions_diagnosed() {
    assert!(import(
        r#"{"a":"one","a":"two"}"#,
        TranslationFileFormat::ProTranslateJson,
        &Default::default(),
        None
    )
    .has_errors());
    let result = read(
        r#"{"a.b":"literal","a":{"b":"nested"}}"#,
        TranslationFileFormat::I18NextJson,
        "en",
    );
    assert!(result.diagnostics.iter().any(|d| d.code == "PTF003"));
}

#[test]
fn arb_preserves_placeholder_metadata() {
    let result = read(
        r#"{"@@locale":"pl","welcome":"Witaj {name}","@welcome":{"description":"Greeting","placeholders":{"name":{"type":"String","example":"Ada"}}}}"#,
        TranslationFileFormat::FlutterArb,
        "en",
    );
    let entry = &result.catalog.entries[0];
    assert_eq!(entry.culture.as_deref(), Some("pl"));
    assert_eq!(entry.comment.as_deref(), Some("Greeting"));
    let exported = export(
        &result.catalog,
        TranslationFileFormat::FlutterArb,
        &options("pl"),
    );
    let json: serde_json::Value = serde_json::from_str(&exported.content).unwrap();
    assert_eq!(json["@welcome"]["placeholders"]["name"]["example"], "Ada");
}

#[test]
fn exact_culture_precedes_neutral_in_any_entry_order() {
    for reverse in [false, true] {
        let mut catalog = TranslationCatalog::default();
        catalog.add("Title", Some("pl-PL"), "Tytuł");
        catalog.add("Title", None, "Neutral");
        catalog.add("Subtitle", None, "Fallback");
        if reverse {
            catalog.entries.reverse();
        }
        let exported = export(
            &catalog,
            TranslationFileFormat::ProTranslateJson,
            &options("PL-pl"),
        );
        let imported = read(
            &exported.content,
            TranslationFileFormat::ProTranslateJson,
            "pl-PL",
        );
        assert_eq!(
            imported.catalog.to_dictionary(Some("pl-PL"))["Title"],
            "Tytuł"
        );
        assert_eq!(
            imported.catalog.to_dictionary(Some("pl-PL"))["Subtitle"],
            "Fallback"
        );
    }
}

#[test]
fn csv_and_tsv_preserve_metadata_multiline_and_delimiters() {
    let mut catalog = sample();
    let entry = &mut catalog.entries[0];
    entry.context = Some("menu\r\nmain".into());
    entry.source = Some("source,\t\"quote\"".into());
    entry
        .metadata
        .insert("vendor.note".into(), "nested,\"data\"".into());
    for format in [TranslationFileFormat::Csv, TranslationFileFormat::Tsv] {
        let exported = export(&catalog, format, &Default::default());
        let mut imported = read(&exported.content, format, "pl-PL").catalog;
        imported.entries.sort_by(|a, b| a.key.cmp(&b.key));
        let mut expected = catalog.entries.clone();
        expected.sort_by(|a, b| a.key.cmp(&b.key));
        assert_eq!(imported.entries, expected);
    }
}

#[test]
fn csv_accepts_custom_delimiter_and_reports_formula_cells() {
    let opts = TranslationFormatOptions {
        delimiter: ';',
        ..options("en")
    };
    let imported = import(
        "key;value\nFormula;=SUM(A1:A2)\n",
        TranslationFileFormat::Csv,
        &opts,
        None,
    );
    assert!(!imported.has_errors());
    assert!(imported.diagnostics.iter().any(|d| d.code == "PTF192"));
    let exported = export(&imported.catalog, TranslationFileFormat::Csv, &opts);
    assert!(exported.content.starts_with("key;culture;value;"));
    assert!(exported.diagnostics.iter().any(|d| d.code == "PTF192"));
}

#[test]
fn delimited_invalid_headers_and_rows_are_diagnostics() {
    for text in [
        "key,key,value\na,b,c",
        "key,value\na",
        "key,value\na,\"b\"junk",
        "key,value\na,b\"c",
        "key,value\na,b,c",
    ] {
        assert!(
            import(text, TranslationFileFormat::Csv, &Default::default(), None).has_errors(),
            "{text}"
        );
    }
    let missing = import(
        "name,text\na,b",
        TranslationFileFormat::Csv,
        &Default::default(),
        None,
    );
    assert!(missing.diagnostics.iter().any(|d| d.code == "PTF190"));
}

#[test]
fn apple_unicode_escapes_and_comment_delimiters_round_trip() {
    let content = r#"// Context
"Greeting" = "\U0041\uD83D\uDE00\n\t\\\"";"#;
    let result = read(content, TranslationFileFormat::AppleStrings, "en");
    assert_eq!(result.catalog.entries[0].value, "A😀\n\t\\\"");
    let mut catalog = result.catalog;
    catalog.entries[0].comment = Some("Contains */ and /*\nsecond line".into());
    let exported = export(
        &catalog,
        TranslationFileFormat::AppleStrings,
        &options("en"),
    );
    let imported = read(&exported.content, TranslationFileFormat::AppleStrings, "en");
    assert_eq!(imported.catalog.entries, catalog.entries);
    for bad in [
        r#""a" = "\uD800";"#,
        r#""a" = "\uDC00";"#,
        r#""a" = "\q";"#,
        r#"/* unfinished"#,
    ] {
        assert!(import(
            bad,
            TranslationFileFormat::AppleStrings,
            &Default::default(),
            None
        )
        .has_errors());
    }
}

#[test]
fn infers_platform_cultures_from_paths() {
    for (text, format, path, culture) in [
        (
            "<root><data name=\"A\"><value>B</value></data></root>",
            TranslationFileFormat::Resx,
            "/res/Strings.pl-PL.resx",
            "pl-PL",
        ),
        (
            "<resources><string name=\"a\">b</string></resources>",
            TranslationFileFormat::AndroidResources,
            "/res/values-en-rUS/strings.xml",
            "en-US",
        ),
        (
            "<resources><string name=\"a\">b</string></resources>",
            TranslationFileFormat::AndroidResources,
            "/res/values-b+sr+Latn+RS/strings.xml",
            "sr-Latn-RS",
        ),
        (
            "\"a\"=\"b\";",
            TranslationFileFormat::AppleStrings,
            "/pl_PL.lproj/Localizable.strings",
            "pl-PL",
        ),
    ] {
        let result = import(text, format, &Default::default(), Some(path));
        assert!(!result.has_errors());
        assert_eq!(result.catalog.entries[0].culture.as_deref(), Some(culture));
    }
}

#[test]
fn android_arrays_and_flags_reconstruct_native_shapes() {
    let content = r#"<resources><string name="escaped" formatted="false" translatable="false">"  a\n\'b\' \\ "</string><string-array name="days"><item>Mon</item><item>Tue</item></string-array><plurals name="files"><item quantity="one">one</item><item quantity="other">many</item></plurals></resources>"#;
    let imported = read(content, TranslationFileFormat::AndroidResources, "en");
    assert_eq!(imported.catalog.entries[0].value, "  a\n'b' \\ ");
    let exported = export(
        &imported.catalog,
        TranslationFileFormat::AndroidResources,
        &options("en"),
    );
    assert!(!exported.has_errors());
    assert!(exported.content.contains("<string-array name=\"days\">"));
    assert!(exported.content.contains("translatable=\"false\""));
    let imported_again = read(
        &exported.content,
        TranslationFileFormat::AndroidResources,
        "en",
    );
    assert_eq!(
        imported_again.catalog.to_dictionary(Some("en")),
        imported.catalog.to_dictionary(Some("en"))
    );
}

#[test]
fn stringsdict_preserves_variables_and_accepts_standard_doctype() {
    let content = r#"<?xml version="1.0"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>Items</key><dict><key>NSStringLocalizedFormatKey</key><string>%#@items@</string><key>items</key><dict><key>NSStringFormatSpecTypeKey</key><string>NSStringPluralRuleType</string><key>NSStringFormatValueTypeKey</key><string>lld</string><key>one</key><string>%lld item</string><key>other</key><string>%lld items</string></dict></dict></dict></plist>"#;
    let imported = read(content, TranslationFileFormat::AppleStringsdict, "en");
    assert_eq!(imported.catalog.entries.len(), 2);
    assert_eq!(
        imported.catalog.entries[0].metadata["apple.variable"],
        "items"
    );
    let exported = export(
        &imported.catalog,
        TranslationFileFormat::AppleStringsdict,
        &options("en"),
    );
    assert!(!exported.has_errors());
    assert!(exported.content.contains("%#@items@"));
    assert!(exported.content.contains("<string>lld</string>"));
    let again = read(
        &exported.content,
        TranslationFileFormat::AppleStringsdict,
        "en",
    );
    assert_eq!(again.catalog.entries, imported.catalog.entries);
    assert!(export(
        &sample(),
        TranslationFileFormat::AppleStringsdict,
        &options("pl-PL")
    )
    .has_errors());
}

#[test]
fn po_context_plural_indices_fuzzy_and_comments_survive() {
    let content="msgid \"\"\nmsgstr \"\"\n\"Language: pl_PL\\n\"\n\"Plural-Forms: nplurals=3; plural=(n == 1 ? 0 : n % 10 >= 2 && n % 10 <= 4 ? 1 : 2);\\n\"\n\n# Translator\n#. Developer\n#: app.rs:42\n#, fuzzy, rust-format\nmsgctxt \"Inbox\"\nmsgid \"Message\"\nmsgid_plural \"Messages\"\nmsgstr[0] \"Wiadomość\"\nmsgstr[1] \"Wiadomości\"\nmsgstr[2] \"Wiadomości wiele\"\n";
    let imported = read(content, TranslationFileFormat::GettextPo, "en");
    assert_eq!(imported.catalog.entries.len(), 3);
    let dictionary = imported.catalog.to_dictionary(Some("pl-PL"));
    assert_eq!(dictionary["Inbox\u{4}Message.one"], "Wiadomość");
    assert_eq!(dictionary["Inbox\u{4}Message.index2"], "Wiadomości wiele");
    assert!(imported.diagnostics.iter().any(|d| d.code == "PTF171"));
    assert!(imported
        .catalog
        .entries
        .iter()
        .all(|entry| entry.state.as_deref() == Some("fuzzy")));
    let exported = export(
        &imported.catalog,
        TranslationFileFormat::GettextPo,
        &options("pl-PL"),
    );
    assert!(!exported.has_errors());
    assert!(exported.content.contains("msgstr[2]"));
    assert!(exported.content.contains("#: app.rs:42"));
    assert!(exported.content.contains("Plural-Forms:"));
    let again = read(&exported.content, TranslationFileFormat::GettextPo, "en");
    assert_eq!(again.catalog.entries, imported.catalog.entries);
}

#[test]
fn po_multiline_adjacent_entries_tabs_and_empty_translation() {
    let content="msgid\t\"First\"\n\" line\"\nmsgstr\t\"Pierwsza\"\n\" linia\"\nmsgid \"Empty\"\nmsgstr \"\"\n";
    let imported = read(content, TranslationFileFormat::GettextPo, "pl");
    assert_eq!(imported.catalog.entries.len(), 2);
    assert_eq!(imported.catalog.entries[0].key, "First line");
    assert_eq!(imported.catalog.entries[0].value, "Pierwsza linia");
    assert_eq!(
        imported.catalog.entries[1].metadata["po.untranslated"],
        "true"
    );
    let exported = export(
        &imported.catalog,
        TranslationFileFormat::GettextPo,
        &options("pl"),
    );
    assert!(exported.content.contains("msgid \"Empty\"\nmsgstr \"\""));
}

#[test]
fn xliff_preserves_context_state_source_and_comments() {
    let mut catalog = sample();
    catalog.entries[0].source = Some("Hello!".into());
    catalog.entries[0].context = Some("Status bar".into());
    catalog.entries[0].state = Some("translated".into());
    for format in [
        TranslationFileFormat::Xliff12,
        TranslationFileFormat::Xliff20,
    ] {
        let exported = export(&catalog, format, &options("pl-PL"));
        assert!(!exported.has_errors());
        let imported = read(&exported.content, format, "pl-PL");
        let greeting = imported
            .catalog
            .entries
            .iter()
            .find(|entry| entry.key == "Greeting")
            .unwrap();
        assert_eq!(greeting.source, Some("Hello!".into()));
        assert_eq!(greeting.context, Some("Status bar".into()));
        assert_eq!(greeting.state, Some("translated".into()));
        assert_eq!(greeting.comment, Some("Translator note".into()));
    }
}

#[test]
fn xliff_multisegments_are_unique_and_inline_markup_is_diagnosed() {
    let content = r#"<xliff version="2.1" srcLang="en" trgLang="pl"><file id="f"><unit id="title"><segment id="a"><source>A <ph id="1"/></source><target>Pierwszy <pc id="2">tekst</pc></target></segment><segment id="b"><source>B</source><target>Drugi</target></segment></unit></file></xliff>"#;
    let result = read(content, TranslationFileFormat::Xliff20, "pl");
    assert_eq!(result.catalog.entries.len(), 2);
    assert_eq!(result.catalog.entries[0].key, "title#a");
    assert_eq!(result.catalog.entries[1].key, "title#b");
    assert_eq!(result.catalog.entries[0].value, "Pierwszy tekst");
    assert!(result.diagnostics.iter().any(|d| d.code == "PTF181"));
    assert!(result.diagnostics.iter().any(|d| d.code == "PTF183"));
}

#[test]
fn xliff12_exports_multiple_languages_as_separate_files() {
    let mut catalog = TranslationCatalog::default();
    catalog.add("Title", Some("en"), "Title");
    catalog.add("Title", Some("pl"), "Tytuł");
    let exported = export(
        &catalog,
        TranslationFileFormat::Xliff12,
        &Default::default(),
    );
    let imported = read(&exported.content, TranslationFileFormat::Xliff12, "en");
    assert_eq!(imported.catalog.entries.len(), 2);
    assert_eq!(imported.catalog.to_dictionary(Some("pl"))["Title"], "Tytuł");
}

#[test]
fn rejects_entity_declarations_and_xml_control_characters() {
    let malicious = r#"<!DOCTYPE root [<!ENTITY x SYSTEM "file:///etc/passwd">]><root><data name="a"><value>&x;</value></data></root>"#;
    assert!(import(
        malicious,
        TranslationFileFormat::Resx,
        &Default::default(),
        None
    )
    .has_errors());
    let mut catalog = sample();
    catalog.entries[0].value.push('\0');
    for format in [
        TranslationFileFormat::Resx,
        TranslationFileFormat::AndroidResources,
        TranslationFileFormat::Xliff12,
        TranslationFileFormat::Xliff20,
    ] {
        assert!(export(&catalog, format, &Default::default()).has_errors());
    }
}

#[test]
fn non_string_resx_and_unsupported_variations_are_diagnosed() {
    let result = read(
        r#"<root><data name="Text"><value>Text</value></data><data name="Image" type="System.Drawing.Bitmap"><value>base64</value></data></root>"#,
        TranslationFileFormat::Resx,
        "en",
    );
    assert_eq!(result.catalog.entries.len(), 1);
    assert!(result.diagnostics.iter().any(|d| d.code == "PTF131"));
    let xc = read(
        r#"{"strings":{"a":{"localizations":{"en":{"variations":{"device":{"iphone":{"stringUnit":{"value":"iPhone"}}}}}}}}}"#,
        TranslationFileFormat::AppleXcstrings,
        "en",
    );
    assert!(xc.catalog.entries.is_empty());
    assert!(xc.diagnostics.iter().any(|d| d.code == "PTF120"));
}

#[test]
fn export_reports_metadata_loss_and_is_deterministic() {
    let mut catalog = sample();
    catalog.entries[0].context = Some("Dialog".into());
    catalog.entries[0]
        .metadata
        .insert("vendor".into(), "data".into());
    let first = export(
        &catalog,
        TranslationFileFormat::ProTranslateJson,
        &options("pl-PL"),
    );
    assert!(first
        .diagnostics
        .iter()
        .any(|d| d.code == "PTF202" && d.message.contains("metadata")));
    catalog.entries.reverse();
    let second = export(
        &catalog,
        TranslationFileFormat::ProTranslateJson,
        &options("pl-PL"),
    );
    assert_eq!(first.content, second.content);
}

#[test]
fn catalog_validates_keys_and_normalizes_culture_spelling() {
    let mut catalog = TranslationCatalog::default();
    catalog.add("A", Some("pl-PL"), "First");
    catalog.add("A", Some("PL-pl"), "Second");
    assert_eq!(catalog.to_dictionary(Some("pl-pl"))["A"], "Second");
    assert_eq!(catalog.cultures().len(), 1);
    assert!(catalog.validate().iter().any(|d| d.code == "PTF003"));
    catalog.add("", None, "invalid");
    assert!(export(&catalog, TranslationFileFormat::Csv, &Default::default()).has_errors());
    assert_eq!(
        TranslationCatalog::normalize_culture_name(Some("sr_latn_rs")),
        "sr-Latn-RS"
    );
    assert_eq!(TranslationCatalog::normalize_culture_name(None), "");
}

#[test]
fn trailing_comma_compatibility_does_not_repair_missing_json_values() {
    for content in [
        r#"{"a":{,}}"#,
        r#"{"a":[,]}"#,
        r#"{"a":[1,,]}"#,
        r#"{"a":,}"#,
    ] {
        assert!(
            import(
                content,
                TranslationFileFormat::ProTranslateJson,
                &Default::default(),
                None
            )
            .has_errors(),
            "{content}"
        );
    }
}

#[test]
fn export_refuses_ambiguous_plural_shapes() {
    let mut catalog = plurals();
    catalog.add("Count", Some("en"), "Singular");
    assert!(export(
        &catalog,
        TranslationFileFormat::AppleXcstrings,
        &options("en")
    )
    .has_errors());
    let mut catalog = plurals();
    let few = catalog.add("Count.few", Some("en"), "Few");
    few.plural_category = Some("few".into());
    assert!(export(&catalog, TranslationFileFormat::GettextPo, &options("en")).has_errors());
}

#[test]
fn po_unrepresentable_workflow_state_is_reported() {
    let mut catalog = sample();
    catalog.entries[0].state = Some("reviewed".into());
    let result = export(
        &catalog,
        TranslationFileFormat::GettextPo,
        &options("pl-PL"),
    );
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "PTF202" && d.message.contains("state")));
}
