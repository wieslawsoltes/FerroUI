// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::core::{DockSerializationError, IDockSerializer, SerializationResult};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Default)]
struct Model {
    values: Cell<[i32; 3]>,
}

#[test]
fn native_fallible_property_callbacks_match_original_order_commits_and_error_identity() {
    let rows: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/Dock/data/serializer-json-audit.accessors.json"
    ))
    .unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 17);
    for row in rows {
        let name = row["Name"].as_str().unwrap();
        let converted = name.contains("converter");
        let readonly = name.starts_with("readonly");
        let writeonly = name.starts_with("writeonly");
        let reading = name.starts_with("set-")
            || name == "invalid-token"
            || name.starts_with("read-converter")
            || (name.contains("-skip") && readonly)
            || name.contains("set-error");
        let fail_get = name.contains("get-error") || name == "persistent-error";
        let fail_set = name.contains("set-error");
        let model = Rc::new(Model::default());
        let trace = Rc::new(RefCell::new(Vec::<String>::new()));
        let failure =
            DockSerializationError::new(std::io::Error::other("original-accessor-failure"));
        let mut fields = Vec::new();
        for (index, member) in ["A", "B", "C"].into_iter().enumerate() {
            let sequence = Cell::new(0);
            let persistent = name == "persistent-getter";
            let events = trace.clone();
            let error = failure.clone();
            let get = move |value: &Rc<Model>| -> SerializationResult<i32> {
                events.borrow_mut().push(format!("get:{member}"));
                if index == 1 && fail_get {
                    return Err(error.clone());
                }
                if index == 1 && persistent {
                    sequence.set(sequence.get() + 1);
                    return Ok(sequence.get());
                }
                Ok(value.values.get()[index])
            };
            let events = trace.clone();
            let error = failure.clone();
            let set = move |value: &Rc<Model>, item: i32| -> SerializationResult<()> {
                events.borrow_mut().push(format!("set:{member}"));
                let mut items = value.values.get();
                items[index] = item;
                value.values.set(items);
                if index == 1 && fail_set {
                    return Err(error.clone());
                }
                Ok(())
            };
            let writing = trace.clone();
            let write_error = failure.clone();
            let fail_write = name == "write-converter-error";
            let reading_events = trace.clone();
            let read_error = failure.clone();
            let fail_read = name == "read-converter-error";
            let converter = JsonTypeInfo::converter(
                move |value: &i32, _| {
                    writing.borrow_mut().push("convert:write:B".into());
                    if fail_write {
                        Err(write_error.clone())
                    } else {
                        Ok(JsonValue::number(i64::from(*value)))
                    }
                },
                move |value, ctx| {
                    reading_events.borrow_mut().push("convert:read:B".into());
                    if fail_read {
                        Err(read_error.clone())
                    } else {
                        ctx.value::<i32>(value)
                    }
                },
            );
            let field = if index != 1 {
                JsonPropertyInfo::try_new(member, get, set)
            } else {
                match (readonly, writeonly, converted) {
                    (true, _, false) => JsonPropertyInfo::try_readonly(member, get),
                    (true, _, true) => {
                        JsonPropertyInfo::try_readonly_with_converter(member, get, converter)
                    }
                    (_, true, false) => JsonPropertyInfo::try_writeonly(member, set),
                    (_, true, true) => {
                        JsonPropertyInfo::try_writeonly_with_converter(member, set, converter)
                    }
                    (false, false, false) => JsonPropertyInfo::try_new(member, get, set),
                    (false, false, true) => {
                        JsonPropertyInfo::try_with_converter(member, get, set, converter)
                    }
                }
            };
            fields.push(field);
        }
        let created = model.clone();
        let events = trace.clone();
        let counted_constructor = name == "constructor-same-instance";
        let info = JsonTypeInfo::object(
            move || {
                events.borrow_mut().push("create".into());
                if counted_constructor {
                    let mut values = created.values.get();
                    values[0] += 1;
                    created.values.set(values);
                }
                created.clone()
            },
            |v: &Rc<Model>| Rc::as_ptr(v) as usize,
            fields,
        );
        let serializer =
            DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(vec![info])))
                .unwrap();
        let mut wire = None;
        let result = if counted_constructor {
            serializer
                .deserialize::<Rc<Model>>(Some("{\"B\":2}"))
                .unwrap();
            serializer
                .deserialize::<Rc<Model>>(Some("{\"C\":3}"))
                .map(|_| ())
        } else if name == "persistent-error" {
            assert!(serializer
                .serialize(model.clone())
                .unwrap_err()
                .ptr_eq(&failure));
            serializer
                .serialize(model.clone())
                .map(|value| wire = Some(value))
        } else if reading {
            let input = if readonly {
                "{\"A\":1,\"B\":{\"ignored\":true},\"C\":3}"
            } else if name == "invalid-token" {
                "{\"A\":1,\"B\":\"bad\",\"C\":3}"
            } else {
                "{\"A\":1,\"B\":2,\"C\":3}"
            };
            serializer.deserialize::<Rc<Model>>(Some(input)).map(|_| ())
        } else {
            if name == "persistent-getter" {
                serializer.serialize(model.clone()).unwrap();
            }
            serializer
                .serialize(model.clone())
                .map(|value| wire = Some(value))
        };
        assert_eq!(result.is_ok(), row["Success"].as_bool().unwrap(), "{name}");
        assert_eq!(
            result.as_ref().err().is_some_and(|e| e.ptr_eq(&failure)),
            row["SameError"].as_bool().unwrap(),
            "{name}"
        );
        assert_eq!(
            serde_json::to_value(&*trace.borrow()).unwrap(),
            row["Trace"],
            "{name}"
        );
        assert_eq!(
            serde_json::to_value(model.values.get()).unwrap(),
            row["Values"],
            "{name}"
        );
        assert_eq!(serde_json::to_value(wire).unwrap(), row["Wire"], "{name}");
    }
}
