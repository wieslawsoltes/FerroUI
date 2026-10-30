// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::{core::IDockSerializer, BoxedValue};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct Payload {
    name: RefCell<Option<String>>,
    own: RefCell<Option<Rc<Payload>>>,
}
#[derive(Default)]
struct Holder {
    content: RefCell<Option<BoxedValue>>,
    other: RefCell<Option<BoxedValue>>,
    typed: RefCell<Option<Rc<Payload>>>,
}
#[derive(Default)]
struct CustomHolder {
    content: RefCell<Option<BoxedValue>>,
}
#[derive(Default)]
struct Generic<T> {
    value: RefCell<T>,
}
#[derive(Default)]
struct TypeNamed {
    kind: RefCell<Option<String>>,
    name: RefCell<Option<String>>,
}
struct UnregisteredPayload;
struct ArrayPayload;
struct ReadonlyHolder {
    content: Option<BoxedValue>,
}
impl Default for ReadonlyHolder {
    fn default() -> Self {
        let payload = Rc::new(Payload::default());
        payload.name.replace(Some("readonly".into()));
        Self {
            content: Some(boxed(payload)),
        }
    }
}

fn field<'a>(value: &'a JsonValue, name: &str) -> &'a str {
    let JsonValue::String(value) = value.property(name).unwrap() else {
        panic!("source field {name}");
    };
    value
}
fn check(name: &str, value: bool) {
    assert!(value, "{name}");
    assert!(
        include_str!("../../../tests/Dock/data/serializer-json-audit.generated.tsv")
            .lines()
            .any(|line| line == format!("{name}\t1"))
    );
}
fn generic<T: Clone + Default + 'static>() -> Rc<JsonTypeInfo> {
    JsonTypeInfo::object(
        || Rc::new(Generic::<T>::default()),
        |v: &Rc<Generic<T>>| Rc::as_ptr(v) as usize,
        vec![JsonPropertyInfo::new(
            "Value",
            |v: &Rc<Generic<T>>| v.value.borrow().clone(),
            |v, x| {
                v.value.replace(x);
            },
        )],
    )
}
fn serializer() -> Rc<DockSerializer> {
    let generic_json = JsonValue::parse(include_str!(
        "../../../tests/Dock/data/serializer-json-audit.generated.generic.json"
    ))
    .unwrap();
    let int_name = field(generic_json.property("Content").unwrap(), "$type");
    let string_name = field(generic_json.property("Other").unwrap(), "$type");
    let converter = JsonTypeInfo::object_payload(vec![
        JsonObjectPayloadType::new::<Rc<Payload>>("GeneratedPayload"),
        JsonObjectPayloadType::new::<Rc<Generic<i32>>>(int_name),
        JsonObjectPayloadType::new::<Rc<Generic<Option<String>>>>(string_name),
        JsonObjectPayloadType::new::<Rc<TypeNamed>>("GeneratedTypeNamed"),
        JsonObjectPayloadType::new::<Rc<Holder>>("GeneratedHolder"),
        JsonObjectPayloadType::new::<Rc<CustomHolder>>("GeneratedCustomHolder"),
        JsonObjectPayloadType::new::<Rc<ReadonlyHolder>>("GeneratedReadonlyHolder"),
        JsonObjectPayloadType::new::<Rc<ArrayPayload>>("GeneratedArrayPayload"),
    ]);
    let payload = JsonTypeInfo::object(
        || Rc::new(Payload::default()),
        |v: &Rc<Payload>| Rc::as_ptr(v) as usize,
        vec![
            JsonPropertyInfo::new(
                "Name",
                |v: &Rc<Payload>| v.name.borrow().clone(),
                |v, x| {
                    v.name.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "Self",
                |v: &Rc<Payload>| v.own.borrow().clone(),
                |v, x| {
                    v.own.replace(x);
                },
            ),
        ],
    );
    let holder = JsonTypeInfo::object(
        || Rc::new(Holder::default()),
        |v: &Rc<Holder>| Rc::as_ptr(v) as usize,
        vec![
            JsonPropertyInfo::with_converter(
                "Content",
                |v: &Rc<Holder>| v.content.borrow().clone(),
                |v, x| {
                    v.content.replace(x);
                },
                converter.clone(),
            ),
            JsonPropertyInfo::with_converter(
                "Other",
                |v: &Rc<Holder>| v.other.borrow().clone(),
                |v, x| {
                    v.other.replace(x);
                },
                converter.clone(),
            ),
            JsonPropertyInfo::new(
                "Typed",
                |v: &Rc<Holder>| v.typed.borrow().clone(),
                |v, x| {
                    v.typed.replace(x);
                },
            ),
        ],
    );
    let readonly = JsonTypeInfo::object(
        || Rc::new(ReadonlyHolder::default()),
        |v: &Rc<ReadonlyHolder>| Rc::as_ptr(v) as usize,
        vec![JsonPropertyInfo::readonly_with_converter(
            "Content",
            |v: &Rc<ReadonlyHolder>| v.content.clone(),
            converter,
        )],
    );
    let array = JsonTypeInfo::converter(
        |_: &Rc<ArrayPayload>, _| Ok(JsonValue::Array(vec![JsonValue::number(1)])),
        |value, _| {
            Ok(if matches!(value, JsonValue::Null) {
                None
            } else {
                Some(Rc::new(ArrayPayload))
            })
        },
    );
    let string_converter = JsonTypeInfo::optional_with::<BoxedValue>(JsonTypeInfo::converter(
        |v: &BoxedValue, ctx| ctx.value(v.downcast_ref::<String>().unwrap()),
        |v, ctx| Ok(ctx.value::<String>(v)?.map(|v| Rc::new(v) as BoxedValue)),
    ));
    let custom = JsonTypeInfo::object(
        || Rc::new(CustomHolder::default()),
        |v: &Rc<CustomHolder>| Rc::as_ptr(v) as usize,
        vec![JsonPropertyInfo::with_converter(
            "Content",
            |v: &Rc<CustomHolder>| v.content.borrow().clone(),
            |v, x| {
                v.content.replace(x);
            },
            string_converter,
        )],
    );
    let named = JsonTypeInfo::object(
        || Rc::new(TypeNamed::default()),
        |v: &Rc<TypeNamed>| Rc::as_ptr(v) as usize,
        vec![
            JsonPropertyInfo::new(
                "$type",
                |v: &Rc<TypeNamed>| v.kind.borrow().clone(),
                |v, x| {
                    v.kind.replace(x);
                },
            ),
            JsonPropertyInfo::new(
                "Name",
                |v: &Rc<TypeNamed>| v.name.borrow().clone(),
                |v, x| {
                    v.name.replace(x);
                },
            ),
        ],
    );
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(vec![
        payload,
        holder,
        custom,
        named,
        readonly,
        array,
        generic::<i32>(),
        generic::<Option<String>>(),
        JsonTypeInfo::optional::<String>(),
        JsonTypeInfo::optional::<Rc<Payload>>(),
    ])))
    .unwrap()
}
fn boxed<T: 'static>(value: T) -> BoxedValue {
    Rc::new(value)
}

