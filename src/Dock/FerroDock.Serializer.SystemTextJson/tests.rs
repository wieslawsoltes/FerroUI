// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use ferrodock_model::{
    core::*,
    io::{MemoryStream, Stream},
};
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
struct Payload {
    name: RefCell<Option<String>>,
    own: RefCell<Option<Rc<Payload>>>,
    other: RefCell<Option<Rc<Payload>>>,
}
fn serializer() -> Rc<DockSerializer> {
    let info = JsonTypeInfo::object(
        || Rc::new(Payload::default()),
        |value: &Rc<Payload>| Rc::as_ptr(value) as usize,
        vec![
            JsonPropertyInfo::new(
                "Name",
                |v: &Rc<Payload>| v.name.borrow().clone(),
                |v, value| {
                    v.name.replace(value);
                },
            ),
            JsonPropertyInfo::new(
                "Self",
                |v: &Rc<Payload>| v.own.borrow().clone(),
                |v, value| {
                    v.own.replace(value);
                },
            ),
            JsonPropertyInfo::new(
                "Other",
                |v: &Rc<Payload>| v.other.borrow().clone(),
                |v, value| {
                    v.other.replace(value);
                },
            ),
            JsonPropertyInfo::readonly("ReadOnly", |_: &Rc<Payload>| 7_i32),
        ],
    );
    DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(vec![
        info,
        JsonTypeInfo::optional::<Rc<Payload>>(),
        JsonTypeInfo::optional::<String>(),
    ])))
    .unwrap()
}
fn stream(bytes: &[u8]) -> Rc<MemoryStream> {
    MemoryStream::from_buffer(
        Rc::new(RefCell::new(bytes.to_vec().into_boxed_slice())),
        true,
    )
}
fn category(error: &DockSerializationError) -> JsonErrorKind {
    error.cause().downcast_ref::<JsonError>().unwrap().kind
}
#[test]
fn original_json_reference_scenarios_preserve_graph_properties_and_stream_lifetime() {
    let source = include_str!("../../../tests/Dock/data/serializer-json-audit.tsv");
    let mut seen = Vec::new();
    let mut check = |name: &str, value: bool| {
        assert!(value, "{name}");
        assert!(
            source
                .lines()
                .any(|row| row.starts_with(&format!("{name}\t1"))),
            "source case missing"
        );
        seen.push(name.to_owned());
    };
    let serializer = serializer();
    let payload = Rc::new(Payload::default());
    payload.name.replace(Some("alpha".into()));
    payload.own.replace(Some(payload.clone()));
    let text = serializer.serialize(payload.clone()).unwrap();
    assert_eq!(
        text,
        include_str!("../../../tests/Dock/data/serializer-json-audit.payload.json")
    );
    let node = JsonValue::parse(&text).unwrap();
    check(
        "json-readonly-property-is-included",
        node.property("ReadOnly") == Some(&JsonValue::number(7)),
    );
    check(
        "json-self-reference-preserved-on-wire",
        node.property("Self").unwrap().property("$ref") == node.property("$id"),
    );
    check(
        "json-null-property-omitted",
        node.property("Other").is_none(),
    );
    let restored = serializer
        .deserialize::<Rc<Payload>>(Some(&text))
        .unwrap()
        .unwrap();
    check(
        "json-self-reference-restored",
        Rc::ptr_eq(&restored, &restored.own.borrow().as_ref().unwrap().clone()),
    );
    let saved = MemoryStream::new();
    serializer
        .save(Some(saved.clone()), payload.clone())
        .unwrap();
    check(
        "json-save-closes-stream",
        !saved.can_read() && !saved.can_write(),
    );
    check(
        "json-save-emits-utf8-preamble",
        saved.to_array().starts_with(&[239, 187, 191]),
    );
    let loaded = stream(&saved.to_array());
    let loaded_payload = serializer
        .load::<Rc<Payload>>(Some(loaded.clone()))
        .unwrap()
        .unwrap();
    check(
        "json-load-closes-stream",
        !loaded.can_read() && loaded_payload.name.borrow().as_deref() == Some("alpha"),
    );
    let empty = MemoryStream::new();
    check(
        "json-empty-load-throws",
        serializer.load::<Rc<Payload>>(Some(empty.clone())).is_err(),
    );
    check("json-empty-load-still-closes", !empty.can_read());
    let malformed = stream(b"{");
    check(
        "json-malformed-load-throws",
        serializer
            .load::<Rc<Payload>>(Some(malformed.clone()))
            .is_err(),
    );
    check("json-malformed-load-still-closes", !malformed.can_read());
    check(
        "json-null-load-rejected",
        category(&serializer.load::<Rc<Payload>>(None).err().unwrap())
            == JsonErrorKind::ArgumentNull,
    );
    check(
        "json-null-save-rejected",
        category(&serializer.save(None, payload.clone()).unwrap_err())
            == JsonErrorKind::ArgumentNull,
    );
    assert_eq!(seen.len(), source.lines().count());
    payload.own.take();
    restored.own.take();
    loaded_payload.own.take();
}
#[test]
fn duplicate_properties_and_raw_numbers_survive_dom_replay() {
    let node = JsonValue::parse(r#"{"$type":"first","$type":"last","number":1.2300e+02}"#).unwrap();
    assert_eq!(
        node.property("$type"),
        Some(&JsonValue::String("last".into()))
    );
    let replay = node.to_json().unwrap();
    assert_eq!(replay.matches("\"$type\"").count(), 2);
    assert!(replay.contains("1.2300e+02"));
    assert!(JsonNumber::parse("true").is_err());
    assert_eq!(JsonNumber::parse("1").unwrap().as_str(), "1");
    assert!(JsonValue::parse("{").is_err());
    assert!(JsonValue::parse("01").is_err());
    assert!(JsonValue::parse(&format!("{}{}", "[".repeat(65), "]".repeat(65))).is_err());
    assert!(JsonValue::parse(&format!("{}{}", "[".repeat(64), "]".repeat(64))).is_ok());
}
#[test]
fn typed_metadata_null_errors_and_independent_reference_sessions() {
    assert!(
        matches!(DockSerializer::with_resolver(None),Err(e) if category(&e)==JsonErrorKind::ArgumentNull)
    );
    let serializer = serializer();
    assert!(
        serializer.options().write_indented()
            && serializer.options().preserve_references()
            && serializer.options().ignore_null_properties()
            && serializer.options().allow_named_floating_point_literals()
    );
    assert!(serializer
        .deserialize::<Rc<Payload>>(Some("null"))
        .unwrap()
        .is_none());
    assert!(serializer.deserialize::<i32>(Some("null")).is_err());
    assert_eq!(
        category(&serializer.deserialize::<String>(None).unwrap_err()),
        JsonErrorKind::ArgumentNull
    );
    assert_eq!(
        category(&serializer.deserialize::<u128>(Some("{")).unwrap_err()),
        JsonErrorKind::MissingMetadata
    );
    for json in [
        r#"{"$ref":"missing"}"#,
        r#"{"$ref":"1","Name":"bad"}"#,
        r#"{"Name":"a","$id":"1"}"#,
    ] {
        assert!(serializer.deserialize::<Rc<Payload>>(Some(json)).is_err());
    }
    let payload = Rc::new(Payload::default());
    let first = serializer.serialize(payload.clone()).unwrap();
    let second = serializer.serialize(payload).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        JsonValue::parse(&first).unwrap().property("$id"),
        Some(&JsonValue::String("1".into()))
    );
}
#[test]
fn source_text_encoding_and_existing_position_are_retained() {
    let serializer = serializer();
    for little in [true, false] {
        let mut bytes = if little {
            vec![255, 254]
        } else {
            vec![254, 255]
        };
        for unit in "\"é\"".encode_utf16() {
            bytes.extend_from_slice(&if little {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            });
        }
        assert_eq!(
            serializer
                .load::<String>(Some(stream(&bytes)))
                .unwrap()
                .as_deref(),
            Some("é")
        );
    }
    let target = MemoryStream::new();
    target.write(b"prefix").unwrap();
    serializer
        .save(Some(target.clone()), "text".to_owned())
        .unwrap();
    assert_eq!(target.to_array(), b"prefix\"text\"");
}

