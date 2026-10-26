use super::*;

fn catalog(name: &str, content: &str) -> CatalogInput {
    CatalogInput::new(name, content)
}

#[test]
fn generation_is_deterministic_and_culture_specific() {
    let en = catalog("Strings.en-US.json", r#"{"Hello":"Hello {0}","Empty":""}"#);
    let pl = catalog("Strings.pl-PL.json", r#"{"Empty":"","Hello":"Cześć {0}"}"#);
    let mut first = CatalogCompiler::new();
    first.add_catalog(en.clone()).add_catalog(pl.clone());
    let mut second = CatalogCompiler::new();
    second.add_catalog(pl).add_catalog(en);
    let first = first.compile();
    let second = second.compile();
    assert_eq!(first.rust_source, second.rust_source);
    assert_eq!(first.manifest_json, second.manifest_json);
    assert!(first.diagnostics.is_empty(), "{:?}", first.diagnostics);
    assert!(first.rust_source.contains("FormatArgument; 1]"));
    assert!(first.rust_source.contains("binary_search_by"));
    syn::parse_file(&first.rust_source).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&first.manifest_json).unwrap();
    assert_eq!(manifest["cultures"], serde_json::json!(["en-US", "pl-PL"]));
}

#[test]
fn duplicates_and_placeholder_disagreement_are_errors() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(catalog("Strings.en.json", r#"{"Hello":"{0}"}"#));
    compiler.add_catalog(catalog("Strings.pl.json", r#"{"Hello":"{1}"}"#));
    compiler.add_catalog(catalog("Other.json", r#"{"Hello":"again"}"#).with_culture("en"));
    let result = compiler.compile();
    assert!(result.has_errors());
    assert!(result.diagnostics.iter().any(|d| d.code == "PTSG002"));
    assert!(result.diagnostics.iter().any(|d| d.code == "PTA002"));
}

#[test]
fn missing_keys_coverage_arity_and_dynamic_usages_have_stable_codes() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(catalog(
        "Strings.en.json",
        r#"{"Hello":"{0}","OnlyEnglish":"Yes"}"#,
    ));
    compiler.add_catalog(catalog("Strings.pl.json", r#"{"Hello":"{0}"}"#));
    compiler.add_usage(KeyUsage::lookup("Absent"));
    compiler.add_usage(KeyUsage::format("Hello", 0));
    compiler.add_usage(KeyUsage {
        key: None,
        ..KeyUsage::lookup("")
    });
    let result = compiler.compile();
    for code in ["PTA001", "PTA002", "PTA003", "PTA004"] {
        assert!(result.diagnostics.iter().any(|d| d.code == code), "{code}");
    }
    assert!(!result.has_errors());
}

#[test]
fn identifiers_escape_keywords_unicode_and_collisions_and_source_is_valid() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(catalog(
        "Strings.json",
        r#"{"1 key":"a","a-b":"b","a.b":"c","A_B":"d","fn":"e","日本語":"f","quote\"":"\"\\\n"}"#,
    ));
    let result = compiler.compile();
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    syn::parse_file(&result.rust_source).unwrap();
    assert!(result.rust_source.contains("pub const KEY_1_KEY"));
    assert!(result.rust_source.contains("pub const A_B_2"));
    assert!(result.rust_source.contains("pub const A_B_3"));
}

#[test]
fn keys_only_catalogs_generate_accessors_without_fabricated_translations() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_key_list(
        "app.protranslate.keys.txt",
        "# keys\nHello\nHello\nSecond\n",
    );
    let result = compiler.compile();
    assert!(result.rust_source.contains("pub const HELLO"));
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "PTSG002" && d.line == Some(3)));
    let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
    assert_eq!(manifest["entries"], serde_json::json!([]));
}

#[test]
fn key_list_manifest_preserves_each_source_and_one_based_line_deterministically() {
    let source = "\u{feff}# declarations\r\n Shared \r\nSecond\r\nShared\r\n";
    let mut first = CatalogCompiler::new();
    first.add_key_list("z.protranslate.keys.txt", source);
    first.add_key_list("a.protranslate.keys.txt", "\nShared\n");
    first.add_catalog(catalog("Strings.en.json", r#"{"Shared":"translated"}"#));
    let mut second = CatalogCompiler::new();
    second.add_catalog(catalog("Strings.en.json", r#"{"Shared":"translated"}"#));
    second.add_key_list("a.protranslate.keys.txt", "\nShared\n");
    second.add_key_list("z.protranslate.keys.txt", source);
    let first = first.compile();
    let second = second.compile();
    assert_eq!(first.manifest_json, second.manifest_json);
    assert_eq!(first.rust_source, second.rust_source);
    assert!(!first.has_errors());
    let manifest: serde_json::Value = serde_json::from_str(&first.manifest_json).unwrap();
    assert_eq!(manifest["keys"], serde_json::json!(["Second", "Shared"]));
    assert_eq!(
        manifest["key_declarations"],
        serde_json::json!([
            {"key":"Second", "source_file":"z.protranslate.keys.txt", "line":3},
            {"key":"Shared", "source_file":"a.protranslate.keys.txt", "line":2},
            {"key":"Shared", "source_file":"z.protranslate.keys.txt", "line":2},
            {"key":"Shared", "source_file":"z.protranslate.keys.txt", "line":4}
        ])
    );
    assert_eq!(manifest["entries"].as_array().unwrap().len(), 1);
    assert_eq!(manifest["entries"][0]["key"], "Shared");
    assert_eq!(manifest["entries"][0]["source_file"], "Strings.en.json");
    assert!(first
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "PTSG002"
            && diagnostic.file.as_deref() == Some("z.protranslate.keys.txt")
            && diagnostic.line == Some(4)));
}

#[test]
fn syntax_analysis_is_scoped_and_handles_raw_strings_constants_and_arrays() {
    let source = r###"
        const KEY: &str = "Hello";
        fn run() {
            translations.get_string(r#"Missing"#);
            translations.format(KEY, &[1.into(), 2.into()]);
            translations.observe(ProTranslateKeys::HELLO, vec![1.into()]);
            translations.get_string(dynamic_key);
            unrelated.format("Ignored", &[]);
        }
    "###;
    let options = SourceAnalysisOptions::default().receiver("translations");
    let analysis = analyze_rust_source("app.rs", source, &options);
    assert!(analysis.diagnostics.is_empty());
    assert_eq!(analysis.usages.len(), 4);
    assert_eq!(analysis.usages[0].key.as_deref(), Some("Missing"));
    assert_eq!(analysis.usages[1].key.as_deref(), Some("Hello"));
    assert_eq!(analysis.usages[1].argument_count, Some(2));
    assert_eq!(
        analysis.usages[2].generated_constant.as_deref(),
        Some("HELLO")
    );
    assert_eq!(analysis.usages[2].argument_count, Some(1));
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(catalog("Strings.en.json", r#"{"Hello":"{0}"}"#));
    compiler.add_rust_source("app.rs", source, &options);
    let result = compiler.compile();
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "PTA001")
            .count(),
        1
    );
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "PTA002")
            .count(),
        1
    );
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "PTA004")
            .count(),
        1
    );
}

#[test]
fn invalid_sources_and_invalid_catalogs_report_errors() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(catalog("Strings.en.json", "{"));
    compiler.add_rust_source("broken.rs", "fn {", &SourceAnalysisOptions::default());
    assert!(compiler.compile().has_errors());
}

#[test]
fn sparse_placeholders_require_maximum_index_plus_one() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(catalog(
        "Strings.en.json",
        r#"{"Sparse":"{{literal}} {2,8:N2} {0}"}"#,
    ));
    let result = compiler.compile();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert!(result.rust_source.contains("FormatArgument; 3]"));
}

#[test]
fn source_scanner_does_not_resolve_shadowed_constants_as_global_values() {
    let source = r#"
        const KEY: &str = "Global";
        fn run() {
            const KEY: &str = "Local";
            translations.get_string(KEY);
            { let key = dynamic(); translations.get_string(key); }
        }
        mod inner { fn run() { translations.get_string(KEY); } }
    "#;
    let result = analyze_rust_source(
        "app.rs",
        source,
        &SourceAnalysisOptions::default().receiver("translations"),
    );
    assert_eq!(result.usages[0].key.as_deref(), Some("Local"));
    assert!(result.usages[1].key.is_none());
    assert!(result.usages[2].key.is_none());
}

#[test]
fn invalid_filename_cultures_are_errors_including_empty_catalogs() {
    for content in [r#"{"Hello":"Hello"}"#, "{}"] {
        let mut compiler = CatalogCompiler::new();
        compiler.add_catalog(catalog("Strings.en_US.json", content));
        let result = compiler.compile();
        assert!(result.has_errors());
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "PTA005" && d.culture.as_deref() == Some("en_US")));
        let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
        assert_eq!(manifest["entries"], serde_json::json!([]));
    }
}

