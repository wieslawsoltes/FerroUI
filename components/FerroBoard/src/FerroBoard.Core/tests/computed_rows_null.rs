use ferroboard_core::{keys::*, layouts::*};
use serde_json::{Value, json};
use std::{rc::Rc, sync::Arc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/computed-rows-null.jsonl")
        .lines()
        .map(|v| serde_json::from_str(v).unwrap())
        .collect()
}
fn build() -> ComputedKeyboard {
    let r = Rc::new(KeyboardResources::create_empty());
    r.add_layout(Rc::new(KeyboardLayout::new(
        LayoutType::Numeric,
        "probe".into(),
        "main".into(),
        Rc::new(LayoutArrangement::from_rows(vec![
            vec![
                TextKeyData::char("a".into(), None).unwrap(),
                TextKeyData::char("b".into(), None).unwrap(),
            ],
            vec![TextKeyData::char("c".into(), None).unwrap()],
        ])),
    )));
    let s = Arc::new(Subtype::from_values(
        ferroboard_core::layouts::SubtypeValues {
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    numeric: ComponentName::new("probe", "main"),
                    ..(*Subtype::shared_default().layouts().unwrap()).values()
                },
            ))),
            ..(*Subtype::shared_default()).values()
        },
    ));
    let b = LayoutComputer::new(r)
        .compute(KeyboardMode::Numeric, s, false)
        .unwrap();
    b.compute(context()).unwrap();
    b
}
fn context() -> Rc<KeyComputeContext> {
    Rc::new(KeyComputeContext::from_values(KeyComputeContextValues {
        mode: KeyboardMode::Numeric,
        ..Default::default()
    }))
}
fn label(key: Option<Rc<ComputedKey>>) -> Value {
    json!(key.and_then(|k| k.label()).map(|v| v.to_utf8().unwrap()))
}
fn mutation(b: &ComputedKeyboard, op: &str) {
    let r = b.rows();
    match op {
        "none" => {}
        "outer-null" => r.set_nullable(0, None).unwrap(),
        "outer-add-null" => r.push_nullable(None),
        "inner-null" => r.get(0).unwrap().set_nullable(1, None).unwrap(),
        "inner-add-null" => r.get(0).unwrap().push_nullable(None),
        "null-row-first" => r.insert_nullable(0, None).unwrap(),
        "null-key-first" => r.get(0).unwrap().insert_nullable(0, None).unwrap(),
        _ => panic!("{op}"),
    }
}
fn error(e: KeyDataError) -> &'static str {
    match e {
        KeyDataError::NullReference
        | KeyDataError::Collection(ResourceTextListError::NullReference) => {
            "NullReferenceException"
        }
        KeyDataError::Collection(ResourceTextListError::ArgumentNull) => "ArgumentNullException",
        KeyDataError::Collection(ResourceTextListError::IndexOutOfRange) => {
            "ArgumentOutOfRangeException"
        }
        KeyDataError::Collection(ResourceTextListError::Modified) => "InvalidOperationException",
        KeyDataError::Collection(ResourceTextListError::NotSupported) => "NotSupportedException",
        v => panic!("{v:?}"),
    }
}
fn result(v: Result<Value, KeyDataError>) -> Value {
    match v {
        Ok(v) => json!({"value":v}),
        Err(e) => json!({"error":error(e)}),
    }
}
fn caught(call: impl FnOnce() -> Result<Value, KeyDataError>) -> Value {
    result(call())
}
#[test]
fn native_nullable_key_iteration_retains_source_values_errors_and_resumption() {
    let all = rows();
    let mut retained = 0;
    for query in ["keys", "visible"] {
        for op in [
            "none",
            "outer-null",
            "outer-add-null",
            "inner-null",
            "inner-add-null",
            "null-row-first",
            "null-key-first",
        ] {
            for timing in ["before-first", "after-first"] {
                let b = build();
                let mut iterator = if query == "keys" {
                    b.keys()
                } else {
                    b.visible_keys()
                };
                if timing == "after-first" {
                    assert!(iterator.next().unwrap().is_ok());
                }
                mutation(&b, op);
                for step in 0..7 {
                    let expected = all
                        .iter()
                        .find(|v| {
                            v["tag"] == "cursor"
                                && v["query"] == query
                                && v["mutation"] == op
                                && v["timing"] == timing
                                && v["step"] == step
                        })
                        .unwrap();
                    let (moved, item) = match iterator.next() {
                        Some(Ok(value)) => (json!({"value":true}), Some(value)),
                        Some(Err(failure)) => (json!({"error":error(failure)}), None),
                        None => (json!({"value":false}), None),
                    };
                    assert_eq!(moved, expected["moved"], "{expected}");
                    if let Some(value) = item {
                        assert_eq!(
                            json!({"value":label(value)}),
                            expected["current"],
                            "{expected}"
                        );
                    }
                    retained += 1;
                }
            }
        }
    }
    // Current/boxed Current after failure, Reset and Dispose are runtime facets.
    assert_eq!(retained, 196);
}

