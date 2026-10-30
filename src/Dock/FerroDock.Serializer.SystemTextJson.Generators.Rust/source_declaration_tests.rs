// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferroui_build_scan::{
    model::RustType,
    scanner::{scan_crate, Scan, ScanOptions, Severity},
};
use std::path::Path;
fn fixture() -> Scan {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/source_declarations/lib.rs");
    let mut options = ScanOptions::new("declarations", root);
    options.extern_crates = vec!["source_dependency".into()];
    scan_crate(&options)
}
fn named<'a>(facts: &'a RustSourceDeclarations, path: &str) -> &'a RustSourceDeclaration {
    facts.definition(&RustType::resolved(path)).unwrap()
}
#[test]
fn transparent_storage_requires_literal_layout_and_one_unconditional_original_field() {
    let facts = read_source_declarations(&fixture()).unwrap();
    for (name, field) in [
        ("NumericTuple", Some("i32")),
        ("NumericNamed", Some("i32")),
        ("NumericUnsigned", Some("u32")),
        ("NumericC", None),
        ("NumericDefault", None),
        ("NumericMarker", None),
        ("NumericConditional", None),
        ("NumericConditionalLayout", None),
        ("Choice", None),
    ] {
        let source = named(&facts, &format!("::declarations::{name}"));
        assert_eq!(
            source.transparent_field.as_ref().map(|t| t.text.as_str()),
            field,
            "{name}"
        );
    }
    assert!(!named(&facts, "::declarations::NumericConditionalLayout")
        .cfg
        .is_empty());
}
#[test]
fn actual_declared_kinds_visibility_paths_and_source_locations_are_retained() {
    let facts = read_source_declarations(&fixture()).unwrap();
    let plain = named(&facts, "::declarations::Plain");
    assert_eq!(plain.kind, RustDeclarationKind::Struct);
    assert_eq!(plain.visibility, RustDeclarationVisibility::Public);
    assert!(
        plain.location.ends_with("lib.rs:2:12"),
        "{}",
        plain.location
    );
    assert_eq!(
        named(&facts, "::declarations::Private").visibility,
        RustDeclarationVisibility::Private
    );
    assert_eq!(
        named(&facts, "::declarations::Internal").visibility,
        RustDeclarationVisibility::Crate
    );
    assert_eq!(
        named(&facts, "::declarations::Choice").kind,
        RustDeclarationKind::Enum
    );
    assert_eq!(
        named(&facts, "::declarations::IRoot").kind,
        RustDeclarationKind::Trait
    );
    assert_eq!(
        named(&facts, "::declarations::nested::Parent").visibility,
        RustDeclarationVisibility::Restricted("super".into())
    );
    assert_eq!(
        named(&facts, "::declarations::nested::Visible").visibility,
        RustDeclarationVisibility::Crate
    );
    assert!(facts.types.iter().any(|t| t.path.text.ends_with("::Raw")));
}
#[test]
fn generic_lifetime_const_bounds_defaults_and_trait_implementations_remain_source_facts() {
    let facts = read_source_declarations(&fixture()).unwrap();
    let generic = named(&facts, "::declarations::Generic<'static,String,4>");
    assert_eq!(
        generic
            .parameters
            .iter()
            .map(|p| p.kind)
            .collect::<Vec<_>>(),
        [
            RustGenericParameterKind::Lifetime,
            RustGenericParameterKind::Type,
            RustGenericParameterKind::Const
        ]
    );
    assert_eq!(
        generic
            .parameters
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        ["'a", "T", "N"]
    );
    assert!(generic.parameters[2].declaration.contains("= 4"));
    assert!(generic.where_clause.as_ref().unwrap().contains("Default"));
    let child = named(&facts, "::declarations::IChild<String>");
    assert_eq!(child.supertraits[0].text, "::declarations::IRoot");
    assert!(child.where_clause.as_ref().unwrap().contains("Clone"));
    assert_eq!(facts.implementations.len(), 2);
    let implementation = &facts.implementations[1];
    assert!(!implementation.negative);
    assert!(implementation.trait_type.text.contains("IChild"));
    assert_eq!(implementation.parameters.len(), 3);
    assert!(implementation.self_type.unresolved.iter().any(|s| s == "T"));
}
#[test]
fn path_modules_inherit_conditions_and_ambiguous_conditional_definitions_are_not_selected() {
    let facts = read_source_declarations(&fixture()).unwrap();
    let moved = named(&facts, "::declarations::moved::Moved");
    assert_eq!(moved.cfg.len(), 2);
    assert!(moved.cfg[0].contains("outer"));
    assert!(moved.cfg[1].contains("inner"));
    assert_eq!(moved.module, "declarations::moved");
    assert!(moved.location.contains("elsewhere.rs"));
    assert_eq!(
        facts
            .types
            .iter()
            .filter(|t| t.path.text.ends_with("::Conditional"))
            .count(),
        2
    );
    assert!(facts
        .definition(&RustType::resolved("::declarations::nested::Conditional"))
        .is_none());
    assert!(!facts.types.iter().any(
        |t| t.path.text.ends_with("::NotProduction") || t.path.text.ends_with("::NotExpanded")
    ));
    assert!(facts
        .definition(&RustType::resolved("::declarations::Alias"))
        .is_none());
    assert!(facts
        .definition(&RustType::resolved("Option<::declarations::Plain>"))
        .is_none());
}
#[test]
fn actual_application_fixture_definitions_supply_kind_visibility_and_generic_facts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/Dock/FerroDock.Serializer.Generated.Tests/payload_metadata.rs");
    let mut options = ScanOptions::new("ferrodock_serializer_generated_tests", root);
    options.extern_crates = vec![
        "ferrodock_model".into(),
        "ferrodock_serializer_system_text_json".into(),
    ];
    let scan = scan_crate(&options);
    let errors: Vec<_> = scan.diagnostics_of(Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:?}");
    let facts = read_source_declarations(&scan).unwrap();
    let ty = scan
        .normalise(
            &scan.model.crate_name,
            "crate::payload_models::Generic<i32>",
        )
        .unwrap();
    let generic = facts.definition(&ty).unwrap();
    assert_eq!(generic.kind, RustDeclarationKind::Struct);
    assert_eq!(generic.visibility, RustDeclarationVisibility::Public);
    assert_eq!(generic.parameters.len(), 1);
    assert_eq!(generic.parameters[0].kind, RustGenericParameterKind::Type);
    let ty = scan
        .normalise(&scan.model.crate_name, "crate::payload_models::Payload")
        .unwrap();
    let payload = facts.definition(&ty).unwrap();
    assert!(payload.cfg.is_empty());
    assert!(payload.parameters.is_empty());
    assert!(payload.location.contains("payload_models.rs"));
}