#[test]
fn generated_property_payloads_keep_the_original_independent_reference_scopes() {
    let serializer = serializer();
    let payload = Rc::new(Payload::default());
    payload.name.replace(Some("alpha".into()));
    payload.own.replace(Some(payload.clone()));
    let holder = Rc::new(Holder::default());
    holder.content.replace(Some(boxed(payload.clone())));
    holder.other.replace(Some(boxed(payload.clone())));
    holder.typed.replace(Some(payload.clone()));
    let text = serializer.serialize(holder).unwrap();
    assert_eq!(
        text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.generated.holder.json")
    );
    let loaded = serializer
        .deserialize::<Rc<Holder>>(Some(&text))
        .unwrap()
        .unwrap();
    let first = loaded
        .content
        .borrow()
        .as_ref()
        .unwrap()
        .downcast_ref::<Rc<Payload>>()
        .unwrap()
        .clone();
    let second = loaded
        .other
        .borrow()
        .as_ref()
        .unwrap()
        .downcast_ref::<Rc<Payload>>()
        .unwrap()
        .clone();
    let typed = loaded.typed.borrow().clone().unwrap();
    check(
        "generated-payload-actual-class",
        first.name.borrow().as_deref() == Some("alpha"),
    );
    check(
        "generated-payload-inner-self-reference",
        Rc::ptr_eq(&first, first.own.borrow().as_ref().unwrap())
            && Rc::ptr_eq(&second, second.own.borrow().as_ref().unwrap()),
    );
    check(
        "generated-payload-scopes-are-independent",
        !Rc::ptr_eq(&first, &second) && !Rc::ptr_eq(&first, &typed) && !Rc::ptr_eq(&second, &typed),
    );
    check(
        "generated-payload-outer-typed-self-reference",
        Rc::ptr_eq(&typed, typed.own.borrow().as_ref().unwrap()),
    );
    let value = JsonValue::parse(&text).unwrap();
    let content = value.property("Content").unwrap();
    check(
        "generated-payload-tag-before-inner-id",
        matches!(content,JsonValue::Object(fields) if fields[0].0 == "$type"),
    );
    check(
        "generated-payload-fresh-reference-ids",
        field(content, "$id") == "1"
            && field(value.property("Other").unwrap(), "$id") == "1"
            && field(value.property("Typed").unwrap(), "$id") == "2",
    );
    payload.own.take();
    first.own.take();
    second.own.take();
    typed.own.take();
}