#[test]
fn culture_inference_supports_compound_suffixes_and_windows_paths() {
    for name in [
        "Strings.pl-PL.i18next.json",
        "strings.pl-PL.protranslate.json",
        r"C:\resources\Strings.pl-PL.json",
        "Labels.pl-PL.resx",
    ] {
        let content = if name.ends_with("resx") {
            r#"<root><data name="Hello"><value>Cześć</value></data></root>"#
        } else {
            r#"{"Hello":"Cześć"}"#
        };
        let mut compiler = CatalogCompiler::new();
        compiler.add_catalog(catalog(name, content));
        let result = compiler.compile();
        assert!(!result.has_errors(), "{name}: {:?}", result.diagnostics);
        let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
        assert_eq!(manifest["cultures"], serde_json::json!(["pl-PL"]), "{name}");
    }
}

#[test]
fn canonical_culture_identity_catches_duplicates() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(catalog("one.json", r#"{"Hello":"one"}"#).with_culture("EN-us"));
    compiler.add_catalog(catalog("two.json", r#"{"Hello":"two"}"#).with_culture("en-US"));
    let result = compiler.compile();
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "PTSG002" && d.severity == DiagnosticSeverity::Error));
    let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
    assert_eq!(manifest["cultures"], serde_json::json!(["en-US"]));
}

#[test]
fn diagnostic_context_and_repeated_placeholder_identity_are_preserved() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(catalog(
        "Strings.en.json",
        r#"{"Repeated":"{0} {0} {2}","ignored":3}"#,
    ));
    compiler.add_catalog(catalog("Strings.pl.json", r#"{"Repeated":"{2} {0}"}"#));
    let result = compiler.compile();
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert!(!result.diagnostics.iter().any(|d| d.code == "PTA002"));
    assert!(result.diagnostics.iter().any(|d| d.code == "PTF101"
        && d.key.as_deref() == Some("ignored")
        && d.culture.as_deref() == Some("en")));
    let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
    assert_eq!(
        manifest["entries"][0]["placeholder_indexes"],
        serde_json::json!([0, 2])
    );
}

#[test]
fn key_only_accessors_accept_arguments_without_inventing_an_arity() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_key_list("keys.protranslate.keys.txt", "\u{feff}Dynamic\n");
    let result = compiler.compile();
    assert!(result
        .rust_source
        .contains("pub const DYNAMIC: &'static str = \"Dynamic\""));
    assert!(result.rust_source.contains(
        "format_dynamic(&self, arguments: &[::ferroui_protranslate_core::FormatArgument])"
    ));
    assert!(result.rust_source.contains(
        "observe_dynamic(&self, arguments: Vec<::ferroui_protranslate_core::FormatArgument>)"
    ));
}

#[test]
fn missing_generated_constant_is_a_static_key_diagnostic() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(catalog("Strings.en.json", r#"{"Hello":"Hello"}"#));
    compiler.add_rust_source(
        "use.rs",
        "fn run() { translations.get_string(ProTranslateKeys::ABSENT); }",
        &SourceAnalysisOptions::default().receiver("translations"),
    );
    let result = compiler.compile();
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "PTA001" && d.message.contains("ABSENT")));
    assert!(!result.diagnostics.iter().any(|d| d.code == "PTA004"));
}

