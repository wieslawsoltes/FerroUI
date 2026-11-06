use ferroboard_core::text::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;
fn units(s: &str) -> Utf16Text {
    Utf16Text::from_units(if s.is_empty() {
        vec![]
    } else {
        s.split(',')
            .map(|s| s.parse::<u16>().unwrap())
            .collect::<Vec<_>>()
    })
}
fn rules() -> RulesComposer {
    RulesComposer::new(
        "rules".into(),
        "Rules".into(),
        Arc::new(parking_lot::RwLock::new(
            [
                ("aa".into(), "â".into()),
                ("aaa".into(), "a".into()),
                ("as".into(), "á".into()),
                ("dd".into(), "đ".into()),
                ("".into(), "empty".into()),
            ]
            .into_iter()
            .collect(),
        )),
    )
}
#[test]
fn unchanged_csharp_focused_composer_corpus() {
    let rules = rules();
    for line in include_str!("../../../tests/fixtures/composers.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let composer: &dyn IComposer = match row["id"].as_str().unwrap() {
            "appender" => &AppenderComposer,
            "hangul-unicode" => &HangulComposer,
            "kana-unicode" => &KanaComposer,
            "rules" => &rules,
            _ => panic!("unknown composer"),
        };
        let actual = composer.get_actions(
            &units(row["before"].as_str().unwrap()),
            &units(row["insert"].as_str().unwrap()),
        );
        assert_eq!(
            actual,
            (
                row["delete"].as_i64().unwrap() as i32,
                units(row["result"].as_str().unwrap())
            ),
            "{line}"
        );
    }
}
#[test]
fn exhaustive_hangul_syllables_and_kana_utf16_match_csharp_digest() {
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/composer-digests.json"
    ))
    .unwrap();
    for row in rows {
        let composer: &dyn IComposer = if row["id"] == "hangul-unicode" {
            &HangulComposer
        } else {
            &KanaComposer
        };
        let mut hash = Sha256::new();
        let mut count = 0u64;
        for cp in row["first"].as_u64().unwrap()..=row["end"].as_u64().unwrap() {
            for n in row["next"].as_array().unwrap() {
                let (delete, insert) = composer.get_actions(
                    &Utf16Text::from_units(vec![cp as u16]),
                    &Utf16Text::from_units(vec![n.as_u64().unwrap() as u16]),
                );
                hash.update(delete.to_le_bytes());
                hash.update((insert.len() as i32).to_le_bytes());
                for unit in insert.units() {
                    hash.update(unit.to_le_bytes());
                }
                count += 1;
            }
        }
        assert_eq!(count, row["count"].as_u64().unwrap());
        assert_eq!(
            format!("{:x}", hash.finalize()),
            row["sha"].as_str().unwrap()
        );
    }
}
#[test]
fn kana_composer_applies_dakuten() {
    assert_eq!(
        KanaComposer.get_actions(&"か".into(), &"゙".into()),
        (1, "が".into())
    );
}
#[test]
fn rules_preserve_snapshot_identity_and_longest_match() {
    let map: ComposerRules = Arc::new(parking_lot::RwLock::new(
        [("aa".into(), "â".into()), ("aaa".into(), "a".into())]
            .into_iter()
            .collect(),
    ));
    let composer = RulesComposer::new("id".into(), "Label".into(), map.clone());
    assert!(Arc::ptr_eq(&composer.rules(), &map));
    assert_eq!(composer.to_read(), 2);
    assert_eq!(
        composer.get_actions(&"aa".into(), &"a".into()),
        (2, "a".into())
    );
    assert_eq!(
        composer.get_actions(&"A".into(), &"a".into()),
        (1, "Â".into())
    );
    map.write().insert("aa".into(), "changed".into());
    assert_eq!(
        composer.get_actions(&"a".into(), &"a".into()),
        (1, "â".into())
    );
    assert!(std::sync::Arc::ptr_eq(
        &AppenderComposer::instance(),
        &AppenderComposer::instance()
    ));
}
#[test]
fn telex_rules_load_from_original_embedded_manifest() {
    use ferroboard_core::{layouts::LayoutJson, resources::EmbeddedResources};
    let json = EmbeddedResources::read_text("keyboard/org.florisboard.composers/extension.json")
        .unwrap()
        .unwrap()
        .to_utf8()
        .unwrap();
    let manifest = LayoutJson::read_extension_manifest(&json).unwrap();
    let definition = manifest
        .composers
        .borrow()
        .iter()
        .find(|c| c.id == Utf16Text::from("telex"))
        .unwrap()
        .clone();
    let composer = RulesComposer::new(
        definition.id.clone(),
        definition.label.clone(),
        definition.rules.clone(),
    );
    assert_eq!(
        composer.get_actions(&"a".into(), &"a".into()),
        (1, "â".into())
    );
    assert_eq!(
        composer.get_actions(&"A".into(), &"a".into()),
        (1, "Â".into())
    );
}
