// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use crate as native_adapter;
use crate::compiler_fixture_input as fixture_input;
use ferrodock_serializer_system_text_json_generators::*;
use ferroui_build_scan::{
    model::{RustType, TypeModel},
    scanner::{scan_crate, Scan, ScanOptions},
};
#[path = "../../../tests/Dock/FerroDock.Serializer.Generated.Tests/payload_dependency_reference.rs"]
mod dependency_reference;

fn fixture() -> (Compilation, Scan, RustSourceDeclarations, usize, usize) {
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
    let mut o = ScanOptions::new(
        "ferrodock_serializer_generated_tests",
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../tests/Dock/FerroDock.Serializer.Generated.Tests/payload_metadata.rs"),
    );
    o.extern_crates = vec![
        "ferrodock_model".into(),
        "ferrodock_serializer_system_text_json".into(),
    ];
    let scan = scan_crate(&o);
    let declarations = read_source_declarations(&scan).unwrap();
    let builtin = |name| {
        c.types
            .iter()
            .position(|t| t.namespace == "System" && t.metadata_name == name)
            .unwrap()
    };
    let (object, string) = (builtin("Object"), builtin("String"));
    (c, scan, declarations, object, string)
}
fn metadata(scan: &Scan, name: &str) -> TypeModel {
    scan.model
        .types
        .iter()
        .find(|t| t.name == name)
        .unwrap()
        .clone()
}
fn options(m: &TypeModel, object: usize) -> RustSemanticTypeOptions {
    RustSemanticTypeOptions {
        metadata_name: m.name.clone(),
        namespace: m.namespace.clone(),
        assembly: Some(0),
        accessibility: Accessibility::Public,
        is_abstract: false,
        containing_type: None,
        base_type: Some(object),
        all_interfaces: Vec::new(),
        property_overrides: Vec::new(),
    }
}

#[test]
fn forward_and_mutual_property_references_allocate_native_ids_without_seed_headers() {
    let (mut c, scan, source, object, string) = fixture();
    let holder = metadata(&scan, "GeneratedHolder");
    let mut payload = metadata(&scan, "GeneratedPayload");
    // Compiler metadata is authoritative for property types. Exercise a mutual graph
    // by redirecting the fixture's self property to the other actual declaration.
    payload.properties[1].value_type = RustType::resolved("native_holder_handle");
    let opts = [options(&holder, object), options(&payload, object)];
    let declarations: Vec<_> = [&holder, &payload]
        .into_iter()
        .zip(&opts)
        .map(|(m, o)| RustSemanticDeclaration {
            metadata: m,
            source: source.definition(&m.rust_path).unwrap(),
            options: o,
        })
        .collect();
    let first = c.types.len();
    let discovery = c.source_types.clone();
    let ids = bind_semantic_declarations(&mut c, &declarations, |ty, ids| {
        if ty.text == "native_holder_handle" {
            Some(ids[0])
        } else if ty.text.contains("::Payload") {
            Some(ids[1])
        } else if ty.text == "Option<String>" {
            Some(string)
        } else if ty.text.contains("BoxedValue") {
            Some(object)
        } else {
            None
        }
    })
    .unwrap();
    assert_eq!(ids, [first, first + 1]);
    let props = |id: usize| {
        c.types[id]
            .properties
            .iter()
            .map(|p| &c.properties[*p])
            .collect::<Vec<_>>()
    };
    assert_eq!(props(ids[0])[2].type_id, ids[1]);
    assert_eq!(props(ids[1])[1].type_id, ids[0]);
    assert_eq!(c.types[ids[0]].expression, holder.rust_path.text);
    assert_eq!(c.types[ids[1]].expression, payload.rust_path.text);
    assert_eq!(c.source_types, discovery);
}

#[test]
fn later_member_failure_rolls_back_the_entire_compilation() {
    let (mut c, scan, source, object, string) = fixture();
    let a = metadata(&scan, "GeneratedPayload");
    let mut b = metadata(&scan, "GeneratedTypeNamed");
    b.properties[1].value_type = RustType::resolved("missing");
    let opts = [options(&a, object), options(&b, object)];
    let declarations: Vec<_> = [&a, &b]
        .into_iter()
        .zip(&opts)
        .map(|(m, o)| RustSemanticDeclaration {
            metadata: m,
            source: source.definition(&m.rust_path).unwrap(),
            options: o,
        })
        .collect();
    let before = format!("{c:?}");
    let error = bind_semantic_declarations(&mut c, &declarations, |ty, ids| {
        if ty.text.contains("::Payload") {
            Some(ids[0])
        } else if ty.text == "Option<String>" {
            Some(string)
        } else {
            None
        }
    })
    .unwrap_err();
    assert!(error.0.contains("missing"));
    assert_eq!(format!("{c:?}"), before);
}

