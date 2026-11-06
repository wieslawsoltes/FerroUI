use ferroboard_core::text::*;
use serde_json::{Value, json};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    rc::Rc,
};
fn state(s: &Suggestion) -> Value {
    json!({"text":s.text().as_ref().map(Utf16Text::units),"kind":s.kind().0,"confidence":format!("{:016x}",s.confidence().to_bits()),"auto":s.is_auto_commit(),"description":s.description().as_ref().map(Utf16Text::units),"removable":s.is_removable()})
}
fn hash(s: &Suggestion) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}
#[test]
fn candidate_shared_mutation_independent_clone_and_value_equality() {
    let original = Suggestion::new_nullable(
        Some("original".into()),
        SuggestionKind::Correction,
        0.75,
        true,
    );
    original.set_description(Some("description".into()));
    original.set_is_removable(true);
    let values = [
        Suggestion::new_nullable(None, SuggestionKind::Typed, 0.0, false),
        Suggestion::new("x".into(), SuggestionKind::Typed),
        Suggestion::new_nullable(Some("x".into()), SuggestionKind::Typed, -0.0, false),
        Suggestion::new_nullable(
            Some("x".into()),
            SuggestionKind::Typed,
            f64::from_bits(0xfff8000000000000),
            false,
        ),
        Suggestion::new_nullable(
            Some("x".into()),
            SuggestionKind::Typed,
            f64::from_bits(0x7ff0000000000001),
            false,
        ),
        {
            let s = Suggestion::new("x".into(), SuggestionKind::Typed);
            s.set_description(Some("".into()));
            s
        },
        {
            let s = Suggestion::new("x".into(), SuggestionKind::Typed);
            s.set_is_removable(true);
            s
        },
        Suggestion::new("x".into(), SuggestionKind::Correction),
        Suggestion::new("X".into(), SuggestionKind::Typed),
    ];
    let mut matched = 0;
    let mut missing = 0;
    for line in include_str!("../../../tests/fixtures/suggestion-record.jsonl").lines() {
        let mut row: Value = serde_json::from_str(line).unwrap();
        for key in [
            "equalsObject",
            "deconstruction",
            "boxed",
            "nullEquals",
            "otherEquals",
        ] {
            row.as_object_mut().unwrap().remove(key);
        }
        let actual = match row["tag"].as_str().unwrap() {
            "init" => {
                let value = Rc::new(original.clone());
                let alias = value.clone();
                let copy = Rc::new((*value).clone());
                let variant = row["variant"].as_u64().unwrap() as usize;
                match row["field"].as_str().unwrap() {
                    f @ ("Text" | "Description") => {
                        let v = match variant {
                            0 => None,
                            1 => Some("".into()),
                            2 => Some(Utf16Text::from_units(vec![
                                97, 0xd800, 0xd83d, 0xde00, 0xdfff,
                            ])),
                            3 => Some("changed".into()),
                            4 => Some("original".into()),
                            _ => Some("description".into()),
                        };
                        if f == "Text" {
                            value.set_text(v)
                        } else {
                            value.set_description(v)
                        }
                    }
                    "Kind" => {
                        value.set_kind(SuggestionKind([i32::MIN, -1, 0, 6, 99, i32::MAX][variant]))
                    }
                    "Confidence" => value.set_confidence(
                        [
                            f64::from_bits(0xfff8000000000000),
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                            -0.0,
                            0.0,
                            f64::from_bits(1),
                        ][variant],
                    ),
                    "IsAutoCommit" => value.set_is_auto_commit(variant.is_multiple_of(2)),
                    "IsRemovable" => value.set_is_removable(variant.is_multiple_of(2)),
                    field => panic!("unmeasured field {field}"),
                }
                json!({"tag":"init","field":row["field"],"variant":variant,"state":state(&value),"alias":state(&alias),"copy":state(&copy),"aliasSame":Rc::ptr_eq(&value,&alias),"copySame":Rc::ptr_eq(&value,&copy),"equalsCopy":value==copy,"copyEqualsOriginal":*copy==original,"aliasHashSame":hash(&value)==hash(&alias)})
            }
            "equality" => {
                let a = row["a"].as_u64().unwrap() as usize;
                let b = row["b"].as_u64().unwrap() as usize;
                let x = &values[a];
                let y = &values[b];
                let eq = x == y;
                json!({"tag":"equality","a":a,"b":b,"left":state(x),"right":state(y),"typed":eq,"op":eq,"notEqual":x!=y,"hashContract":!eq||hash(x)==hash(y)})
            }
            "constructor" | "property" | "method" | "display" => {
                missing += 1;
                continue;
            }
            tag => panic!("unmeasured tag {tag}"),
        };
        assert_eq!(actual, row, "candidate source row {matched}");
        matched += 1;
    }
    assert_eq!((matched, missing), (117, 205));
}
