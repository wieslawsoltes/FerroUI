// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use crate::compiler_fixture_input as fixture_input;
use ferrodock_serializer_system_text_json_generators::*;
use ferroui_build_scan::{
    model::{AttributeModel, RustType, TypeModel},
    scanner::{scan_crate, ScanOptions},
};
fn input() -> (
    Compilation,
    TypeModel,
    RustSourceDeclaration,
    RustSemanticTypeOptions,
    TypeId,
) {
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
    let c = fixture_input::compilation(&row["input"]);
    let mut options = ScanOptions::new(
        "ferrodock_serializer_generated_tests",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/Dock/FerroDock.Serializer.Generated.Tests/payload_metadata.rs"),
    );
    options.extern_crates = vec![
        "ferrodock_model".into(),
        "ferrodock_serializer_system_text_json".into(),
    ];
    let scan = scan_crate(&options);
    let declarations = read_source_declarations(&scan).unwrap();
    let metadata = scan
        .model
        .types
        .iter()
        .find(|t| t.name == "GeneratedPayload")
        .unwrap()
        .clone();
    let source = declarations
        .definition(&metadata.rust_path)
        .unwrap()
        .clone();
    let object = c
        .types
        .iter()
        .position(|t| t.namespace == "System" && t.metadata_name == "Object")
        .unwrap();
    let string = c
        .types
        .iter()
        .position(|t| t.namespace == "System" && t.metadata_name == "String")
        .unwrap();
    let options = RustSemanticTypeOptions {
        metadata_name: metadata.name.clone(),
        namespace: metadata.namespace.clone(),
        assembly: Some(0),
        accessibility: Accessibility::Public,
        is_abstract: false,
        containing_type: None,
        base_type: Some(object),
        all_interfaces: Vec::new(),
        property_overrides: Vec::new(),
    };
    (c, metadata, source, options, string)
}
fn resolve(ty: &RustType, string: TypeId, own: TypeId) -> Option<TypeId> {
    if ty.text == "Option<String>" {
        Some(string)
    } else if ty.text.contains("::Payload") {
        Some(own)
    } else {
        None
    }
}
#[test]
fn actual_metadata_builds_fresh_source_symbols_and_self_references_without_old_class_rows() {
    let (mut c, metadata, source, options, string) = input();
    let own = c.types.len();
    let first = c.properties.len();
    bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| {
        resolve(t, string, own)
    })
    .unwrap();
    let symbol = &c.types[own];
    assert_eq!(symbol.metadata_name, "GeneratedPayload");
    assert_eq!(symbol.namespace, "");
    assert_eq!(symbol.kind, TypeKind::Class);
    assert_eq!(symbol.expression, source.path.text);
    assert_eq!(symbol.location.as_deref(), Some(source.location.as_str()));
    assert_eq!(symbol.base_type, options.base_type);
    assert_eq!(symbol.properties, [first, first + 1]);
    assert_eq!(c.properties[first].name, "Name");
    assert_eq!(c.properties[first].type_id, string);
    assert_eq!(c.properties[first + 1].name, "Self");
    assert_eq!(c.properties[first + 1].type_id, own);
    assert_eq!(full_name(&c, own).unwrap(), "GeneratedPayload");
}
#[test]
fn source_ignores_overrides_and_hiding_feed_the_original_semantic_algorithm() {
    let (mut c, mut metadata, source, options, string) = input();
    metadata.properties[0].attributes.push(AttributeModel {
        name: "IgnoreDataMember".into(),
        ..Default::default()
    });
    let base = c.types.len();
    bind_declared_semantic_type(&mut c, base, &metadata, &source, &options, |t| {
        resolve(t, string, base)
    })
    .unwrap();
    let parent = c.types[base].properties[0];
    metadata.name = "NativeDerived".into();
    metadata.properties[0].attributes.clear();
    let mut options = options;
    options.metadata_name = metadata.name.clone();
    options.base_type = Some(base);
    options.property_overrides = vec![RustPropertyOverride {
        name: "Name".into(),
        overridden_property: parent,
    }];
    let derived = c.types.len();
    bind_declared_semantic_type(&mut c, derived, &metadata, &source, &options, |t| {
        resolve(t, string, derived)
    })
    .unwrap();
    c.activated = true;
    c.source_types.clear();
    c.registrations = vec![Registration {
        type_id: Some(derived),
        valid_type_constant: true,
        display_name: "NativeDerived".into(),
        location: None,
    }];
    let model = build(&c).unwrap();
    assert_eq!(model.serializable_types[0].ignored_members, ["Name"]);
    options.property_overrides.clear();
    let hidden = c.types.len();
    metadata.name = "NativeHidden".into();
    options.metadata_name = metadata.name.clone();
    bind_declared_semantic_type(&mut c, hidden, &metadata, &source, &options, |t| {
        resolve(t, string, hidden)
    })
    .unwrap();
    c.registrations[0].type_id = Some(hidden);
    assert!(build(&c).unwrap().serializable_types[0]
        .ignored_members
        .is_empty());
}
#[test]
fn source_command_and_static_rules_keep_original_names_before_json_renaming() {
    let (mut c, mut metadata, source, options, string) = input();
    let command = c.required_symbols[6].unwrap();
    let own = c.types.len();
    let mut property = metadata.properties[0].clone();
    property.name = "Command".into();
    property.value_type = RustType::resolved("NativeCommand");
    metadata.properties.push(property.clone());
    property.name = "StaticCommand".into();
    metadata.static_properties.push(property);
    metadata.property_attributes.push((
        "Name".into(),
        vec![AttributeModel {
            name: "System.Runtime.Serialization.IgnoreDataMemberAttribute".into(),
            ..Default::default()
        }],
    ));
    metadata.properties[0].attributes.push(AttributeModel {
        name: "JsonPropertyName".into(),
        arguments: vec![ferroui_build_scan::model::AttributeValueModel::Str(
            "wire".into(),
        )],
        ..Default::default()
    });
    bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| {
        if t.text == "NativeCommand" {
            Some(command)
        } else {
            resolve(t, string, own)
        }
    })
    .unwrap();
    c.source_types.clear();
    c.registrations = vec![Registration {
        type_id: Some(own),
        valid_type_constant: true,
        display_name: "payload".into(),
        location: None,
    }];
    let model = build(&c).unwrap();
    assert_eq!(
        model.serializable_types[0].ignored_members,
        ["Command", "Name"]
    );
    assert!(c.properties[*c.types[own].properties.last().unwrap()].is_static);
}
#[test]
fn unresolved_members_and_invalid_compiler_references_do_not_commit_partial_types_or_properties() {
    let (mut c, metadata, source, options, string) = input();
    let own = c.types.len();
    let before = format!("{c:?}");
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |_| None)
            .unwrap_err()
            .0
            .contains("Missing semantic type binding")
    );
    assert_eq!(format!("{c:?}"), before);
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |_| Some(
            usize::MAX
        ))
        .unwrap_err()
        .0
        .contains("Missing semantic target")
    );
    assert_eq!(format!("{c:?}"), before);
    let mut invalid = options;
    invalid.assembly = Some(usize::MAX);
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &invalid, |t| resolve(
            t, string, own
        ))
        .unwrap_err()
        .0
        .contains("assembly")
    );
    assert_eq!(format!("{c:?}"), before);
}
#[test]
fn unsupported_generic_cfg_and_incomplete_inheritance_decisions_are_explicit_errors() {
    let (mut c, mut metadata, mut source, mut options, string) = input();
    let own = c.types.len();
    source.cfg.push("feature=x".into());
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| resolve(
            t, string, own
        ))
        .unwrap_err()
        .0
        .contains("cfg selection")
    );
    source.cfg.clear();
    source.parameters.push(RustGenericParameter {
        name: "T".into(),
        kind: RustGenericParameterKind::Type,
        declaration: "T".into(),
    });
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| resolve(
            t, string, own
        ))
        .unwrap_err()
        .0
        .contains("substitutions")
    );
    source.parameters.clear();
    metadata
        .interfaces
        .push(RustType::resolved("NativeInterface"));
    let interface = c.required_symbols[0].unwrap();
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| {
            if t.text == "NativeInterface" {
                Some(interface)
            } else {
                resolve(t, string, own)
            }
        })
        .unwrap_err()
        .0
        .contains("closure")
    );
    options.all_interfaces = vec![interface, interface];
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |_| Some(
            interface
        ))
        .unwrap_err()
        .0
        .contains("duplicate semantic interface")
    );
}
#[test]
fn abstraction_is_explicit_and_missing_constructor_does_not_make_the_class_abstract() {
    let (mut c, mut metadata, source, mut options, string) = input();
    metadata.constructors.clear();
    metadata.default_constructor = None;
    let own = c.types.len();
    bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| {
        resolve(t, string, own)
    })
    .unwrap();
    assert!(!c.types[own].is_abstract);
    options.is_abstract = true;
    bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| {
        resolve(t, string, own)
    })
    .unwrap();
    assert!(c.types[own].is_abstract);
    c.source_types.clear();
    c.registrations = vec![Registration {
        type_id: Some(own),
        valid_type_constant: true,
        display_name: "abstract".into(),
        location: None,
    }];
    assert_eq!(build(&c).unwrap().diagnostics[0].id, "DSTJ001");
}
#[test]
fn base_and_containment_cycles_and_unrelated_overrides_are_rejected() {
    let (mut c, metadata, source, mut options, string) = input();
    let own = c.types.len();
    options.base_type = Some(own);
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| resolve(
            t, string, own
        ))
        .unwrap_err()
        .0
        .contains("base class")
    );
    options.base_type = None;
    options.containing_type = Some(own);
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| resolve(
            t, string, own
        ))
        .unwrap_err()
        .0
        .contains("containing type")
    );
    options.containing_type = None;
    let property = c.properties.iter().position(|p| p.name == "Name").unwrap();
    options.property_overrides = vec![RustPropertyOverride {
        name: "Name".into(),
        overridden_property: property,
    }];
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| resolve(
            t, string, own
        ))
        .unwrap_err()
        .0
        .contains("original base property")
    );
}