#[test]
fn duplicate_identities_and_out_of_batch_references_are_rejected_atomically() {
    let (mut c, scan, source, object, string) = fixture();
    let m = metadata(&scan, "GeneratedPayload");
    let o = options(&m, object);
    let d = || RustSemanticDeclaration {
        metadata: &m,
        source: source.definition(&m.rust_path).unwrap(),
        options: &o,
    };
    let before = format!("{c:?}");
    assert!(
        bind_semantic_declarations(&mut c, &[d(), d()], |_, _| Some(string))
            .unwrap_err()
            .0
            .contains("Duplicate")
    );
    assert_eq!(format!("{c:?}"), before);
    assert!(
        bind_semantic_declarations(&mut c, &[d()], |_, ids| Some(ids[0] + 1))
            .unwrap_err()
            .0
            .contains("Missing semantic target")
    );
    assert_eq!(format!("{c:?}"), before);
    let ids = bind_semantic_declarations(&mut c, &[d()], |ty, ids| {
        Some(if ty.text == "Option<String>" {
            string
        } else {
            ids[0]
        })
    })
    .unwrap();
    assert_eq!(ids.len(), 1);
    let before = format!("{c:?}");
    assert!(
        bind_semantic_declarations(&mut c, &[d()], |_, _| Some(string))
            .unwrap_err()
            .0
            .contains("Duplicate")
    );
    assert_eq!(format!("{c:?}"), before);
    assert!(bind_semantic_declarations(&mut c, &[], |_, _| None)
        .unwrap()
        .is_empty());
    assert_eq!(format!("{c:?}"), before);
}

#[test]
fn closed_generic_arguments_can_reference_a_later_native_header() {
    let (mut c, scan, source, object, string) = fixture();
    let payload = metadata(&scan, "GeneratedPayload");
    let template = metadata(&scan, "GeneratedGeneric`1");
    let target = scan
        .normalise(
            &scan.model.crate_name,
            "crate::payload_models::Generic<::std::rc::Rc<crate::payload_models::Payload>>",
        )
        .unwrap();
    let generic_source = source.definition(&target).unwrap();
    let generic = instantiate_generic_metadata(&template, generic_source, &target, |text| {
        scan.normalise(&template.module, text)
    })
    .unwrap();
    let opts = [options(&generic, object), options(&payload, object)];
    let ds = [
        RustSemanticDeclaration {
            metadata: &generic,
            source: generic_source,
            options: &opts[0],
        },
        RustSemanticDeclaration {
            metadata: &payload,
            source: source.definition(&payload.rust_path).unwrap(),
            options: &opts[1],
        },
    ];
    let ids = bind_semantic_declarations(&mut c, &ds, |ty, ids| {
        Some(if ty.text == "Option<String>" {
            string
        } else {
            ids[1]
        })
    })
    .unwrap();
    assert_eq!(c.types[ids[0]].type_arguments, [ids[1]]);
    assert_eq!(c.properties[c.types[ids[0]].properties[0]].type_id, ids[1]);
    assert_eq!(full_name(&c,ids[0]).unwrap(), "GeneratedGeneric`1[[GeneratedPayload, Dock.Model, Version=12.1.0.7, Culture=neutral, PublicKeyToken=535dd7fb7a6cf113]]");
}

#[test]
fn late_generic_argument_errors_do_not_publish_valid_earlier_headers() {
    let (mut c, scan, source, object, string) = fixture();
    let payload = metadata(&scan, "GeneratedPayload");
    let template = metadata(&scan, "GeneratedGeneric`1");
    let target = scan
        .normalise(
            &scan.model.crate_name,
            "crate::payload_models::Generic<Option<String>>",
        )
        .unwrap();
    let generic_source = source.definition(&target).unwrap();
    let generic = instantiate_generic_metadata(&template, generic_source, &target, |text| {
        scan.normalise(&template.module, text)
    })
    .unwrap();
    let opts = [options(&payload, object), options(&generic, object)];
    let ds = [
        RustSemanticDeclaration {
            metadata: &payload,
            source: source.definition(&payload.rust_path).unwrap(),
            options: &opts[0],
        },
        RustSemanticDeclaration {
            metadata: &generic,
            source: generic_source,
            options: &opts[1],
        },
    ];
    c.types[string].kind = TypeKind::TypeParameter;
    let before = format!("{c:?}");
    assert!(bind_semantic_declarations(&mut c, &ds, |ty, ids| Some(
        if ty.text.contains("::Payload") {
            ids[0]
        } else {
            string
        }
    ))
    .unwrap_err()
    .0
    .contains("not closed"));
    assert_eq!(format!("{c:?}"), before);
}

