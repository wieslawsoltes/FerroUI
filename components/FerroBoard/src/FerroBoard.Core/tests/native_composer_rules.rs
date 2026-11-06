use ferroboard_core::{
    layouts::{KeyboardResources, LayoutJson},
    text::{ComposerRules, IComposer, RulesComposer, Utf16Text},
};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
fn text(value: &Value) -> Utf16Text {
    Utf16Text::from_units(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
#[test]
fn native_hash_map_insertion_order_and_hash_seeds_do_not_change_source_actions() {
    let rows: Vec<Value> = include_str!("../../../tests/fixtures/composer-ownership.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let entries = rows[0]["rules"].as_array().unwrap();
    let cases = rows[0]["cases"].as_array().unwrap();
    let expected = rows[1]["actions"].as_array().unwrap();
    for _ in 0..16 {
        for reverse in [false, true] {
            for offset in 0..entries.len() {
                // Deliberately construct an ordinary std HashMap, not a collection adapter.
                let mut native = HashMap::new();
                for index in 0..entries.len() {
                    let i = if reverse {
                        entries.len() - 1 - index
                    } else {
                        index
                    };
                    let entry = &entries[(i + offset) % entries.len()];
                    native.insert(text(&entry["key"]), text(&entry["value"]));
                }
                let rules: ComposerRules = Arc::new(parking_lot::RwLock::new(native));
                let composer = RulesComposer::new("native".into(), "Native".into(), rules.clone());
                assert!(Arc::ptr_eq(&rules, &composer.rules()));
                assert_eq!(
                    composer.to_read(),
                    rows[1]["toRead"].as_i64().unwrap() as i32
                );
                for (case, action) in cases.iter().zip(expected) {
                    assert_eq!(
                        composer.get_actions(&text(&case["before"]), &text(&case["insert"])),
                        (
                            action["delete"].as_i64().unwrap() as i32,
                            text(&action["insert"])
                        )
                    );
                }
            }
        }
    }
}
#[test]
fn native_entry_and_retain_updates_keep_original_composer_snapshot() {
    let rules = ComposerRules::default();
    for index in 0..1024 {
        rules.write().insert(
            Utf16Text::from(format!("r{index:04}!").as_str()),
            Utf16Text::from(format!("v{index}").as_str()),
        );
    }
    let original = RulesComposer::new("original".into(), "Original".into(), rules.clone());
    assert_eq!(original.to_read(), 5);
    rules
        .write()
        .entry("r0257!".into())
        .and_modify(|value| *value = "updated".into())
        .or_insert("missing".into());
    rules
        .write()
        .retain(|key, _| key != &Utf16Text::from("r0000!"));
    let rebuilt = RulesComposer::new("rebuilt".into(), "Rebuilt".into(), rules.clone());
    assert_eq!(
        original.get_actions(&"r0257".into(), &"!".into()),
        (5, "v257".into())
    );
    assert_eq!(
        rebuilt.get_actions(&"r0257".into(), &"!".into()),
        (5, "updated".into())
    );
    assert_eq!(
        original.get_actions(&"r0000".into(), &"!".into()),
        (5, "v0".into())
    );
    assert_eq!(
        rebuilt.get_actions(&"r0000".into(), &"!".into()),
        (0, "!".into())
    );
    assert!(Arc::ptr_eq(&rebuilt.rules(), &rules));
    rules.write().clear();
    assert_eq!(
        original.get_actions(&"r0257".into(), &"!".into()),
        (5, "v257".into())
    );
    assert_eq!(
        RulesComposer::new("empty".into(), "Empty".into(), rules).to_read(),
        0
    );
}
#[test]
fn native_json_duplicate_rules_and_case_sensitive_suffixes_retain_source_semantics() {
    let manifest = LayoutJson::read_extension_manifest(r#"{"meta":{"id":"custom"},"composers":[{"$":"with-rules","id":"native","rules":{"aa":"old","AA":"ignored","aaa":"long","aa":"last","":"empty"}}]}"#).unwrap();
    let definition = manifest.composers.borrow()[0].clone();
    assert_eq!(definition.rules.read().len(), 4);
    assert_eq!(
        definition.rules.read().get(&Utf16Text::from("aa")),
        Some(&Utf16Text::from("last"))
    );
    let composer = KeyboardResources::create_composer(&definition);
    assert_eq!(
        composer.get_actions(&"a".into(), &"a".into()),
        (1, "last".into())
    );
    assert_eq!(
        composer.get_actions(&"aa".into(), &"a".into()),
        (2, "long".into())
    );
    assert_eq!(
        composer.get_actions(&"A".into(), &"a".into()),
        (1, "LAST".into())
    );
    assert_eq!(
        composer.get_actions(&"".into(), &"!".into()),
        (0, "!".into())
    );
}
