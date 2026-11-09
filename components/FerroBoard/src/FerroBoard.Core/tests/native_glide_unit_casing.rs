//! Glide casing uses code units directly, without per-character string workspaces.
use ferroboard_core::{
    glide::{GesturePoint, GlideTypingClassifier},
    layouts::KeyRect,
    text::Utf16Text,
};
use serde_json::{Value, json};
use std::hint::black_box;
#[path = "support/rust_allocation_counter.rs"]
mod allocations;
use allocations::counted;

#[test]
fn layout_casing_does_not_allocate_one_string_per_utf16_unit() {
    for first in [0x8000, 0xd800] {
        let keys = (first..first + 1024)
            .map(|unit| (unit, KeyRect::new(0.0, 0.0, 40.0, 40.0)))
            .collect::<Vec<_>>();
        let classifier = GlideTypingClassifier::default();
        let (result, allocations) = counted(|| classifier.set_layout(keys.iter().copied()));
        result.unwrap();
        println!(
            "first={first:04x}, keys={}, allocations={allocations:?}",
            keys.len()
        );
        // Input/output Vecs and HashMap growth are permitted; 1,024 temporary strings are not.
        assert!(
            allocations.calls <= 64,
            "per-unit casing allocations: {allocations:?}"
        );
        assert!(!classifier.is_ready()); // A layout alone is not a dictionary.
    }
}
fn path_allocations(repeats: usize) -> allocations::Allocations {
    let word = Utf16Text::from(format!("{}B", "A".repeat(repeats)).as_str());
    let classifier = GlideTypingClassifier::default();
    classifier
        .set_layout([
            (b'A' as u16, KeyRect::new(0.0, 0.0, 40.0, 40.0)),
            (b'B' as u16, KeyRect::new(200.0, 0.0, 40.0, 40.0)),
        ])
        .unwrap();
    classifier.set_words([(word.clone(), 200)]).unwrap();
    let points = [
        GesturePoint { x: 20.0, y: 20.0 },
        GesturePoint { x: 220.0, y: 20.0 },
    ];
    let (result, allocations) = counted(|| classifier.classify(points, 3));
    let result = result.unwrap();
    assert_eq!(result.as_slice(), &[word]);
    black_box(result);
    allocations
}
#[test]
fn candidate_path_casing_allocations_do_not_grow_with_repeated_letters() {
    let short = path_allocations(2);
    let long = path_allocations(96);
    println!("short={short:?}, long={long:?}");
    assert_eq!(short, long, "per-letter temporary casing buffers remain");
}
#[test]
fn unicode_layouts_and_ranked_words_match_unchanged_classifier() {
    for line in include_str!("../../../tests/fixtures/glide-unit-casing.jsonl")
        .lines()
        .skip(1)
    {
        let row: Value = serde_json::from_str(line).unwrap();
        let classifier = GlideTypingClassifier::default();
        classifier
            .set_layout(row["keys"].as_array().unwrap().iter().map(|key| {
                (
                    key["unit"].as_u64().unwrap() as u16,
                    KeyRect::new(key["x"].as_f64().unwrap(), 0.0, 40.0, 40.0),
                )
            }))
            .unwrap();
        classifier
            .set_words(row["words"].as_array().unwrap().iter().map(|word| {
                (
                    Utf16Text::from_units(
                        word["units"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_u64().unwrap() as u16)
                            .collect::<Vec<_>>(),
                    ),
                    word["frequency"].as_i64().unwrap() as i32,
                )
            }))
            .unwrap();
        let points = row["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|point| GesturePoint {
                x: point[0].as_f64().unwrap(),
                y: point[1].as_f64().unwrap(),
            });
        let result = classifier.classify(points, 8).unwrap();
        assert_eq!(
            json!(result.iter().map(|word| word.units()).collect::<Vec<_>>()),
            row["ranked"],
            "{}",
            row["id"]
        );
        assert_eq!(
            classifier.point_count(),
            row["count"].as_u64().unwrap() as usize
        );
    }
}
