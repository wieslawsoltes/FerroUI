//! Native field/copy/compute cases retained from unchanged C# record observations.
use ferroboard_core::{
    editor::EnterAction,
    keys::*,
    layouts::*,
    settings::{KeyHintMode, SpaceBarMode, UtilityKeyAction},
    text::{TextCulture, Utf16Text},
};
use serde_json::{Value, json};
use std::rc::Rc;
fn remove_object_fields(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for key in [
                "overrideSame",
                "computedSame",
                "cultureSame",
                "nullEqual",
                "wrongEqual",
            ] {
                map.remove(key);
            }
            for v in map.values_mut() {
                remove_object_fields(v);
            }
        }
        Value::Array(a) => {
            for v in a {
                remove_object_fields(v);
            }
        }
        _ => {}
    }
}
fn fields(index: usize, culture_override: Option<TextCulture>) -> KeyEvaluatorValues {
    let [mode, shift, variation, direction, half, kana] = [
        [0, 0, 1, 0, 0, 0],
        [6, 3, 4, 1, 1, 1],
        [-1, 1, -1, -1, 1, 0],
        [i32::MIN, 99, i32::MAX, 99, 0, 1],
    ][index];
    KeyEvaluatorValues {
        mode: KeyboardMode(mode),
        shift_state: ShiftState(shift),
        variation: KeyVariation(variation),
        direction: LayoutDirection(direction),
        is_char_half_width: half != 0,
        is_kana_kata: kana != 0,
        culture_override,
    }
}
fn state(e: &KeyEvaluator) -> Value {
    let v = e.values();
    json!({"mode":v.mode.0,"shift":v.shift_state.0,"variation":v.variation.0,"direction":v.direction.0,"half":v.is_char_half_width,"kana":v.is_kana_kata,"overrideNull":v.culture_override.is_none()})
}
#[test]
fn source_native_evaluator_values_copies_compute_and_equality() {
    let mut count = 0;
    for line in include_str!("../../../tests/fixtures/evaluator-record.jsonl").lines() {
        let mut row: Value = serde_json::from_str(line).unwrap();
        let locale = match row["kind"].as_str() {
            Some("null") => None,
            Some("invariant") => Some(TextCulture::invariant()),
            Some("turkish") => Some(TextCulture::get("tr-TR").unwrap()),
            _ => continue,
        };
        let actual = match row["tag"].as_str().unwrap() {
            "record" if row["op"] != "string" => {
                let a =
                    KeyEvaluator::from_values(fields(row["v"].as_u64().unwrap() as usize, locale));
                let value = match row["op"].as_str().unwrap() {
                    "fields" => state(&a),
                    "clone" => {
                        let copy = KeyEvaluator::from_values(a.values());
                        let v = copy.values();
                        assert_eq!(copy.culture_override(), locale);
                        json!({"same":a.ptr_eq(&copy),"equal":a==copy,"mode":v.mode.0,"shift":v.shift_state.0,"variation":v.variation.0,"direction":v.direction.0,"half":v.is_char_half_width,"kana":v.is_kana_kata})
                    }
                    "compute" => {
                        let key = Rc::new(AutoTextKeyData {
                            base: TextKeyData::from_fields(TextKeyDataFields {
                                code: 105,
                                label: Some("i".into()),
                                ..Default::default()
                            }),
                        });
                        json!(
                            key.compute(&a)
                                .unwrap()
                                .and_then(|k| k.as_string(false).unwrap())
                                .map(|t| t.units().to_vec())
                        )
                    }
                    _ => unreachable!(),
                };
                json!({"tag":"record","v":row["v"],"kind":row["kind"],"op":row["op"],"result":{"value":value,"error":null},"trace":[],"state":state(&a)})
            }
            "equals" => {
                let a = KeyEvaluator::from_values(fields(0, locale));
                let b = KeyEvaluator::from_values(a.values());
                match row["mismatch"].as_i64().unwrap() {
                    0 => b.set_mode(KeyboardMode(-1)),
                    1 => b.set_shift_state(ShiftState(-1)),
                    2 => b.set_variation(KeyVariation(-1)),
                    3 => b.set_direction(LayoutDirection(-1)),
                    4 => b.set_is_char_half_width(true),
                    5 => b.set_is_kana_kata(true),
                    6 => b.set_culture_override(Some(TextCulture::get("tr-TR").unwrap())),
                    _ => {}
                }
                json!({"tag":"equals","kind":row["kind"],"mismatch":row["mismatch"],"result":{"value":a==b,"error":null},"trace":[],"self":a==a.clone()})
            }
            _ => continue,
        };
        remove_object_fields(&mut row);
        assert_eq!(actual, row, "{line}");
        count += 1;
    }
    assert_eq!(count, 60);
}
#[test]
fn source_context_field_equality_uses_native_snapshots() {
    let mut count = 0;
    for line in include_str!("../../../tests/fixtures/compute-context-record.jsonl").lines() {
        let mut row: Value = serde_json::from_str(line).unwrap();
        if row["tag"] != "field-equality" {
            continue;
        }
        let a = Rc::new(KeyComputeContext::default());
        let b = Rc::new((*a).clone());
        match row["property"].as_str().unwrap() {
            "Mode" => b.set_mode(KeyboardMode(-17)),
            "ShiftState" => b.set_shift_state(ShiftState(-17)),
            "Variation" => b.set_variation(KeyVariation(-17)),
            "Direction" => b.set_direction(LayoutDirection(-17)),
            "EnterAction" => b.set_enter_action(EnterAction(-17)),
            "UtilityKeyAction" => b.set_utility_key_action(UtilityKeyAction(-17)),
            "SpaceBarMode" => b.set_space_bar_mode(SpaceBarMode(-17)),
            "HintMode" => b.set_hint_mode(KeyHintMode(-17)),
            "IsCharHalfWidth" => b.set_is_char_half_width(true),
            "IsKanaKata" => b.set_is_kana_kata(true),
            "HasMultipleSubtypes" => b.set_has_multiple_subtypes(true),
            "ShowSymbolHints" => b.set_show_symbol_hints(false),
            "ShowNumberHints" => b.set_show_number_hints(false),
            "Culture" => b.set_culture(Some(TextCulture::get("en-US").unwrap())),
            "CurrencySet" => b.set_currency_set(Some(Rc::new(CurrencySet::default()))),
            "SpaceBarLabel" => {
                b.set_space_bar_label(Some(Utf16Text::from_units(vec![120, 0xd800])))
            }
            _ => unreachable!(),
        }
        row.as_object_mut().unwrap().remove("nullEqual");
        row.as_object_mut().unwrap().remove("wrong");
        let actual = json!({"tag":"field-equality","property":row["property"],"equal":*a==*b,"reverse":*b==*a,"operators":a==b,"notEqual":a!=b,"clone":*a==(*a).clone(),"self":a==a.clone()});
        assert_eq!(actual, row);
        count += 1;
    }
    assert_eq!(count, 16);
}
