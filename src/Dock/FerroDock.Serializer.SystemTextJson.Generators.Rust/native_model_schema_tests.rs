// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate as native_adapter;
use crate::compiler_fixture_input as fixture_input;
use ferrodock_serializer_system_text_json_generators::*;
use ferroui_build_scan::{
    model_set::ModelSet,
    scanner::{scan_crate, Scan, ScanOptions, Severity},
};
#[path = "../../../tests/Dock/FerroDock.Markup.Tests/native_model_schemas.rs"]
mod native_model_schemas;

fn fixture() -> (Compilation, Scan, ModelSet) {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/Dock/data/json-generator-semantics.json"
    ))
    .unwrap();
    let row = rows
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "actual-models")
        .unwrap();
    let reference = fixture_input::compilation(&row["input"]);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    let mut options = ScanOptions::new(
        "ferrodock_model_reactiveui_ferro",
        root.join("FerroDock.Model.ReactiveUI.Ferro/lib.rs"),
    );
    options.extern_crates = [
        "ferrodock_model_ferro",
        "ferrodock_model",
        "ferrodock_model_reactiveui",
        "ferrodock_controls_recycling",
        "ferroui_base",
        "ferroui_reactiveui",
        "ferroui_system",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let scan = scan_crate(&options);
    let mut options = ScanOptions::new(
        "ferrodock_model_ferro",
        root.join("FerroDock.Model.Ferro/lib.rs"),
    );
    options.extern_crates = [
        "ferrodock_model",
        "ferrodock_runtime",
        "ferrodock_controls_recycling_model_ferro",
        "ferroui_base",
        "ferroui_reactiveui",
        "ferroui_system",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let core = scan_crate(&options);
    for scan in [&scan, &core] {
        let errors: Vec<_> = scan.diagnostics_of(Severity::Error).collect();
        assert!(errors.is_empty(), "{errors:?}");
    }
    let models = ModelSet::new(vec![scan.model.clone(), core.model]);
    (reference, scan, models)
}

fn type_key(c: &Compilation, id: TypeId) -> String {
    let t = &c.types[id];
    format!(
        "{}.{}[{}]",
        t.namespace,
        t.metadata_name,
        t.type_arguments
            .iter()
            .map(|id| type_key(c, *id))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn properties(
    c: &Compilation,
    id: TypeId,
) -> Vec<(String, String, bool, bool, bool, Option<String>)> {
    c.types[id]
        .properties
        .iter()
        .map(|id| {
            let p = &c.properties[*id];
            (
                p.name.clone(),
                type_key(c, p.type_id),
                p.is_public,
                p.is_static,
                p.ignore_data_member,
                p.overridden_property
                    .map(|id| c.properties[id].name.clone()),
            )
        })
        .collect()
}

#[test]
fn actual_sdk_class_schemas_match_original_headers_types_and_ordered_members() {
    let (reference, scan, models) = fixture();
    let source_ids = native_model_schemas::source_ids(&reference);
    let native = native_model_schemas::compilation(&reference, &scan, &models, &source_ids);
    let c = &native.compilation;
    assert_eq!(native.native_types.len(), 7);
    assert!(native
        .native_types
        .windows(2)
        .all(|pair| pair[1] == pair[0] + 1));
    assert_eq!(
        native
            .native_types
            .iter()
            .map(|id| c.types[*id].properties.len())
            .sum::<usize>(),
        101
    );
    let expected = build(&reference).unwrap();
    let actual = build(c).unwrap();
    assert!(expected.diagnostics.is_empty() && actual.diagnostics.is_empty());
    for old in source_ids {
        let id = native.source_types[old].unwrap();
        let source = &reference.types[old];
        let target = &c.types[id];
        assert_eq!(type_key(c, id), type_key(&reference, old));
        assert_eq!(target.kind, source.kind);
        assert_eq!(target.is_abstract, source.is_abstract);
        assert_eq!(target.accessibility, source.accessibility);
        assert_eq!(
            target.base_type.map(|id| type_key(c, id)),
            source.base_type.map(|id| type_key(&reference, id))
        );
        assert_eq!(
            target
                .all_interfaces
                .iter()
                .map(|id| type_key(c, *id))
                .collect::<Vec<_>>(),
            source
                .all_interfaces
                .iter()
                .map(|id| type_key(&reference, *id))
                .collect::<Vec<_>>(),
            "ordered interface closure for {}",
            target.metadata_name
        );
        assert_eq!(
            properties(c, id),
            properties(&reference, old),
            "declared members for {}",
            target.metadata_name
        );
        assert!(target.location.as_ref().unwrap().contains("_reference.rs:"));
        assert!(target
            .expression
            .starts_with("::ferrodock_model_reactiveui_ferro::"));
        assert_eq!(c.assemblies[target.assembly.unwrap()].name, scan.model.name);
        let original_ignore = expected
            .ignored_members
            .iter()
            .find(|m| m.type_expression == source.expression);
        let native_ignore = actual
            .ignored_members
            .iter()
            .find(|m| m.type_expression == target.expression);
        assert_eq!(
            native_ignore.map(|m| &m.member_names),
            original_ignore.map(|m| &m.member_names)
        );
    }
    assert_eq!(
        c.registrations
            .iter()
            .map(|r| r.type_id.map(|id| type_key(c, id)))
            .collect::<Vec<_>>(),
        reference
            .registrations
            .iter()
            .map(|r| r.type_id.map(|id| type_key(&reference, id)))
            .collect::<Vec<_>>()
    );
}

#[test]
fn actual_sdk_schemas_discard_all_seven_old_class_headers_and_property_rows() {
    let (mut reference, scan, models) = fixture();
    let source_ids = native_model_schemas::source_ids(&reference);
    let expected = native_model_schemas::compilation(&reference, &scan, &models, &source_ids);
    for id in source_ids {
        let t = &mut reference.types[id];
        for id in &t.properties {
            reference.properties[*id].name = "discarded_old_class_property".into();
            reference.properties[*id].type_id = usize::MAX;
            reference.properties[*id].overridden_property = Some(usize::MAX);
        }
        t.expression = "discarded_old_class_expression".into();
        t.display_name = "discarded_old_class_name".into();
        t.metadata_name = "discarded_old_class_name".into();
        t.namespace = "discarded_old_class_namespace".into();
        t.kind = TypeKind::OtherNonNamed;
        t.is_abstract = false;
        t.accessibility = Accessibility::Private;
        t.assembly = Some(usize::MAX);
        t.containing_type = Some(usize::MAX);
        t.base_type = Some(usize::MAX);
        t.all_interfaces = vec![usize::MAX];
        t.type_arguments = vec![usize::MAX];
        t.tuple_underlying_type = Some(usize::MAX);
        t.element_type = Some(usize::MAX);
        t.properties = vec![usize::MAX];
    }
    let actual = native_model_schemas::compilation(&reference, &scan, &models, &source_ids);
    assert_eq!(expected.source_types, actual.source_types);
    assert_eq!(expected.native_types, actual.native_types);
    assert_eq!(
        format!("{:?}", expected.compilation),
        format!("{:?}", actual.compilation)
    );
    assert_eq!(
        format!("{:?}", build(&expected.compilation).unwrap()),
        format!("{:?}", build(&actual.compilation).unwrap())
    );
}

#[test]
fn class_schema_reads_native_property_types_order_and_ignore_changes() {
    let (reference, mut scan, models) = fixture();
    let source_ids = native_model_schemas::source_ids(&reference);
    let document = scan
        .model
        .types
        .iter_mut()
        .find(|t| t.name == "Document")
        .unwrap();
    document.properties.swap(0, 1);
    let first = document.properties[0].name.clone();
    document.properties[0].value_type = ferroui_build_scan::model::RustType::resolved("bool");
    document.properties[0]
        .attributes
        .push(ferroui_build_scan::model::AttributeModel {
            name: "IgnoreDataMember".into(),
            ..Default::default()
        });
    let native = native_model_schemas::compilation(&reference, &scan, &models, &source_ids);
    let id = native.source_types[source_ids[5]].unwrap();
    let t = &native.compilation.types[id];
    let p = &native.compilation.properties[t.properties[0]];
    assert_eq!(p.name, first);
    assert_eq!(type_key(&native.compilation, p.type_id), "System.Boolean[]");
    assert!(p.ignore_data_member);
    let original = &reference.types[source_ids[5]];
    assert_ne!(reference.properties[original.properties[0]].name, p.name);
    assert_ne!(
        properties(&native.compilation, id),
        properties(&reference, source_ids[5])
    );
}