#[test]
fn scanner_handles_function_closure_loop_match_and_condition_scopes() {
    let source = r#"
        const KEY: &str = "Global";
        fn argument(KEY: &str) { translations.get_string(KEY); }
        fn run() {
            let closure = |KEY| translations.get_string(KEY);
            for KEY in values { translations.get_string(KEY); }
            if let Some(KEY) = option {
                translations.get_string(KEY);
            } else { translations.get_string(KEY); }
            while let Some(KEY) = option { translations.get_string(KEY); }
            match option {
                Some(KEY) if translations.get_string(KEY).is_ok() => translations.get_string(KEY),
                _ => translations.get_string(KEY),
            }
            translations.get_string(KEY);
        }
        struct Example;
        impl Example {
            fn method(&self, KEY: &str) { translations.get_string(KEY); }
        }
        fn generic<const KEY: usize>() { translations.get_string(KEY); }
    "#;
    let result = analyze_rust_source(
        "scopes.rs",
        source,
        &SourceAnalysisOptions::default().receiver("translations"),
    );
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let keys: Vec<_> = result
        .usages
        .iter()
        .map(|usage| usage.key.as_deref())
        .collect();
    assert_eq!(
        keys,
        vec![
            None,
            None,
            None,
            None,
            Some("Global"),
            None,
            None,
            None,
            Some("Global"),
            Some("Global"),
            None,
            None
        ]
    );
}

