// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_serializer_system_text_json_generators::{
    CompilationError, RustPropertyBinding, RustTypeBinding, TypeId,
};
use ferroui_build_scan::model::{
    AccessorModel, AttributeModel, AttributeValueModel, CallForm, RustType, TypeKind, TypeModel,
};

/// A compile-time converter type and its typed metadata factory expression.
#[derive(Clone, Debug)]
pub struct RustMetadataConverter {
    /// Resolved Rust type named by a JsonConverter attribute.
    pub converter_type: String,
    /// Expression creating the converter's original typed runtime descriptor.
    pub factory: String,
}
/// Explicit target-compiler transport for a fallible native accessor's error.
#[derive(Clone, Debug)]
pub enum RustAccessorErrorMapping {
    /// The accessor already returns DockSerializationError; preserve it unchanged.
    Preserve,
    /// Typed function or closure mapping a foreign error to DockSerializationError.
    Map(String),
}

/// Compiler facts not represented by a shared declaration's accessor metadata.
#[derive(Clone, Debug)]
pub struct RustMetadataOptions {
    /// Crate into which the binding is emitted, for crate-private call forms.
    pub emitting_crate: String,
    /// Crate which owns the declaration and its typed metadata functions.
    pub declaring_crate: String,
    /// Typed callback returning the original model's reference identity.
    pub identity: String,
    /// Canonical Rust value types representing a source System.Object property.
    pub object_types: Vec<String>,
    /// Converter factories already resolved by the target compiler.
    pub converters: Vec<RustMetadataConverter>,
    /// Bound base properties after source inheritance/hiding resolution, in source order.
    /// A declaration with a base requires this decision, even when the result is empty.
    pub inherited_properties: Option<Vec<RustPropertyBinding>>,
    /// Explicit typed mapping from native accessor errors to DockSerializationError.
    /// Required only for fallible accessors; identity preserves an existing error handle.
    pub accessor_error_mapper: Option<RustAccessorErrorMapping>,
}

/// Borrowed compiler facts needed to bind only a declaration's own properties.
/// Abstract base views and interfaces need neither construction nor reference identity.
#[derive(Clone, Copy, Debug)]
pub struct RustPropertyMetadataOptions<'a> {
    /// Crate receiving the generated typed calls.
    pub emitting_crate: &'a str,
    /// Crate owning the declaration and its typed functions.
    pub declaring_crate: &'a str,
    /// Canonical value types representing source System.Object properties.
    pub object_types: &'a [String],
    /// Explicitly resolved converter factories.
    pub converters: &'a [RustMetadataConverter],
    /// Typed transport for fallible accessor errors.
    pub accessor_error_mapper: Option<&'a RustAccessorErrorMapping>,
}

impl RustMetadataOptions {
    /// Borrows the property compiler facts without constructor or identity requirements.
    pub fn property_options(&self) -> RustPropertyMetadataOptions<'_> {
        RustPropertyMetadataOptions {
            emitting_crate: &self.emitting_crate,
            declaring_crate: &self.declaring_crate,
            object_types: &self.object_types,
            converters: &self.converters,
            accessor_error_mapper: self.accessor_error_mapper.as_ref(),
        }
    }
}

fn error(message: impl Into<String>) -> CompilationError {
    CompilationError(message.into())
}
fn resolved(value: &RustType) -> Result<&str, CompilationError> {
    if value.is_resolved() {
        Ok(&value.text)
    } else {
        Err(error(format!(
            "Unresolved Rust metadata type: {}",
            value.text
        )))
    }
}
fn attribute<'a>(
    attributes: &'a [AttributeModel],
    name: &str,
) -> Result<Option<&'a AttributeModel>, CompilationError> {
    let qualified = format!("System.Text.Json.Serialization.{name}Attribute");
    let suffixed = format!("{name}Attribute");
    let mut matches = attributes
        .iter()
        .filter(|a| a.name == name || a.name == suffixed || a.name == qualified);
    let value = matches.next();
    if matches.next().is_some() {
        return Err(error(format!("Duplicate {name} metadata attribute")));
    }
    Ok(value)
}
fn converter(
    attributes: &[AttributeModel],
    options: &RustPropertyMetadataOptions<'_>,
) -> Result<Option<String>, CompilationError> {
    let Some(a) = attribute(attributes, "JsonConverter")? else {
        return Ok(None);
    };
    let [AttributeValueModel::Type(ty)] = a.arguments.as_slice() else {
        return Err(error("JsonConverter requires one resolved converter type"));
    };
    if !a.properties.is_empty() {
        return Err(error("Unsupported JsonConverter named arguments"));
    }
    let name = resolved(ty)?;
    let mut bindings = options
        .converters
        .iter()
        .filter(|c| c.converter_type == name);
    let binding = bindings
        .next()
        .ok_or_else(|| error(format!("No compiled converter factory for {name}")))?;
    if bindings.next().is_some() {
        return Err(error(format!(
            "Ambiguous compiled converter factory for {name}"
        )));
    }
    Ok(Some(binding.factory.clone()))
}

