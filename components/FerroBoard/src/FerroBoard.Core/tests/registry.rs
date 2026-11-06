use ferroboard_core::{
    keys::KeyVariation,
    layouts::*,
    text::{AppenderComposer, IComposer, Utf16Text},
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
fn text(t: &Utf16Text) -> String {
    t.to_utf8().unwrap()
}
fn parse(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}
fn corpus() -> Vec<Value> {
    include_str!("../../../tests/fixtures/registry.jsonl")
        .lines()
        .map(parse)
        .collect()
}
fn manifest(json: &str) -> Rc<KeyboardExtensionManifest> {
    Rc::new(LayoutJson::read_extension_manifest(json).unwrap())
}
const MANIFEST: &str =
    r#"{"meta":{"id":"e"},"layouts":{"characters":[{"id":"x"}]},"popupMappings":[{"id":"x"}]}"#;
fn arrangement(c: &str) -> String {
    format!(r#"[["{c}"]]"#)
}
fn popup(c: &str) -> String {
    format!(r#"{{"all":{{"a":["{c}"]}}}}"#)
}
fn popup_model(c: &str) -> Rc<PopupMapping> {
    Rc::new(LayoutJson::read_popup_mapping(&popup(c), "x".into(), "e".into(), None).unwrap())
}
fn layout_document(c: &str) -> String {
    format!(r#"{{"id":"x","arrangement":[["{c}"]]}}"#)
}
fn dump(a: &LayoutArrangement) -> Value {
    parse(&LayoutJson::write_arrangement(a, false).unwrap())
}
fn dump_popup(p: &PopupMapping) -> Value {
    let set = p.get(KeyVariation::All, &"a".into()).unwrap().unwrap();
    dump(&LayoutArrangement::from_rows(vec![
        set.relevant().unwrap().as_concrete().unwrap().snapshot(),
    ]))
}
#[test]
fn all_default_registry_entries_match_unchanged_source() {
    let resources = KeyboardResources::create_default().unwrap();
    let mut layouts = Vec::new();
    for t in LayoutType::ALL {
        for name in resources.get_layout_names(t) {
            let layout = resources.get_layout(t, &name).unwrap().unwrap();
            layouts.push(json!({"type":t.to_id().unwrap(),"name":text(&name.to_text()),"same":Rc::ptr_eq(&layout,&resources.get_layout(t,&name).unwrap().unwrap()),"document":parse(&LayoutJson::write_layout_document(&layout,false).unwrap())}));
        }
    }
    let popups:Vec<_>=resources.popup_mapping_names().iter().map(|name| {
        let p=resources.get_popup_mapping(name).unwrap().unwrap();
        let mut counts:Vec<_>=p.mappings().as_ref().unwrap().iter().unwrap().map(|entry|{let entry=entry.unwrap().unwrap();[(entry.key.unwrap()).0 as usize,entry.value.unwrap().count().unwrap() as usize]}).collect();
        counts.sort();
        json!({"name":text(&name.to_text()),"same":Rc::ptr_eq(&p,&resources.get_popup_mapping(name).unwrap().unwrap()),"authors":p.authors().unwrap().as_concrete().unwrap().snapshot().iter().map(|s|s.as_ref().map(text)).collect::<Vec<_>>(),"counts":counts})
    }).collect();
    let result = json!({"kind":"default",
        "extensions":resources.extensions().iter().map(|e| text(&e.id)).collect::<Vec<_>>(),
        "subtypes":resources.subtype_presets().iter().map(|s| text(&s.id())).collect::<Vec<_>>(),
        "currencies":resources.currency_sets().iter().map(|s| json!({"Id":s.id().as_ref().map(text),"Label":s.label().as_ref().map(text),"count":s.slots().unwrap().count().unwrap()})).collect::<Vec<_>>(),
        "composers":resources.composers().iter().map(|c| json!({"id":text(&c.id()),"label":text(&c.label()),"toRead":c.to_read()})).collect::<Vec<_>>(),"layouts":layouts,"popups":popups});
    let expected = &corpus()[0];
    for (key, value) in result.as_object().unwrap() {
        assert_eq!(value, &expected[key], "{key}");
    }
}
#[test]
fn reentrant_loaders_match_distinct_source_cache_rules() {
    for expected in &corpus()[1..] {
        let is_popup = expected["popup"].as_bool().unwrap();
        let replace = expected["replace"].as_bool().unwrap();
        let registry = Rc::new(KeyboardResources::create_empty());
        let calls = Rc::new(Cell::new(0));
        let events = Rc::new(Cell::new(0));
        registry.changed().add(Rc::new({
            let e = events.clone();
            move |_| e.set(e.get() + 1)
        }));
        let inner_layout: Rc<RefCell<Option<Rc<KeyboardLayout>>>> = Rc::default();
        let inner_popup: Rc<RefCell<Option<Rc<PopupMapping>>>> = Rc::default();
        let name = ComponentName::from("e:x");
        registry
            .add_extension(
                manifest(MANIFEST),
                Rc::new({
                    let registry = Rc::downgrade(&registry);
                    let calls = calls.clone();
                    let il = inner_layout.clone();
                    let ip = inner_popup.clone();
                    let name = name.clone();
                    move |_| {
                        let registry = registry.upgrade().unwrap();
                        calls.set(calls.get() + 1);
                        let c = if calls.get() == 1 {
                            if is_popup {
                                *ip.borrow_mut() = Some(if replace {
                                    let p = popup_model("c");
                                    registry.add_popup_mapping(p.clone());
                                    p
                                } else {
                                    registry.get_popup_mapping(&name)?.unwrap()
                                });
                            } else {
                                *il.borrow_mut() = Some(if replace {
                                    registry
                                        .add_layout_json(&layout_document("c"), Some("e".into()))?
                                } else {
                                    registry.get_layout(LayoutType::Characters, &name)?.unwrap()
                                });
                            }
                            "a"
                        } else {
                            "b"
                        };
                        Ok(Some(
                            if is_popup { popup(c) } else { arrangement(c) }
                                .as_str()
                                .into(),
                        ))
                    }
                }),
            )
            .unwrap();
        let (first, second, same_inner, same_next) = if is_popup {
            let first = registry.get_popup_mapping(&name).unwrap().unwrap();
            let second = registry.get_popup_mapping(&name).unwrap().unwrap();
            (
                dump_popup(&first),
                dump_popup(&second),
                Rc::ptr_eq(&first, inner_popup.borrow().as_ref().unwrap()),
                Rc::ptr_eq(&first, &second),
            )
        } else {
            let first = registry
                .get_layout(LayoutType::Characters, &name)
                .unwrap()
                .unwrap();
            let second = registry
                .get_layout(LayoutType::Characters, &name)
                .unwrap()
                .unwrap();
            (
                dump(first.arrangement().as_ref().unwrap()),
                dump(second.arrangement().as_ref().unwrap()),
                Rc::ptr_eq(&first, inner_layout.borrow().as_ref().unwrap()),
                Rc::ptr_eq(&first, &second),
            )
        };
        assert_eq!(
            &json!({"kind":"reentrant","popup":is_popup,"replace":replace,"calls":calls.get(),"events":events.get(),"first":first,"second":second,"sameInner":same_inner,"sameNext":same_next}),
            expected
        );
    }
}
#[test]
fn providers_are_lazy_missing_is_empty_and_failures_are_retried() {
    let registry = KeyboardResources::create_empty();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let fail = Rc::new(Cell::new(true));
    registry
        .add_extension(
            manifest(MANIFEST),
            Rc::new({
                let calls = calls.clone();
                let fail = fail.clone();
                move |p| {
                    calls.borrow_mut().push(text(p));
                    if fail.replace(false) {
                        return Err(std::io::Error::other("test provider failure").into());
                    }
                    Ok(None)
                }
            }),
        )
        .unwrap();
    assert!(calls.borrow().is_empty());
    assert!(
        registry
            .get_layout(LayoutType::Characters, &"e:x".into())
            .is_err()
    );
    let layout = registry
        .get_layout(LayoutType::Characters, &"e:x".into())
        .unwrap()
        .unwrap();
    assert!(Rc::ptr_eq(
        layout.arrangement().as_ref().unwrap(),
        &LayoutArrangement::empty()
    ));
    assert!(
        registry
            .get_popup_mapping(&"e:x".into())
            .unwrap()
            .unwrap()
            .mappings()
            .as_ref()
            .unwrap()
            .count()
            .unwrap()
            == 0
    );
    assert_eq!(
        &*calls.borrow(),
        &[
            "layouts/characters/x.json",
            "layouts/characters/x.json",
            "popupMappings/x.json"
        ]
    );
    assert!(
        registry
            .get_layout(LayoutType::Symbols, &"e:x".into())
            .unwrap()
            .is_none()
    );
    assert!(
        registry
            .get_popup_mapping(&"missing".into())
            .unwrap()
            .is_none()
    );
    assert!(registry.get_currency_set(&"missing".into()).is_none());
}
#[test]
fn extension_replacement_keeps_stale_keys_and_replaces_lazy_entries() {
    let registry = KeyboardResources::create_empty();
    registry
        .add_extension(manifest(MANIFEST), Rc::new(|_| Ok(None)))
        .unwrap();
    let old = registry
        .get_layout(LayoutType::Characters, &"e:x".into())
        .unwrap()
        .unwrap();
    let m = manifest(r#"{"meta":{"id":"e"},"layouts":{"characters":[{"id":"y"},{"id":"x"}]}}"#);
    registry
        .add_extension(m.clone(), Rc::new(|_| Ok(Some("[[\"new\"]]".into()))))
        .unwrap();
    let new = registry
        .get_layout(LayoutType::Characters, &"e:x".into())
        .unwrap()
        .unwrap();
    assert!(!Rc::ptr_eq(&old, &new));
    assert!(
        new.authors()
            .as_ref()
            .unwrap()
            .ptr_eq(m.layouts.borrow()[1].authors.as_ref().unwrap())
    );
    let ids: Vec<_> = registry
        .get_layout_names(LayoutType::Characters)
        .iter()
        .map(|n| text(n.component_id.as_ref().unwrap()))
        .collect();
    assert_eq!(ids, ["x", "y"]);
    registry
        .add_extension(
            manifest(r#"{"meta":{"id":"e"}}"#),
            Rc::new(|_| panic!("not a loader for stale entries")),
        )
        .unwrap();
    assert_eq!(registry.extensions().len(), 1);
    assert!(Rc::ptr_eq(
        &new,
        &registry
            .get_layout(LayoutType::Characters, &"e:x".into())
            .unwrap()
            .unwrap()
    ));
    assert!(registry.get_popup_mapping(&"e:x".into()).unwrap().is_some());
}
#[test]
fn changed_dispatch_is_synchronous_reentrant_and_removable() {
    let registry = KeyboardResources::create_empty();
    let order = Rc::new(RefCell::new(Vec::new()));
    let first = registry.changed().add(Rc::new({
        let order = order.clone();
        move |r| {
            order.borrow_mut().push(r.subtype_presets().len());
            if r.subtype_presets().len() == 1 {
                r.add_subtype_preset(Arc::new(Subtype::new("b".into())));
            }
        }
    }));
    registry.changed().add(Rc::new({
        let order = order.clone();
        move |r| order.borrow_mut().push(10 + r.subtype_presets().len())
    }));
    registry.add_subtype_preset(Arc::new(Subtype::new("a".into())));
    assert_eq!(&*order.borrow(), &[1, 2, 12, 12]);
    let snapshot = registry.subtype_presets();
    assert!(registry.changed().remove(first));
    registry.add_subtype_preset(Arc::new(Subtype::new("a".into())));
    assert_eq!(snapshot.len(), 2);
    assert_eq!(text(&snapshot[0].language_tag().unwrap()), "a");
    assert_eq!(
        text(&registry.subtype_presets()[0].language_tag().unwrap()),
        "b"
    );
    assert!(!Arc::ptr_eq(&snapshot[0], &registry.subtype_presets()[1]));
    assert_eq!(order.borrow().last(), Some(&12));
}
#[test]
fn original_telex_test_and_composer_registration_identity() {
    let registry = KeyboardResources::create_default().unwrap();
    let composer = registry.get_composer(&ComponentName::new(Subtype::CORE_COMPOSERS, "telex"));
    assert_eq!(
        composer.get_actions(&"a".into(), &"a".into()),
        (1, "â".into())
    );
    assert_eq!(
        composer.get_actions(&"A".into(), &"a".into()),
        (1, "Â".into())
    );
    let fallback: Arc<dyn IComposer> = AppenderComposer::instance();
    assert!(Arc::ptr_eq(
        &fallback,
        &registry.get_composer(&"missing".into())
    ));
    let unknown = ComposerDefinition::new("unknown".into(), "x".into());
    assert!(Arc::ptr_eq(
        &fallback,
        &KeyboardResources::create_composer(&unknown)
    ));
    let count = registry.composers().len();
    registry.add_composer(fallback.clone(), None);
    assert_eq!(registry.composers().len(), count);
    assert!(Arc::ptr_eq(
        &fallback,
        &registry.get_composer(&"custom:appender".into())
    ));
}

#[test]
fn subtype_lookup_matches_reference_and_retains_preset_identity() {
    let bundled = KeyboardResources::create_default().unwrap();
    let custom = KeyboardResources::create_empty();
    for tag in [
        "en-US", "EN-gb", "i", "ı", "İ", "s", "ſ", "Σ-x", "σ-y", "", "𐐀-ab",
    ] {
        custom.add_subtype_preset(Arc::new(Subtype::new(tag.into())));
    }
    custom.add_subtype_preset(Arc::new(Subtype::new(Utf16Text::from_units(vec![
        0xd800, 45, 97,
    ]))));
    for line in include_str!("../../../tests/fixtures/subtype-lookup.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let registry = if row["custom"].as_bool().unwrap() {
            &custom
        } else {
            &bundled
        };
        let tag = Utf16Text::from_units(
            row["word"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        );
        let found = registry.find_subtype_preset(&tag);
        let presets = registry.subtype_presets();
        let index = found.map_or(-1, |found| {
            presets.iter().position(|s| Arc::ptr_eq(s, &found)).unwrap() as i64
        });
        assert_eq!(index, row["index"].as_i64().unwrap(), "{row}");
    }
}