#[test]
fn native_value_and_interface_declarations_retain_logical_kinds_and_abstraction_contracts() {
    use ferroui_build_scan::model::TypeKind as MetadataKind;
    let (mut c, _, _, mut options, _) = input();
    options.base_type = None;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/source_declarations/lib.rs");
    let mut scan_options = ScanOptions::new("declarations", root);
    scan_options.extern_crates = vec!["source_dependency".into()];
    let scan = scan_crate(&scan_options);
    let declarations = read_source_declarations(&scan).unwrap();
    for (name, metadata_kind, kind, abstract_) in [
        ("Plain", MetadataKind::Struct, TypeKind::Struct, false),
        ("Choice", MetadataKind::Enum, TypeKind::Enum, false),
        ("IRoot", MetadataKind::Interface, TypeKind::Interface, true),
    ] {
        let path = scan.normalise(&scan.model.crate_name, name).unwrap();
        let source = declarations.definition(&path).unwrap();
        let metadata = TypeModel::new(name, metadata_kind, path, source.module.as_str());
        options.metadata_name = name.into();
        options.is_abstract = abstract_;
        let id = c.types.len();
        bind_declared_semantic_type(&mut c, id, &metadata, source, &options, |_| None).unwrap();
        assert_eq!(c.types[id].kind, kind);
        assert_eq!(c.types[id].is_abstract, abstract_);
        options.is_abstract = !abstract_;
        assert!(
            bind_declared_semantic_type(&mut c, id, &metadata, source, &options, |_| None)
                .unwrap_err()
                .0
                .contains("Abstraction differs")
        );
    }
}

