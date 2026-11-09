//! Full-dictionary regression for transient path storage, not ranking shortcuts.
#[path = "support/rust_allocation_counter.rs"]
mod allocations;

use allocations::counted;
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
fn empty_candidate_bucket_keeps_small_allocations_and_original_result_cache_identity() {
    let classifier = GlideTypingClassifier::default();
    classifier
        .set_layout([
            (b'a' as u16, KeyRect::new(0.0, 0.0, 40.0, 40.0)),
            (b'c' as u16, KeyRect::new(100.0, 0.0, 40.0, 40.0)),
            (b'b' as u16, KeyRect::new(1_000.0, 0.0, 40.0, 40.0)),
            (b'z' as u16, KeyRect::new(2_000.0, 0.0, 40.0, 40.0)),
        ])
        .unwrap();
    classifier
        .set_words([("bb".into(), 10), ("zz".into(), 20)])
        .unwrap();
    let points = [
        GesturePoint { x: 20.0, y: 20.0 },
        GesturePoint { x: 120.0, y: 20.0 },
    ];
    let previous = classifier.classify(points, 3).unwrap();
    let (result, allocation) = counted(|| classifier.classify(points, 4));
    let result = result.unwrap();
    assert!(result.is_empty());
    assert!(!std::sync::Arc::ptr_eq(&previous, &result));
    let (cached, cached_allocation) = counted(|| classifier.classify(points, 4));
    assert!(std::sync::Arc::ptr_eq(&result, &cached.unwrap()));
    assert_eq!(cached_allocation.calls, 0);
    println!("empty bucket: {allocation:?}");
    assert!(
        allocation.requested_bytes <= 32 * 1024,
        "unused candidate buffers: {allocation:?}"
    );
}

#[test]
fn warmed_full_dictionary_ranking_does_not_allocate_gesture_buffers_per_candidate() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/native-android-glide.json"
    ))
    .unwrap();
    let dictionary = WordDictionary::load_bundled_english().unwrap();
    assert_eq!(dictionary.count(), 49_981);
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
    let cases = fixture["cases"].as_array().unwrap();
    let traces = cases
        .iter()
        .map(|case| {
            case["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| GesturePoint {
                    x: number(&p[0]),
                    y: number(&p[1]),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for _ in 0..3 {
        for points in &traces {
            classifier.classify(points.iter().copied(), 3).unwrap();
        }
    }
    for (index, (case, points)) in cases.iter().zip(&traces).enumerate() {
        let other = classifier
            .classify(traces[(index + 1) % traces.len()].iter().copied(), 3)
            .unwrap();
        let (ranked, allocation) = counted(|| classifier.classify(points.iter().copied(), 3));
        let ranked = ranked.unwrap();
        assert!(
            !std::sync::Arc::ptr_eq(&other, &ranked),
            "force a ranking cache miss"
        );
        assert_eq!(
            json!(ranked.iter().map(|word| word.units()).collect::<Vec<_>>()),
            case["ranked"]
        );
        assert_eq!(
            classifier.point_count(),
            case["pointCount"].as_u64().unwrap() as usize
        );
        println!("{}: {allocation:?}", case["name"]);
        // The source retains/prunes the same words, loops and 200-point comparisons.
        // This fixture budget only guards repeated 500-point temporary buffers; it
        // is requested Rust allocation bytes, not a latency/live-heap/device budget.
        assert!(
            allocation.requested_bytes <= 256 * 1024,
            "per-candidate gesture storage remains for {}: {allocation:?}",
            case["name"]
        );
    }
}