#[test]
fn inheritance_requires_real_preceding_headers_and_has_no_placeholder_fallback() {
    let (mut c, scan, source, object, string) = fixture();
    let payload = metadata(&scan, "GeneratedPayload");
    let named = metadata(&scan, "GeneratedTypeNamed");
    let mut a = options(&payload, object);
    a.base_type = Some(c.types.len() + 1);
    let b = options(&named, object);
    let ds = [
        RustSemanticDeclaration {
            metadata: &payload,
            source: source.definition(&payload.rust_path).unwrap(),
            options: &a,
        },
        RustSemanticDeclaration {
            metadata: &named,
            source: source.definition(&named.rust_path).unwrap(),
            options: &b,
        },
    ];
    let before = format!("{c:?}");
    assert!(bind_semantic_declarations(&mut c, &ds, |_, _| Some(string))
        .unwrap_err()
        .0
        .contains("Invalid semantic base"));
    assert_eq!(format!("{c:?}"), before);
}

#[test]
fn application_snapshot_data_is_excluded_from_the_dependency_reference() {
    let (c, _, _, _, _) = fixture();
    let dependencies = dependency_reference::compilation(c.clone());
    let mut poisoned = c;
    for t in &mut poisoned.types {
        if t.assembly
            .is_some_and(|a| poisoned.assemblies[a].grants_internal_access)
        {
            for p in &t.properties {
                poisoned.properties[*p].type_id = usize::MAX;
                poisoned.properties[*p].overridden_property = Some(usize::MAX);
            }
            t.expression = "discarded_application_type".into();
            t.metadata_name = "discarded_application_name".into();
            t.kind = TypeKind::OtherNonNamed;
            t.properties = vec![usize::MAX];
            t.base_type = Some(usize::MAX);
            t.type_arguments = vec![usize::MAX];
        }
    }
    poisoned.activated = !poisoned.activated;
    poisoned.registrations.clear();
    poisoned.source_types = vec![usize::MAX];
    poisoned.occupied_type_names = vec!["discarded_application_scope".into()];
    assert_eq!(
        format!("{:?}", dependency_reference::compilation(poisoned)),
        format!("{dependencies:?}")
    );
    assert!(!dependencies.activated);
    assert!(dependencies.registrations.is_empty());
    assert!(dependencies.source_types.is_empty());
    assert!(dependencies.occupied_type_names.is_empty());
    for (id, name) in dependencies
        .required_symbols
        .into_iter()
        .zip(METADATA_NAMES)
    {
        assert_eq!(full_name(&dependencies, id.unwrap()).unwrap(), name);
    }
}

#[test]
fn preceding_native_base_headers_and_original_override_rows_are_available_to_dependents() {
    let (mut c, scan, source, object, string) = fixture();
    let base = metadata(&scan, "GeneratedPayload");
    let derived = metadata(&scan, "GeneratedTypeNamed");
    let first = c.types.len();
    let parent_name = c.properties.len();
    let a = options(&base, object);
    let mut b = options(&derived, object);
    b.base_type = Some(first);
    b.property_overrides = vec![RustPropertyOverride {
        name: "Name".into(),
        overridden_property: parent_name,
    }];
    let ds = [
        RustSemanticDeclaration {
            metadata: &base,
            source: source.definition(&base.rust_path).unwrap(),
            options: &a,
        },
        RustSemanticDeclaration {
            metadata: &derived,
            source: source.definition(&derived.rust_path).unwrap(),
            options: &b,
        },
    ];
    let ids = bind_semantic_declarations(&mut c, &ds, |ty, ids| {
        Some(if ty.text.contains("::Payload") {
            ids[0]
        } else {
            string
        })
    })
    .unwrap();
    assert_eq!(c.types[ids[1]].base_type, Some(ids[0]));
    let name = c.types[ids[1]].properties[1];
    assert_eq!(c.properties[name].name, "Name");
    assert_eq!(c.properties[name].overridden_property, Some(parent_name));
}