#[test]
fn generated_closed_payloads_explicit_converters_and_duplicate_tags_match_source() {
    let serializer = serializer();
    let source = include_str!("../../../tests/Dock/data/serializer-json-audit.generated.generic.json");
    let loaded = serializer
        .deserialize::<Rc<Holder>>(Some(source))
        .unwrap()
        .unwrap();
    assert_eq!(serializer.serialize(loaded.clone()).unwrap(), source);
    check(
        "generated-payload-closed-generic-types",
        *loaded
            .content
            .borrow()
            .as_ref()
            .unwrap()
            .downcast_ref::<Rc<Generic<i32>>>()
            .unwrap()
            .value
            .borrow()
            == 37
            && loaded
                .other
                .borrow()
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<Generic<Option<String>>>>()
                .unwrap()
                .value
                .borrow()
                .as_deref()
                == Some("text"),
    );
    let source = include_str!("../../../tests/Dock/data/serializer-json-audit.generated.custom.json");
    let custom = serializer
        .deserialize::<Rc<CustomHolder>>(Some(source))
        .unwrap()
        .unwrap();
    check(
        "generated-payload-explicit-converter-wins",
        custom
            .content
            .borrow()
            .as_ref()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap()
            == "override",
    );
    assert_eq!(serializer.serialize(custom).unwrap(), source);
    let named = Rc::new(TypeNamed::default());
    named.kind.replace(Some("payload-type".into()));
    named.name.replace(Some("named".into()));
    let root_text = serializer.serialize(named.clone()).unwrap();
    assert_eq!(
        root_text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.generated.named-root.json")
    );
    check(
        "generated-concrete-type-named-metadata-rejected",
        serializer
            .deserialize::<Rc<TypeNamed>>(Some(&root_text))
            .is_err(),
    );
    let holder = Rc::new(Holder::default());
    holder.content.replace(Some(boxed(named)));
    let text = serializer.serialize(holder).unwrap();
    assert_eq!(
        text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.generated.duplicate.json")
    );
    let loaded = serializer
        .deserialize::<Rc<Holder>>(Some(&text))
        .unwrap()
        .unwrap();
    check("generated-payload-duplicate-tag-is-raw", loaded.content.borrow().as_ref().unwrap().downcast_ref::<JsonValue>().is_some_and(|v| matches!(v,JsonValue::Object(fields) if fields.iter().filter(|(n,_)| n == "$type").count() == 2)));
    check(
        "generated-payload-duplicate-tag-replay",
        serializer.serialize(loaded).unwrap() == text,
    );
    let raw_null = Rc::new(Holder::default());
    raw_null.content.replace(Some(boxed(JsonValue::Null)));
    let text = serializer.serialize(raw_null).unwrap();
    assert_eq!(
        text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.generated.null.json")
    );
    check(
        "generated-payload-boxed-raw-null-is-written",
        matches!(
            JsonValue::parse(&text).unwrap().property("Content"),
            Some(JsonValue::Null)
        ),
    );
    for (name, value) in [
        (
            "generated-payload-unregistered-class-rejected",
            boxed(Rc::new(UnregisteredPayload)),
        ),
        (
            "generated-payload-unregistered-primitive-rejected",
            boxed("primitive".to_owned()),
        ),
        (
            "generated-payload-nonobject-converter-rejected",
            boxed(Rc::new(ArrayPayload)),
        ),
    ] {
        let holder = Rc::new(Holder::default());
        holder.content.replace(Some(value));
        let failure = serializer.serialize(holder).unwrap_err();
        check(
            name,
            failure.cause().downcast_ref::<JsonError>().unwrap().kind
                == JsonErrorKind::MissingMetadata,
        );
    }
}

