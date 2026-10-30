// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
type DerivedSelector<B> = Rc<dyn Fn(&B) -> Option<Box<dyn Any>>>;
type DerivedDecoder<B> =
    Rc<dyn Fn(&JsonValue, &mut JsonReadContext<'_>) -> SerializationResult<Option<B>>>;
type RuntimeType<B> = Rc<dyn Fn(&B) -> TypeId>;
/// Source policy for a runtime class absent from the declared polymorphic table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum JsonUnknownDerivedTypeHandling {
    /// Reject the unregistered runtime class.
    FailSerialization = 0,
    /// Serialize through the declared base schema.
    FallBackToBaseType = 1,
    /// Select the most specific registered ancestor, rejecting an ambiguous match.
    FallBackToNearestAncestor = 2,
}
/// One compiled derived-class registration for a source polymorphic contract.
pub struct JsonDerivedType<B> {
    name: String,
    type_id: TypeId,
    ancestors: Vec<TypeId>,
    select: DerivedSelector<B>,
    encode_selected: Encoder,
    decode: DerivedDecoder<B>,
}
impl<B: 'static> JsonDerivedType<B> {
    /// Registers a typed reference conversion and its source discriminator.
    /// Policy-aware dispatch uses the compiled runtime type to distinguish exact casts.
    pub fn new<D: 'static>(
        name: impl Into<String>,
        downcast: impl Fn(&B) -> Option<D> + 'static,
        upcast: impl Fn(D) -> B + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            type_id: TypeId::of::<D>(),
            ancestors: Vec::new(),
            select: Rc::new(move |value| {
                downcast(value).map(|value| Box::new(value) as Box<dyn Any>)
            }),
            encode_selected: Rc::new(|value, ctx| {
                ctx.value(
                    value
                        .downcast_ref::<D>()
                        .expect("compiled derived selection"),
                )
            }),
            decode: Rc::new(move |value, ctx| Ok(ctx.value::<D>(value)?.map(&upcast))),
        }
    }
    /// Supplies the complete compiled ancestor handle identities used for nearest selection.
    pub fn with_ancestors(mut self, ancestors: Vec<TypeId>) -> Self {
        self.ancestors = ancestors;
        self
    }
}
fn tag_derived(value: JsonValue, name: &str) -> SerializationResult<JsonValue> {
    let JsonValue::Object(mut fields) = value else {
        return Err(error(
            JsonErrorKind::MissingMetadata,
            "Polymorphic derived codec is not an object.",
        ));
    };
    if !fields.iter().any(|(name, _)| name == "$ref") {
        let index = usize::from(fields.first().is_some_and(|(name, _)| name == "$id"));
        fields.insert(index, ("$type".into(), JsonValue::String(name.into())));
    }
    Ok(JsonValue::Object(fields))
}
impl JsonTypeInfo {
    /// Source polymorphic configuration with no registered derived types fails on use.
    pub fn empty_polymorphic<T: 'static>() -> Rc<Self> {
        Self::converter(
            |_: &T, _| {
                Err(error(
                    JsonErrorKind::MissingMetadata,
                    "Polymorphic configuration must specify at least one derived type.",
                ))
            },
            |_, _| {
                Err(error(
                    JsonErrorKind::MissingMetadata,
                    "Polymorphic configuration must specify at least one derived type.",
                ))
            },
        )
    }
    /// Builds source polymorphic metadata using the first matching exact compiled conversion.
    /// Use policy-aware metadata when conversions also expose ancestor class views.
    pub fn polymorphic<B: Clone + 'static>(
        derived: Vec<JsonDerivedType<B>>,
        fallback: Option<Rc<JsonTypeInfo>>,
        ignore_unrecognized: bool,
    ) -> Rc<Self> {
        Self::polymorphic_core(derived, fallback, ignore_unrecognized, None)
    }
    /// Builds source exact-type and unknown-runtime handling with compiled ancestry facts.
    /// Casts and runtime identities must refer to the same original model/view handles.
    pub fn polymorphic_with_policy<B: Clone + 'static>(
        derived: Vec<JsonDerivedType<B>>,
        fallback: Option<Rc<JsonTypeInfo>>,
        ignore_unrecognized: bool,
        policy: JsonUnknownDerivedTypeHandling,
        runtime_type: impl Fn(&B) -> TypeId + 'static,
    ) -> Rc<Self> {
        if derived.is_empty() {
            return Self::empty_polymorphic::<B>();
        }
        Self::polymorphic_core(
            derived,
            fallback,
            ignore_unrecognized,
            Some((policy, Rc::new(runtime_type))),
        )
    }
    fn polymorphic_core<B: Clone + 'static>(
        derived: Vec<JsonDerivedType<B>>,
        fallback: Option<Rc<JsonTypeInfo>>,
        ignore_unrecognized: bool,
        policy: Option<(JsonUnknownDerivedTypeHandling, RuntimeType<B>)>,
    ) -> Rc<Self> {
        let derived = Rc::new(derived);
        let reading = derived.clone();
        let decoding = fallback.clone();
        if let Some(info) = &fallback {
            assert_eq!(
                info.id,
                TypeId::of::<B>(),
                "base fallback uses the declared type"
            );
        }
        Self::converter(
            move |value: &B, ctx| {
                let encode = |registration: &JsonDerivedType<B>,
                              selected: Box<dyn Any>,
                              ctx: &mut JsonWriteContext<'_>| {
                    tag_derived(
                        (registration.encode_selected)(selected.as_ref(), ctx)?,
                        &registration.name,
                    )
                };
                if let Some((policy, runtime_type)) = &policy {
                    let runtime_type = runtime_type(value);
                    if let Some(registration) = derived.iter().find(|r| r.type_id == runtime_type) {
                        let selected = (registration.select)(value).ok_or_else(|| {
                            error(
                                JsonErrorKind::MissingMetadata,
                                "Compiled exact runtime cast is unavailable.",
                            )
                        })?;
                        return encode(registration, selected, ctx);
                    }
                    match policy {
                        JsonUnknownDerivedTypeHandling::FailSerialization => {
                            return Err(error(
                                JsonErrorKind::MissingMetadata,
                                "Runtime class has no registered polymorphic metadata.",
                            ))
                        }
                        JsonUnknownDerivedTypeHandling::FallBackToBaseType => {}
                        JsonUnknownDerivedTypeHandling::FallBackToNearestAncestor => {
                            let mut candidates = Vec::new();
                            for (index, registration) in derived.iter().enumerate() {
                                if let Some(selected) = (registration.select)(value) {
                                    candidates.push((index, selected));
                                }
                            }
                            let mut chosen = None;
                            for (position, (index, _)) in candidates.iter().enumerate() {
                                if candidates.iter().any(|(other, _)| {
                                    other != index
                                        && derived[*other]
                                            .ancestors
                                            .contains(&derived[*index].type_id)
                                }) {
                                    continue;
                                }
                                if chosen.replace(position).is_some() {
                                    return Err(error(JsonErrorKind::MissingMetadata, "The runtime class has ambiguous registered polymorphic ancestors."));
                                }
                            }
                            if let Some(position) = chosen {
                                let (index, selected) = candidates.swap_remove(position);
                                return encode(&derived[index], selected, ctx);
                            }
                            if !candidates.is_empty() {
                                return Err(error(
                                    JsonErrorKind::MissingMetadata,
                                    "Compiled polymorphic ancestry is cyclic.",
                                ));
                            }
                        }
                    }
                } else {
                    for registration in derived.iter() {
                        if let Some(selected) = (registration.select)(value) {
                            return encode(registration, selected, ctx);
                        }
                    }
                }
                if let Some(info) = &fallback {
                    ctx.encode_info(info, value)
                } else {
                    Err(error(
                        JsonErrorKind::MissingMetadata,
                        "Runtime class has no registered polymorphic metadata.",
                    ))
                }
            },
            move |value, ctx| {
                if matches!(value, JsonValue::Null) {
                    return Ok(None);
                }
                let JsonValue::Object(fields) = value else {
                    return Err(error(
                        JsonErrorKind::InvalidJson,
                        "Expected a polymorphic object.",
                    ));
                };
                if let Some((name, JsonValue::String(id))) = fields.first() {
                    if name == "$ref" {
                        if fields.len() != 1 {
                            return Err(error(
                                JsonErrorKind::InvalidReference,
                                "A reference object cannot contain other properties.",
                            ));
                        }
                        return ctx.resolve::<B>(id).map(Some);
                    }
                }
                let mut discriminator = None;
                let mut body = Vec::with_capacity(fields.len());
                let mut regular = false;
                for (name, value) in fields {
                    if name == "$type" {
                        if regular || discriminator.is_some() {
                            return Err(error(
                                JsonErrorKind::InvalidJson,
                                "Type discriminator metadata is repeated or out of order.",
                            ));
                        }
                        let JsonValue::String(value) = value else {
                            return Err(error(
                                JsonErrorKind::InvalidJson,
                                "Type discriminator must be a string.",
                            ));
                        };
                        discriminator = Some(value.as_str());
                    } else {
                        if name != "$id" {
                            regular = true;
                        }
                        body.push((name.clone(), value.clone()));
                    }
                }
                let body = JsonValue::Object(body);
                if let Some(name) = discriminator {
                    if let Some(registration) = reading
                        .iter()
                        .find(|registration| registration.name == name)
                    {
                        return (registration.decode)(&body, ctx);
                    }
                    if !ignore_unrecognized {
                        return Err(error(
                            JsonErrorKind::InvalidJson,
                            "Unrecognized type discriminator.",
                        ));
                    }
                }
                if let Some(info) = &decoding {
                    (info.decode)(&body, ctx)?
                        .map(|value| {
                            value.downcast::<B>().map(|value| *value).map_err(|_| {
                                error(
                                    JsonErrorKind::MissingMetadata,
                                    "Base fallback returned an incompatible type.",
                                )
                            })
                        })
                        .transpose()
                } else {
                    Err(error(
                        JsonErrorKind::MissingMetadata,
                        "Polymorphic base type cannot be instantiated.",
                    ))
                }
            },
        )
    }
}
