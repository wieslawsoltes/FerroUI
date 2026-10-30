// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use crate as native_adapter;
use crate::compiler_fixture_input as fixture_input;
use ferrodock_serializer_system_text_json_generators::*;
use ferroui_build_scan::{
    model::{RustType, TypeKind as MetadataKind},
    scanner::Scan,
};
#[path = "../../../tests/Dock/FerroDock.Serializer.Generated.Tests/native_contracts.rs"]
mod native_contracts;

fn fixture() -> (Compilation, Scan) {
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
    let scan = native_contracts::scan(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../FerroDock.Model.Ferro"),
    );
    (c, scan)
}
fn type_key(c: &Compilation, id: usize) -> String {
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
fn properties(c: &Compilation, id: usize) -> Vec<(String, String, bool, bool, bool)> {
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
            )
        })
        .collect()
}

#[test]
fn actual_native_contract_schemas_match_all_original_declared_members_and_interface_edges() {
    let (reference, scan) = fixture();
    let mut native = native_contracts::compilation(&reference, &scan);
    native.activated = true;
    let ids: Vec<_> = native.required_symbols[..6]
        .iter()
        .map(|id| id.unwrap())
        .collect();
    assert!(ids.windows(2).all(|pair| pair[1] == pair[0] + 1));
    assert!(ids
        .iter()
        .all(|id| !reference.required_symbols[..6].contains(&Some(*id))));
    assert_eq!(
        ids.iter()
            .map(|id| native.types[*id].properties.len())
            .sum::<usize>(),
        81
    );
    for (index, id) in ids.into_iter().enumerate() {
        let old = reference.required_symbols[index].unwrap();
        assert_eq!(native.types[id].kind, TypeKind::Interface);
        assert!(native.types[id]
            .expression
            .starts_with("dyn ::ferrodock_model_ferro::"));
        assert_eq!(
            native.types[id].metadata_name,
            reference.types[old].metadata_name
        );
        assert_eq!(native.types[id].namespace, reference.types[old].namespace);
        assert_eq!(
            native.types[id]
                .all_interfaces
                .iter()
                .map(|id| type_key(&native, *id))
                .collect::<Vec<_>>(),
            reference.types[old]
                .all_interfaces
                .iter()
                .map(|id| type_key(&reference, *id))
                .collect::<Vec<_>>()
        );
        assert_eq!(properties(&native, id), properties(&reference, old));
        assert!(native.types[id]
            .location
            .as_ref()
            .unwrap()
            .contains("_metadata.rs:"));
    }
    let original = build(&reference).unwrap();
    let actual = build(&native).unwrap();
    assert_eq!(actual.diagnostics.len(), original.diagnostics.len());
    for (index, id) in native.required_symbols[..6].iter().flatten().enumerate() {
        let old = reference.required_symbols[index].unwrap();
        let expected = original
            .ignored_members
            .iter()
            .find(|m| m.type_expression == reference.types[old].expression);
        let actual = actual
            .ignored_members
            .iter()
            .find(|m| m.type_expression == native.types[*id].expression);
        assert_eq!(
            actual.map(|m| &m.member_names),
            expected.map(|m| &m.member_names)
        );
    }
}

#[test]
fn native_contracts_do_not_consume_old_headers_or_property_rows() {
    let (reference, scan) = fixture();
    let enum_ids = native_contracts::enum_source_ids(&reference);
    let mut expected = native_contracts::compilation(&reference, &scan);
    expected.activated = true;
    let mut poisoned = reference;
    let discarded: Vec<_> = poisoned.required_symbols[..7]
        .iter()
        .flatten()
        .copied()
        .chain(enum_ids)
        .collect();
    for id in discarded {
        let t = &mut poisoned.types[id];
        for p in &t.properties {
            poisoned.properties[*p].name = "discarded_reference_property".into();
            poisoned.properties[*p].type_id = usize::MAX;
            poisoned.properties[*p].overridden_property = Some(usize::MAX);
        }
        t.expression = "discarded_reference_header".into();
        t.metadata_name = "discarded_reference_identity".into();
        t.namespace = "discarded_reference_namespace".into();
        t.kind = TypeKind::OtherNonNamed;
        t.assembly = Some(usize::MAX);
        t.properties = vec![usize::MAX];
        t.all_interfaces = vec![usize::MAX];
        t.base_type = Some(usize::MAX);
        t.is_abstract = false;
        t.accessibility = Accessibility::Private;
    }
    let mut poisoned = native_contracts::compilation_with_enum_ids(&poisoned, &scan, &enum_ids);
    poisoned.activated = true;
    assert!(build(&expected).unwrap().diagnostics.is_empty());
    for id in expected.required_symbols[..7].iter().flatten() {
        assert_eq!(
            format!("{:?}", expected.types[*id]),
            format!("{:?}", poisoned.types[*id])
        );
        assert_eq!(properties(&expected, *id), properties(&poisoned, *id));
    }
    for id in expected.types.len() - 5..expected.types.len() {
        assert_eq!(
            format!("{:?}", expected.types[id]),
            format!("{:?}", poisoned.types[id])
        );
    }
    assert_eq!(
        format!("{:?}", build(&expected).unwrap()),
        format!("{:?}", build(&poisoned).unwrap())
    );
}