#[test]
fn generated_readonly_payload_uses_its_converter_and_ignores_read_input() {
    let serializer = serializer();
    let text = serializer
        .serialize(Rc::new(ReadonlyHolder::default()))
        .unwrap();
    assert_eq!(
        text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.generated.readonly.json")
    );
    check(
        "generated-payload-readonly-included",
        field(
            JsonValue::parse(&text)
                .unwrap()
                .property("Content")
                .unwrap(),
            "Name",
        ) == "readonly",
    );
    let loaded = serializer
        .deserialize::<Rc<ReadonlyHolder>>(Some(
            "{\"Content\":{\"$type\":\"GeneratedPayload\",\"Name\":false}}",
        ))
        .unwrap()
        .unwrap();
    check(
        "generated-payload-readonly-input-ignored",
        loaded
            .content
            .as_ref()
            .unwrap()
            .downcast_ref::<Rc<Payload>>()
            .unwrap()
            .name
            .borrow()
            .as_deref()
            == Some("readonly"),
    );
}

#[test]
fn generated_payload_reader_keeps_raw_values_and_source_tag_selection_rules() {
    let serializer = serializer();
    let JsonValue::Array(rows) = JsonValue::parse(include_str!(
        "../../../tests/Dock/data/serializer-json-audit.generated.read.json"
    ))
    .unwrap() else {
        panic!("source corpus");
    };
    assert_eq!(rows.len(), 25);
    for row in rows {
        let input = field(&row, "Input");
        let kind = field(&row, "Kind");
        let result = serializer.deserialize::<Rc<Holder>>(Some(input));
        if kind == "error-json" {
            let error = result
                .err()
                .unwrap_or_else(|| panic!("source failure expected: {input}"));
            assert!(matches!(
                error.cause().downcast_ref::<JsonError>().unwrap().kind,
                JsonErrorKind::InvalidJson | JsonErrorKind::InvalidReference
            ));
            continue;
        }
        let holder = result.unwrap().unwrap();
        let observed = holder.content.borrow();
        match kind {
            "null" => assert!(observed.is_none()),
            "raw" => assert!(
                observed
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<JsonValue>()
                    .is_some(),
                "{input}"
            ),
            "payload" => assert!(
                observed
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<Rc<Payload>>()
                    .is_some(),
                "{input}"
            ),
            _ => panic!("source kind"),
        }
        assert_eq!(
            serializer.serialize(holder.clone()).unwrap(),
            field(&row, "Replay"),
            "{input}"
        );
        if let Some(payload) = observed
            .as_ref()
            .and_then(|v| v.downcast_ref::<Rc<Payload>>())
        {
            payload.own.take();
        }
    }
}
