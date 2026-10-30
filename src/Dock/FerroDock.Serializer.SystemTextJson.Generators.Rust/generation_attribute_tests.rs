// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use crate::compiler_fixture_input as fixture_input;
use ferrodock_serializer_system_text_json_generators::*;
use ferroui_build_scan::{
    model::RustType,
    scanner::{scan_crate, Scan, ScanOptions, Severity},
};
fn scan() -> Scan {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut options = ScanOptions::new(
        "ferrodock_serializer_generated_tests",
        here.join("../../../tests/Dock/FerroDock.Serializer.Generated.Tests/payload_metadata.rs"),
    );
    options.extern_crates = vec![
        "ferrodock_model".into(),
        "ferrodock_serializer_system_text_json".into(),
    ];
    let scan = scan_crate(&options);
    let errors: Vec<_> = scan.diagnostics_of(Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    scan
}
fn parse(scan: &Scan, source: &str) -> Result<RustGenerationAttributes, CompilationError> {
    parse_generation_attributes(scan, &scan.model.crate_name, "registration.rs", source)
}
fn compilation() -> Compilation {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/Dock/data/json-generator-semantics.json"
    ))
    .unwrap();
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "generated-payloads")
        .unwrap();
    fixture_input::compilation(&row["input"])
}
#[test]
fn real_scan_resolves_aliased_and_qualified_markers_preserving_order_duplicates_and_spans() {
    let scan = scan();
    let attributes=parse(&scan,"generate_json!();\nregister_json!(Payload);\n::ferrodock_serializer_system_text_json::dock_json_serializable!(crate::payload_models::Generic<i32>);\nregister_json!(Payload);").unwrap();
    assert!(attributes.activated);
    assert_eq!(
        attributes.activation_location.as_deref(),
        Some("registration.rs:1:1")
    );
    assert_eq!(attributes.registrations.len(), 3);
    assert_eq!(
        attributes.registrations[0].type_.text,
        attributes.registrations[2].type_.text
    );
    assert_ne!(
        attributes.registrations[0].type_.text,
        attributes.registrations[1].type_.text
    );
    assert_eq!(attributes.registrations[1].location, "registration.rs:3:1");
    assert!(attributes.registrations[0]
        .type_
        .text
        .ends_with("::payload_models::Payload"));
}
#[test]
fn duplicate_activation_invalid_arguments_and_unresolved_types_are_source_errors() {
    let scan = scan();
    for (source, part) in [
        ("generate_json!();generate_json!();", "more than once"),
        ("generate_json!(Payload);", "no arguments"),
        ("register_json!();", "one Rust type"),
        ("register_json!(Payload,Holder);", "one Rust type"),
        ("register_json!(MissingClass);", "Unresolved"),
        ("generate_json!(;", "invalid Rust"),
    ] {
        let error = parse(&scan, source).unwrap_err();
        assert!(error.0.contains(part), "{}", error.0);
        assert!(error.0.starts_with("registration.rs:"));
    }
}
#[test]
fn assembly_scope_and_cfg_selection_are_enforced_without_expanding_arbitrary_macros() {
    let scan = scan();
    assert!(parse_generation_attributes(
        &scan,
        "missing_module",
        "registration.rs",
        "generate_json!();"
    )
    .unwrap_err()
    .0
    .contains("unknown scanner module"));
    let unrelated = parse(&scan, "::unrelated::dock_json_source_generation!();::unrelated::dock_json_serializable!(Payload);").unwrap();
    assert!(!unrelated.activated);
    assert!(unrelated.registrations.is_empty());
    for source in [
        "mod nested {generate_json!();}",
        "fn f(){register_json!(Payload);}",
        "#[cfg(feature=\"x\")] generate_json!();",
        "#[cfg_attr(feature=\"x\",cfg(any()))] register_json!(Payload);",
    ] {
        assert!(parse(&scan, source).is_err(), "{source}");
    }
    let a=parse(&scan,"macro_rules! another {()=>{generate_json!();}}\n#[cfg(test)]mod tests {generate_json!();}\n#[cfg(test)]fn example(){register_json!(Payload);}").unwrap();
    assert!(!a.activated);
    assert!(a.registrations.is_empty());
    let a = parse(
        &scan,
        r#"
        #[cfg(test)] generate_json!();
        #[cfg(all(test,feature="x"))] register_json!(Payload);
        #[cfg(all(test,feature="x"))] mod tests {generate_json!();}
        fn production() { #[cfg(test)] register_json!(Payload); }
    "#,
    )
    .unwrap();
    assert!(!a.activated);
    assert!(a.registrations.is_empty());
    let nested = &scan.model.types[0].module;
    if nested != &scan.model.crate_name {
        assert!(parse_generation_attributes(
            &scan,
            nested,
            "nested.rs",
            "::ferrodock_serializer_system_text_json::dock_json_source_generation!();"
        )
        .unwrap_err()
        .0
        .contains("crate scope"));
    }
}
#[test]
fn parsed_rust_registrations_feed_original_activation_validation_and_deduplication() {
    let scan = scan();
    let mut c = compilation();
    let id = c
        .types
        .iter()
        .position(|t| t.metadata_name == "GeneratedPayload")
        .unwrap();
    let a = parse(
        &scan,
        "generate_json!();register_json!(Payload);register_json!(Payload);",
    )
    .unwrap();
    c.activated = false;
    c.registrations.clear();
    apply_generation_attributes(&mut c, &a, |_| Some(id)).unwrap();
    assert_eq!(c.registrations.len(), 2);
    assert_eq!(
        c.registrations[1].location.as_deref(),
        Some("registration.rs:1:42")
    );
    let model = build(&c).unwrap();
    assert!(model.diagnostics.is_empty());
    assert!(model.should_generate);
    assert_eq!(model.serializable_types.len(), 1);
    assert_eq!(model.serializable_types[0].type_id, id);
    let a = parse(&scan, "register_json!(Payload);").unwrap();
    apply_generation_attributes(&mut c, &a, |_| Some(id)).unwrap();
    let model = build(&c).unwrap();
    assert!(!model.should_generate);
    assert_eq!(model.diagnostics.len(), 1);
    assert_eq!(model.diagnostics[0].id, "DSTJ002");
    assert_eq!(
        model.diagnostics[0].location.as_deref(),
        Some("registration.rs:1:1")
    );
}
#[test]
fn missing_target_symbol_facts_leave_the_existing_compilation_unchanged() {
    let scan = scan();
    let mut c = compilation();
    let a = parse(&scan, "register_json!(Payload);").unwrap();
    let before = format!("{c:?}");
    assert!(apply_generation_attributes(&mut c, &a, |_| None)
        .unwrap_err()
        .0
        .contains("Missing semantic"));
    assert_eq!(format!("{c:?}"), before);
    assert!(
        apply_generation_attributes(&mut c, &a, |_| Some(usize::MAX))
            .unwrap_err()
            .0
            .contains("Missing compiler")
    );
    assert_eq!(format!("{c:?}"), before);
    let mut unresolved = a.clone();
    unresolved.registrations[0]
        .type_
        .unresolved
        .push("Missing".into());
    assert!(
        apply_generation_attributes(&mut c, &unresolved, |_| Some(0))
            .unwrap_err()
            .0
            .contains("Unresolved registration")
    );
    assert_eq!(format!("{c:?}"), before);
    let a = RustGenerationAttributes::default();
    apply_generation_attributes(&mut c, &a, |_| None).unwrap();
    assert!(!c.activated);
    assert!(c.registrations.is_empty());
}
#[test]
fn actual_compiled_registration_fragment_contains_all_original_source_payload_registrations() {
    let scan = scan();
    let source =
        include_str!("../../../tests/Dock/FerroDock.Serializer.Generated.Tests/json_registration.rs");
    let a = parse(&scan, source).unwrap();
    assert!(a.activated);
    assert_eq!(a.registrations.len(), 9);
    let generic: Vec<&RustType> = a
        .registrations
        .iter()
        .map(|r| &r.type_)
        .filter(|t| t.text.contains("Generic<"))
        .collect();
    assert_eq!(generic.len(), 2);
    assert_ne!(generic[0].text, generic[1].text);
}