#[test]
fn numeric_enum_semantics_require_both_source_storage_and_explicit_shared_factory() {
    use ferroui_build_scan::model::TypeKind as MetadataKind;
    let (mut c, _, _, mut options, _) = input();
    options.base_type = None;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/source_declarations/lib.rs");
    let mut scan_options = ScanOptions::new("declarations", root);
    scan_options.extern_crates = vec!["source_dependency".into()];
    let scan = scan_crate(&scan_options);
    let declarations = read_source_declarations(&scan).unwrap();
    for name in [
        "NumericTuple",
        "NumericNamed",
        "NumericUnsigned",
        "NumericC",
        "NumericDefault",
        "NumericMarker",
        "NumericConditional",
        "NumericConditionalLayout",
    ] {
        let path = scan.normalise(&scan.model.crate_name, name).unwrap();
        let source = declarations.definition(&path).unwrap();
        options.metadata_name = name.into();
        for factory in [
            None,
            Some(""),
            Some("not::an::associated_function"),
            Some("typed_cast"),
        ] {
            let mut metadata =
                TypeModel::new(name, MetadataKind::Enum, path.clone(), &source.module);
            metadata.enum_i32_factory = factory.map(str::to_owned);
            let id = c.types.len();
            let properties = c.properties.len();
            let result =
                bind_declared_semantic_type(&mut c, id, &metadata, source, &options, |_| None);
            let valid =
                matches!(name, "NumericTuple" | "NumericNamed") && factory == Some("typed_cast");
            if valid {
                result.unwrap();
                assert_eq!(c.types[id].kind, TypeKind::Enum);
                assert_eq!(c.types[id].expression, source.path.text);
                assert_eq!(
                    c.types[id].location.as_deref(),
                    Some(source.location.as_str())
                );
            } else {
                assert!(result.is_err(), "{name}, factory {factory:?}");
                assert_eq!(c.types.len(), id, "failed binding remains atomic");
                assert_eq!(c.properties.len(), properties);
            }
        }
    }
}

