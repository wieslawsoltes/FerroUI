use ferroboard_core::{
    glide::{GesturePoint, GlideError, GlideTypingClassifier},
    layouts::KeyRect,
};
use serde_json::{Value, json};
use std::sync::Arc;

fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/native-glide-order.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn bounds(c: u16) -> KeyRect {
    match c {
        97 | 99 => KeyRect::new(-20.0, -20.0, 40.0, 40.0),
        98 => KeyRect::new(-40.0, -20.0, 80.0, 40.0),
        100 => KeyRect::new(180.0, -20.0, 40.0, 40.0),
        _ => unreachable!(),
    }
}

fn points() -> impl Iterator<Item = GesturePoint> {
    [0.0, 5.0, 15.0, 30.0, 200.0]
        .into_iter()
        .map(|x| GesturePoint { x, y: 0.0 })
}

fn classifier(order: &str) -> GlideTypingClassifier {
    let classifier = GlideTypingClassifier::default();
    classifier
        .set_layout(order.encode_utf16().map(|c| (c, bounds(c))))
        .unwrap();
    classifier
        .set_words([("ad".into(), 100), ("bd".into(), 100), ("cd".into(), 100)])
        .unwrap();
    classifier
}

#[test]
fn native_lookup_maps_preserve_source_ranked_ties_and_first_key_sampling() {
    for row in rows().iter().filter(|row| row["kind"] == "ordered-layout") {
        // Repeated independent instances guard against randomized lookup-map iteration.
        for _ in 0..16 {
            let classifier = classifier(row["order"].as_str().unwrap());
            let ranked = classifier.classify(points(), 3).unwrap();
            assert_eq!(
                json!(
                    ranked
                        .iter()
                        .map(|word| word.to_utf8().unwrap())
                        .collect::<Vec<_>>()
                ),
                row["ranked"]
            );
            assert_eq!(
                classifier.point_count(),
                row["count"].as_u64().unwrap() as usize
            );
        }
    }
}

#[test]
fn reordered_equal_layout_retains_original_sampling_and_cached_rankings() {
    for row in rows().iter().filter(|row| row["kind"] == "ordered-layout") {
        let order = row["order"].as_str().unwrap();
        let classifier = classifier(order);
        let ranked = classifier.classify(points(), 3).unwrap();
        classifier
            .set_layout(order.bytes().rev().map(|c| {
                let c = u16::from(c);
                (c, bounds(c))
            }))
            .unwrap();
        assert_eq!(
            Arc::ptr_eq(&ranked, &classifier.get_suggestions(3).unwrap()),
            row["sameLayoutCache"].as_bool().unwrap()
        );
        assert_eq!(
            Arc::ptr_eq(&ranked, &classifier.classify(points(), 3).unwrap()),
            row["sameReplayCache"].as_bool().unwrap()
        );
        assert_eq!(
            classifier.point_count(),
            row["replayCount"].as_u64().unwrap() as usize
        );
    }
}

#[test]
fn duplicate_lowercase_layout_failure_preserves_live_state_and_cached_result() {
    let row = rows()
        .into_iter()
        .find(|row| row["kind"] == "duplicate-layout")
        .unwrap();
    let classifier = classifier("abcd");
    let before = classifier.classify(points(), 3).unwrap();
    assert!(matches!(
        classifier.set_layout([(65, bounds(97)), (97, bounds(100))]),
        Err(GlideError::DuplicateLowercaseKey(97))
    ));
    let after = classifier.get_suggestions(3).unwrap();
    assert_eq!(
        Arc::ptr_eq(&before, &after),
        row["sameCache"].as_bool().unwrap()
    );
    assert_eq!(
        json!(
            after
                .iter()
                .map(|word| word.to_utf8().unwrap())
                .collect::<Vec<_>>()
        ),
        row["ranked"]
    );
    assert_eq!(
        classifier.point_count(),
        row["count"].as_u64().unwrap() as usize
    );
    assert!(classifier.is_ready());
}
