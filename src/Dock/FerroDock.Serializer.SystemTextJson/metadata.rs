// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{error, JsonErrorKind, JsonValue};
use ferrodock_model::core::SerializationResult;
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};
/// Options fixed by the source Dock serializer options factory.
pub struct JsonSerializerOptions {
    list_type: JsonListType,
    cache: RefCell<HashMap<TypeId, Option<Rc<JsonTypeInfo>>>>,
}
impl Default for JsonSerializerOptions {
    fn default() -> Self {
        Self::with_list_type(JsonListType::observable_collection())
    }
}
impl JsonSerializerOptions {
    pub(crate) fn with_list_type(list_type: JsonListType) -> Self {
        Self {
            list_type,
            cache: RefCell::new(HashMap::new()),
        }
    }
    /// Gets the original configured generic IList creation policy.
    pub fn list_type(&self) -> &JsonListType {
        &self.list_type
    }

    fn lookup(&self, resolver: &dyn IJsonTypeInfoResolver, id: TypeId) -> Option<Rc<JsonTypeInfo>> {
        let cached = self.cache.borrow().get(&id).cloned();
        if let Some(value) = cached {
            return value;
        }
        let value = resolver.get_type_info(id, self);
        let previous = self.cache.borrow_mut().insert(id, value.clone());
        drop(previous);
        value
    }

