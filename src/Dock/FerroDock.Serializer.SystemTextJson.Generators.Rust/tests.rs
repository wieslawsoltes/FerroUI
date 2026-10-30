// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferroui_build_scan::model::*;
fn accessor(path: &str) -> AccessorModel {
    AccessorModel {
        call: Some(CallForm::Path(path.into())),
        ..Default::default()
    }
}
fn property(name: &str) -> PropertyModel {
    PropertyModel {
        name: name.into(),
        value_type: RustType::resolved("Option<String>"),
        getter: Some(accessor("::consumer::Thing::get")),
        setter: Some(accessor("::consumer::Thing::set")),
        ..Default::default()
    }
}
fn attr(name: &str, arguments: Vec<AttributeValueModel>) -> AttributeModel {
    AttributeModel {
        name: name.into(),
        arguments,
        ..Default::default()
    }
}
fn fixture() -> (TypeModel, RustMetadataOptions) {
    let mut declaration = TypeModel::new(
        "Thing",
        TypeKind::Class,
        RustType::resolved("::consumer::Thing"),
        "consumer",
    );
    declaration.this = Some(RustType::resolved("::std::rc::Rc<::consumer::Thing>"));
    declaration.constructors.push(MemberModel {
        call: Some(CallForm::Path("::consumer::Thing::new".into())),
        ..Default::default()
    });
    declaration.properties = vec![property("Name")];
    (
        declaration,
        RustMetadataOptions {
            emitting_crate: "consumer".into(),
            declaring_crate: "consumer".into(),
            identity: "|v:&::std::rc::Rc<::consumer::Thing>|::std::rc::Rc::as_ptr(v) as usize"
                .into(),
            object_types: vec!["Option<BoxedValue>".into()],
            converters: vec![RustMetadataConverter {
                converter_type: "::consumer::Converter".into(),
                factory: "::consumer::Converter::info()".into(),
            }],
            inherited_properties: None,
            accessor_error_mapper: None,
        },
    )
}
#[test]
fn declared_accessors_keep_order_renames_and_readonly_setonly_contracts() {
    let (mut d, o) = fixture();
    d.properties[0].attributes.push(attr(
        "JsonPropertyName",
        vec![AttributeValueModel::Str("$type".into())],
    ));
    let mut readonly = property("Read");
    readonly.setter = None;
    let mut writeonly = property("Write");
    writeonly.getter = None;
    let mut ignored = property("Skip");
    ignored.value_type.unresolved.push("Unknown".into());
    ignored.attributes.push(attr("JsonIgnore", vec![]));
    d.properties.extend([readonly, writeonly, ignored]);
    let b = bind_declared_type(17, &d, &o).unwrap();
    assert_eq!(b.symbol, 17);
    assert_eq!(b.constructor.as_deref(), Some("::consumer::Thing::new"));
    assert_eq!(
        b.properties
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        ["$type", "Read", "Write"]
    );
    assert!(b.properties[1].setter.is_none());
    assert!(b.properties[2].getter.is_none());
    assert!(b.properties[0]
        .getter
        .as_ref()
        .unwrap()
        .contains("::consumer::Thing::get(v)"));
}
#[test]
fn fallible_native_accessors_require_explicit_error_mapping_and_keep_individual_flags() {
    let (mut d, mut options) = fixture();
    d.properties[0].getter.as_mut().unwrap().fallible = true;
    assert!(bind_declared_type(0, &d, &options)
        .unwrap_err()
        .0
        .contains("explicit typed error mapper"));
    options.accessor_error_mapper = Some(RustAccessorErrorMapping::Preserve);
    let binding = bind_declared_type(0, &d, &options).unwrap();
    let property = &binding.properties[0];
    assert!(property.getter_fallible && !property.setter_fallible);
    assert!(!property.getter.as_ref().unwrap().contains("map_err"));
    assert!(!property.setter.as_ref().unwrap().contains("map_err"));
    d.properties[0].getter.as_mut().unwrap().fallible = false;
    d.properties[0].setter.as_mut().unwrap().fallible = true;
    options.accessor_error_mapper = Some(RustAccessorErrorMapping::Map(
        "::target::retain_native_error".into(),
    ));
    let binding = bind_declared_type(0, &d, &options).unwrap();
    let property = &binding.properties[0];
    assert!(!property.getter_fallible && property.setter_fallible);
    assert!(property
        .setter
        .as_ref()
        .unwrap()
        .contains(".map_err(::target::retain_native_error)"));
    options.accessor_error_mapper = Some(RustAccessorErrorMapping::Map(" ".into()));
    assert!(bind_declared_type(0, &d, &options).is_err());
    d.properties[0].attributes.push(attr("JsonIgnore", vec![]));
    assert!(bind_declared_type(0, &d, &options)
        .unwrap()
        .properties
        .is_empty());
}
#[test]
fn converter_precedence_retains_object_property_and_bypasses_type_members() {
    let (mut d, o) = fixture();
    d.properties[0].value_type = RustType::resolved("Option<BoxedValue>");
    d.properties[0].attributes.push(attr(
        "JsonConverter",
        vec![AttributeValueModel::Type(RustType::resolved(
            "::consumer::Converter",
        ))],
    ));
    let b = bind_declared_type(0, &d, &o).unwrap();
    assert!(b.properties[0].object_payload);
    assert_eq!(
        b.properties[0].converter.as_deref(),
        Some("::consumer::Converter::info()")
    );
    d.attributes = d.properties[0].attributes.clone();
    d.properties[0].getter.as_mut().unwrap().fallible = true;
    d.constructors[0].fallible = true;
    d.base = Some(RustType::resolved("Base"));
    let b = bind_declared_type(0, &d, &o).unwrap();
    assert_eq!(
        b.custom_codec.as_deref(),
        Some("::consumer::Converter::info()")
    );
    assert!(b.properties.is_empty());
    assert!(b.constructor.is_none());
}
#[test]
fn call_forms_respect_crate_scope_and_use_public_typed_functions_for_invokers() {
    let (mut d, mut o) = fixture();
    let get = d.properties[0].getter.as_mut().unwrap();
    get.call = Some(CallForm::CratePath("::consumer::Thing::private_get".into()));
    get.typed_function = Some("__markup_get_Name".into());
    let b = bind_declared_type(0, &d, &o).unwrap();
    assert!(b.properties[0]
        .getter
        .as_ref()
        .unwrap()
        .contains("private_get"));
    o.emitting_crate = "downstream".into();
    let b = bind_declared_type(0, &d, &o).unwrap();
    assert!(b.properties[0]
        .getter
        .as_ref()
        .unwrap()
        .contains("__markup_get_Name"));
    let get = d.properties[0].getter.as_mut().unwrap();
    get.call = Some(CallForm::Invoker);
    get.typed_function = None;
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("neither an accessible path"));
}
#[test]
fn base_members_require_semantic_hiding_resolution_and_keep_source_order() {
    let (mut d, mut o) = fixture();
    d.base = Some(RustType::resolved("Base"));
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("inheritance"));
    o.inherited_properties = Some(vec![
        ferrodock_serializer_system_text_json_generators::RustPropertyBinding {
            getter_fallible: false,
            setter_fallible: false,
            name: "BaseProperty".into(),
            value_type: "i32".into(),
            getter: Some("base_getter".into()),
            setter: None,
            converter: None,
            object_payload: false,
        },
    ]);
    let b = bind_declared_type(0, &d, &o).unwrap();
    assert_eq!(
        b.properties
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>(),
        ["Name", "BaseProperty"]
    );
}
#[test]
fn unresolved_target_cfg_and_unsupported_accessor_contracts_do_not_guess() {
    let (mut d, o) = fixture();
    d.cfg.push("feature=\"x\"".into());
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("cfg selection"));
    d.cfg.clear();
    d.properties[0].value_type.unresolved.push("Missing".into());
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("Unresolved"));
    d.properties[0].value_type.unresolved.clear();
    d.properties[0].setter.as_mut().unwrap().fallible = true;
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("Fallible"));
    d.properties[0].setter.as_mut().unwrap().fallible = false;
    let mut ignore = attr("JsonIgnore", vec![]);
    ignore
        .properties
        .push(("Condition".into(), AttributeValueModel::Int(1)));
    d.properties[0].attributes.push(ignore);
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("Conditional JsonIgnore"));
}
#[test]
fn missing_parameterless_constructors_are_not_invented_and_ambiguity_is_reported() {
    let (mut d, o) = fixture();
    d.constructors[0].parameters.push(ParameterModel {
        type_: RustType::resolved("i32"),
        ..Default::default()
    });
    assert!(bind_declared_type(0, &d, &o).unwrap().constructor.is_none());
    d.constructors[0].parameters.clear();
    d.constructors.push(d.constructors[0].clone());
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("Ambiguous parameterless"));
}
#[test]
fn converter_and_rename_attributes_require_unambiguous_resolved_bindings() {
    let (mut d, mut o) = fixture();
    d.properties[0].attributes.push(attr(
        "JsonConverter",
        vec![AttributeValueModel::Type(RustType::resolved(
            "::consumer::Converter",
        ))],
    ));
    o.converters.clear();
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("No compiled converter"));
    d.properties[0].attributes = vec![attr("JsonPropertyName", vec![AttributeValueModel::Int(1)])];
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("one string"));
    d.properties[0].attributes = vec![
        attr(
            "JsonPropertyName",
            vec![AttributeValueModel::Str("a".into())],
        ),
        attr(
            "JsonPropertyName",
            vec![AttributeValueModel::Str("b".into())],
        ),
    ];
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("Duplicate"));
}