#[test]
fn unchanged_null_rows_preserve_compute_layout_partial_state_and_find_hit_short_circuit() {
    let mut count = 0;
    for row in rows().into_iter().filter(|v| v["tag"] == "operation") {
        let b = build();
        b.layout(300.0, 120.0, None).unwrap();
        let originals = b
            .keys()
            .map(|key| key.unwrap().unwrap())
            .collect::<Vec<_>>();
        let op = row["op"].as_str().unwrap();
        let change = row["mutation"].as_str().unwrap();
        mutation(&b, change);
        let ctx = context();
        let actual = caught(|| {
            Ok(match op {
                "compute" => {
                    b.compute(ctx.clone())?;
                    Value::Null
                }
                "layout" => {
                    b.layout(320.5, 100.25, None)?;
                    Value::Null
                }
                "find-a" => label(b.find_key('a' as i32)?),
                "find-c" => label(b.find_key('c' as i32)?),
                "find-missing" => label(b.find_key(-999)?),
                "hit-a" => label(b.hit_test(0.0, 0.0)?),
                "hit-bottom" => label(b.hit_test(0.0, 999.0)?),
                "characters" => json!(
                    b.character_keys()?
                        .into_iter()
                        .map(|v| label(Some(v)))
                        .collect::<Vec<_>>()
                ),
                _ => panic!("{op}"),
            })
        });
        let state = b
            .rows()
            .snapshot_nullable()
            .into_iter()
            .map(|r| {
                r.map(|r| {
                    r.snapshot_nullable()
                        .into_iter()
                        .map(label)
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            json!({"tag":"operation","mutation":change,"op":op,"result":actual,"version":b.version(),"contextSame":b.context().is_some_and(|v|Rc::ptr_eq(&v,&ctx)),"width":format!("{:016x}",b.width().to_bits()),"height":format!("{:016x}",b.height().to_bits()),"rows":state,"originals":originals.into_iter().map(|v|label(Some(v))).collect::<Vec<_>>()}),
            row
        );
        count += 1;
    }
    assert_eq!(count, 48);
}
#[test]
fn nullable_computed_lists_keep_original_handles_counts_and_versioned_mutation() {
    let b = build();
    let rows = b.rows();
    assert!(rows.ptr_eq(&b.rows()));
    let row = rows.get(0).unwrap();
    assert!(row.ptr_eq(&rows.get(0).unwrap()));
    let storage = row.clone();
    row.push_nullable(None);
    assert_eq!(row.len(), 3);
    assert!(storage.get_nullable(2).unwrap().is_none());
    assert!(row.get_nullable(2).unwrap().is_none());
    assert!(matches!(
        row.get(2),
        Err(ResourceTextListError::NullReference)
    ));
    let mut cursor = row.iter_nullable();
    assert!(cursor.next().unwrap().unwrap().is_some());
    row.insert_nullable(0, None).unwrap();
    assert!(matches!(
        cursor.next(),
        Some(Err(ResourceTextListError::Modified))
    ));
    row.remove(0).unwrap();
    row.set_nullable(0, None).unwrap();
    assert!(storage.get_nullable(0).unwrap().is_none());
    assert!(matches!(
        row.insert_nullable(usize::MAX, None),
        Err(ResourceTextListError::IndexOutOfRange)
    ));
    assert!(matches!(
        row.insert_nullable(row.len() + 1, None),
        Err(ResourceTextListError::IndexOutOfRange)
    ));
    let a = ComputedRow::default();
    let other = ComputedRow::default();
    assert!(!a.ptr_eq(&other));
    a.push_nullable(None);
    assert_eq!(a.len(), 1);
    assert!(other.is_empty());
}

#[test]
fn unchanged_list_insert_bounds_null_values_and_interface_cursor_versions() {
    let mut count = 0;
    for row in rows().into_iter().filter(|v| v["tag"] == "insert") {
        let b = build();
        let originals = b
            .keys()
            .map(|key| key.unwrap().unwrap())
            .collect::<Vec<_>>();
        let initial = row["initial"].as_u64().unwrap() as usize;
        let index = row["index"].as_i64().unwrap() as i32;
        let is_null = row["isNull"].as_bool().unwrap();
        let list = ComputedRow::new(originals[..initial].to_vec());
        let mut cursor = list.iter_nullable();
        let value = (!is_null).then(|| originals[2].clone());
        let inserted = result(
            usize::try_from(index)
                .map_err(|_| ResourceTextListError::IndexOutOfRange)
                .and_then(|index| list.insert_nullable(index, value))
                .map(|_| Value::Null)
                .map_err(Into::into),
        );
        let moved = result(match cursor.next() {
            Some(Ok(_)) => Ok(json!(true)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(json!(false)),
        });
        assert_eq!(
            json!({"tag":"insert","initial":initial,"index":index,"isNull":is_null,"result":inserted,"count":list.len(),"values":list.snapshot_nullable().into_iter().map(label).collect::<Vec<_>>(),"moved":moved}),
            row
        );
        count += 1;
    }
    assert_eq!(count, 28);
}