#[test]
fn string_wire_output_matches_original_json_encoder() {
    let serializer = serializer();
    let values = [
        ("ascii", "alpha"),
        ("quote", "\""),
        ("slash", "\\/"),
        ("html", "<>&'+`"),
        ("controls", "\n\r\t\u{0008}\u{000c}\0"),
        ("unicode", "é界🙂"),
    ];
    let corpus = include_str!("../../../tests/Dock/data/serializer-json-audit.wire.tsv");
    for (name, value) in values {
        let expected = corpus
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}\t")))
            .unwrap();
        let actual = serializer.serialize(value.to_owned()).unwrap();
        assert_eq!(actual, expected, "{name}");
        assert_eq!(
            serializer
                .deserialize::<String>(Some(expected))
                .unwrap()
                .as_deref(),
            Some(value)
        );
    }
}

#[test]
fn resolver_metadata_is_cached_per_options_and_nullable_values_remain_distinct() {
    use std::{any::TypeId, cell::Cell};
    struct Resolver {
        inner: Rc<GeneratedJsonTypeInfoResolver>,
        calls: Cell<usize>,
        options: Cell<usize>,
    }
    impl IJsonTypeInfoResolver for Resolver {
        fn get_type_info(
            &self,
            id: TypeId,
            options: &JsonSerializerOptions,
        ) -> Option<Rc<JsonTypeInfo>> {
            self.calls.set(self.calls.get() + 1);
            let pointer = options as *const _ as usize;
            if self.options.get() == 0 {
                self.options.set(pointer);
            } else {
                assert_eq!(self.options.get(), pointer);
            }
            self.inner.get_type_info(id, options)
        }
    }
    let resolver = Rc::new(Resolver {
        inner: GeneratedJsonTypeInfoResolver::new(vec![JsonTypeInfo::optional::<i32>()]),
        calls: Cell::new(0),
        options: Cell::new(0),
    });
    let serializer = DockSerializer::with_resolver(Some(resolver.clone())).unwrap();
    assert_eq!(serializer.serialize(1_i32).unwrap(), "1");
    assert_eq!(serializer.deserialize::<i32>(Some("2")).unwrap(), Some(2));
    assert_eq!(resolver.calls.get(), 1);
    assert_eq!(
        serializer.deserialize::<Option<i32>>(Some("null")).unwrap(),
        Some(None)
    );
    assert_eq!(
        serializer.deserialize::<Option<i32>>(Some("7")).unwrap(),
        Some(Some(7))
    );
    assert_eq!(serializer.serialize(None::<i32>).unwrap(), "null");
    assert_eq!(resolver.calls.get(), 2);
}