#[test]
fn scanner_preserves_let_shadowing_and_resolves_raw_identifiers() {
    let source = r#"
        const r#type: &str = "Hello";
        fn run() {
            let KEY = runtime();
            const KEY: &str = "Constant";
            translations.get_string(KEY);
            translations.get_string(r#type);
            translations.format_for(ProTranslateKeys::r#HELLO, &[1.into(); 3], culture);
        }
    "#;
    let result = analyze_rust_source(
        "raw.rs",
        source,
        &SourceAnalysisOptions::default().receiver("translations"),
    );
    assert!(result.diagnostics.is_empty());
    assert!(result.usages[0].key.is_none());
    assert_eq!(result.usages[1].key.as_deref(), Some("Hello"));
    assert_eq!(
        result.usages[2].generated_constant.as_deref(),
        Some("HELLO")
    );
    assert_eq!(result.usages[2].argument_count, Some(3));
}

#[test]
fn notification_option_is_explicit_and_manifest_records_it() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(catalog("Strings.en.json", r#"{"Title":"Title"}"#));
    assert!(!compiler.compile().rust_source.contains("ferroui_system"));
    compiler.notify_properties(true);
    let result = compiler.compile();
    syn::parse_file(&result.rust_source).unwrap();
    assert!(result.rust_source.contains("INotifyPropertyChanged"));
    assert!(result.rust_source.contains("Rc::new_cyclic"));
    let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
    assert_eq!(manifest["notify_properties"], true);
}

struct TestDirectory(std::path::PathBuf);
impl TestDirectory {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "protranslate-build-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn builder_rejects_diagnostics_before_writing_outputs() {
    let directory = TestDirectory::new();
    let catalog_path = directory.0.join("Strings.en_US.json");
    std::fs::write(&catalog_path, r#"{"Hello":"Hello"}"#).unwrap();
    let out = directory.0.join("output");
    assert!(matches!(
        Builder::new().catalog(&catalog_path).generate_to(&out),
        Err(BuildError::Diagnostics(_))
    ));
    assert!(!out.exists());
}

#[cfg(unix)]
#[test]
fn builder_rejects_newlines_in_cargo_dependency_paths() {
    let directory = TestDirectory::new();
    let catalog_path = directory
        .0
        .join("Strings.en\ncargo:rustc-cfg=unexpected.json");
    std::fs::write(&catalog_path, "{}").unwrap();
    let result = Builder::new()
        .catalog(&catalog_path)
        .generate_to(&directory.0.join("out"));
    assert!(
        matches!(result, Err(BuildError::Io { source, .. }) if source.kind() == std::io::ErrorKind::InvalidInput)
    );
}

#[test]
fn generated_providers_and_notification_handles_compile_and_run() {
    let mut compiler = CatalogCompiler::new();
    let dangerous = "};\ncompile_error!(\"injected\"); // 🦀\0";
    compiler.add_catalog(catalog(
        "Strings.en-US.json",
        &serde_json::json!({"Greeting":"Hello {0}","Empty":"","Dangerous\"key":dangerous})
            .to_string(),
    ));
    compiler.add_catalog(catalog(
        "Strings.pl-PL.json",
        &serde_json::json!({"Greeting":"Witaj {0}","Empty":"","Dangerous\"key":dangerous})
            .to_string(),
    ));
    compiler.add_catalog(catalog("Strings.json", r#"{"Shared":"neutral"}"#));
    compiler.add_key_list(
        "keys.protranslate.keys.txt",
        "# declarations\n\nDynamic\nGreeting\n",
    );
    let plain = compiler.compile();
    assert!(!plain.has_errors());
    compiler.notify_properties(true);
    let notifying = compiler.compile();
    assert!(!notifying.has_errors());
    let mut program = format!(
        "#![allow(dead_code)]\nmod plain {{ {} }}\nmod notifying {{ {} }}\n",
        plain.rust_source, notifying.rust_source
    );
    program.push_str(&format!("const EXPECTED_ESCAPED: &str = {dangerous:?};\n"));
    program.push_str(r#"
        use ferroui_protranslate_core::*;
        use plain::ProTranslateAccessors;
        use std::{rc::Rc, cell::Cell};
        fn main() {
            let cultures = CultureService::new(Culture::parse("EN-us").unwrap());
            let translations = TranslationService::new(Rc::new(plain::GeneratedTranslationProvider),cultures.clone());
            let strings = plain::ProTranslateStrings::new(translations.clone());
            assert_eq!(strings.culture().name(), "en-US");
            assert_eq!(strings.ui_culture().name(), "en-US");
            assert_eq!(strings.value_empty().unwrap(), "");
            assert_eq!(strings.value_dangerous_key().unwrap(), EXPECTED_ESCAPED);
            assert_eq!(strings.value_shared().unwrap(), "neutral");
            assert_eq!(strings.format_greeting(["Ada".into()]).unwrap(), "Hello Ada");
            assert_eq!(translations.format_greeting(["Ada".into()]).unwrap(), "Hello Ada");
            assert_eq!(translations.value_empty().unwrap(), "");
            assert!(matches!(plain::GeneratedTranslationProvider.get_string(" \t", &Culture::parse("en").unwrap()), Err(TranslationError::InvalidKey)));
            let neutral = plain::GeneratedTranslationProvider.get_string("Shared", &Culture::parse("fr").unwrap()).unwrap();
            assert_eq!(neutral.value, "neutral");
            assert_eq!(neutral.culture.name(), "fr");
            assert!(!neutral.resource_not_found);
            assert!(plain::GeneratedTranslationProvider::with_name("  ").is_err());
            let named = plain::GeneratedTranslationProvider::with_name("Application resources").unwrap();
            assert_eq!(named.get_string("Shared", &Culture::parse("de").unwrap()).unwrap().provider_name, "Application resources");
            assert!(plain::ProTranslateProviderManifest::KEYS.contains(&"Greeting"));
            assert!(plain::ProTranslateProviderManifest::CULTURES.contains(&"pl-PL"));
            let manifest_entry = plain::ProTranslateProviderManifest::ENTRIES.iter().find(|entry| entry.key == "Greeting" && entry.culture == Some("en-US")).unwrap();
            assert_eq!(manifest_entry.source_file, "Strings.en-US.json");
            assert_eq!(manifest_entry.placeholder_indexes, &[0]);
            assert!(manifest_entry.format_valid);
            let declarations = plain::ProTranslateProviderManifest::KEY_DECLARATIONS;
            assert_eq!(declarations.len(), 2);
            assert_eq!(declarations[0], plain::ProTranslateKeyDeclaration {
                key: "Dynamic", source_file: "keys.protranslate.keys.txt", line: 3,
            });
            assert_eq!(declarations[1].key, "Greeting");
            assert_eq!(declarations[1].line, 4);
            assert!(!plain::ProTranslateProviderManifest::ENTRIES.iter().any(|entry| entry.key == "Dynamic"));
            assert!(plain::GeneratedTranslationProvider.get_string("Dynamic", &Culture::parse("en-US").unwrap()).unwrap().resource_not_found);
            assert!(plain::GeneratedTranslationProvider.get_string("Missing", &Culture::parse("en-US").unwrap()).unwrap().resource_not_found);
            let observed = strings.observe_greeting(["Ada".into()]);
            cultures.set_culture(Culture::parse("pl-PL").unwrap());
            assert_eq!(observed.value().unwrap(), "Witaj Ada");
            drop(observed);
            let provider = Rc::new(InMemoryTranslationProvider::new("runtime"));
            provider.insert(Culture::parse("en").unwrap(),"Dynamic","Runtime {0}");
            let runtime = TranslationService::new(provider, CultureService::new(Culture::parse("en").unwrap()));
            let dynamic = plain::ProTranslateStrings::new(runtime);
            assert_eq!(dynamic.format_dynamic(&["Ada".into()]).unwrap(), "Runtime Ada");
            assert_eq!(dynamic.observe_dynamic(vec!["Ada".into()]).value().unwrap(), "Runtime Ada");

            let before = translations.subscriber_count();
            let notifying = notifying::ProTranslateStrings::new(translations.clone());
            let clone = notifying.clone();
            assert!(notifying == clone);
            assert_eq!(translations.subscriber_count(),before+1);
            let count = Rc::new(Cell::new(0));
            let capture = count.clone();
            let token = notifying.property_changed().add(Rc::new(move |name| { assert_eq!(name, ""); capture.set(capture.get()+1); }));
            cultures.set_culture(Culture::parse("en-US").unwrap());
            assert_eq!(notifying.value_greeting().unwrap(), "Hello {0}");
            translations.refresh();
            notifying.refresh();
            assert_eq!(count.get(),3);
            assert!(notifying.property_changed().remove(token));
            drop(notifying);
            assert_eq!(translations.subscriber_count(),before+1);
            drop(clone);
            assert_eq!(translations.subscriber_count(),before);
        }
    "#);
    let directory = TestDirectory::new();
    let source = directory.0.join("generated.rs");
    std::fs::write(&source, program).unwrap();
    let deps = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned();
    let dependency = |crate_name: &str| {
        let prefix = format!("lib{crate_name}-");
        let mut matches: Vec<_> = std::fs::read_dir(&deps)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".rlib"))
            })
            .collect();
        matches.sort_by_key(|path| std::fs::metadata(path).unwrap().modified().unwrap());
        matches
            .pop()
            .unwrap_or_else(|| panic!("Missing test dependency {crate_name} in {}", deps.display()))
    };
    let executable = directory.0.join(if cfg!(windows) {
        "generated.exe"
    } else {
        "generated"
    });
    let compiled =
        std::process::Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
            .args([
                "--edition=2021",
                "--crate-name=protranslate_generated_validation",
            ])
            .arg(&source)
            .arg("-L")
            .arg(format!("dependency={}", deps.display()))
            .arg("--extern")
            .arg(format!(
                "ferroui_protranslate_core={}",
                dependency("ferroui_protranslate_core").display()
            ))
            .arg("--extern")
            .arg(format!(
                "ferroui_system={}",
                dependency("ferroui_system").display()
            ))
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("rustc is available while running Cargo tests");
    assert!(
        compiled.status.success(),
        "Generated provider compilation failed:\n{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let executed = std::process::Command::new(&executable).output().unwrap();
    assert!(
        executed.status.success(),
        "Generated provider runtime assertions failed:\n{}",
        String::from_utf8_lossy(&executed.stderr)
    );
}

#[test]
fn platform_catalog_directories_select_the_actual_resource_culture() {
    for (name, expected) in [
        ("res/values-en-rUS/strings.xml", "en-US"),
        ("res/values-sr-Latn-rRS/strings.xml", "sr-Latn-RS"),
        ("res/values-sr-latn-rRS-night/strings.xml", "sr-Latn-RS"),
        ("res/values-sr-Cyrl-RRS-land/strings.xml", "sr-Cyrl-RS"),
        ("res/values-sr-Latn/strings.xml", "sr-Latn"),
        ("res/values-mcc310-mnc004-pl-rPL-night/strings.xml", "pl-PL"),
        ("res/values-b+zh+Hant+TW-night/strings.xml", "zh-Hant-TW"),
        (r"C:\project\res\values-es-r419\strings.xml", "es-419"),
    ] {
        let mut compiler = CatalogCompiler::new();
        compiler.add_catalog(CatalogInput::new(
            name,
            "<resources><string name=\"Hello\">Hello</string></resources>",
        ));
        let result = compiler.compile();
        assert!(!result.has_errors(), "{name}: {:?}", result.diagnostics);
        let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
        assert_eq!(
            manifest["cultures"],
            serde_json::json!([expected]),
            "{name}"
        );
    }
    for (name, expected) in [
        ("Resources/pl.lproj/Localizable.strings", "pl"),
        (r"C:\Resources\en_US.lproj\Localizable.strings", "en-US"),
        ("Resources/Base.lproj/Localizable.strings", ""),
    ] {
        let mut compiler = CatalogCompiler::new();
        compiler.add_catalog(CatalogInput::new(name, "\"Hello\" = \"Hello\";"));
        let result = compiler.compile();
        assert!(!result.has_errors(), "{name}: {:?}", result.diagnostics);
        let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
        assert_eq!(
            manifest["cultures"],
            serde_json::json!([expected]),
            "{name}"
        );
    }
    assert!(CatalogInput::new("res/values-night/strings.xml", "")
        .culture
        .is_none());
    assert!(CatalogInput::new("res/values-car/strings.xml", "")
        .culture
        .is_none());
    for qualifier in [
        "land", "port", "long", "desk", "dpad", "ldpi", "mdpi", "hdpi",
    ] {
        let input = CatalogInput::new(format!("res/values-sr-{qualifier}/strings.xml"), "");
        assert_eq!(input.culture.as_deref(), Some("sr"), "{qualifier}");
    }
}

#[test]
fn empty_declared_cultures_participate_in_coverage_and_manifest() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(CatalogInput::new(
        "Strings.en-US.json",
        r#"{"Hello":"Hello"}"#,
    ));
    compiler.add_catalog(CatalogInput::new("Strings.pl-PL.json", "{}"));
    let result = compiler.compile();
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "PTA003"
            && diagnostic.key.as_deref() == Some("Hello")
            && diagnostic.culture.as_deref() == Some("pl-PL")));
    let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
    assert_eq!(manifest["cultures"], serde_json::json!(["en-US", "pl-PL"]));
}

#[test]
fn invalid_format_does_not_invent_a_zero_argument_contract() {
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(CatalogInput::new(
        "Strings.en.json",
        r#"{"Message":"{name}"}"#,
    ));
    compiler.add_catalog(CatalogInput::new("Strings.pl.json", r#"{"Message":"{0}"}"#));
    let result = compiler.compile();
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    assert!(result
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "PTSG003"));
    assert!(result.rust_source.contains(
        "fn format_message(&self, arguments: &[::ferroui_protranslate_core::FormatArgument])"
    ));
    let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
    assert_eq!(manifest["entries"][0]["format_valid"], false);
}

#[test]
fn builder_allows_explicit_culture_and_format_for_named_inputs() {
    let directory = TestDirectory::new();
    let path = directory.0.join("application.catalog");
    std::fs::write(
        &path,
        "<root><data name=\"Hello\"><value>Witaj</value></data></root>",
    )
    .unwrap();
    let result = Builder::new()
        .catalog_with_options(
            &path,
            CatalogFileOptions {
                culture: Some("pl-PL".into()),
                format: Some(ferroui_protranslate_formats::TranslationFileFormat::Resx),
            },
        )
        .generate_to(&directory.0.join("out"))
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&result.manifest_json).unwrap();
    assert_eq!(manifest["cultures"], serde_json::json!(["pl-PL"]));
    assert!(result.rust_source.contains("Witaj"));
}

#[cfg(unix)]
#[test]
fn builder_rejects_non_utf8_tracking_paths_without_lossy_aliases() {
    use std::os::unix::ffi::OsStringExt;
    let directory = TestDirectory::new();
    let path = directory
        .0
        .join(std::ffi::OsString::from_vec(b"catalog\xff.json".to_vec()));
    let out = directory.0.join("out");
    // Reject the tracking name before touching the filesystem: some filesystems
    // reject non-UTF-8 names themselves, and others would report a missing file.
    for builder in [
        Builder::new().catalog(&path),
        Builder::new().source(&path, SourceAnalysisOptions::default()),
    ] {
        match builder.generate_to(&out) {
            Err(BuildError::Io {
                path: rejected,
                source,
            }) => {
                assert_eq!(rejected, path);
                assert_eq!(source.kind(), std::io::ErrorKind::InvalidInput);
                assert_eq!(source.to_string(), "Cargo dependency paths must be UTF-8");
            }
            result => panic!("Expected an invalid tracking path, got {result:?}"),
        }
    }
    assert!(!out.exists());
}

#[test]
fn scanner_does_not_resolve_imports_or_conditional_constants_as_known_keys() {
    let source = r#"
        const KEY: &str = "Global";
        #[cfg(first)] const CONDITIONAL: &str = "First";
        #[cfg(second)] const CONDITIONAL: &str = "Second";
        fn run() {
            { use dependency::KEY; translations.get_string(KEY); }
            { use dependency::SOME_NAME as KEY; translations.get_string(KEY); }
            { use dependency::*; translations.get_string(KEY); }
            translations.get_string(CONDITIONAL);
            translations.get_string(KEY);
        }
    "#;
    let analysis = analyze_rust_source(
        "app.rs",
        source,
        &SourceAnalysisOptions::default().receiver("translations"),
    );
    assert_eq!(analysis.usages.len(), 5);
    assert!(analysis.usages[..4].iter().all(|usage| usage.key.is_none()));
    assert_eq!(analysis.usages[4].key.as_deref(), Some("Global"));
}

#[test]
fn scanner_resolves_block_constants_in_nested_modules_without_outer_name_leakage() {
    let source = r#"
        const KEY: &str = "Outer";
        mod nested {
            fn run() {
                const LOCAL: &str = "Inner";
                translations.get_string(LOCAL);
                translations.get_string(KEY);
            }
        }
    "#;
    let analysis = analyze_rust_source(
        "app.rs",
        source,
        &SourceAnalysisOptions::default().receiver("translations"),
    );
    assert_eq!(analysis.usages[0].key.as_deref(), Some("Inner"));
    assert!(analysis.usages[1].key.is_none());
}

#[test]
fn modules_inside_blocks_do_not_capture_outer_local_constants() {
    let source = r#"
        const KEY: &str = "Crate";
        fn run() {
            const KEY: &str = "Block";
            mod inner {
                use super::KEY;
                fn run() {
                    translations.get_string(KEY);
                    const LOCAL: &str = "Inner";
                    translations.get_string(LOCAL);
                }
            }
            translations.get_string(KEY);
        }
        fn after() { translations.get_string(KEY); }
    "#;
    let analysis = analyze_rust_source(
        "app.rs",
        source,
        &SourceAnalysisOptions::default().receiver("translations"),
    );
    assert_eq!(analysis.usages.len(), 4);
    assert!(analysis.usages[0].key.is_none());
    assert_eq!(analysis.usages[1].key.as_deref(), Some("Inner"));
    assert_eq!(analysis.usages[2].key.as_deref(), Some("Block"));
    assert_eq!(analysis.usages[3].key.as_deref(), Some("Crate"));
}

#[test]
fn scanner_checks_explicit_static_facades_and_generated_extension_calls() {
    let source = r#"
        fn run() {
            ui::TranslationService::t("Missing");
            ui::TranslationService::format("Hello", &[1.into(), 2.into()]);
            unrelated::TranslationService::t("Ignored");
            translations.format_hello([1.into()]);
            translations.value_unknown();
            self.r#translations.translate("Hello", &[1.into()]);
            translations.observe("Hello", vec![]);
        }
    "#;
    let options = SourceAnalysisOptions::default()
        .receiver("ui::TranslationService")
        .receiver("translations")
        .receiver("self.translations")
        .generated_accessors(true);
    let analysis = analyze_rust_source("app.rs", source, &options);
    assert_eq!(analysis.usages.len(), 6);
    assert_eq!(
        analysis.usages[2].generated_constant.as_deref(),
        Some("HELLO")
    );
    assert_eq!(analysis.usages[2].argument_count, Some(1));
    assert_eq!(analysis.usages[4].key.as_deref(), Some("Hello"));
    assert_eq!(analysis.usages[4].argument_count, Some(1));
    assert_eq!(analysis.usages[5].argument_count, None);
    let mut compiler = CatalogCompiler::new();
    compiler.add_catalog(CatalogInput::new("Strings.en.json", r#"{"Hello":"{0}"}"#));
    compiler.add_rust_source("app.rs", source, &options);
    let diagnostics = compiler.compile().diagnostics;
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "PTA001")
            .count(),
        2
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "PTA002")
            .count(),
        1
    );
    assert!(!diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "PTA004"));
}
