// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{error, JsonErrorKind, JsonReadContext, JsonTypeInfo, JsonValue, JsonWriteContext};
use ferrodock_model::{core::SerializationResult, BoxedValue};
use std::{any::TypeId, collections::HashMap, rc::Rc};

type PayloadEncoder =
    Rc<dyn Fn(&BoxedValue, &JsonWriteContext<'_>) -> SerializationResult<JsonValue>>;
type PayloadDecoder =
    Rc<dyn Fn(&JsonValue, &JsonReadContext<'_>) -> SerializationResult<Option<BoxedValue>>>;

/// A compiled entry in the source generator's object-payload discriminator tables.
pub struct JsonObjectPayloadType {
    type_id: TypeId,
    discriminator: String,
    encode: PayloadEncoder,
    decode: PayloadDecoder,
}
impl JsonObjectPayloadType {
    /// Registers the exact boxed value type and its source full-name discriminator.
    /// Reference types use their original shared handle, such as `Rc<MyPayload>`.
    pub fn new<T: 'static>(discriminator: impl Into<String>) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            discriminator: discriminator.into(),
            encode: Rc::new(|value, ctx| {
                ctx.isolated_value(value.downcast_ref::<T>().expect("compiled payload type"))
            }),
            decode: Rc::new(|value, ctx| {
                Ok(ctx
                    .isolated_value::<T>(value)?
                    .map(|value| Rc::new(value) as BoxedValue))
            }),
        }
    }
}

impl JsonTypeInfo {
    /// Creates the generated nullable object-property converter with its compiled registrations.
    /// Each typed payload uses a fresh source reference scope. Unrecognized or ambiguous tags
    /// retain raw JSON; this converter is applied to properties, not globally to object roots.
    pub fn object_payload(registrations: Vec<JsonObjectPayloadType>) -> Rc<Self> {
        let mut by_type = HashMap::new();
        let mut by_name = HashMap::new();
        for registration in registrations {
            let registration = Rc::new(registration);
            by_type.insert(registration.type_id, registration.clone());
            by_name.insert(registration.discriminator.clone(), registration);
        }
        Self::optional_with::<BoxedValue>(Self::converter(
            move |value: &BoxedValue, ctx| {
                if let Some(value) = value.downcast_ref::<JsonValue>() {
                    return Ok(value.clone());
                }
                let registration = by_type.get(&value.as_ref().type_id()).ok_or_else(|| {
                    error(
                        JsonErrorKind::MissingMetadata,
                        "Dock source-generated object payload is not registered.",
                    )
                })?;
                let value = (registration.encode)(value, ctx)?;
                let JsonValue::Object(fields) = value else {
                    return Err(error(
                        JsonErrorKind::MissingMetadata,
                        "Dock source-generated object payloads must serialize as JSON objects.",
                    ));
                };
                let mut tagged = Vec::with_capacity(fields.len() + 1);
                tagged.push((
                    "$type".into(),
                    JsonValue::String(registration.discriminator.clone()),
                ));
                tagged.extend(fields);
                Ok(JsonValue::Object(tagged))
            },
            move |value, ctx| {
                if matches!(value, JsonValue::Null) {
                    return Ok(None);
                }
                if let JsonValue::Object(fields) = value {
                    let mut count = 0;
                    let mut selected = None;
                    for (index, (name, value)) in fields.iter().enumerate() {
                        if name == "$type" {
                            if let JsonValue::String(name) = value {
                                count += 1;
                                if selected.is_none() && !name.chars().all(char::is_whitespace) {
                                    if let Some(registration) = by_name.get(name) {
                                        selected = Some((index, registration));
                                    }
                                }
                            }
                        }
                    }
                    if count <= 1 {
                        if let Some((selected, registration)) = selected {
                            let body = JsonValue::Object(
                                fields
                                    .iter()
                                    .enumerate()
                                    .filter(|(index, _)| *index != selected)
                                    .map(|(_, field)| field.clone())
                                    .collect(),
                            );
                            return (registration.decode)(&body, ctx);
                        }
                    }
                }
                Ok(Some(Rc::new(value.clone()) as BoxedValue))
            },
        ))
    }
}
