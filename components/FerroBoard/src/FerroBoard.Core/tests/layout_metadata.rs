#![cfg(target_os = "macos")]
use ferroboard_core::{
    layouts::*,
    text::{
        Utf16Text,
        locale_text::{self, CollationError},
    },
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
thread_local! {static CULTURE:RefCell<String>=const {RefCell::new(String::new())};static CALLS:Cell<usize>=const {Cell::new(0)};}
fn culture() -> Result<String, CollationError> {
    CALLS.with(|n| n.set(n.get() + 1));
    Ok(CULTURE.with(|c| c.borrow().clone()))
}
fn other() -> Result<String, CollationError> {
    Ok("other".into())
}
fn units(value: &Value) -> Option<Utf16Text> {
    value.as_array().map(|a| {
        Utf16Text::from_units(
            a.iter()
                .map(|n| n.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn ids(items: &[Arc<LayoutMetadata>]) -> Value {
    json!(
        items
            .iter()
            .map(|m| m.id.as_ref().unwrap().to_utf8().unwrap())
            .collect::<Vec<_>>()
    )
}
fn labels(items: &[Arc<LayoutMetadata>]) -> Value {
    json!(
        items
            .iter()
            .map(|m| m.label.as_ref().map(|s| s.units().to_vec()))
            .collect::<Vec<_>>()
    )
}
#[test]
fn native_metadata_order_keeps_source_identity_snapshots_lazy_loading_and_replacement() {
    locale_text::install_locale_provider(culture).unwrap();
    locale_text::install_locale_provider(culture).unwrap();
    assert!(locale_text::install_locale_provider(other).is_err());
    let rows = include_str!("../../../tests/fixtures/layout-metadata.jsonl")
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect::<Vec<_>>();
    let input = &rows[0];
    // Native ICU order is independently frozen; unchanged C# rows still define identities,
    // notifications, lazy loading and snapshot behavior. Width tailoring is an explicit migration.
    let native_rows = include_str!("../../../tests/fixtures/native-locale-metadata.jsonl")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(input["kind"], "inputs");
    let original = LayoutType::ALL
        .into_iter()
        .flat_map(|kind| {
            input["labels"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(move |(i, label)| {
                    Arc::new(LayoutMetadata {
                        layout_type: kind,
                        id: Some(format!("{}:{i}", kind.0).as_str().into()),
                        label: units(label),
                        authors: Some(ResourceTexts::new(vec!["author".into()])),
                        direction: ferroboard_core::keys::LayoutDirection::Ltr,
                        modifier: None,
                    })
                })
        })
        .collect::<Vec<_>>();
    let manifest =
        Rc::new(LayoutJson::read_extension_manifest(r#"{"meta":{"id":"fixture"}}"#).unwrap());
    manifest
        .layouts
        .borrow_mut()
        .extend(original.iter().cloned());
    let registry = KeyboardResources::create_empty();
    let loaded = Rc::new(Cell::new(0));
    let load_calls = loaded.clone();
    let events = Rc::new(Cell::new(0));
    let observed = events.clone();
    registry
        .changed()
        .add(Rc::new(move |_| observed.set(observed.get() + 1)));
    registry
        .add_extension(
            manifest.clone(),
            Rc::new(move |_| {
                load_calls.set(load_calls.get() + 1);
                panic!("metadata loaded a layout")
            }),
        )
        .unwrap();
    let bundled = KeyboardResources::create_default().unwrap();
    let mut synthetic = 0;
    let mut bundled_count = 0;
    let mut source_only = 0;
    for row in &rows[1..] {
        match row["kind"].as_str().unwrap() {
            "synthetic" | "bundled" => {
                let kind = LayoutType::ALL[row["type"].as_u64().unwrap() as usize];
                let name = row["culture"].as_str().unwrap();
                CULTURE.with(|c| *c.borrow_mut() = name.into());
                let resource = if row["kind"] == "synthetic" {
                    &registry
                } else {
                    &bundled
                };
                let before = CALLS.with(Cell::get);
                let items = resource.get_layout_metadata(kind).unwrap();
                assert_eq!(
                    CALLS.with(Cell::get),
                    before + 1,
                    "ambient culture captured once"
                );
                let native = native_rows
                    .iter()
                    .find(|native| {
                        native["kind"] == row["kind"]
                            && native["culture"] == row["culture"]
                            && native["type"] == row["type"]
                    })
                    .unwrap();
                assert_eq!(
                    ids(&items),
                    native["ids"],
                    "{} {name}",
                    kind.to_id().unwrap()
                );
                assert_eq!(labels(&items), native["labels"]);
                let repeated = resource.get_layout_metadata(kind).unwrap();
                assert!(items.iter().zip(&repeated).all(|(a, b)| Arc::ptr_eq(a, b)));
                if row["kind"] == "synthetic" {
                    assert!(
                        items
                            .iter()
                            .all(|m| original.iter().any(|o| Arc::ptr_eq(m, o)))
                    );
                    assert!(
                        row["originalIdentity"].as_bool().unwrap()
                            && row["repeatedIdentity"].as_bool().unwrap()
                            && row["snapshotNew"].as_bool().unwrap()
                    );
                    assert_eq!(loaded.get(), row["calls"].as_u64().unwrap() as usize);
                    assert_eq!(events.get(), row["events"].as_u64().unwrap() as usize);
                    synthetic += 1;
                    let mut changed = items;
                    changed.clear();
                    assert!(
                        !repeated.is_empty(),
                        "snapshots have independent list storage"
                    );
                } else {
                    bundled_count += 1;
                }
            }
            "replacement" => {
                CULTURE.with(|c| c.borrow_mut().clear());
                let held = registry
                    .get_layout_metadata(LayoutType::Characters)
                    .unwrap();
                let first = held[0].clone();
                first.authors.as_ref().unwrap().push(Some("live".into()));
                let name = first.id.as_ref().unwrap().to_utf8().unwrap();
                registry
                    .add_layout_json(
                        &format!(r#"{{"id":"{name}","label":"new replacement","arrangement":[]}}"#),
                        Some("fixture".into()),
                    )
                    .unwrap();
                let next = registry
                    .get_layout_metadata(LayoutType::Characters)
                    .unwrap();
                assert!(original.iter().any(|m| Arc::ptr_eq(m, &held[0])));
                assert!(
                    first
                        .authors
                        .as_ref()
                        .unwrap()
                        .ptr_eq(held[0].authors.as_ref().unwrap())
                );
                assert_eq!(
                    held[0].authors.as_ref().unwrap().len(),
                    row["authorsCount"].as_u64().unwrap() as usize
                );
                assert!(!Arc::ptr_eq(
                    &first,
                    next.iter().find(|m| m.id == first.id).unwrap()
                ));
                assert_eq!(
                    json!(
                        registry
                            .get_layout_names(LayoutType::Characters)
                            .iter()
                            .map(|n| n.component_id.as_ref().unwrap().to_utf8().unwrap())
                            .collect::<Vec<_>>()
                    ),
                    row["names"]
                );
                assert_eq!(events.get(), 2);
                assert_eq!(loaded.get(), 0);
            }
            "unknown-source-only" => {
                assert_eq!(row["count"], 0);
                source_only += 1;
            }
            other => panic!("unknown source row {other}"),
        }
    }
    assert_eq!((synthetic, bundled_count, source_only), (96, 96, 1));
    assert_eq!(
        manifest.layouts.borrow().len(),
        12 * input["labels"].as_array().unwrap().len()
    );
    // The host resolver executes on the caller; this does NOT move/share the Rc registry.
    let worker = std::thread::spawn(|| {
        CULTURE.with(|c| *c.borrow_mut() = "sv-SE".into());
        assert_eq!(locale_text::current_locale_name().unwrap(), "sv-SE");
    });
    worker.join().unwrap();
    assert_eq!(locale_text::current_locale_name().unwrap(), "de-DE_phoneb");
    assert!(
        registry
            .get_layout_metadata_for_culture(LayoutType::Characters, "bad\0locale")
            .is_err()
    );
    assert_eq!(loaded.get(), 0);
    assert_eq!(events.get(), 2);
}
