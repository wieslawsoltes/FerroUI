//! Exact public original nullable ComponentName/KeyboardLayout and registration contracts.
use ferroboard_core::{keys::*, layouts::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::Cell,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::Arc,
};
fn units(s: &Option<Utf16Text>) -> Value {
    json!(s.as_ref().map(|s| s.units()))
}
fn name(n: &ComponentName) -> Value {
    json!({"extension":units(&n.extension_id),"component":units(&n.component_id),"text":n.to_text().units()})
}
fn values() -> Vec<Option<Utf16Text>> {
    vec![
        None,
        Some("".into()),
        Some("e".into()),
        Some("x:y".into()),
        Some(Utf16Text::from_units(vec![0xd800])),
        Some("😀".into()),
    ]
}
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/keyboard-layout.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn authors_same(a: &Option<ResourceTexts>, b: &Option<ResourceTexts>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.ptr_eq(b),
        (None, None) => true,
        _ => false,
    }
}
fn arrangement_same(a: &Option<Rc<LayoutArrangement>>, b: &Option<Rc<LayoutArrangement>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}
#[test]
fn original_nullable_component_record_format_parse_equality_hash_and_deconstruction() {
    let expected = rows();
    let mut n = 0;
    let vals = values();
    for extension in &vals {
        for component in &vals {
            let a = ComponentName::new_nullable(extension.clone(), component.clone());
            let b = a.clone();
            let mut changed = a.clone();
            changed.component_id = Some("changed".into());
            let (e, c) = (a.extension_id.clone(), a.component_id.clone());
            let mut ah = std::collections::hash_map::DefaultHasher::new();
            let mut bh = std::collections::hash_map::DefaultHasher::new();
            a.hash(&mut ah);
            b.hash(&mut bh);
            let actual = json!({"kind":"component","extension":units(extension),"component":units(component),"name":name(&a),"equal":a==b,"hashEqual":ah.finish()==bh.finish(),"emptyEqual":a==ComponentName::new("",""),"defaultEqual":a==ComponentName::default(),"deconstruct":[units(&e),units(&c)],"changed":name(&changed)});
            assert_eq!(actual, expected[n]);
            n += 1;
        }
    }
    let mut inputs = values();
    inputs.extend([":", ":a", "a:", "a:b:c"].map(|s| Some(s.into())));
    for value in &inputs {
        for extension in &vals {
            let result = match ComponentName::parse_nullable(value.as_ref(), extension.as_ref()) {
                Ok(v) => json!({"value":name(&v)}),
                Err(ResourceTextListError::NullReference) => {
                    json!({"error":"NullReferenceException"})
                }
                Err(e) => panic!("unexpected {e:?}"),
            };
            let actual = json!({"kind":"parse","value":units(value),"extension":units(extension),"result":result});
            assert_eq!(actual, expected[n]);
            n += 1;
        }
    }
    assert_eq!(n, 96);
}
#[test]
fn original_nullable_layout_defaults_serialization_registration_metadata_identity_and_compute() {
    let expected = rows();
    let mut n = 96;
    for extension in [None, Some(""), Some("e")] {
        for id in [None, Some(""), Some("i")] {
            for label in [None, Some(""), Some("label")] {
                for authors_kind in ["default", "null", "list"] {
                    for arrangement_kind in ["normal", "null", "null-rows"] {
                        let extension = extension.map(Utf16Text::from);
                        let id = id.map(Utf16Text::from);
                        let label = label.map(Utf16Text::from);
                        let authors =
                            ResourceTexts::from_nullable(vec![Some("author".into()), None]);
                        let arrangement = match arrangement_kind {
                            "null" => None,
                            "null-rows" => Some(Rc::new(LayoutArrangement::new_nullable(None))),
                            _ => Some(LayoutJson::read_arrangement(r#"[["a"]]"#).unwrap()),
                        };
                        let layout = KeyboardLayout::new_nullable(
                            LayoutType::Numeric,
                            extension.clone(),
                            id.clone(),
                            arrangement.clone(),
                        );
                        layout.set_label(label.clone());
                        layout.set_modifier(Some(ComponentName::new_nullable(
                            None,
                            Some("m".into()),
                        )));
                        if authors_kind == "null" {
                            layout.set_authors(None);
                        }
                        if authors_kind == "list" {
                            layout.set_authors(Some(authors.clone()));
                        }
                        let layout = Rc::new(layout);
                        let resources = Rc::new(KeyboardResources::create_empty());
                        let events = Rc::new(Cell::new(0));
                        let e = events.clone();
                        resources
                            .changed()
                            .add(Rc::new(move |_| e.set(e.get() + 1)));
                        resources.add_layout(layout.clone());
                        let cached = resources
                            .get_layout(layout.layout_type(), &layout.name())
                            .unwrap()
                            .unwrap();
                        let meta =
                            resources.get_layout_metadata(LayoutType::Numeric).unwrap()[0].clone();
                        let written = match LayoutJson::write_layout_document(&layout, false) {
                            Ok(v) => json!({"value":serde_json::from_str::<Value>(&v).unwrap()}),
                            Err(e) => {
                                assert_eq!(e.0, "Collection::NullReference");
                                json!({"error":"NullReferenceException"})
                            }
                        };
                        let subtype = Arc::new(Subtype::from_values(
                            ferroboard_core::layouts::SubtypeValues {
                                layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                                    ferroboard_core::layouts::SubtypeLayoutMapValues {
                                        numeric: layout.name(),
                                        ..(*Subtype::shared_default().layouts().unwrap()).values()
                                    },
                                ))),
                                ..(*Subtype::shared_default()).values()
                            },
                        ));
                        let mut keyboard = None;
                        let error = match LayoutComputer::new(resources.clone()).compute(
                            KeyboardMode::Numeric,
                            subtype,
                            false,
                        ) {
                            Err(ResourceError::Collection(
                                ResourceTextListError::NullReference,
                            )) => Some("NullReferenceException"),
                            Err(e) => panic!("unexpected {e:?}"),
                            Ok(k) => {
                                k.compute(Rc::new(KeyComputeContext::from_values(
                                    ferroboard_core::layouts::KeyComputeContextValues {
                                        mode: KeyboardMode::Numeric,
                                        show_number_hints: false,
                                        show_symbol_hints: false,
                                        ..Default::default()
                                    },
                                )))
                                .unwrap();
                                keyboard = Some(k);
                                None
                            }
                        };
                        if authors_kind == "list" {
                            authors.push(Some("later".into()));
                        }
                        let actual = json!({"kind":"layout","extension":units(&extension),"id":units(&id),"label":units(&label),"authorsKind":authors_kind,"arrangementKind":arrangement_kind,"name":name(&layout.name()),"cachedSame":Rc::ptr_eq(&cached,&layout),"metadataId":units(&meta.id),"metadataLabel":units(&meta.label),"authorsShared":authors_same(&meta.authors,&layout.authors()),"metadataAuthors":meta.authors.as_ref().map(|a|a.snapshot().iter().map(units).collect::<Vec<_>>()),"arrangementSame":arrangement_same(&cached.arrangement(),&arrangement),"names":resources.get_layout_names(LayoutType::Numeric).iter().map(name).collect::<Vec<_>>(),"events":events.get(),"written":written,"error":error,"rows":keyboard.as_ref().map(ComputedKeyboard::row_count),"version":keyboard.as_ref().map(ComputedKeyboard::version)});
                        assert_eq!(
                            actual, expected[n],
                            "{extension:?}/{id:?}/{label:?}/{authors_kind}/{arrangement_kind}"
                        );
                        n += 1;
                    }
                }
            }
        }
    }
    let resources = KeyboardResources::create_empty();
    let a = Rc::new(KeyboardLayout::new_nullable(
        LayoutType::Numeric,
        None,
        None,
        Some(LayoutArrangement::empty()),
    ));
    let b = Rc::new(KeyboardLayout::new(
        LayoutType::Numeric,
        "".into(),
        "".into(),
        LayoutArrangement::empty(),
    ));
    resources.add_layout(a.clone());
    resources.add_layout(b.clone());
    let actual = json!({"kind":"distinct-null-empty","count":resources.get_layout_names(LayoutType::Numeric).len(),"nullSame":Rc::ptr_eq(&a,&resources.get_layout(LayoutType::Numeric,&ComponentName::default()).unwrap().unwrap()),"emptySame":Rc::ptr_eq(&b,&resources.get_layout(LayoutType::Numeric,&ComponentName::new("","")).unwrap().unwrap()),"sameText":a.name().to_text()==b.name().to_text(),"names":resources.get_layout_names(LayoutType::Numeric).iter().map(name).collect::<Vec<_>>()});
    assert_eq!(actual, expected[n]);
    n += 1;
    let a = KeyboardLayout::new(
        LayoutType::Characters,
        "e".into(),
        "i".into(),
        LayoutArrangement::empty(),
    );
    let b = KeyboardLayout::new(
        LayoutType::Characters,
        "e".into(),
        "i".into(),
        a.arrangement().clone().unwrap(),
    );
    let actual = json!({"kind":"defaults","label":units(&a.label()),"direction":a.direction().0,"modifier":a.modifier().is_none(),"arrangementSame":arrangement_same(&a.arrangement(),&b.arrangement()),"name":name(&a.name())});
    let mut native_expected = expected[n].clone();
    native_expected
        .as_object_mut()
        .unwrap()
        .remove("authorsShared");
    native_expected
        .as_object_mut()
        .unwrap()
        .remove("authorsReadOnly");
    assert!(!authors_same(&a.authors(), &b.authors()));
    assert_eq!(actual, native_expected);
    n += 1;
    assert_eq!(n, 341);
}
#[test]
fn original_document_nullable_extension_modifier_default_and_invalid_json_contracts() {
    let expected = rows();
    let mut n = 341;
    for extension in [None, Some(""), Some("e")] {
        for input in [
            r#"{"id":"i"}"#,
            r#"{"id":null}"#,
            r#"{"id":"i","label":null,"modifier":"mod"}"#,
            r#"{"id":"i","modifier":":m","authors":[null]}"#,
            r#"{"id":"i","arrangement":null}"#,
        ] {
            let extension = extension.map(Utf16Text::from);
            let result = match LayoutJson::read_layout_document_nullable(input, extension.clone()) {
                Ok(l) => {
                    json!({"value":{"extension":units(&l.extension_id()),"id":units(&l.id()),"label":units(&l.label()),"name":name(&l.name()),"modifier":l.modifier().as_ref().map(name),"authors":l.authors().as_ref().unwrap().snapshot().iter().map(units).collect::<Vec<_>>(),"arrangementSame":Rc::ptr_eq(l.arrangement().as_ref().unwrap(),&LayoutArrangement::empty())}})
                }
                Err(e) => {
                    assert!(
                        e.0.contains("requires an 'id'") || e.0.starts_with("Layout arrangement")
                    );
                    json!({"error":"FormatException"})
                }
            };
            let actual = json!({"kind":"document","extension":units(&extension),"input":input,"result":result});
            assert_eq!(actual, expected[n]);
            n += 1;
        }
    }
    assert_eq!(n, 356);
    let mut inputs = values();
    inputs.extend([":", ":a", "a:", "a:b:c"].map(|s| Some(s.into())));
    for value in inputs {
        let result = match ComponentName::try_from(value.clone()) {
            Ok(v) => {
                assert_eq!(v, ComponentName::from(value.clone().unwrap()));
                json!({"value":name(&v)})
            }
            Err(ResourceTextListError::NullReference) => json!({"error":"NullReferenceException"}),
            Err(e) => panic!("unexpected {e:?}"),
        };
        assert_eq!(
            json!({"kind":"implicit","value":units(&value),"result":result}),
            expected[n]
        );
        n += 1;
    }
    assert_eq!(n, 366);
    assert_eq!(expected.len(), 366);
}