#[test]
fn actual_docking_contract_supertraits_are_read_from_original_rust_engine_declarations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../FerroDock.Model/lib.rs");
    let mut options = ScanOptions::new("ferrodock_model", root);
    options.extern_crates = vec![
        "ferroui_system".into(),
        "ferroui_reactiveui".into(),
        "ferrodock_controls_recycling_model".into(),
    ];
    let scan = scan_crate(&options);
    let facts = read_source_declarations(&scan).unwrap();
    let dockable = scan
        .normalise(&scan.model.crate_name, "crate::core::IDockable")
        .unwrap();
    let dock = scan
        .normalise(&scan.model.crate_name, "crate::core::IDock")
        .unwrap();
    let root = scan
        .normalise(&scan.model.crate_name, "crate::controls::IRootDock")
        .unwrap();
    let dock = facts.definition(&dock).unwrap();
    assert_eq!(dock.kind, RustDeclarationKind::Trait);
    assert!(dock.supertraits.iter().any(|t| t.text == dockable.text));
    let root = facts.definition(&root).unwrap();
    assert!(
        root.supertraits.iter().any(|t| t.text == dock.path.text),
        "root {:?}; dock {:?}",
        root.supertraits,
        dock.path
    );
    assert!(dock.location.contains("i_dock.rs"));
    assert!(root.location.contains("i_root_dock.rs"));
}

#[test]
fn source_file_graph_mismatches_and_unresolved_definition_lookups_fail_explicitly() {
    let mut scan = fixture();
    let duplicate = scan.files[0].clone();
    scan.files.push(duplicate);
    assert!(read_source_declarations(&scan)
        .unwrap_err()
        .0
        .contains("Duplicate source module"));
    let mut scan = fixture();
    scan.files[0].module = "missing".into();
    assert!(read_source_declarations(&scan)
        .unwrap_err()
        .0
        .contains("has no file"));
    let facts = read_source_declarations(&fixture()).unwrap();
    let unresolved = RustType {
        text: "::declarations::Plain".into(),
        unresolved: vec!["Unknown".into()],
    };
    assert!(facts.definition(&unresolved).is_none());
    assert!(facts
        .definition(&RustType::resolved("&::declarations::Plain"))
        .is_none());
    assert!(facts
        .definition(&RustType::resolved("invalid type syntax"))
        .is_none());
}

#[test]
fn dependency_public_reexports_resolve_through_actual_shared_source_models() {
    use ferroui_build_scan::model_set::ModelSet;
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/source_dependency/lib.rs");
    let dependency = scan_crate(&ScanOptions::new("source_dependency", root));
    let scan = fixture();
    let own = read_source_declarations(&scan).unwrap();
    assert_eq!(
        named(&own, "::declarations::IDependency").supertraits[0].text,
        "::source_dependency::Facade"
    );
    let models = ModelSet::new(vec![scan.model.clone(), dependency.model]);
    let facts = read_source_declarations_with_models(&scan, &models).unwrap();
    assert_eq!(
        named(&facts, "::declarations::IDependency").supertraits[0].text,
        "::source_dependency::definitions::Contract"
    );
    assert!(
        read_source_declarations_with_models(&scan, &ModelSet::new(Vec::new()))
            .unwrap_err()
            .0
            .contains("omits the source crate")
    );
}