#[test]
fn locked_command_source_and_foreign_lists_use_fresh_native_contract_ids() {
    let (reference, scan) = fixture();
    let native = native_contracts::compilation(&reference, &scan);
    let ids: Vec<_> = native.required_symbols[..7]
        .iter()
        .map(|id| id.unwrap())
        .collect();
    assert_eq!(
        ids,
        (native.types.len() - 12..native.types.len() - 5).collect::<Vec<_>>()
    );
    let command = &native.types[ids[6]];
    assert_eq!(
        command.expression,
        "::ferroui_reactiveui::reactive_command::i_command::ICommand"
    );
    assert_eq!(command.namespace, "System.Windows.Input");
    assert_eq!(command.metadata_name, "ICommand");
    assert_eq!(command.kind, TypeKind::Interface);
    assert!(command.is_abstract && command.properties.is_empty());
    assert!(reference.types[reference.required_symbols[6].unwrap()]
        .properties
        .is_empty());
    let version = native.assemblies[command.assembly.unwrap()]
        .version
        .strip_suffix(".0")
        .unwrap();
    let location = command.location.as_ref().unwrap().replace('\\', "/");
    // The published package, or the crate of the workspace when it is a member.
    assert!(
        location.contains(&format!(
            "ferroui-reactiveui-{version}/reactive_command/i_command.rs:"
        )) || location.contains("FerroUI.ReactiveUI/reactive_command/i_command.rs:"),
        "{location}"
    );
    for (name, arg) in [("IDockable", ids[0]), ("IDockWindow", ids[3])] {
        assert!(
            native
                .types
                .iter()
                .any(|t| t.namespace == "System.Collections.Generic"
                    && t.metadata_name == "IList`1"
                    && t.type_arguments == [arg]),
            "{name}"
        );
    }
    for (id, names) in [
        (ids[1], &["GoBack", "GoForward", "Navigate", "Close"][..]),
        (ids[2], &["ShowWindows", "ExitWindows"][..]),
    ] {
        for name in names {
            let property = native.types[id]
                .properties
                .iter()
                .map(|p| &native.properties[*p])
                .find(|p| p.name == *name)
                .unwrap();
            assert_eq!(property.type_id, ids[6]);
        }
    }
}

#[test]
fn five_enum_headers_come_from_native_declarations_with_original_inheritance_and_nullable_links() {
    let (reference, scan) = fixture();
    let source_ids = native_contracts::enum_source_ids(&reference);
    let native = native_contracts::compilation(&reference, &scan);
    for (id, old) in (native.types.len() - 5..native.types.len()).zip(source_ids) {
        let source = &reference.types[old];
        let actual = &native.types[id];
        assert_ne!(id, old);
        assert_eq!(actual.kind, TypeKind::Enum);
        assert!(!actual.is_abstract && actual.properties.is_empty());
        assert_eq!(actual.metadata_name, source.metadata_name);
        assert_eq!(actual.namespace, source.namespace);
        assert_eq!(actual.accessibility, source.accessibility);
        assert!(actual.expression.starts_with("::ferrodock_model::core::"));
        assert!(actual
            .location
            .as_ref()
            .unwrap()
            .replace('\\', "/")
            .contains("src/Dock/FerroDock.Model/core/"));
        assert_eq!(
            type_key(&native, actual.base_type.unwrap()),
            type_key(&reference, source.base_type.unwrap())
        );
        assert_eq!(
            actual
                .all_interfaces
                .iter()
                .map(|id| type_key(&native, *id))
                .collect::<Vec<_>>(),
            source
                .all_interfaces
                .iter()
                .map(|id| type_key(&reference, *id))
                .collect::<Vec<_>>()
        );
        assert!(
            native
                .properties
                .iter()
                .any(|p| p.type_id == id || native.types[p.type_id].type_arguments.contains(&id)),
            "native enum not linked from a real property: {}",
            actual.metadata_name
        );
        if actual.metadata_name == "PinnedDockDisplayMode" {
            assert!(native.types.iter().any(|t| t.namespace == "System"
                && t.metadata_name == "Nullable`1"
                && t.type_arguments == [id]));
        }
    }
}

