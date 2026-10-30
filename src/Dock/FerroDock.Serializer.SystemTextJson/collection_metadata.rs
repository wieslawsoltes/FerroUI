// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::collections::ListHandle;
/// Source open list type represented by compiled generic creators.
#[derive(Clone)]
pub struct JsonListType {
    name: String,
    family: ListFamily,
}
#[derive(Clone)]
enum ListFamily {
    Observable,
    Plain,
    Custom(Rc<dyn Fn(TypeId) -> SerializationResult<Box<dyn Any>>>),
}
impl JsonListType {
    /// Source ObservableCollection<> list type.
    pub fn observable_collection() -> Self {
        Self {
            name: "System.Collections.ObjectModel.ObservableCollection`1".into(),
            family: ListFamily::Observable,
        }
    }
    /// Source List<> list type.
    pub fn list() -> Self {
        Self {
            name: "System.Collections.Generic.List`1".into(),
            family: ListFamily::Plain,
        }
    }
    /// Supplies compiled closed generic creators for an application-defined list type.
    pub fn custom(
        name: impl Into<String>,
        create: impl Fn(TypeId) -> SerializationResult<Box<dyn Any>> + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            family: ListFamily::Custom(Rc::new(create)),
        }
    }
    /// Gets the original declared list type name.
    pub fn name(&self) -> &str {
        &self.name
    }
    fn create<T: Clone + PartialEq + 'static>(&self) -> SerializationResult<ListHandle<T>> {
        match &self.family {
            ListFamily::Observable => Ok(ListHandle::new()),
            ListFamily::Plain => Ok(ListHandle::from_plain(Vec::new())),
            ListFamily::Custom(create) => create(TypeId::of::<T>())?
                .downcast::<ListHandle<T>>()
                .map(|value| *value)
                .map_err(|_| {
                    error(
                        JsonErrorKind::MissingMetadata,
                        "Compiled list constructor returned an incompatible IList.",
                    )
                }),
        }
    }
}
impl JsonTypeInfo {
    /// Source IList<T> metadata, preserving the original list and configured list creator.
    pub fn list<T: Clone + PartialEq + 'static>() -> Rc<Self> {
        Self::converter(
            |list: &ListHandle<T>, ctx| {
                let (id, fresh) = ctx.reference_id(list.identity());
                if !fresh {
                    return Ok(JsonValue::Object(vec![(
                        "$ref".into(),
                        JsonValue::String(id),
                    )]));
                }
                ctx.with_container_depth(2, |ctx| {
                    let mut values = Vec::with_capacity(list.count());
                    for item in list.iter() {
                        values.push(ctx.value(&item)?);
                    }
                    Ok(JsonValue::Object(vec![
                        ("$id".into(), JsonValue::String(id)),
                        ("$values".into(), JsonValue::Array(values)),
                    ]))
                })
            },
            |value, ctx| {
                if matches!(value, JsonValue::Null) {
                    return Ok(None);
                }
                let (id, values) = match value {
                    JsonValue::Array(values) => (None, values),
                    JsonValue::Object(fields) => {
                        if let Some((name, JsonValue::String(id))) = fields.first() {
                            if name == "$ref" {
                                if fields.len() != 1 {
                                    return Err(error(
                                        JsonErrorKind::InvalidReference,
                                        "A list reference cannot contain other properties.",
                                    ));
                                }
                                return ctx.resolve::<ListHandle<T>>(id).map(Some);
                            }
                        }
                        let mut id = None;
                        let mut values = None;
                        for (index, (name, value)) in fields.iter().enumerate() {
                            match name.as_str() {
                                "$id" if index == 0 => {
                                    let JsonValue::String(value) = value else {
                                        return Err(error(
                                            JsonErrorKind::InvalidReference,
                                            "List reference ID must be a string.",
                                        ));
                                    };
                                    id = Some(value.as_str());
                                }
                                "$values" => {
                                    if values.is_some() {
                                        return Err(error(
                                            JsonErrorKind::InvalidReference,
                                            "Duplicate list values.",
                                        ));
                                    }
                                    let JsonValue::Array(value) = value else {
                                        return Err(error(
                                            JsonErrorKind::InvalidJson,
                                            "List values must be an array.",
                                        ));
                                    };
                                    values = Some(value);
                                }
                                _ => {
                                    return Err(error(
                                        JsonErrorKind::InvalidReference,
                                        "Invalid preserved-list metadata.",
                                    ))
                                }
                            }
                        }
                        (
                            id,
                            values.ok_or_else(|| {
                                error(
                                    JsonErrorKind::InvalidReference,
                                    "Preserved list has no values.",
                                )
                            })?,
                        )
                    }
                    _ => {
                        return Err(error(
                            JsonErrorKind::InvalidJson,
                            "Expected an array or preserved IList.",
                        ))
                    }
                };
                let list = ctx.options.list_type.create::<T>()?;
                if let Some(id) = id {
                    ctx.register(id, list.clone())?;
                }
                for value in values {
                    let value = ctx.value::<T>(value)?.ok_or_else(|| {
                        error(
                            JsonErrorKind::InvalidJson,
                            "Null cannot be assigned to this list element type.",
                        )
                    })?;
                    list.add(value);
                }
                Ok(Some(list))
            },
        )
    }
}
impl JsonWriteContext<'_> {
    fn with_container_depth<T>(
        &mut self,
        cost: usize,
        action: impl FnOnce(&mut Self) -> SerializationResult<T>,
    ) -> SerializationResult<T> {
        let previous = self.depth;
        if previous + cost > 64 {
            return Err(error(
                JsonErrorKind::InvalidJson,
                "The maximum JSON depth was exceeded.",
            ));
        }
        self.depth += cost;
        let result = action(self);
        self.depth = previous;
        result
    }
}
