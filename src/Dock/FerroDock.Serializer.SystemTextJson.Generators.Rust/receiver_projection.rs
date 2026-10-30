// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_serializer_system_text_json_generators::{
    CompilationError, RustPropertyBinding, RustReferenceCast, RustTypeBinding,
};
use ferroui_build_scan::model::RustType;

/// Explicit compiler conversions between an original model handle and its native view.
/// Conversions must retain the same enclosing model, not copy its state or create another.
#[derive(Clone, Debug)]
pub struct RustReceiverProjection {
    /// Resolved target handle used by the serializer's callers and reference table.
    pub value_type: RustType,
    /// Typed callable from a borrowed target handle to the bound native receiver value.
    /// The callable expression is captured once per callback, then invoked on its live value.
    pub to_receiver: String,
    /// Typed callable extracting the target handle from an owned constructor result.
    /// Required if the original binding has a constructor; never used to invent one.
    pub from_constructor: Option<String>,
}

fn error(message: &str) -> CompilationError {
    CompilationError(message.into())
}
fn concrete(type_: &str) -> Result<(), CompilationError> {
    let parsed = syn::parse_str::<syn::Type>(type_)
        .map_err(|_| error("Invalid receiver projection type"))?;
    crate::generic_metadata::concrete_argument(&parsed)
        .map_err(|_| error("Receiver projection requires concrete type facts"))
}
fn callback(
    original: &str,
    source: &str,
    target: &str,
    convert: &str,
    setter: Option<&str>,
) -> String {
    if let Some(value) = setter {
        format!("{{let original={original};let project={convert};move|value: &{target},item: {value}|{{let receiver: {source}=project(value);original(&receiver,item)}}}}")
    } else {
        format!("{{let original={original};let project={convert};move|value: &{target}|{{let receiver: {source}=project(value);original(&receiver)}}}}")
    }
}

/// Projects property callbacks onto an explicitly supplied concrete receiver type.
///
/// Used for inherited abstract views without requiring constructors or identity callbacks.
/// Each callback captures the original callable and conversion once, then converts the live
/// receiver on every invocation. Property order, flags, converter and Result contracts are
/// retained. No base accessors or casts are inferred; the target compiler checks signatures.
/// Unresolved types and absent receiver conversions return errors without modifying input.
pub fn project_property_bindings(
    properties: &[RustPropertyBinding],
    source_type: &RustType,
    target_type: &RustType,
    to_receiver: &str,
) -> Result<Vec<RustPropertyBinding>, CompilationError> {
    if !source_type.is_resolved() || !target_type.is_resolved() {
        return Err(error("Unresolved receiver projection type"));
    }
    concrete(&source_type.text)?;
    concrete(&target_type.text)?;
    if to_receiver.trim().is_empty() {
        return Err(error("Missing typed receiver conversion"));
    }
    let mut result = properties.to_vec();
    for property in &mut result {
        property.getter = property
            .getter
            .as_ref()
            .map(|get| callback(get, &source_type.text, &target_type.text, to_receiver, None));
        property.setter = property.setter.as_ref().map(|set| {
            callback(
                set,
                &source_type.text,
                &target_type.text,
                to_receiver,
                Some(&property.value_type),
            )
        });
    }
    Ok(result)
}

