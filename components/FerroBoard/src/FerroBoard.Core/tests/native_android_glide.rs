use ferroboard_core::{
    glide::{GesturePoint, GlideTypingClassifier},
    layouts::KeyRect,
    text::WordDictionary,
};
use serde_json::{Value, json};

fn number(value: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap())
}

#[test]
fn actual_android_histories_match_unchanged_source_rankings_and_sampling() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/native-android-glide.json"
    ))
    .unwrap();
    assert_eq!(
        fixture["sourceRevision"],
        "3694995fabbebc45a801a455880e3d31888367f1"
    );
    let dictionary = WordDictionary::load_bundled_english().unwrap();
    assert_eq!(
        dictionary.words().len(),
        fixture["dictionaryWords"].as_u64().unwrap() as usize
    );
    for case in fixture["cases"].as_array().unwrap() {
        let classifier = GlideTypingClassifier::default();
        classifier
            .set_layout(fixture["layout"].as_array().unwrap().iter().map(|key| {
                let r = &key["r"];
                (
                    key["c"].as_u64().unwrap() as u16,
                    KeyRect::new(number(&r[0]), number(&r[1]), number(&r[2]), number(&r[3])),
                )
            }))
            .unwrap();
        classifier.set_dictionary(&dictionary).unwrap();
        let ranked = classifier
            .classify(
                case["points"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| GesturePoint {
                        x: number(&p[0]),
                        y: number(&p[1]),
                    }),
                3,
            )
            .unwrap();
        assert_eq!(
            json!(ranked.iter().map(|word| word.units()).collect::<Vec<_>>()),
            case["ranked"],
            "{}",
            case["name"]
        );
        assert_eq!(
            classifier.point_count(),
            case["pointCount"].as_u64().unwrap() as usize,
            "{}",
            case["name"]
        );
    }
}
