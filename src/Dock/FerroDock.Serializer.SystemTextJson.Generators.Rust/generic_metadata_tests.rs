// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_serializer_system_text_json_generators::*;
use ferroui_build_scan::{
    model::{AttributeModel, AttributeValueModel, CallForm, RustType, TypeModel},
    scanner::{scan_crate, Scan, ScanOptions},
};
fn fixture() -> (Scan, TypeModel, RustSourceDeclaration) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../tests/Dock/FerroDock.Serializer.Generated.Tests/payload_metadata.rs");
    let mut options = ScanOptions::new("ferrodock_serializer_generated_tests", root);
    options.extern_crates = vec![
        "ferrodock_model".into(),
        "ferrodock_serializer_system_text_json".into(),
    ];
    let scan = scan_crate(&options);
    let facts = read_source_declarations(&scan).unwrap();
    let concrete = scan
        .normalise(
            &scan.model.crate_name,
            "crate::payload_models::Generic<i32>",
        )
        .unwrap();
    let source = facts.definition(&concrete).unwrap().clone();
    let template = scan
        .model
        .types
        .iter()
        .find(|t| t.name == "GeneratedGeneric`1")
        .unwrap()
        .clone();
    (scan, template, source)
}
fn instantiate(
    scan: &Scan,
    template: &TypeModel,
    source: &RustSourceDeclaration,
    arg: &str,
) -> Result<TypeModel, CompilationError> {
    let target = scan
        .normalise(
            &template.module,
            &format!("crate::payload_models::Generic<{arg}>"),
        )
        .unwrap();
    instantiate_generic_metadata(template, source, &target, |text| {
        scan.normalise(&template.module, text)
    })
}
#[test]
fn actual_template_instantiates_distinct_closed_property_and_handle_types_without_mutation() {
    let (scan, template, source) = fixture();
    let before = format!("{template:?}");
    let int = instantiate(&scan, &template, &source, "i32").unwrap();
    let string = instantiate(&scan, &template, &source, "Option<String>").unwrap();
    assert_eq!(int.properties[0].value_type.text, "i32");
    assert_eq!(string.properties[0].value_type.text, "Option<String>");
    assert!(int.this.as_ref().unwrap().text.contains("Generic<i32>"));
    assert_ne!(int.rust_path, string.rust_path);
    assert_eq!(int.generic.unwrap().arguments[0].text, "i32");
    assert_eq!(format!("{template:?}"), before);
}
#[test]
fn nested_types_and_type_attributes_substitute_while_names_and_literals_remain_unchanged() {
    let (scan, mut template, source) = fixture();
    template.properties[0].name = "T".into();
    template.properties[0].value_type = scan
        .normalise(&template.module, "(T,[Option<T>;4],fn(T)->Option<T>)")
        .unwrap();
    template.properties[0].attributes.push(AttributeModel {
        name: "Typed".into(),
        arguments: vec![
            AttributeValueModel::Type(RustType {
                text: "T".into(),
                unresolved: vec!["T".into()],
            }),
            AttributeValueModel::Str("T".into()),
        ],
        properties: vec![],
    });
    let result = instantiate(&scan, &template, &source, "i32").unwrap();
    assert_eq!(
        result.properties[0].value_type,
        scan.normalise(
            &template.module,
            "(i32,[Option<i32>;4],fn(i32)->Option<i32>)"
        )
        .unwrap()
    );
    assert_eq!(result.properties[0].name, "T");
    assert_eq!(
        result.properties[0].attributes[0].arguments[1],
        AttributeValueModel::Str("T".into())
    );
    let AttributeValueModel::Type(ty) = &result.properties[0].attributes[0].arguments[0] else {
        panic!("type attribute");
    };
    assert_eq!(ty.text, "i32");
}
#[test]
fn typed_callable_generic_arguments_specialize_with_the_same_source_functions() {
    let (scan, mut template, source) = fixture();
    let path = format!("{}::<T>::new", source.path.text);
    template.public_path = Some(format!("{}<T>", source.path.text));
    template.stated_path = template.public_path.clone();
    template.constructors[0].callable.path = Some(path.clone());
    template.constructors[0].callable.resolved = Some(path.clone());
    template.constructors[0].call = Some(CallForm::CratePath(path));
    let result = instantiate(&scan, &template, &source, "i32").unwrap();
    assert!(result.constructors[0]
        .callable
        .resolved
        .as_ref()
        .unwrap()
        .contains("i32"));
    let Some(CallForm::CratePath(path)) = &result.constructors[0].call else {
        panic!("source call form");
    };
    assert!(path.contains("i32"));
    assert!(result.public_path.as_ref().unwrap().contains("i32"));
    assert_eq!(result.public_path, result.stated_path);
}
#[test]
fn associated_projections_open_inference_and_unsupported_parameter_kinds_are_not_guessed() {
    let (scan, mut template, mut source) = fixture();
    assert!(instantiate(&scan, &template, &source, "Option<_>")
        .unwrap_err()
        .0
        .contains("concrete compiler"));
    template.properties[0].value_type = RustType {
        text: "T::Item".into(),
        unresolved: vec!["T".into()],
    };
    assert!(instantiate(&scan, &template, &source, "i32")
        .unwrap_err()
        .0
        .contains("Associated"));
    source.parameters[0].kind = RustGenericParameterKind::Const;
    assert!(instantiate(&scan, &template, &source, "i32")
        .unwrap_err()
        .0
        .contains("type parameters only"));
}
fn compilation() -> Compilation {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/Dock/data/json-generator-semantics.json"
    ))
    .unwrap();
    crate::compiler_fixture_input::compilation(
        &rows
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "generated-payloads")
            .unwrap()["input"],
    )
}
#[test]
fn closed_symbols_rebuild_the_original_generic_full_names_and_substituted_member_types() {
    let (scan, template, source) = fixture();
    let mut c = compilation();
    let object = c
        .types
        .iter()
        .position(|t| t.metadata_name == "Object" && t.namespace == "System")
        .unwrap();
    let int = c
        .types
        .iter()
        .position(|t| t.metadata_name == "Int32" && t.namespace == "System")
        .unwrap();
    let old = c
        .types
        .iter()
        .position(|t| t.metadata_name == "GeneratedGeneric`1" && t.type_arguments == vec![int])
        .unwrap();
    let expected = full_name(&c, old).unwrap();
    let metadata = instantiate(&scan, &template, &source, "i32").unwrap();
    let own = c.types.len();
    let options = RustSemanticTypeOptions {
        metadata_name: "GeneratedGeneric`1".into(),
        namespace: "".into(),
        assembly: Some(0),
        accessibility: Accessibility::Public,
        is_abstract: false,
        containing_type: None,
        base_type: Some(object),
        all_interfaces: vec![],
        property_overrides: vec![],
    };
    bind_closed_generic_semantic_type(&mut c, own, &metadata, &source, &options, |ty| {
        if ty.text == "i32" {
            Some(int)
        } else {
            None
        }
    })
    .unwrap();
    assert_eq!(full_name(&c, own).unwrap(), expected);
    assert_eq!(c.types[own].type_arguments, [int]);
    assert_eq!(c.properties[c.types[own].properties[0]].type_id, int);
    c.types[int].kind = TypeKind::TypeParameter;
    let before = format!("{c:?}");
    assert!(bind_closed_generic_semantic_type(
        &mut c,
        own,
        &metadata,
        &source,
        &options,
        |_| Some(int)
    )
    .unwrap_err()
    .0
    .contains("not closed"));
    assert_eq!(format!("{c:?}"), before);
}
#[test]
fn wrong_definition_arity_cfg_and_metadata_arguments_leave_templates_unchanged() {
    let (scan, mut template, source) = fixture();
    let before = format!("{template:?}");
    assert!(instantiate(&scan, &template, &source, "i32,String")
        .unwrap_err()
        .0
        .contains("definition/arity"));
    assert_eq!(format!("{template:?}"), before);
    template.generic.as_mut().unwrap().arguments[0].text = "U".into();
    assert!(instantiate(&scan, &template, &source, "i32")
        .unwrap_err()
        .0
        .contains("source order"));
    template.cfg.push("feature=x".into());
    assert!(instantiate(&scan, &template, &source, "i32")
        .unwrap_err()
        .0
        .contains("cfg selection"));
}