fn require_supported_attributes(attributes: &[AttributeModel]) -> Result<(), CompilationError> {
    for name in [
        "JsonPropertyOrder",
        "JsonRequired",
        "JsonObjectCreationHandling",
        "JsonExtensionData",
        "JsonNumberHandling",
        "JsonPolymorphic",
        "JsonDerivedType",
        "JsonUnmappedMemberHandling",
        "JsonConstructor",
        "JsonInclude",
    ] {
        if attribute(attributes, name)?.is_some() {
            return Err(error(format!(
                "{name} requires a resolved typed JSON binding"
            )));
        }
    }
    Ok(())
}

fn callable(
    call: Option<&CallForm>,
    typed_function: Option<&str>,
    declaration: &TypeModel,
    options: &RustPropertyMetadataOptions<'_>,
) -> Result<String, CompilationError> {
    match call {
        Some(CallForm::Path(path)) => Ok(path.clone()),
        Some(CallForm::CratePath(path)) if options.emitting_crate == options.declaring_crate => {
            Ok(path.clone())
        }
        _ => {
            // These are the shared declaration macro's public, typed functions,
            // not the object-valued runtime metadata invokers.
            let function = typed_function.ok_or_else(|| {
                error("Accessor has neither an accessible path nor a typed metadata function")
            })?;
            let owner = declaration
                .public_path
                .as_deref()
                .unwrap_or(resolved(&declaration.rust_path)?);
            Ok(format!("<{owner}>::{function}"))
        }
    }
}
fn accessor(
    value: Option<&AccessorModel>,
    declaration: &TypeModel,
    options: &RustPropertyMetadataOptions<'_>,
) -> Result<Option<String>, CompilationError> {
    value
        .map(|a| {
            if a.fallible
                && options.accessor_error_mapper.as_ref().is_none_or(
                    |m| matches!(m,RustAccessorErrorMapping::Map(path) if path.trim().is_empty()),
                )
            {
                return Err(error(
                    "Fallible metadata accessor requires an explicit typed error mapper",
                ));
            }
            callable(
                a.call.as_ref(),
                a.typed_function.as_deref(),
                declaration,
                options,
            )
        })
        .transpose()
}
fn map_accessor_error(
    call: String,
    fallible: bool,
    options: &RustPropertyMetadataOptions<'_>,
) -> String {
    if fallible {
        match options
            .accessor_error_mapper
            .as_ref()
            .expect("validated mapper")
        {
            RustAccessorErrorMapping::Preserve => call,
            RustAccessorErrorMapping::Map(path) => format!("{call}.map_err({path})"),
        }
    } else {
        call
    }
}

/// Gets a declaration's resolved instance handle for emitted Rust calls.
/// When that handle is the declaring type itself, its explicit public export is
/// used so downstream code does not name a private implementation module. Other
/// handle shapes retain their declared spelling and require compiler export resolution.
/// Missing or unresolved instance/declaration types return errors.
pub fn metadata_instance_type(declaration: &TypeModel) -> Result<RustType, CompilationError> {
    let declaring = resolved(&declaration.rust_path)?;
    let instance = declaration
        .this
        .as_ref()
        .ok_or_else(|| error("Shared declaration has no explicit instance handle"))?;
    resolved(instance)?;
    if instance.text == declaring {
        if let Some(public) = &declaration.public_path {
            return Ok(RustType::resolved(public));
        }
    }
    Ok(instance.clone())
}

