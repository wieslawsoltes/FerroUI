//! Actual source Int32 enum storage, folder contracts and registry failure boundaries.
use ferroboard_core::{keys::*, layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/layout-enums.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn text(v: &Utf16Text) -> String {
    v.to_utf8().unwrap()
}
fn range_error(error: UnsupportedLayoutType, raw: i32) -> Value {
    assert_eq!(error.0.0, raw);
    json!({"error":"ArgumentOutOfRangeException","parameter":error.parameter_name(),"actual":null})
}
fn json_attempt(result: Result<String, LayoutJsonError>) -> Value {
    match result {
        Ok(value) => json!({"value":serde_json::from_str::<Value>(&value).unwrap()}),
        Err(error) if error.0.starts_with("unsupported layout type:") => {
            json!({"error":"ArgumentOutOfRangeException","parameter":"type","actual":null})
        }
        Err(error) if error.0 == "Collection::NullReference" => {
            json!({"error":"NullReferenceException","parameter":null,"actual":null})
        }
        Err(error) => panic!("unexpected writer error: {error:?}"),
    }
}
#[test]
fn source_layout_integer_values_registration_and_writer_error_precedence() {
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "type" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let value = LayoutType(raw);
                let id = match value.to_id() {
                    Ok(id) => json!({"value":id}),
                    Err(e) => range_error(e, raw),
                };
                let subtype = match SubtypeLayoutMap::default().get(value) {
                    Ok(name) => json!({"value":text(&name.to_text())}),
                    Err(e) => range_error(e, raw),
                };
                assert_eq!(
                    json!({"kind":"type","raw":raw,"id":id,"subtype":subtype}),
                    row
                );
            }
            "layout" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let direction = row["direction"].as_i64().unwrap() as i32;
                let profile = row["profile"].as_str().unwrap();
                let l = Rc::new(KeyboardLayout::new(
                    LayoutType(raw),
                    "ext".into(),
                    "layout".into(),
                    LayoutJson::read_arrangement("[[\"a\"]]").unwrap(),
                ));
                l.set_label(Some("fixture".into()));
                l.set_direction(LayoutDirection(direction));
                l.set_authors(if matches!(profile, "null-authors" | "both-null") {
                    None
                } else {
                    Some(ResourceTexts::new(vec!["writer".into()]))
                });
                if matches!(profile, "null-arrangement" | "both-null") {
                    l.set_arrangement(None);
                }
                let r = KeyboardResources::create_empty();
                let events = Rc::new(Cell::new(0));
                let changed = events.clone();
                r.changed()
                    .add(Rc::new(move |_| changed.set(changed.get() + 1)));
                r.add_layout(l.clone());
                let m = r.get_layout_metadata(LayoutType(raw)).unwrap()[0].clone();
                let same_authors = match (l.authors(), m.authors.as_ref()) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a.ptr_eq(b),
                    _ => false,
                };
                assert_eq!(
                    json!({"kind":"layout","raw":raw,"direction":direction,"profile":profile,"typeRetained":l.layout_type().0,"directionRetained":l.direction().0,"cachedSame":Rc::ptr_eq(&l,&r.get_layout(LayoutType(raw),&l.name()).unwrap().unwrap()),"metadataType":m.layout_type.0,"metadataDirection":m.direction.0,"authorsSame":same_authors,"names":r.get_layout_names(LayoutType(raw)).iter().map(|n|text(&n.to_text())).collect::<Vec<_>>(),"events":events.get(),"document":json_attempt(LayoutJson::write_layout_document(&l,false))}),
                    row
                );
            }
            _ => {}
        }
    }
}
#[test]
fn source_extension_type_failure_keeps_partial_registration_and_lazy_load_boundaries() {
    for row in rows().into_iter().filter(|r| r["kind"] == "extension") {
        let raw = row["raw"].as_i64().unwrap() as i32;
        let bad = row["badIndex"].as_i64().unwrap() as usize;
        let r = KeyboardResources::create_empty();
        let old = Rc::new(
            LayoutJson::read_extension_manifest(r#"{"meta":{"id":"ext","title":"old"}}"#).unwrap(),
        );
        r.add_extension(old, Rc::new(|_| Ok(None))).unwrap();
        let next = Rc::new(
            LayoutJson::read_extension_manifest(r#"{"meta":{"id":"ext","title":"next"}}"#).unwrap(),
        );
        for i in 0..3 {
            let mut m = LayoutMetadata::new(
                if i == bad {
                    LayoutType(raw)
                } else {
                    LayoutType::Numeric
                },
                format!("entry{i}").as_str().into(),
            );
            m.label = m.id.clone();
            next.layouts.borrow_mut().push(Arc::new(m));
        }
        let events = Rc::new(Cell::new(0));
        let changed = events.clone();
        r.changed()
            .add(Rc::new(move |_| changed.set(changed.get() + 1)));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let called = trace.clone();
        let result = match r.add_extension(
            next.clone(),
            Rc::new(move |path| {
                called.borrow_mut().push(text(path));
                Ok(Some("[[\"a\"]]".into()))
            }),
        ) {
            Err(ResourceError::LayoutType(e)) => range_error(e, raw),
            other => panic!("expected source type error, got {other:?}"),
        };
        let before = trace.borrow().clone();
        let names = r
            .get_layout_names(LayoutType::Numeric)
            .iter()
            .map(|n| {
                text(
                    &r.get_layout(LayoutType::Numeric, n)
                        .unwrap()
                        .unwrap()
                        .id()
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            json!({"kind":"extension","raw":raw,"badIndex":bad,"result":result,"extensions":r.extensions().iter().map(|e|text(&e.title)).collect::<Vec<_>>(),"replacementSame":Rc::ptr_eq(&r.extensions()[0],&next),"names":names,"beforeLoads":before,"trace":*trace.borrow(),"events":events.get()}),
            row
        );
    }
}
#[test]
fn source_layout_and_evaluator_direction_are_independent_and_folder_parsing_stays_strict() {
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "compute" => {
                let direction = row["direction"].as_i64().unwrap() as i32;
                let context = row["contextDirection"].as_i64().unwrap() as i32;
                let r = Rc::new(KeyboardResources::create_empty());
                let l = Rc::new(KeyboardLayout::new(
                    LayoutType::Numeric,
                    "ext".into(),
                    "layout".into(),
                    LayoutJson::read_arrangement(
                        r#"[[{"$":"layout_direction_selector","ltr":"a","rtl":"b"},"c"]]"#,
                    )
                    .unwrap(),
                ));
                l.set_direction(LayoutDirection(direction));
                r.add_layout(l);
                let subtype = Arc::new(Subtype::from_values(
                    ferroboard_core::layouts::SubtypeValues {
                        layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                            ferroboard_core::layouts::SubtypeLayoutMapValues {
                                numeric: ComponentName::new("ext", "layout"),
                                ..Default::default()
                            },
                        ))),
                        ..(*Subtype::shared_default()).values()
                    },
                ));
                let k = LayoutComputer::new(r)
                    .compute(KeyboardMode::Numeric, subtype, false)
                    .unwrap();
                k.compute(Rc::new(KeyComputeContext::from_values(
                    ferroboard_core::layouts::KeyComputeContextValues {
                        mode: KeyboardMode::Numeric,
                        direction: LayoutDirection(context),
                        ..Default::default()
                    },
                )))
                .unwrap();
                k.layout(
                    321.375,
                    207.125,
                    Some(&KeyboardGeometryOptions::from_values(
                        KeyboardGeometryOptionsValues {
                            split_gap: 73.125,
                            ..Default::default()
                        },
                    )),
                )
                .unwrap();
                assert_eq!(
                    json!({"kind":"compute","direction":direction,"contextDirection":context,"retained":k.direction().0,"keys":k.keys().map(|key| key.unwrap().unwrap()).map(|v|{let b=v.touch_bounds();json!({"Code":v.code(),"bounds":([b.x,b.y,b.width,b.height].map(|v|format!("{:016x}",v.to_bits())))})}).collect::<Vec<_>>()}),
                    row
                );
            }
            "parse" => {
                let id: Option<Vec<u16>> = serde_json::from_value(row["units"].clone()).unwrap();
                let id = id.map(Utf16Text::from_units);
                let value = LayoutType::try_parse_nullable(id.as_ref());
                assert_eq!(
                    json!({"kind":"parse","units":id.map(|v|v.units().to_vec()),"ok":value.is_some(),"value":value.unwrap_or_default().0}),
                    row
                );
            }
            "document" => {
                let input = row["input"].as_str().unwrap();
                let result = match LayoutJson::read_layout_document(input, "ext".into()) {
                    Ok(l) => {
                        json!({"value":{"type":l.layout_type().0,"direction":l.direction().0}})
                    }
                    Err(e) if e.0.starts_with("Unknown layout type") => {
                        json!({"error":"FormatException","parameter":null,"actual":null})
                    }
                    Err(e) => panic!("unexpected document failure: {e:?}"),
                };
                assert_eq!(
                    json!({"kind":"document","input":input,"result":result}),
                    row
                );
            }
            _ => {}
        }
    }
}