/// Rebinds class metadata from its native view to the same original model handle.
///
/// Getter/setter results, fallibility, converters, order and declared reference-cast targets
/// are retained. Each operation converts its current receiver; no model/view is cached.
/// The original constructor is invoked once, then its resulting model is extracted by the
/// supplied conversion. An absent source constructor remains absent. Callback expressions
/// are captured at metadata construction. All conversions are explicit typed Rust calls;
/// no names, casts, constructors or source identities are inferred.
///
/// Custom type codecs require their own target metadata and are rejected here. Base-member
/// hiding/overrides and inheritance must already be resolved in the original binding.
/// Unresolved types, missing conversions and custom codecs return errors without
/// mutating the supplied binding; the target compiler checks the callable signatures.
pub fn project_type_binding(
    binding: &RustTypeBinding,
    projection: &RustReceiverProjection,
) -> Result<RustTypeBinding, CompilationError> {
    if !projection.value_type.is_resolved() {
        return Err(error("Unresolved receiver projection target"));
    }
    concrete(&binding.value_type)?;
    concrete(&projection.value_type.text)?;
    if projection.to_receiver.trim().is_empty() {
        return Err(error("Missing typed receiver conversion"));
    }
    if binding.custom_codec.is_some() {
        return Err(error("Custom codecs require explicit target metadata"));
    }
    if binding.constructor.is_some()
        && projection
            .from_constructor
            .as_ref()
            .is_none_or(|v| v.trim().is_empty())
    {
        return Err(error(
            "Original constructor requires a typed result conversion",
        ));
    }
    let mut result = binding.clone();
    let source = &binding.value_type;
    result.value_type = projection.value_type.text.clone();
    result.identity = callback(
        &binding.identity,
        source,
        &projection.value_type.text,
        &projection.to_receiver,
        None,
    );
    result.constructor=binding.constructor.as_ref().map(|constructor| {
        let convert=projection.from_constructor.as_ref().expect("validated constructor conversion");
        let target=&projection.value_type.text;
        format!("{{let original={constructor};let extract={convert};move || -> {target}{{let receiver: {source}=original();extract(receiver)}}}}")
    });
    result.properties = project_property_bindings(
        &binding.properties,
        &RustType::resolved(source),
        &projection.value_type,
        &projection.to_receiver,
    )?;
    result.reference_casts = binding
        .reference_casts
        .iter()
        .map(|cast| RustReferenceCast {
            target: cast.target.clone(),
            convert: callback(
                &cast.convert,
                source,
                &projection.value_type.text,
                &projection.to_receiver,
                None,
            ),
        })
        .collect();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrodock_serializer_system_text_json_generators::RustPropertyBinding;
    fn binding() -> RustTypeBinding {
        RustTypeBinding {
            symbol: 7,
            value_type: "::native::View".into(),
            constructor: Some("::native::create".into()),
            identity: "::native::identity".into(),
            properties: vec![RustPropertyBinding {
                name: "Name".into(),
                value_type: "Option<String>".into(),
                getter: Some("::native::get".into()),
                setter: Some("::native::set".into()),
                getter_fallible: true,
                setter_fallible: true,
                converter: Some("::native::property_codec()".into()),
                object_payload: false,
            }],
            custom_codec: None,
            reference_casts: vec![RustReferenceCast {
                target: "::native::Contract".into(),
                convert: "::native::upcast".into(),
            }],
        }
    }
    fn projection() -> RustReceiverProjection {
        RustReceiverProjection {
            value_type: RustType::resolved("::std::rc::Rc<::model::Original>"),
            to_receiver: "::native::view_of".into(),
            from_constructor: Some("::native::original_of".into()),
        }
    }
    #[test]
    fn projection_keeps_source_contracts_casts_and_input_unchanged() {
        let source = binding();
        let before = format!("{source:?}");
        let projected = project_type_binding(&source, &projection()).unwrap();
        assert_eq!(projected.symbol, source.symbol);
        assert_eq!(projected.value_type, "::std::rc::Rc<::model::Original>");
        assert_eq!(projected.properties.len(), source.properties.len());
        let property = &projected.properties[0];
        let original = &source.properties[0];
        assert_eq!(
            (
                &property.name,
                &property.value_type,
                &property.converter,
                property.object_payload
            ),
            (
                &original.name,
                &original.value_type,
                &original.converter,
                original.object_payload
            )
        );
        assert!(property.getter_fallible && property.setter_fallible);
        assert_eq!(
            projected.reference_casts[0].target,
            source.reference_casts[0].target
        );
        assert!(projected.reference_casts[0]
            .convert
            .contains("::native::upcast"));
        assert!(projected.identity.contains("::native::identity"));
        let constructor = projected.constructor.unwrap();
        assert_eq!(constructor.matches("::native::create").count(), 1);
        assert!(constructor.contains("::native::original_of"));
        assert_eq!(format!("{source:?}"), before);
    }
    #[test]
    fn projection_requires_real_conversions_and_never_fabricates_constructors_or_codecs() {
        let source = binding();
        let mut p = projection();
        p.from_constructor = None;
        assert!(project_type_binding(&source, &p)
            .unwrap_err()
            .0
            .contains("constructor"));
        let mut absent = source.clone();
        absent.constructor = None;
        assert!(project_type_binding(&absent, &p)
            .unwrap()
            .constructor
            .is_none());
        p = projection();
        p.to_receiver.clear();
        assert!(project_type_binding(&source, &p).is_err());
        p = projection();
        p.value_type.unresolved.push("Missing".into());
        assert!(project_type_binding(&source, &p).is_err());
        p = projection();
        p.value_type = RustType::resolved("_");
        assert!(project_type_binding(&source, &p).is_err());
        p = projection();
        let mut invalid = source.clone();
        invalid.value_type = "invalid type syntax".into();
        assert!(project_type_binding(&invalid, &p).is_err());
        let mut custom = source;
        custom.custom_codec = Some("native_codec()".into());
        assert!(project_type_binding(&custom, &p)
            .unwrap_err()
            .0
            .contains("Custom codecs"));
    }
    #[test]
    fn property_projection_needs_no_class_identity_or_constructor_and_preserves_missing_accessors()
    {
        let mut properties = binding().properties;
        let mut readonly = properties[0].clone();
        readonly.name = "ReadOnly".into();
        readonly.setter = None;
        readonly.setter_fallible = false;
        properties.push(readonly);
        let before = format!("{properties:?}");
        let source = RustType::resolved("::native::AbstractView");
        let target = RustType::resolved("::native::DerivedView");
        let result =
            project_property_bindings(&properties, &source, &target, "::native::derive_base")
                .unwrap();
        assert_eq!(result.len(), 2);
        assert!(result[0].getter_fallible && result[0].setter_fallible);
        assert_eq!(result[0].converter, properties[0].converter);
        assert!(result[0]
            .getter
            .as_ref()
            .unwrap()
            .contains("::native::derive_base"));
        assert!(result[0]
            .setter
            .as_ref()
            .unwrap()
            .contains("::native::DerivedView"));
        assert!(result[1].setter.is_none());
        assert_eq!(format!("{properties:?}"), before);
        assert!(project_property_bindings(&properties, &source, &target, " ").is_err());
        let unresolved = RustType {
            text: source.text.clone(),
            unresolved: vec!["Missing".into()],
        };
        assert!(project_property_bindings(&properties, &unresolved, &target, "base").is_err());
        assert!(
            project_property_bindings(&properties, &source, &RustType::resolved("_"), "base")
                .is_err()
        );
    }
}