/// Binds only instance properties declared by a shared class or interface.
///
/// Retains declared order, explicit JSON attributes, converters and accessor errors.
/// Base properties and hiding are resolved separately by the semantic compiler. No
/// constructor, identity or base-member decision is requested or fabricated here.
/// Type codecs and registered properties require complete typed bindings instead.
/// Unselected cfg, unresolved types and unsupported JSON contracts return errors.
pub fn bind_declared_properties(
    declaration: &TypeModel,
    options: &RustPropertyMetadataOptions<'_>,
) -> Result<Vec<RustPropertyBinding>, CompilationError> {
    if !matches!(declaration.kind, TypeKind::Class | TypeKind::Interface) {
        return Err(error(
            "Shared non-object properties require a compiled value codec",
        ));
    }
    if !declaration.cfg.is_empty() {
        return Err(error(
            "Conditional declaration requires the target compiler's cfg selection",
        ));
    }
    let instance_type = metadata_instance_type(declaration)?;
    let owner = &instance_type.text;
    require_supported_attributes(&declaration.attributes)?;
    if attribute(&declaration.attributes, "JsonConverter")?.is_some() {
        return Err(error("Type converters require complete typed JSON binding"));
    }
    if !declaration.registered.is_empty() {
        return Err(error(
            "Registered properties require resolved JSON property bindings",
        ));
    }
    let mut properties = Vec::new();
    for p in &declaration.properties {
        if !p.parameters.is_empty() {
            continue;
        }
        let mut attributes = p.attributes.clone();
        for (name, extra) in &declaration.property_attributes {
            if name == &p.name {
                attributes.extend(extra.iter().cloned());
            }
        }
        // Ignore wins before converters and accessors, as in the source metadata.
        if let Some(ignore) = attribute(&attributes, "JsonIgnore")? {
            if !ignore.arguments.is_empty() || !ignore.properties.is_empty() {
                return Err(error(
                    "Conditional JsonIgnore requires a typed JSON binding",
                ));
            }
            continue;
        }
        require_supported_attributes(&attributes)?;
        let name = if let Some(rename) = attribute(&attributes, "JsonPropertyName")? {
            let [AttributeValueModel::Str(name)] = rename.arguments.as_slice() else {
                return Err(error("JsonPropertyName requires one string"));
            };
            if !rename.properties.is_empty() {
                return Err(error("Unsupported JsonPropertyName named arguments"));
            }
            name.clone()
        } else {
            p.name.clone()
        };
        let ty = resolved(&p.value_type)?;
        let getter_fallible = p.getter.as_ref().is_some_and(|a| a.fallible);
        let setter_fallible = p.setter.as_ref().is_some_and(|a| a.fallible);
        let getter = accessor(p.getter.as_ref(), declaration, options)?.map(|path| {
            let call = map_accessor_error(format!("{path}(v)"), getter_fallible, options);
            format!("|v:&{owner}|{call}")
        });
        let setter = accessor(p.setter.as_ref(), declaration, options)?.map(|path| {
            let call = map_accessor_error(format!("{path}(v,value)"), setter_fallible, options);
            format!("|v:&{owner},value: {ty}|{call}")
        });
        if getter.is_none() && setter.is_none() {
            return Err(error(format!(
                "Property {} has no accessible accessor",
                p.name
            )));
        }
        properties.push(RustPropertyBinding {
            name,
            value_type: ty.into(),
            getter,
            setter,
            getter_fallible,
            setter_fallible,
            converter: converter(&attributes, options)?,
            object_payload: options.object_types.iter().any(|t| t == ty),
        });
    }
    Ok(properties)
}

/// Binds a shared class declaration to the original typed constructors/accessors.
///
/// Declared properties retain declaration order, JsonPropertyName and explicit
/// converter precedence. Inherited properties must already have been resolved by
/// the semantic adapter; omitted base decisions, unresolved types, unselected cfg
/// declarations and unsupported call forms return errors instead of guessed bindings.
/// JsonIgnore without a condition is supported. Fallible native accessors require an
/// explicit error mapper. Conditional ignores and parameterized constructors still
/// need their full JSON contracts.
pub fn bind_declared_type(
    symbol: TypeId,
    declaration: &TypeModel,
    options: &RustMetadataOptions,
) -> Result<RustTypeBinding, CompilationError> {
    if declaration.kind != TypeKind::Class {
        return Err(error(
            "Shared non-class metadata requires a compiled value codec",
        ));
    }
    if !declaration.cfg.is_empty() {
        return Err(error(
            "Conditional declaration requires the target compiler's cfg selection",
        ));
    }
    let instance_type = metadata_instance_type(declaration)?;
    let owner = &instance_type.text;
    require_supported_attributes(&declaration.attributes)?;
    let property_options = options.property_options();
    let custom_codec = converter(&declaration.attributes, &property_options)?;
    if custom_codec.is_none() && !declaration.registered.is_empty() {
        return Err(error(
            "Registered properties require resolved JSON property bindings",
        ));
    }
    if custom_codec.is_none()
        && declaration.base.is_some()
        && options.inherited_properties.is_none()
    {
        return Err(error(
            "Base properties require resolved inheritance bindings",
        ));
    }
    let mut properties = Vec::new();
    if custom_codec.is_none() {
        properties = bind_declared_properties(declaration, &property_options)?;
        if let Some(inherited) = &options.inherited_properties {
            properties.extend(inherited.iter().cloned());
        }
    }
    let mut constructors = declaration
        .constructors
        .iter()
        .filter(|c| c.parameters.is_empty());
    let constructor = if custom_codec.is_some() {
        None
    } else if let Some(c) = constructors.next() {
        if constructors.next().is_some() {
            return Err(error("Ambiguous parameterless metadata constructors"));
        }
        if c.fallible {
            return Err(error(
                "Fallible metadata constructor requires a typed JSON binding",
            ));
        }
        Some(callable(
            c.call.as_ref(),
            c.typed_function.as_deref(),
            declaration,
            &property_options,
        )?)
    } else if let Some(c) = &declaration.default_constructor {
        Some(
            c.resolved
                .clone()
                .ok_or_else(|| error("Unresolved default constructor"))?,
        )
    } else {
        None
    };
    Ok(RustTypeBinding {
        symbol,
        value_type: owner.clone(),
        constructor,
        identity: options.identity.clone(),
        properties,
        custom_codec,
        reference_casts: Vec::new(),
    })
}
