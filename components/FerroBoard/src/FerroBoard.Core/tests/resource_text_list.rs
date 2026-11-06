use ferroboard_core::{layouts::*, text::Utf16Text};
use serde_json::{Value, json};

fn text(value: &Value) -> Option<Utf16Text> {
    value.as_array().map(|a| {
        Utf16Text::from_units(
            a.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn value(text: Option<Utf16Text>) -> Value {
    text.map_or(Value::Null, |s| json!(s.units()))
}
fn error(error: ResourceTextListError) -> &'static str {
    match error {
        ResourceTextListError::IndexOutOfRange => "ArgumentOutOfRangeException",
        ResourceTextListError::NotSupported => "NotSupportedException",
        ResourceTextListError::NullReference => "NullReferenceException",
        ResourceTextListError::ArgumentNull => "ArgumentNullException",
        ResourceTextListError::KeyNotFound => "KeyNotFoundException",
        ResourceTextListError::DuplicateKey => "ArgumentException",
        ResourceTextListError::Modified => "InvalidOperationException",
    }
}
fn result<T>(r: Result<T, ResourceTextListError>, project: impl FnOnce(T) -> Value) -> Value {
    match r {
        Ok(v) => json!({"value":project(v)}),
        Err(e) => json!({"error":error(e)}),
    }
}
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/resource-text-list.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn meta(field: Option<&str>) -> std::sync::Arc<LayoutMetadata> {
    let extra = field.map_or(String::new(), |v| format!(",\"authors\":{v}"));
    LayoutJson::read_extension_manifest(&format!(
        r#"{{"meta":{{"id":"fixture"}},"layouts":{{"characters":[{{"id":"a"{extra}}}]}}}}"#
    ))
    .unwrap()
    .layouts
    .borrow()[0]
        .clone()
}
fn list(kind: &str, enumeration: bool) -> ResourceTexts {
    match kind {
        "default-array" => ResourceTexts::default(),
        "missing-list" => meta(None).authors.as_ref().unwrap().clone(),
        "empty-list" => meta(Some("[]")).authors.as_ref().unwrap().clone(),
        "list" if enumeration => ResourceTexts::new(vec!["a".into(), "b".into()]),
        "list" => ResourceTexts::from_nullable(vec![
            Some("a".into()),
            None,
            Some(Utf16Text::from_units(vec![0xd800])),
            Some("a".into()),
        ]),
        _ => panic!("unexpected kind {kind}"),
    }
}

fn operation(values: &ResourceTexts, op: &str, index: i32, input: Option<Utf16Text>) -> Value {
    let native_index = usize::try_from(index).map_err(|_| ResourceTextListError::IndexOutOfRange);
    match op {
        "get" => result(native_index.and_then(|i| values.get(i)), value),
        "set" => result(native_index.and_then(|i| values.set(i, input)), |_| {
            Value::Null
        }),
        "add" => {
            values.push(input);
            json!({"value":null})
        }
        "insert" => result(native_index.and_then(|i| values.insert(i, input)), |_| {
            Value::Null
        }),
        "remove" => json!({"value":values.remove_value(&input)}),
        "remove-at" => result(native_index.and_then(|i| values.remove_at(i)), |_| {
            Value::Null
        }),
        "contains" => json!({"value":values.contains(&input)}),
        "index-of" => json!({"value":values.position(&input).map_or(-1,|i|i as i32)}),
        "clear" => {
            values.clear();
            json!({"value":null})
        }
        _ => panic!("unexpected operation {op}"),
    }
}

/// Retain data/mutation/serialization outcomes; runtime cursor and fixed-array facets are retired.
#[test]
fn resource_texts_retain_source_data_mutations_and_json() {
    let mut current = None::<ResourceTexts>;
    let mut kind = String::new();
    let mut retained = 0;
    for row in rows() {
        match row["phase"].as_str().unwrap() {
            "identity" => {
                let missing = meta(None);
                let wrong = meta(Some("42"));
                let empty = meta(Some("[]"));
                let defaults = ResourceTexts::default();
                let actual = json!({
                    "missingShared":missing.authors.as_ref().unwrap().ptr_eq(meta(None).authors.as_ref().unwrap()),
                    "wrongShared":missing.authors.as_ref().unwrap().ptr_eq(wrong.authors.as_ref().unwrap()),
                    "defaultShared":missing.authors.as_ref().unwrap().ptr_eq(&defaults),
                    "presentEmptyShared":empty.authors.as_ref().unwrap().ptr_eq(meta(Some("[]")).authors.as_ref().unwrap()),
                    "presentEmptyIsDefault":empty.authors.as_ref().unwrap().ptr_eq(missing.authors.as_ref().unwrap())
                });
                for (key, value) in actual.as_object().unwrap() {
                    assert_eq!(value, &row[key], "{key}");
                }
                retained += 1;
            }
            "shape" => {
                kind = row["kind"].as_str().unwrap().to_owned();
                let values = list(&kind, false);
                assert_eq!(json!(values.len()), row["count"]);
                assert_eq!(
                    json!(values.iter().map(value).collect::<Vec<_>>()),
                    row["values"]
                );
                current = Some(values);
                retained += 1;
            }
            "operation" if kind != "default-array" => {
                let values = current.as_ref().unwrap();
                assert_eq!(
                    operation(
                        values,
                        row["op"].as_str().unwrap(),
                        row["index"].as_i64().unwrap() as i32,
                        text(&row["input"])
                    ),
                    row["result"],
                    "{row}"
                );
                assert_eq!(
                    json!(values.snapshot().into_iter().map(value).collect::<Vec<_>>()),
                    row["values"],
                    "{row}"
                );
                retained += 1;
            }
            "null-authors-json" => {
                let layout = KeyboardLayout::new(
                    LayoutType::Characters,
                    "local".into(),
                    "null-authors".into(),
                    LayoutArrangement::empty(),
                );
                layout.set_authors(Some(ResourceTexts::from_nullable(vec![
                    None,
                    Some("".into()),
                    Some("é".into()),
                ])));
                let written = LayoutJson::write_layout_document(&layout, false).unwrap();
                assert_eq!(json!(written), row["written"]);
                assert_eq!(
                    json!(
                        layout
                            .authors()
                            .unwrap()
                            .iter()
                            .map(value)
                            .collect::<Vec<_>>()
                    ),
                    row["sourceNulls"]
                );
                let decoded = LayoutJson::read_layout_document(
                    &written,
                    KeyboardResources::CUSTOM_EXTENSION.into(),
                )
                .unwrap();
                assert_eq!(
                    json!(
                        decoded
                            .authors()
                            .unwrap()
                            .iter()
                            .map(value)
                            .collect::<Vec<_>>()
                    ),
                    row["readAuthors"]
                );
                retained += 1;
            }
            _ => {}
        }
    }
    assert_eq!(retained, 69); // 1 identity +4 shape +63 mutable operations +1 JSON.
}

#[test]
fn native_resource_text_snapshots_release_locks_and_defaults_are_independent() {
    let first = ResourceTexts::default();
    let second = ResourceTexts::default();
    assert!(!first.ptr_eq(&second));
    first.push(Some("a".into()));
    assert!(second.is_empty());
    let alias = ResourceTexts::from_shared(first.shared_values());
    assert!(first.ptr_eq(&alias));
    let mut snapshot = first.iter();
    alias.set(0, None).unwrap();
    alias.push(Some(Utf16Text::from_units(vec![0xd800])));
    assert_eq!(snapshot.next(), Some(Some("a".into())));
    assert_eq!(snapshot.next(), None);
    let mut visited = 0;
    for _ in first.iter() {
        first.clear();
        first.push(None);
        visited += 1;
    }
    assert_eq!(visited, 2);
    assert_eq!(first.snapshot(), vec![None]);
    assert_eq!(
        first.get(usize::MAX),
        Err(ResourceTextListError::IndexOutOfRange)
    );
    assert_eq!(
        first.set(usize::MAX, None),
        Err(ResourceTextListError::IndexOutOfRange)
    );
    assert_eq!(
        first.insert(usize::MAX, None),
        Err(ResourceTextListError::IndexOutOfRange)
    );
    assert_eq!(
        first.remove_at(usize::MAX),
        Err(ResourceTextListError::IndexOutOfRange)
    );
}

#[test]
#[cfg(target_os = "macos")]
fn shared_metadata_authors_preserve_original_layout_aliases_with_snapshot_iteration() {
    fn shared<T: Send + Sync>() {}
    shared::<LayoutMetadata>();
    shared::<ResourceTexts>();
    shared::<std::vec::IntoIter<Option<Utf16Text>>>();
    let resources = KeyboardResources::create_empty();
    let changes = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = changes.clone();
    resources
        .changed()
        .add(std::rc::Rc::new(move |_| observed.set(observed.get() + 1)));
    let loads = std::rc::Rc::new(std::cell::Cell::new(0));
    let manifest=std::rc::Rc::new(LayoutJson::read_extension_manifest(r#"{"meta":{"id":"shared"},"layouts":{"characters":[{"id":"a","authors":["original"]}]}}"#).unwrap());
    let original = manifest.layouts.borrow()[0].clone();
    let loaded = loads.clone();
    resources
        .add_extension(
            manifest,
            std::rc::Rc::new(move |_| {
                loaded.set(loaded.get() + 1);
                Ok(None)
            }),
        )
        .unwrap();
    let layout = resources
        .get_layout(LayoutType::Characters, &"shared:a".into())
        .unwrap()
        .unwrap();
    assert!(
        layout
            .authors()
            .as_ref()
            .unwrap()
            .ptr_eq(original.authors.as_ref().unwrap())
    );
    changes.set(0);
    let mut snapshot = original.authors.as_ref().unwrap().iter();
    let held_current = snapshot.next().unwrap();
    let capture = |phase: &str, mutation_worker: bool, moved: Value| {
        json!({
            "phase":phase,"mutationWorker":mutation_worker,"authorsShared":layout.authors().as_ref().unwrap().ptr_eq(original.authors.as_ref().unwrap()),
            "metadataSame":std::sync::Arc::ptr_eq(&original,&resources.get_layout_metadata_for_culture(LayoutType::Characters,"").unwrap()[0]),
            "cachedSame":std::rc::Rc::ptr_eq(&layout,&resources.get_layout(LayoutType::Characters,&"shared:a".into()).unwrap().unwrap()),
            "changes":changes.get(),"loads":loads.get(),"values":original.authors.as_ref().unwrap().snapshot().into_iter().map(value).collect::<Vec<_>>(),
            "current":value(held_current.clone()),"moved":moved,
        })
    };
    let rows: Vec<Value> = rows()
        .into_iter()
        .map(|mut r| {
            if let Some(o) = r.as_object_mut() {
                o.remove("current");
                o.remove("moved");
            }
            r
        })
        .collect();
    let capture = |phase, worker, moved| {
        let mut r = capture(phase, worker, moved);
        r.as_object_mut().unwrap().remove("current");
        r.as_object_mut().unwrap().remove("moved");
        r
    };
    assert_eq!(
        capture("metadata-before", false, Value::Null),
        *rows
            .iter()
            .find(|r| r["phase"] == "metadata-before")
            .unwrap()
    );
    let main = std::thread::current().id();
    let worker = {
        let original = original.clone();
        let authors = layout.authors().as_ref().unwrap().clone();
        std::thread::spawn(move || {
            assert!(authors.ptr_eq(original.authors.as_ref().unwrap()));
            original.authors.as_ref().unwrap().set(0, None).unwrap();
            authors.push(Some(Utf16Text::from_units(vec![0xd800])));
            (original, std::thread::current().id() != main)
        })
    };
    let (returned, on_worker) = worker.join().unwrap();
    assert!(std::sync::Arc::ptr_eq(&original, &returned));
    assert_eq!(layout.authors().as_ref().unwrap().get(0).unwrap(), None);
    assert_eq!(
        layout
            .authors()
            .as_ref()
            .unwrap()
            .get(1)
            .unwrap()
            .unwrap()
            .units(),
        [0xd800]
    );
    assert_eq!(snapshot.next(), None);
    let moved = Value::Null;
    assert_eq!(
        capture("metadata-after", on_worker, moved),
        *rows
            .iter()
            .find(|r| r["phase"] == "metadata-after")
            .unwrap()
    );
    // Source resource Changed is not raised by plain author-list mutations.
    let next = resources
        .get_layout_metadata_for_culture(LayoutType::Characters, "")
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(&next[0], &original));
    assert!(
        resources
            .get_layout(LayoutType::Characters, &"shared:a".into())
            .unwrap()
            .is_some_and(|v| std::rc::Rc::ptr_eq(&v, &layout))
    );
}