#[test]
fn separate_property_attributes_apply_without_silently_dropping_other_json_contracts() {
    let (mut d, o) = fixture();
    d.property_attributes.push((
        "Name".into(),
        vec![attr(
            "JsonPropertyName",
            vec![AttributeValueModel::Str("wire".into())],
        )],
    ));
    assert_eq!(
        bind_declared_type(0, &d, &o).unwrap().properties[0].name,
        "wire"
    );
    d.property_attributes[0].1.push(attr(
        "JsonNumberHandling",
        vec![AttributeValueModel::Int(1)],
    ));
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("JsonNumberHandling"));
    d.property_attributes[0].1.push(attr("JsonIgnore", vec![]));
    assert!(bind_declared_type(0, &d, &o).unwrap().properties.is_empty());
}

#[test]
fn property_binding_accepts_constructorless_base_views_and_interfaces_without_identity() {
    let (mut d, mut o) = fixture();
    d.constructors.clear();
    d.base = Some(RustType::resolved("::consumer::AbstractBase"));
    o.identity.clear();
    let properties = bind_declared_properties(&d, &o.property_options()).unwrap();
    assert_eq!(properties.len(), 1);
    assert_eq!(properties[0].name, "Name");
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("inheritance"));
    d.kind = TypeKind::Interface;
    d.this = Some(RustType::resolved("::std::rc::Rc<dyn ::consumer::IThing>"));
    let properties = bind_declared_properties(&d, &o.property_options()).unwrap();
    assert!(properties[0]
        .getter
        .as_ref()
        .unwrap()
        .contains("dyn ::consumer::IThing"));
    d.kind = TypeKind::Struct;
    assert!(bind_declared_properties(&d, &o.property_options())
        .unwrap_err()
        .0
        .contains("value codec"));
}