    /// Source WriteIndented is enabled.
    pub fn write_indented(&self) -> bool {
        true
    }
    /// Source ReferenceHandler.Preserve is enabled.
    pub fn preserve_references(&self) -> bool {
        true
    }
    /// Source DefaultIgnoreCondition is WhenWritingNull.
    pub fn ignore_null_properties(&self) -> bool {
        true
    }
    /// Source number handling allows named floating-point literals.
    pub fn allow_named_floating_point_literals(&self) -> bool {
        true
    }
}
/// Source metadata resolver using compiled type identities instead of reflection.
pub trait IJsonTypeInfoResolver {
    /// Resolves the original declared type against these serializer options.
    fn get_type_info(
        &self,
        type_id: TypeId,
        options: &JsonSerializerOptions,
    ) -> Option<Rc<JsonTypeInfo>>;
}
type Encoder = Rc<dyn Fn(&dyn Any, &mut JsonWriteContext<'_>) -> SerializationResult<JsonValue>>;
type Decoder =
    Rc<dyn Fn(&JsonValue, &mut JsonReadContext<'_>) -> SerializationResult<Option<Box<dyn Any>>>>;
pub(crate) type ReferenceCast = (TypeId, Rc<dyn Fn(&dyn Any) -> Box<dyn Any>>);
type NullTest = Rc<dyn Fn(&dyn Any) -> bool>;
/// Generated typed codec for one declared source type.
pub struct JsonTypeInfo {
    id: TypeId,
    object: bool,
    encode: Encoder,
    decode: Decoder,
    is_null: NullTest,
    casts: Vec<ReferenceCast>,
}
impl JsonTypeInfo {
    /// Creates a typed custom converter without invoking it or inspecting fields.
    pub fn converter<T: 'static>(
        encode: impl Fn(&T, &mut JsonWriteContext<'_>) -> SerializationResult<JsonValue> + 'static,
        decode: impl Fn(&JsonValue, &mut JsonReadContext<'_>) -> SerializationResult<Option<T>>
            + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            id: TypeId::of::<T>(),
            object: false,
            encode: Rc::new(move |value, ctx| {
                encode(
                    value
                        .downcast_ref::<T>()
                        .expect("generated codec declared type"),
                    ctx,
                )
            }),
            decode: Rc::new(move |value, ctx| {
                Ok(decode(value, ctx)?.map(|value| Box::new(value) as Box<dyn Any>))
            }),
            is_null: Rc::new(|_| false),
            casts: Vec::new(),
        })
    }
    /// Adds a compiled reference conversion without replacing or copying the model.
    pub fn with_reference_cast<S: 'static, T: 'static>(
        info: Rc<Self>,
        cast: impl Fn(&S) -> T + 'static,
    ) -> Rc<Self> {
        assert_eq!(
            info.id,
            TypeId::of::<S>(),
            "reference cast source matches its descriptor"
        );
        let mut casts = info.casts.clone();
        casts.push((
            TypeId::of::<T>(),
            Rc::new(move |value| {
                Box::new(cast(
                    value
                        .downcast_ref::<S>()
                        .expect("compiled reference cast source"),
                )) as Box<dyn Any>
            }),
        ));
        Rc::new(Self {
            id: info.id,
            object: info.object,
            encode: info.encode.clone(),
            decode: info.decode.clone(),
            is_null: info.is_null.clone(),
            casts,
        })
    }
    /// Creates nullable metadata retaining null separately from a decoder failure.
    pub fn optional<T: 'static>() -> Rc<Self> {
        Rc::new(Self {
            id: TypeId::of::<Option<T>>(),
            object: false,
            encode: Rc::new(|value, ctx| {
                match value
                    .downcast_ref::<Option<T>>()
                    .expect("optional codec type")
                {
                    Some(value) => ctx.value(value),
                    None => Ok(JsonValue::Null),
                }
            }),
            decode: Rc::new(|value, ctx| {
                let value = if matches!(value, JsonValue::Null) {
                    None
                } else {
                    ctx.value::<T>(value)?
                };
                Ok(Some(Box::new(value) as Box<dyn Any>))
            }),
            is_null: Rc::new(|value| {
                value
                    .downcast_ref::<Option<T>>()
                    .expect("optional codec type")
                    .is_none()
            }),
            casts: Vec::new(),
        })
    }
    /// Wraps a property-specific converter without changing the resolver's declared-type codec.
    pub fn optional_with<T: 'static>(info: Rc<Self>) -> Rc<Self> {
        assert_eq!(
            info.id,
            TypeId::of::<T>(),
            "nullable converter declared type"
        );
        let reading = info.clone();
        Rc::new(Self {
            id: TypeId::of::<Option<T>>(),
            object: false,
            encode: Rc::new(move |value, ctx| {
                match value
                    .downcast_ref::<Option<T>>()
                    .expect("nullable converter type")
                {
                    Some(value) => ctx.encode_info(&info, value),
                    None => Ok(JsonValue::Null),
                }
            }),
            decode: Rc::new(move |value, ctx| {
                let value = if matches!(value, JsonValue::Null) {
                    None
                } else {
                    ctx.decode_info::<T>(&reading, value)?
                };
                Ok(Some(Box::new(value) as Box<dyn Any>))
            }),
            is_null: Rc::new(|value| {
                value
                    .downcast_ref::<Option<T>>()
                    .expect("nullable converter type")
                    .is_none()
            }),
            casts: Vec::new(),
        })
    }
    /// Creates reference-preserving class metadata over the original shared instance.
    /// Create registers each instance before its property setters run, permitting cycles.
    pub fn object<T: Clone + 'static>(
        create: impl Fn() -> T + 'static,
        identity: impl Fn(&T) -> usize + 'static,
        properties: Vec<JsonPropertyInfo<T>>,
    ) -> Rc<Self> {
        Self::object_core(Some(Rc::new(create)), identity, properties)
    }
    /// Creates an interface or abstract schema with source getters and no object constructor.
    /// Existing preserved references can still resolve; a new nonnull instance cannot be read.
    pub fn abstract_object<T: Clone + 'static>(
        identity: impl Fn(&T) -> usize + 'static,
        properties: Vec<JsonPropertyInfo<T>>,
    ) -> Rc<Self> {
        Self::object_core(None, identity, properties)
    }
    fn object_core<T: Clone + 'static>(
        create: Option<Rc<dyn Fn() -> T>>,
        identity: impl Fn(&T) -> usize + 'static,
        properties: Vec<JsonPropertyInfo<T>>,
    ) -> Rc<Self> {
        let properties = Rc::new(properties);
        let reading = properties.clone();
        let mut info = Self::converter(
            move |value: &T, ctx| {
                let (id, fresh) = ctx.reference_id(identity(value));
                if !fresh {
                    return Ok(JsonValue::Object(vec![(
                        "$ref".into(),
                        JsonValue::String(id),
                    )]));
                }
                let mut fields = vec![("$id".into(), JsonValue::String(id))];
                for property in properties.iter() {
                    if let Some(value) = (property.encode)(value, ctx)? {
                        fields.push((property.name.into(), value));
                    }
                }
                Ok(JsonValue::Object(fields))
            },
            move |value, ctx| {
                if matches!(value, JsonValue::Null) {
                    return Ok(None);
                }
                let JsonValue::Object(fields) = value else {
                    return Err(error(JsonErrorKind::InvalidJson, "Expected a JSON object."));
                };
                if let Some((name, value)) = fields.first() {
                    if name == "$ref" {
                        let JsonValue::String(id) = value else {
                            return Err(error(
                                JsonErrorKind::InvalidReference,
                                "Reference ID must be a string.",
                            ));
                        };
                        if fields.len() != 1 {
                            return Err(error(
                                JsonErrorKind::InvalidReference,
                                "A reference object cannot contain other properties.",
                            ));
                        }
                        return ctx.resolve(id).map(Some);
                    }
                }
                let object = create.as_ref().ok_or_else(|| {
                    error(
                        JsonErrorKind::MissingMetadata,
                        "The declared class or interface has no object constructor.",
                    )
                })?();
                for (index, (name, value)) in fields.iter().enumerate() {
                    if name == "$id" {
                        if index != 0 {
                            return Err(error(
                                JsonErrorKind::InvalidReference,
                                "Reference ID must precede object properties.",
                            ));
                        }
                        let JsonValue::String(id) = value else {
                            return Err(error(
                                JsonErrorKind::InvalidReference,
                                "Reference ID must be a string.",
                            ));
                        };
                        ctx.register(id, object.clone())?;
                        continue;
                    }
                    if name == "$ref" {
                        return Err(error(
                            JsonErrorKind::InvalidReference,
                            "Reference metadata is out of order.",
                        ));
                    }
                    if name.starts_with('$') {
                        return Err(error(
                            JsonErrorKind::InvalidReference,
                            "Unexpected metadata in a preserved class object.",
                        ));
                    }
                    if let Some(property) = reading.iter().find(|property| property.name == name) {
                        if let Some(decode) = &property.decode {
                            decode(&object, value, ctx)?;
                        }
                    }
                }
                Ok(Some(object))
            },
        );
        Rc::get_mut(&mut info)
            .expect("new object descriptor is unique")
            .object = true;
        info
    }
}
type PropertyEncoder<T> =
    Rc<dyn Fn(&T, &mut JsonWriteContext<'_>) -> SerializationResult<Option<JsonValue>>>;
type PropertyDecoder<T> =
    Rc<dyn Fn(&T, &JsonValue, &mut JsonReadContext<'_>) -> SerializationResult<()>>;
/// Generated getter/setter pair preserving the original source property order.
pub struct JsonPropertyInfo<T> {
    name: &'static str,
    encode: PropertyEncoder<T>,
    decode: Option<PropertyDecoder<T>>,
}
impl<T: 'static> JsonPropertyInfo<T> {
    /// Creates a set-only property; writing never calls a getter or emits its value.
    pub fn writeonly<V: 'static>(name: &'static str, set: impl Fn(&T, V) + 'static) -> Self {
        Self::try_writeonly(name, move |model, value| {
            set(model, value);
            Ok(())
        })
    }
    /// Creates a set-only property retaining a typed accessor failure unchanged.
    /// Value decoding completes before the original setter is called.
    pub fn try_writeonly<V: 'static>(
        name: &'static str,
        set: impl Fn(&T, V) -> SerializationResult<()> + 'static,
    ) -> Self {
        Self {
            name,
            encode: Rc::new(|_, _| Ok(None)),
            decode: Some(Rc::new(move |model, value, ctx| {
                let value = ctx.value::<V>(value)?.ok_or_else(|| {
                    error(
                        JsonErrorKind::InvalidJson,
                        "Null is invalid for this property type.",
                    )
                })?;
                set(model, value)
            })),
        }
    }
    /// Creates a set-only property with its explicit custom converter.
    pub fn writeonly_with_converter<V: 'static>(
        name: &'static str,
        set: impl Fn(&T, V) + 'static,
        converter: Rc<JsonTypeInfo>,
    ) -> Self {
        Self::try_writeonly_with_converter(
            name,
            move |model, value| {
                set(model, value);
                Ok(())
            },
            converter,
        )
    }
    /// Creates a set-only property with an explicit converter and fallible setter.
    /// Converter errors prevent setter invocation; setter failures retain identity.
    pub fn try_writeonly_with_converter<V: 'static>(
        name: &'static str,
        set: impl Fn(&T, V) -> SerializationResult<()> + 'static,
        converter: Rc<JsonTypeInfo>,
    ) -> Self {
        assert_eq!(
            converter.id,
            TypeId::of::<V>(),
            "property converter declared type"
        );
        Self {
            name,
            encode: Rc::new(|_, _| Ok(None)),
            decode: Some(Rc::new(move |model, value, ctx| {
                let value = ctx.decode_info::<V>(&converter, value)?.ok_or_else(|| {
                    error(
                        JsonErrorKind::InvalidJson,
                        "Null is invalid for this property type.",
                    )
                })?;
                set(model, value)
            })),
        }
    }
    /// Creates a property with its explicit custom converter, retaining the original accessors.
    /// The converter is local to this property and takes precedence over resolver metadata.
    pub fn with_converter<V: 'static>(
        name: &'static str,
        get: impl Fn(&T) -> V + 'static,
        set: impl Fn(&T, V) + 'static,
        converter: Rc<JsonTypeInfo>,
    ) -> Self {
        Self::try_with_converter(
            name,
            move |model| Ok(get(model)),
            move |model, value| {
                set(model, value);
                Ok(())
            },
            converter,
        )
    }
    /// Creates an explicitly converted property with typed fallible accessors.
    /// Getter, conversion and setter failures propagate without replacing their causes.
    pub fn try_with_converter<V: 'static>(
        name: &'static str,
        get: impl Fn(&T) -> SerializationResult<V> + 'static,
        set: impl Fn(&T, V) -> SerializationResult<()> + 'static,
        converter: Rc<JsonTypeInfo>,
    ) -> Self {
        assert_eq!(
            converter.id,
            TypeId::of::<V>(),
            "property converter declared type"
        );
        let reading = converter.clone();
        let mut property = Self::try_readonly_with_converter(name, get, converter);
        property.decode = Some(Rc::new(move |model, value, ctx| {
            let value = ctx.decode_info::<V>(&reading, value)?.ok_or_else(|| {
                error(
                    JsonErrorKind::InvalidJson,
                    "Null is invalid for this property type.",
                )
            })?;
            set(model, value)
        }));
        property
    }
    /// Creates a get-only property with its explicit converter; reading skips its value.
    pub fn readonly_with_converter<V: 'static>(
        name: &'static str,
        get: impl Fn(&T) -> V + 'static,
        converter: Rc<JsonTypeInfo>,
    ) -> Self {
        Self::try_readonly_with_converter(name, move |model| Ok(get(model)), converter)
    }
    /// Creates a get-only converted property retaining the getter's failure unchanged.
    /// Incoming property values are skipped without invoking the getter or converter.
    pub fn try_readonly_with_converter<V: 'static>(
        name: &'static str,
        get: impl Fn(&T) -> SerializationResult<V> + 'static,
        converter: Rc<JsonTypeInfo>,
    ) -> Self {
        assert_eq!(
            converter.id,
            TypeId::of::<V>(),
            "property converter declared type"
        );
        Self {
            name,
            encode: Rc::new(move |model, ctx| {
                let value = get(model)?;
                if (converter.is_null)(&value) {
                    Ok(None)
                } else {
                    ctx.encode_info(&converter, &value).map(Some)
                }
            }),
            decode: None,
        }
    }
    /// Creates a property using actual model accessors, without field reflection.
    pub fn new<V: 'static>(
        name: &'static str,
        get: impl Fn(&T) -> V + 'static,
        set: impl Fn(&T, V) + 'static,
    ) -> Self {
        Self::try_new(
            name,
            move |model| Ok(get(model)),
            move |model, value| {
                set(model, value);
                Ok(())
            },
        )
    }
    /// Creates a property with typed fallible accessors and original conversion order.
    /// Accessor errors stop subsequent properties and preserve their original identity.
    pub fn try_new<V: 'static>(
        name: &'static str,
        get: impl Fn(&T) -> SerializationResult<V> + 'static,
        set: impl Fn(&T, V) -> SerializationResult<()> + 'static,
    ) -> Self {
        let mut property = Self::try_readonly(name, get);
        property.decode = Some(Rc::new(move |model, value, ctx| {
            let value = ctx.value::<V>(value)?.ok_or_else(|| {
                error(
                    JsonErrorKind::InvalidJson,
                    "Null is invalid for this property type.",
                )
            })?;
            set(model, value)
        }));
        property
    }
    /// Creates a readonly property, which is serialized and ignored when reading.
    pub fn readonly<V: 'static>(name: &'static str, get: impl Fn(&T) -> V + 'static) -> Self {
        Self::try_readonly(name, move |model| Ok(get(model)))
    }
    /// Creates a readonly property with a typed fallible getter.
    /// Reads skip the incoming value and writes obtain the getter before testing null.
    pub fn try_readonly<V: 'static>(
        name: &'static str,
        get: impl Fn(&T) -> SerializationResult<V> + 'static,
    ) -> Self {
        Self {
            name,
            encode: Rc::new(move |model, ctx| {
                let value = get(model)?;
                if ctx.is_null(&value)? {
                    Ok(None)
                } else {
                    ctx.value(&value).map(Some)
                }
            }),
            decode: None,
        }
    }
}
/// Resolver populated by generated typed descriptors; no mutable global registry is used.
pub struct GeneratedJsonTypeInfoResolver {
    types: Vec<Rc<JsonTypeInfo>>,
}
impl GeneratedJsonTypeInfoResolver {
    /// Creates an independent descriptor catalog; application descriptors precede primitives.
    pub fn new(mut types: Vec<Rc<JsonTypeInfo>>) -> Rc<Self> {
        types.extend(primitives());
        Rc::new(Self { types })
    }
}
impl IJsonTypeInfoResolver for GeneratedJsonTypeInfoResolver {
    fn get_type_info(&self, id: TypeId, _: &JsonSerializerOptions) -> Option<Rc<JsonTypeInfo>> {
        self.types.iter().find(|value| value.id == id).cloned()
    }
}
/// Per-call writer state for original object reference IDs and typed recursive codecs.
pub struct JsonWriteContext<'a> {
    resolver: &'a dyn IJsonTypeInfoResolver,
    options: &'a JsonSerializerOptions,
    references: HashMap<usize, String>,
    depth: usize,
}
impl<'a> JsonWriteContext<'a> {
    pub(crate) fn new(
        resolver: &'a dyn IJsonTypeInfoResolver,
        options: &'a JsonSerializerOptions,
    ) -> Self {
        Self {
            resolver,
            options,
            references: HashMap::new(),
            depth: 0,
        }
    }
    /// Encodes the exact declared type using its supplied generated codec.
    pub fn value<T: 'static>(&mut self, value: &T) -> SerializationResult<JsonValue> {
        let info = self.info::<T>()?;
        self.encode_info(&info, value)
    }
    pub(crate) fn isolated_value<T: 'static>(&self, value: &T) -> SerializationResult<JsonValue> {
        let value = Self::new(self.resolver, self.options).value(value)?;
        // SerializeToElement completes the inner serialization before writing to the outer writer.
        value.to_json()?;
        Ok(value)
    }
    fn encode_info(
        &mut self,
        info: &JsonTypeInfo,
        value: &dyn Any,
    ) -> SerializationResult<JsonValue> {
        let previous = self.depth;
        if info.object {
            if previous >= 64 {
                return Err(error(
                    JsonErrorKind::InvalidJson,
                    "The maximum JSON depth was exceeded.",
                ));
            }
            self.depth += 1;
        }
        let result = (info.encode)(value, self);
        self.depth = previous;
        result
    }
    fn info<T: 'static>(&self) -> SerializationResult<Rc<JsonTypeInfo>> {
        self.options
            .lookup(self.resolver, TypeId::of::<T>())
            .ok_or_else(|| {
                error(
                    JsonErrorKind::MissingMetadata,
                    format!("No JSON metadata for {}.", std::any::type_name::<T>()),
                )
            })
    }
    fn is_null<T: 'static>(&self, value: &T) -> SerializationResult<bool> {
        let info = self.info::<T>()?;
        Ok((info.is_null)(value))
    }
    fn reference_id(&mut self, identity: usize) -> (String, bool) {
        if let Some(id) = self.references.get(&identity) {
            return (id.clone(), false);
        }
        let id = (self.references.len() + 1).to_string();
        self.references.insert(identity, id.clone());
        (id, true)
    }
}
struct JsonReference {
    value: Box<dyn Any>,
    info: Rc<JsonTypeInfo>,
}
/// Per-call reader state retaining original instances before recursive setters execute.
pub struct JsonReadContext<'a> {
    resolver: &'a dyn IJsonTypeInfoResolver,
    options: &'a JsonSerializerOptions,
    references: HashMap<String, JsonReference>,
}
impl<'a> JsonReadContext<'a> {
    pub(crate) fn new(
        resolver: &'a dyn IJsonTypeInfoResolver,
        options: &'a JsonSerializerOptions,
    ) -> Self {
        Self {
            resolver,
            options,
            references: HashMap::new(),
        }
    }
    /// Decodes the declared type; null is separate from malformed input or absent metadata.
    pub fn value<T: 'static>(&mut self, value: &JsonValue) -> SerializationResult<Option<T>> {
        let info = self.info::<T>()?;
        self.decode_info(&info, value)
    }
    pub(crate) fn isolated_value<T: 'static>(
        &self,
        value: &JsonValue,
    ) -> SerializationResult<Option<T>> {
        Self::new(self.resolver, self.options).value(value)
    }
    fn decode_info<T: 'static>(
        &mut self,
        info: &JsonTypeInfo,
        value: &JsonValue,
    ) -> SerializationResult<Option<T>> {
        (info.decode)(value, self)?
            .map(|value| {
                value.downcast::<T>().map(|value| *value).map_err(|_| {
                    error(
                        JsonErrorKind::MissingMetadata,
                        "Generated decoder returned the wrong declared type.",
                    )
                })
            })
            .transpose()
    }
    pub(crate) fn info<T: 'static>(&self) -> SerializationResult<Rc<JsonTypeInfo>> {
        self.options
            .lookup(self.resolver, TypeId::of::<T>())
            .ok_or_else(|| {
                error(
                    JsonErrorKind::MissingMetadata,
                    format!("No JSON metadata for {}.", std::any::type_name::<T>()),
                )
            })
    }
    fn register<T: 'static>(&mut self, id: &str, value: T) -> SerializationResult<()> {
        if self.references.contains_key(id) {
            return Err(error(
                JsonErrorKind::InvalidReference,
                "Duplicate reference ID.",
            ));
        }
        let info = self.info::<T>()?;
        self.references.insert(
            id.into(),
            JsonReference {
                value: Box::new(value),
                info,
            },
        );
        Ok(())
    }
    fn resolve<T: Clone + 'static>(&self, id: &str) -> SerializationResult<T> {
        let entry = self
            .references
            .get(id)
            .ok_or_else(|| error(JsonErrorKind::InvalidReference, "Reference ID is missing."))?;
        if let Some(value) = entry.value.downcast_ref::<T>() {
            return Ok(value.clone());
        }
        if let Some((_, cast)) = entry
            .info
            .casts
            .iter()
            .find(|(target, _)| *target == TypeId::of::<T>())
        {
            return cast(entry.value.as_ref())
                .downcast::<T>()
                .map(|value| *value)
                .map_err(|_| {
                    error(
                        JsonErrorKind::InvalidReference,
                        "Compiled reference cast returned an incompatible type.",
                    )
                });
        }
        Err(error(
            JsonErrorKind::InvalidReference,
            "Reference ID has an incompatible declared type.",
        ))
    }
}
fn primitives() -> Vec<Rc<JsonTypeInfo>> {
    let mut types = vec![
        JsonTypeInfo::converter(
            |value: &String, _| Ok(JsonValue::String(value.clone())),
            |value, _| match value {
                JsonValue::String(v) => Ok(Some(v.clone())),
                JsonValue::Null => Ok(None),
                _ => Err(error(JsonErrorKind::InvalidJson, "Expected a string.")),
            },
        ),
        JsonTypeInfo::converter(
            |value: &bool, _| Ok(JsonValue::Bool(*value)),
            |value, _| match value {
                JsonValue::Bool(v) => Ok(Some(*v)),
                _ => Err(error(JsonErrorKind::InvalidJson, "Expected Boolean.")),
            },
        ),
    ];
    types.extend(crate::number_metadata::numbers());
    types
}

include!("collection_metadata.rs");
include!("polymorphic_metadata.rs");
include!("value_metadata.rs");
