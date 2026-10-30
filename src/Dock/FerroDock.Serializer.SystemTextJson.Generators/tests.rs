// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use serde_json::{json, Value};
#[path = "../../../tests/Dock/support/json_compiler_input.rs"]
mod fixture_input;
use fixture_input::{compilation, id, text};
fn corpus() -> Vec<Value> {
    serde_json::from_str::<Value>(include_str!(
        "../../../tests/Dock/data/json-generator-semantics.json"
    ))
    .unwrap()
    .as_array()
    .unwrap()
    .clone()
}
#[test]
fn semantic_selection_diagnostics_contexts_ignored_members_and_polymorphisms_match_original_compiler(
) {
    let rows = corpus();
    assert_eq!(rows.len(), 19);
    for row in rows {
        let c = compilation(&row["input"]);
        let m = build(&c).unwrap();
        let actual = json!({"enabled":m.should_generate,"contextName":m.context_type_name,"contexts":m.context_types,
            "diagnostics":m.diagnostics.iter().map(|d|json!({"id":d.id,"message":d.message,"location":d.location})).collect::<Vec<_>>(),
            "ignored":m.ignored_members.iter().map(|i|json!({"expression":i.type_expression,"names":i.member_names})).collect::<Vec<_>>(),
            "polymorphisms":m.polymorphisms.iter().map(|p|json!({"kind":format!("{:?}",p.kind),"types":p.derived_types.iter().map(|t|&t.type_expression).collect::<Vec<_>>(),"handling":format!("{:?}",p.unknown_derived_type_handling),"ignore":p.ignore_unrecognized_type_discriminators})).collect::<Vec<_>>()});
        assert_eq!(actual, row["expected"], "{}", row["name"]);
    }
}
#[test]
fn compiler_type_names_match_actual_runtime_closed_generic_array_tuple_and_nested_full_names() {
    let mut count = 0;
    for row in corpus() {
        let c = compilation(&row["input"]);
        for name in row["nameCases"].as_array().unwrap() {
            assert_eq!(
                full_name(&c, id(&name["type_id"]).unwrap()).unwrap(),
                text(name, "expected"),
                "{}",
                row["name"]
            );
            count += 1;
        }
    }
    assert_eq!(count, 152);
}
#[test]
fn malformed_compiler_symbol_tables_return_errors_without_runtime_model_substitution() {
    let row = corpus()
        .into_iter()
        .find(|r| r["name"] == "discovery")
        .unwrap();
    let mut c = compilation(&row["input"]);
    c.required_symbols[0] = Some(usize::MAX);
    assert!(build(&c).is_err());
    let mut c = compilation(&row["input"]);
    let id = c.source_types[0];
    c.types[id].accessibility = Accessibility::Internal;
    c.types[id].assembly = Some(usize::MAX);
    assert!(build(&c).is_err());
    let mut c = compilation(&row["input"]);
    let id = c.source_types[0];
    c.types[id].containing_type = Some(id);
    assert!(full_name(&c, id).is_err());
    let mut c = compilation(&row["input"]);
    let id = c.source_types[0];
    c.types[id].kind = TypeKind::Array;
    c.types[id].array_rank = 1;
    c.types[id].element_type = Some(id);
    assert!(full_name(&c, id).is_err());
    let mut c = compilation(&row["input"]);
    let id = c.source_types[0];
    c.types[id].kind = TypeKind::Array;
    c.types[id].array_rank = 0;
    assert!(full_name(&c, id).is_err());
}
#[test]
fn emission_respects_activation_and_rejects_incomplete_compiler_bindings() {
    let rows = corpus();
    let inactive = rows.iter().find(|r| r["name"] == "inactive").unwrap();
    let c = compilation(&inactive["input"]);
    let model = build(&c).unwrap();
    let bindings = RustBindings {
        runtime_crate: "::ferrodock_serializer_system_text_json".into(),
        model_crate: "::ferrodock_model".into(),
        types: Vec::new(),
        families: Vec::new(),
        additional_codecs: Vec::new(),
        additional_metadata_collections: Vec::new(),
    };
    assert!(emit_rust(&c, &model, &bindings).unwrap().is_none());
    let active = rows.iter().find(|r| r["name"] == "actual-models").unwrap();
    let c = compilation(&active["input"]);
    let model = build(&c).unwrap();
    let error = emit_rust(&c, &model, &bindings).unwrap_err();
    assert!(error.0.contains("No Rust codec binding"));
}
