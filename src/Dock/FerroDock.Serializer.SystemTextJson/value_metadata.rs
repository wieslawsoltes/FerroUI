// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
type ValuePropertyDecoder<T> =
    Rc<dyn Fn(&mut T, &JsonValue, &mut JsonReadContext<'_>) -> SerializationResult<()>>;
/// Typed property of a source value type, in declaration order.
pub struct JsonValuePropertyInfo<T> {
    name: &'static str,
    encode: PropertyEncoder<T>,
    decode: Option<ValuePropertyDecoder<T>>,
}
impl<T: 'static> JsonValuePropertyInfo<T> {
    /// Uses the actual value getter and mutable setter without reflection or shared wrappers.
    pub fn new<V: 'static>(
        name: &'static str,
        get: impl Fn(&T) -> V + 'static,
        set: impl Fn(&mut T, V) + 'static,
    ) -> Self {
        let mut property = Self::readonly(name, get);
        property.decode = Some(Rc::new(move |model, value, ctx| {
            let value = ctx.value::<V>(value)?.ok_or_else(|| {
                error(
                    JsonErrorKind::InvalidJson,
                    "Null is invalid for this value property type.",
                )
            })?;
            set(model, value);
            Ok(())
        }));
        property
    }
    /// Writes an original get-only property and skips its incoming JSON value.
    /// A parameterized value constructor is not inferred from getter names.
    pub fn readonly<V: 'static>(name: &'static str, get: impl Fn(&T) -> V + 'static) -> Self {
        Self {
            name,
            encode: Rc::new(move |model, ctx| {
                let value = get(model);
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
impl JsonTypeInfo {
    /// Creates object-shaped metadata for an actual source value type.
    /// Values use mutable fields and never acquire reference IDs. A leading string
    /// `$id` is accepted without registering a reference, as in the source serializer;
    /// other reserved metadata and null/non-object inputs are rejected.
    pub fn value_object<T: 'static>(
        create: impl Fn() -> T + 'static,
        properties: Vec<JsonValuePropertyInfo<T>>,
    ) -> Rc<Self> {
        let reading = Rc::new(properties);
        let writing = reading.clone();
        let mut info = Self::converter(
            move |model: &T, ctx| {
                let mut fields = Vec::with_capacity(writing.len());
                for property in writing.iter() {
                    if let Some(value) = (property.encode)(model, ctx)? {
                        fields.push((property.name.into(), value));
                    }
                }
                Ok(JsonValue::Object(fields))
            },
            move |value, ctx| {
                let JsonValue::Object(fields) = value else {
                    return Err(error(
                        JsonErrorKind::InvalidJson,
                        "Expected a nonnull value object.",
                    ));
                };
                let mut model = create();
                for (index, (name, value)) in fields.iter().enumerate() {
                    if name == "$id" && index == 0 && matches!(value, JsonValue::String(_)) {
                        continue;
                    }
                    if name.starts_with('$') {
                        return Err(error(
                            JsonErrorKind::InvalidReference,
                            "Invalid reference metadata for a value object.",
                        ));
                    }
                    if let Some(property) = reading.iter().find(|p| p.name == name) {
                        if let Some(decode) = &property.decode {
                            decode(&mut model, value, ctx)?;
                        }
                    }
                }
                Ok(Some(model))
            },
        );
        Rc::get_mut(&mut info).expect("new value metadata").object = true;
        info
    }
}
