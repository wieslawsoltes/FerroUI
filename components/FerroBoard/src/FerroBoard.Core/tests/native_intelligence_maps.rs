use ferroboard_core::{emoji::*, serialization::StrictJsonValue, text::*};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc, sync::Arc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/native-map-order.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
#[test]
fn equal_confidence_candidates_and_upgrades_match_source_ranking_and_score_bits() {
    for row in rows().into_iter().filter(|r| r["kind"] == "suggestions") {
        let dictionary = Arc::new(
            WordDictionary::from_words(
                [
                    ("cat".into(), 10),
                    ("cab".into(), 10),
                    ("can".into(), 10),
                    ("cap".into(), 10),
                ],
                "en".into(),
            )
            .unwrap(),
        );
        let user = UserDictionary::default();
        if row["profile"] != "base" {
            for word in ["cap", "cab", "caz", "cay"] {
                for _ in 0..4 {
                    user.learn(&word.into(), None);
                }
            }
        }
        if row["profile"] == "blocked" {
            user.block(&"cab".into());
        }
        let provider = DictionarySuggestionProvider::new(dictionary, Some(user));
        provider.set_max_correction_distance(0.0);
        let request = SuggestionRequest::from_values(SuggestionRequestValues {
            word: Some(row["query"].as_str().unwrap().into()),
            max_count: 20,
            allow_auto_correct: false,
            ..Default::default()
        });
        let actual: Vec<_> = provider.suggest(&request).unwrap().snapshot().unwrap().into_iter().map(|s| {
            let s = s.unwrap();
            json!({"text":s.text().unwrap().to_utf8().unwrap(),"kind":s.kind().0,"confidence":format!("{:016x}",s.confidence().to_bits()),"removable":s.is_removable(),"auto":s.is_auto_commit()})
        }).collect();
        assert_eq!(json!(actual), row["result"], "{row}");
    }
}
#[test]
fn duplicate_emoji_bases_and_alias_collisions_keep_source_ownership_across_hash_seeds() {
    let row = rows().into_iter().find(|r| r["kind"] == "catalog").unwrap();
    for _ in 0..32 {
        let catalog =
            EmojiCatalog::parse(&row["data"].as_str().unwrap().into(), "en".into()).unwrap();
        assert_eq!(
            json!(catalog.categories().iter().map(|c| c.0).collect::<Vec<_>>()),
            row["categories"]
        );
        let all: Vec<_> = catalog
            .all()
            .map(|i| json!({"value":i.value.to_utf8().unwrap(),"name":i.name.to_utf8().unwrap()}))
            .collect();
        assert_eq!(json!(all), row["all"]);
        for expected in row["lookups"].as_array().unwrap() {
            let info = catalog.get(&expected["value"].as_str().unwrap().into());
            assert_eq!(
                json!(info.map(|i| i.name.to_utf8().unwrap())),
                expected["name"]
            );
        }
        assert!(Arc::ptr_eq(
            &catalog.get(&"collision".into()).unwrap(),
            &catalog.get(&"b".into()).unwrap()
        ));
        assert!(Arc::ptr_eq(
            &catalog.get(&"overlap".into()).unwrap(),
            &catalog.get(&"a".into()).unwrap()
        ));
    }
}
#[test]
fn tone_updates_duplicate_imports_and_export_order_match_unchanged_source() {
    let row = rows().into_iter().find(|r| r["kind"] == "history").unwrap();
    let history = EmojiHistory::default();
    let changes = Rc::new(Cell::new(0));
    let count = changes.clone();
    history
        .changed()
        .add(Rc::new(move |_| count.set(count.get() + 1)));
    for step in row["steps"].as_array().unwrap() {
        let value = step["value"].as_str().unwrap();
        if step["kind"] == "import" {
            history.import(value).unwrap();
        } else {
            history.add(&value.into()).unwrap();
        }
        assert_eq!(history.export(), step["export"].as_str().unwrap());
        assert_eq!(changes.get(), step["events"].as_i64().unwrap());
    }
    assert_eq!(
        history.get_preferred_tone(&"👍".into()),
        Some(EmojiSkinTone(1))
    );
}
#[test]
fn tone_preference_growth_retains_first_encounter_order_after_reverse_updates() {
    let history = EmojiHistory::default();
    let pairs: Vec<_> = (0..512).rev().map(|i| format!("\"x{i:04}\":1")).collect();
    history
        .import(&format!("{{\"tones\":{{{}}}}}", pairs.join(",")))
        .unwrap();
    let updates: Vec<_> = (0..512)
        .step_by(2)
        .map(|i| format!("\"x{i:04}\":5"))
        .collect();
    history
        .import(&format!("{{\"tones\":{{{}}}}}", updates.join(",")))
        .unwrap();
    let exported = history.export();
    let root = StrictJsonValue::parse(&exported).unwrap();
    let keys: Vec<_> = root
        .get("tones")
        .unwrap()
        .unwrap()
        .object()
        .unwrap()
        .iter()
        .map(|(name, _)| name.text.to_utf8().unwrap())
        .collect();
    assert_eq!(
        keys,
        (0..512)
            .rev()
            .map(|i| format!("x{i:04}"))
            .collect::<Vec<_>>()
    );
    for i in 0..512 {
        assert_eq!(
            history.get_preferred_tone(&Utf16Text::from(format!("x{i:04}").as_str())),
            Some(EmojiSkinTone(if i % 2 == 0 { 5 } else { 1 }))
        );
    }
}
