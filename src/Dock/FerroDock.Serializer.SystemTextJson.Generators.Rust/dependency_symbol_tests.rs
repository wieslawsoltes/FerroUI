// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use crate::compiler_fixture_input as fixture_input;
use ferrodock_serializer_system_text_json_generators::*;
fn fixture() -> Compilation {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/Dock/data/json-generator-semantics.json"
    ))
    .unwrap();
    fixture_input::compilation(
        &rows
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "generated-payloads")
            .unwrap()["input"],
    )
}

#[test]
fn selected_graph_preserves_actual_generic_array_tuple_nested_and_property_identities() {
    let source = fixture();
    let before = format!("{source:?}");
    let roots: Vec<_> = source
        .types
        .iter()
        .enumerate()
        .filter(|(_, t)| {
            t.kind == TypeKind::Array
                || t.containing_type.is_some()
                || t.tuple_underlying_type.is_some()
        })
        .map(|(i, _)| i)
        .collect();
    assert!(roots
        .iter()
        .any(|id| source.types[*id].kind == TypeKind::Array));
    assert!(roots
        .iter()
        .any(|id| source.types[*id].containing_type.is_some()));
    let mut map = Vec::new();
    let imported = import_semantic_dependencies(&source, &roots, &[], |c, ids| {
        assert!(ids.native_types.is_empty());
        assert!(!c.activated);
        assert!(
            c.registrations.is_empty()
                && c.source_types.is_empty()
                && c.occupied_type_names.is_empty()
        );
        map = ids.source_types.clone();
        Ok(())
    })
    .unwrap();
    for (old, new) in map
        .iter()
        .enumerate()
        .filter_map(|(i, id)| id.map(|id| (i, id)))
    {
        if let Ok(name) = full_name(&source, old) {
            assert_eq!(full_name(&imported, new).unwrap(), name);
        }
        let a = &source.types[old];
        let b = &imported.types[new];
        assert_eq!(b.base_type, a.base_type.map(|i| map[i].unwrap()));
        assert_eq!(
            b.containing_type,
            a.containing_type.map(|i| map[i].unwrap())
        );
        assert_eq!(b.element_type, a.element_type.map(|i| map[i].unwrap()));
        assert_eq!(
            b.tuple_underlying_type,
            a.tuple_underlying_type.map(|i| map[i].unwrap())
        );
        assert_eq!(
            b.type_arguments,
            a.type_arguments
                .iter()
                .map(|i| map[*i].unwrap())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            b.all_interfaces,
            a.all_interfaces
                .iter()
                .map(|i| map[*i].unwrap())
                .collect::<Vec<_>>()
        );
        for (a, b) in a.properties.iter().zip(&b.properties) {
            let a = &source.properties[*a];
            let b = &imported.properties[*b];
            assert_eq!(b.name, a.name);
            assert_eq!(b.type_id, map[a.type_id].unwrap());
            assert_eq!(b.ignore_data_member, a.ignore_data_member);
        }
    }
    assert_eq!(format!("{source:?}"), before);
}

#[test]
fn native_aliases_require_unique_real_source_identities() {
    let c = fixture();
    let id = c.required_symbols[0].unwrap();
    assert!(
        import_semantic_dependencies(&c, &[], &[id, id], |_, _| panic!(
            "Invalid aliases reached binding"
        ))
        .unwrap_err()
        .0
        .contains("Duplicate native")
    );
    assert!(
        import_semantic_dependencies(&c, &[], &[usize::MAX], |_, _| panic!(
            "Missing source reached binding"
        ))
        .unwrap_err()
        .0
        .contains("Missing native")
    );
}

#[test]
fn a_native_transaction_has_no_placeholder_headers_and_does_not_publish_late_failures() {
    let c = fixture();
    let before = format!("{c:?}");
    let native = c.required_symbols[0].unwrap();
    let error = import_semantic_dependencies(&c, &[], &[native], |working, ids| {
        assert_eq!(ids.native_types, [working.types.len()]);
        assert!(working.types.get(ids.native_types[0]).is_none());
        assert_eq!(working.required_symbols[0], Some(ids.native_types[0]));
        Err(CompilationError("native declaration error".into()))
    })
    .unwrap_err();
    assert_eq!(error.0, "native declaration error");
    assert!(
        import_semantic_dependencies(&c, &[], &[native], |_, _| Ok(()))
            .unwrap_err()
            .0
            .contains("did not fill")
    );
    assert_eq!(format!("{c:?}"), before);
}

#[test]
fn malformed_foreign_links_are_errors_before_native_binding() {
    let source = fixture();
    let ty = source.required_symbols[0].unwrap();
    let property = source.types[ty].properties[0];
    for variant in 0..4 {
        let mut c = source.clone();
        match variant {
            0 => c.types[ty].properties = vec![usize::MAX],
            1 => c.types[ty].base_type = Some(usize::MAX),
            2 => c.types[ty].assembly = Some(usize::MAX),
            _ => c.properties[property].type_id = usize::MAX,
        }
        let before = format!("{c:?}");
        assert!(import_semantic_dependencies(&c, &[], &[], |_, _| panic!(
            "Malformed graph reached binding"
        ))
        .is_err());
        assert_eq!(format!("{c:?}"), before);
    }
}

#[test]
fn overrides_into_excluded_native_schemas_require_explicit_rebinding() {
    let mut c = fixture();
    let native = c.required_symbols[0].unwrap();
    let foreign = c.required_symbols[1].unwrap();
    let old = c.types[native].properties[0];
    let property = c.types[foreign].properties[0];
    c.properties[property].overridden_property = Some(old);
    assert!(
        import_semantic_dependencies(&c, &[], &[native], |_, _| panic!(
            "Excluded property reached binding"
        ))
        .unwrap_err()
        .0
        .contains("explicit property rebinding")
    );
}

#[test]
fn final_table_validation_rejects_callback_links_outside_the_resolved_graph() {
    let c = fixture();
    let before = format!("{c:?}");
    for variant in 0..5 {
        let result = import_semantic_dependencies(&c, &[], &[], |working, _| {
            match variant {
                0 => working.required_symbols[0] = Some(usize::MAX),
                1 => working.types[0].assembly = Some(usize::MAX),
                2 => working.types[0].properties = vec![usize::MAX],
                3 => working.properties[0].type_id = usize::MAX,
                _ => working.properties[0].overridden_property = Some(usize::MAX),
            }
            Ok(())
        });
        assert!(result.unwrap_err().0.contains("Unbound final"));
    }
    assert_eq!(format!("{c:?}"), before);
}