struct ProbeStream {
    inner: Rc<MemoryStream>,
    read_error: bool,
    write_error: bool,
    flush_error: bool,
    close_error: bool,
    readable: bool,
    writable: bool,
    closes: std::cell::Cell<usize>,
}
impl Default for ProbeStream {
    fn default() -> Self {
        Self {
            inner: MemoryStream::new(),
            read_error: false,
            write_error: false,
            flush_error: false,
            close_error: false,
            readable: true,
            writable: true,
            closes: std::cell::Cell::new(0),
        }
    }
}
impl ferroui_system::IDisposable for ProbeStream {
    fn dispose(&self) {
        self.close().unwrap();
    }
}
impl Stream for ProbeStream {
    fn can_read(&self) -> bool {
        self.readable && self.inner.can_read()
    }
    fn can_write(&self) -> bool {
        self.writable && self.inner.can_write()
    }
    fn can_seek(&self) -> bool {
        self.inner.can_seek()
    }
    fn len(&self) -> std::io::Result<u64> {
        self.inner.len()
    }
    fn position(&self) -> std::io::Result<u64> {
        self.inner.position()
    }
    fn set_position(&self, v: i64) -> std::io::Result<()> {
        self.inner.set_position(v)
    }
    fn set_len(&self, v: i64) -> std::io::Result<()> {
        self.inner.set_len(v)
    }
    fn read(&self, b: &mut [u8]) -> std::io::Result<usize> {
        if self.read_error {
            Err(std::io::Error::other("read"))
        } else {
            self.inner.read(b)
        }
    }
    fn write(&self, b: &[u8]) -> std::io::Result<()> {
        if self.write_error {
            Err(std::io::Error::other("write"))
        } else {
            self.inner.write(b)
        }
    }
    fn seek(&self, v: std::io::SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(v)
    }
    fn flush(&self) -> std::io::Result<()> {
        if self.flush_error {
            Err(std::io::Error::other("flush"))
        } else {
            self.inner.flush()
        }
    }
    fn close(&self) -> std::io::Result<()> {
        self.closes.set(self.closes.get() + 1);
        self.inner.close()?;
        if self.close_error {
            Err(std::io::Error::other("close"))
        } else {
            Ok(())
        }
    }
}
#[test]
fn source_stream_failure_and_finally_precedence_are_preserved() {
    let serializer = serializer();
    let corpus = include_str!("../../../tests/Dock/data/serializer-json-audit.failures.tsv");
    let mut seen = Vec::new();
    let mut check = |name: &str, value: bool| {
        assert!(value, "{name}");
        assert!(corpus.lines().any(|row| row == format!("{name}\t1")));
        seen.push(name.to_owned());
    };
    let stream = Rc::new(ProbeStream {
        read_error: true,
        ..Default::default()
    });
    let error = serializer.load::<String>(Some(stream.clone())).unwrap_err();
    check(
        "load-read-error-identity",
        error.cause().downcast_ref::<std::io::Error>().is_some() && error.to_string() == "read",
    );
    check("load-read-error-closes", stream.closes.get() == 1);
    let stream = Rc::new(ProbeStream {
        read_error: true,
        close_error: true,
        ..Default::default()
    });
    check(
        "load-close-error-overrides-read",
        serializer
            .load::<String>(Some(stream))
            .unwrap_err()
            .to_string()
            == "close",
    );
    let stream = Rc::new(ProbeStream {
        close_error: true,
        ..Default::default()
    });
    stream.inner.write(b"{").unwrap();
    stream.inner.set_position(0).unwrap();
    check(
        "load-close-error-overrides-json",
        serializer
            .load::<String>(Some(stream))
            .unwrap_err()
            .to_string()
            == "close",
    );
    let stream = Rc::new(ProbeStream {
        write_error: true,
        ..Default::default()
    });
    check(
        "save-write-error-identity",
        serializer
            .save(Some(stream.clone()), "value".to_owned())
            .unwrap_err()
            .to_string()
            == "write",
    );
    check("save-write-error-closes", stream.closes.get() == 1);
    let stream = Rc::new(ProbeStream {
        flush_error: true,
        ..Default::default()
    });
    check(
        "save-flush-error-identity",
        serializer
            .save(Some(stream.clone()), "value".to_owned())
            .unwrap_err()
            .to_string()
            == "flush",
    );
    check("save-flush-error-closes", stream.closes.get() == 1);
    let stream = Rc::new(ProbeStream {
        write_error: true,
        close_error: true,
        ..Default::default()
    });
    check(
        "save-close-error-overrides-write",
        serializer
            .save(Some(stream), "value".to_owned())
            .unwrap_err()
            .to_string()
            == "close",
    );
    let stream = Rc::new(ProbeStream {
        readable: false,
        ..Default::default()
    });
    check(
        "load-unreadable-constructor-does-not-close",
        category(&serializer.load::<String>(Some(stream.clone())).unwrap_err())
            == JsonErrorKind::Argument
            && stream.closes.get() == 0,
    );
    let stream = Rc::new(ProbeStream {
        writable: false,
        ..Default::default()
    });
    check(
        "save-unwritable-constructor-does-not-close",
        category(
            &serializer
                .save(Some(stream.clone()), "value".to_owned())
                .unwrap_err(),
        ) == JsonErrorKind::Argument
            && stream.closes.get() == 0,
    );
    struct Failing;
    let original = DockSerializationError::new(std::io::Error::other("getter"));
    let failure = original.clone();
    let codec = JsonTypeInfo::converter(
        move |_: &Failing, _| Err(failure.clone()),
        |_, _| Ok(None::<Failing>),
    );
    let serializer =
        DockSerializer::with_resolver(Some(GeneratedJsonTypeInfoResolver::new(vec![codec])))
            .unwrap();
    check(
        "save-serialize-precedes-null-stream",
        serializer
            .save(None, Failing)
            .unwrap_err()
            .ptr_eq(&original),
    );
    assert_eq!(seen.len(), corpus.lines().count());
}
