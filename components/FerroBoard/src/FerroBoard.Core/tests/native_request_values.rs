use ferroboard_core::text::*;
use serde_json::Value;
use std::rc::Rc;
fn make(id: usize, original: &KeyProximity, other: &KeyProximity) -> SuggestionRequest {
    let mut v = SuggestionRequestValues {
        word: Some("base".into()),
        preceding_text: Some("before".into()),
        max_count: 3,
        allow_auto_correct: true,
        culture: Some(TextCulture::get("en-US").unwrap()),
        key_proximity: Some(original.clone()),
    };
    match id {
        0 => (),
        13 => v.culture = Some(TextCulture::get("en-US").unwrap()),
        1 => v.word = None,
        2 => v.word = Some("".into()),
        3 => v.word = Some("Base".into()),
        4 => {
            v.word = Some(Utf16Text::from_units(vec![
                97, 0xd800, 0xd83d, 0xde00, 0xdfff,
            ]))
        }
        5 => v.preceding_text = None,
        6 => v.preceding_text = Some("".into()),
        7 => v.max_count = i32::MIN,
        8 => v.max_count = i32::MAX,
        9 => v.allow_auto_correct = false,
        10 => v.culture = None,
        11 => v.culture = Some(TextCulture::invariant()),
        12 => v.culture = Some(TextCulture::get("tr").unwrap()),
        14 => v.key_proximity = None,
        15 => v.key_proximity = Some(other.clone()),
        _ => unreachable!(),
    }
    SuggestionRequest::from_values(v)
}
fn callbacks() -> (KeyProximity, KeyProximity) {
    (
        Rc::new(|a, b| if a == b { 0.0 } else { 0.5 }),
        Rc::new(|a, b| if a == b { 0.0 } else { 0.5 }),
    )
}
#[test]
fn native_request_value_equality_and_independent_snapshots() {
    let (original, other) = callbacks();
    let mut equal = 0;
    let mut display = 0;
    for line in include_str!("../../../tests/fixtures/request-record.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        match row["tag"].as_str().unwrap() {
            "equal" => {
                let a = make(row["a"].as_u64().unwrap() as usize, &original, &other);
                let b = make(row["b"].as_u64().unwrap() as usize, &original, &other);
                let copy = SuggestionRequest::from_values(a.values());
                assert_eq!(a == b, row["typed"].as_bool().unwrap());
                assert_eq!(a == b, row["op"].as_bool().unwrap());
                assert_eq!(a != b, row["notEqual"].as_bool().unwrap());
                assert_eq!(a == copy, row["copyEqual"].as_bool().unwrap());
                assert_eq!(a.ptr_eq(&copy), row["copySame"].as_bool().unwrap());
                assert_eq!(a == a.clone(), row["sameSelf"].as_bool().unwrap());
                assert!(a.word().zip(copy.word()).is_none_or(|(a, b)| a.ptr_eq(&b)));
                assert!(
                    a.key_proximity()
                        .zip(copy.key_proximity())
                        .is_none_or(|(a, b)| Rc::ptr_eq(&a, &b))
                );
                equal += 1;
            }
            "display" | "custom-display" => {
                display += 1;
            }
            "ambient" => (),
            _ => unreachable!(),
        }
    }
    assert_eq!((equal, display), (256, 65));
}