#[test]
fn actual_open_window_enum_sources_supply_native_semantic_representation_provenance() {
    use ferroui_build_scan::model::TypeKind as MetadataKind;
    let (mut c, _, _, mut options, _) = input();
    options.base_type = None;
    options.namespace = "Dock.Model.Core".into();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../FerroDock.Model/lib.rs");
    let mut scan_options = ScanOptions::new("ferrodock_model", root);
    scan_options.extern_crates = vec![
        "ferroui_system".into(),
        "ferroui_reactiveui".into(),
        "ferrodock_controls_recycling_model".into(),
    ];
    let scan = scan_crate(&scan_options);
    let declarations = read_source_declarations(&scan).unwrap();
    for name in ["DockWindowState", "DockWindowOwnerMode"] {
        let path = scan
            .normalise(&scan.model.crate_name, &format!("crate::core::{name}"))
            .unwrap();
        let source = declarations.definition(&path).unwrap();
        assert_eq!(source.kind, RustDeclarationKind::Struct);
        assert_eq!(source.transparent_field.as_ref().unwrap().text, "i32");
        options.metadata_name = name.into();
        let mut metadata = TypeModel::new(name, MetadataKind::Enum, path, &source.module);
        metadata.enum_i32_factory = Some("__markup_enum_i32".into());
        let id = c.types.len();
        bind_declared_semantic_type(&mut c, id, &metadata, source, &options, |_| None).unwrap();
        assert_eq!(c.types[id].kind, TypeKind::Enum);
        assert_eq!(
            full_name(&c, id).unwrap(),
            format!("Dock.Model.Core.{name}")
        );
        assert_eq!(c.types[id].expression, source.path.text);
    }
}

#[test]
fn source_serialization_identity_is_independent_of_a_markup_alias() {
    let (mut c, mut metadata, source, mut options, string) = input();
    metadata.name = "MarkupAlias".into();
    metadata.namespace = "Markup.Namespace".into();
    options.metadata_name = "OriginalPayload".into();
    options.namespace = "Original.Contract".into();
    let own = c.types.len();
    bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| {
        resolve(t, string, own)
    })
    .unwrap();
    assert_eq!(
        full_name(&c, own).unwrap(),
        "Original.Contract.OriginalPayload"
    );
    assert_eq!(c.types[own].expression, source.path.text);
    options.metadata_name.clear();
    let before = format!("{c:?}");
    assert!(
        bind_declared_semantic_type(&mut c, own, &metadata, &source, &options, |t| resolve(
            t, string, own
        ))
        .unwrap_err()
        .0
        .contains("logical source type identity")
    );
    assert_eq!(format!("{c:?}"), before);
}