#[test]
fn native_dock_contracts_do_not_fabricate_the_missing_system_attribute_symbol() {
    let (mut c, scan) = fixture();
    c.required_symbols[7] = None;
    let mut c = native_contracts::compilation(&c, &scan);
    c.activated = true;
    assert!(c.required_symbols[7].is_none());
    let model = build(&c).unwrap();
    assert_eq!(model.diagnostics.len(), 1);
    assert_eq!(model.diagnostics[0].id, "DSTJ004");
    assert!(model.diagnostics[0].message.contains(METADATA_NAMES[7]));
    assert!(model.serializable_types.is_empty());
}

#[test]
fn metadata_definition_resolves_only_the_single_declared_dyn_trait() {
    let (_, scan) = fixture();
    let source = read_source_declarations(&scan).unwrap();
    let m = scan.model.types.iter().find(|m| m.name == "IDock").unwrap();
    assert!(source.definition(&m.rust_path).is_none());
    let definition = source.metadata_definition(m).unwrap();
    assert_eq!(definition.kind, RustDeclarationKind::Trait);
    assert_eq!(
        definition.path.text,
        "::ferrodock_model_ferro::dock_metadata::IDockMetadata"
    );
    for text in [
        format!("::std::rc::Rc<{}>", m.rust_path.text),
        format!("Option<{}>", m.rust_path.text),
        format!("{} + Send", m.rust_path.text),
        format!("{} + 'static", m.rust_path.text),
        "dyn Missing".into(),
    ] {
        let mut invalid = m.clone();
        invalid.rust_path = RustType::resolved(&text);
        assert!(source.metadata_definition(&invalid).is_none(), "{text}");
    }
    let mut invalid = m.clone();
    invalid.kind = MetadataKind::Class;
    assert!(source.metadata_definition(&invalid).is_none());
    let mut invalid = m.clone();
    invalid.rust_path.unresolved.push("IDockMetadata".into());
    assert!(source.metadata_definition(&invalid).is_none());
}

#[test]
fn interface_base_handles_require_a_resolved_interface_closure_without_class_inheritance() {
    let (mut c, scan) = fixture();
    let source = read_source_declarations(&scan).unwrap();
    let m = scan
        .model
        .types
        .iter()
        .find(|m| m.name == "IDock")
        .unwrap()
        .clone();
    let s = source.metadata_definition(&m).unwrap();
    let mut options = RustSemanticTypeOptions {
        metadata_name: "IDock".into(),
        namespace: "Dock.Model.Core".into(),
        assembly: Some(0),
        accessibility: Accessibility::Public,
        is_abstract: true,
        containing_type: None,
        base_type: None,
        all_interfaces: Vec::new(),
        property_overrides: Vec::new(),
    };
    let dock = c.required_symbols[1].unwrap();
    let dockable = c.required_symbols[0].unwrap();
    let before = format!("{c:?}");
    assert!(
        bind_declared_semantic_type(&mut c, dock, &m, s, &options, |_| Some(dockable)).is_err()
    );
    assert_eq!(format!("{c:?}"), before);
    options.base_type = Some(dockable);
    assert!(
        bind_declared_semantic_type(&mut c, dock, &m, s, &options, |_| Some(dockable))
            .unwrap_err()
            .0
            .contains("Interface inheritance")
    );
    assert_eq!(format!("{c:?}"), before);
    // The base-only metadata form is also a real interface edge, never a class base.
    let mut base_only = m.clone();
    base_only.interfaces.clear();
    options.base_type = None;
    assert!(
        bind_declared_semantic_type(&mut c, dock, &base_only, s, &options, |_| Some(dockable))
            .unwrap_err()
            .0
            .contains("base interface")
    );
    assert_eq!(format!("{c:?}"), before);
}