#[test]
fn property_only_binding_retains_attributes_errors_and_rejects_incomplete_source_facts() {
    let (mut d, mut o) = fixture();
    d.constructors[0].fallible = true;
    d.properties[0].setter.as_mut().unwrap().fallible = true;
    o.accessor_error_mapper = Some(RustAccessorErrorMapping::Preserve);
    d.property_attributes.push((
        "Name".into(),
        vec![attr(
            "JsonPropertyName",
            vec![AttributeValueModel::Str("wire".into())],
        )],
    ));
    let properties = bind_declared_properties(&d, &o.property_options()).unwrap();
    assert_eq!(properties[0].name, "wire");
    assert!(properties[0].setter_fallible && !properties[0].getter_fallible);
    assert!(bind_declared_type(0, &d, &o)
        .unwrap_err()
        .0
        .contains("constructor"));
    d.attributes.push(attr("JsonConverter", vec![]));
    assert!(bind_declared_properties(&d, &o.property_options())
        .unwrap_err()
        .0
        .contains("complete typed"));
    d.attributes.clear();
    d.cfg.push("feature=\"target\"".into());
    assert!(bind_declared_properties(&d, &o.property_options())
        .unwrap_err()
        .0
        .contains("cfg"));
    d.cfg.clear();
    d.this.as_mut().unwrap().unresolved.push("Missing".into());
    assert!(bind_declared_properties(&d, &o.property_options())
        .unwrap_err()
        .0
        .contains("Unresolved"));
    d.this = None;
    assert!(bind_declared_properties(&d, &o.property_options())
        .unwrap_err()
        .0
        .contains("instance handle"));
}

#[test]
fn native_instance_views_emit_the_declared_public_export_instead_of_private_module_paths() {
    let (mut declaration, options) = fixture();
    declaration.rust_path = RustType::resolved("::consumer::private::ThingView");
    declaration.this = Some(declaration.rust_path.clone());
    declaration.public_path = Some("::consumer::ThingView".into());
    let ty = metadata_instance_type(&declaration).unwrap();
    assert_eq!(ty.text, "::consumer::ThingView");
    let binding = bind_declared_type(0, &declaration, &options).unwrap();
    assert_eq!(binding.value_type, ty.text);
    assert!(binding.properties[0]
        .getter
        .as_ref()
        .unwrap()
        .contains("&::consumer::ThingView"));
    assert!(!binding.properties[0]
        .getter
        .as_ref()
        .unwrap()
        .contains("private"));
    declaration.this = Some(RustType::resolved("::std::rc::Rc<dyn ::consumer::IThing>"));
    assert_eq!(
        metadata_instance_type(&declaration).unwrap().text,
        "::std::rc::Rc<dyn ::consumer::IThing>"
    );
    declaration
        .this
        .as_mut()
        .unwrap()
        .unresolved
        .push("Missing".into());
    assert!(metadata_instance_type(&declaration).is_err());
    declaration.this = None;
    assert!(metadata_instance_type(&declaration).is_err());
}
