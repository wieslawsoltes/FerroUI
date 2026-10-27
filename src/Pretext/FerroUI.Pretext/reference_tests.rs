//! Differential fixtures emitted by the unchanged pinned C# implementation.

use serde_json::{json, Value};

use crate::test_support::{engine, slice_prepared_text};
use crate::*;

fn cursor(value: LayoutCursor) -> Value {
    json!({ "segmentIndex": value.segment_index, "graphemeIndex": value.grapheme_index })
}

fn line(value: &LayoutLine) -> Value {
    json!({ "text": value.text, "width": value.width, "start": cursor(value.start), "end": cursor(value.end) })
}

fn actual_case(expected: &Value) -> Value {
    let input = &expected["input"];
    let text = input["text"].as_str().unwrap();
    let font = input["font"].as_str().unwrap();
    let width = input["width"].as_f64().unwrap();
    let line_height = input["lineHeight"].as_f64().unwrap();
    let options = PrepareOptions {
        white_space: if input["whiteSpace"] == "PreWrap" {
            WhiteSpaceMode::PreWrap
        } else {
            WhiteSpaceMode::Normal
        },
        word_break: if input["wordBreak"] == "KeepAll" {
            WordBreakMode::KeepAll
        } else {
            WordBreakMode::Normal
        },
    };
    let engine = engine();
    engine.set_locale(input["locale"].as_str());
    let prepared = engine.prepare_with_segments(text, font, options).unwrap();
    let metrics = PretextLayout::layout(&prepared, width, line_height);
    let stats = PretextLayout::measure_line_stats(&prepared, width);
    let lines = PretextLayout::layout_with_lines(&prepared, width, line_height);
    let mut streamed = Vec::new();
    let mut start = LayoutCursor::default();
    let mut reconstructed = String::new();
    while let Some(range) = PretextLayout::layout_next_line_range(&prepared, start, width) {
        assert!(range.end > start, "cursor must advance for {text:?}");
        let materialized = PretextLayout::materialize_line_range(&prepared, &range);
        assert_eq!(
            Some(materialized.clone()),
            PretextLayout::layout_next_line(&prepared, start, width)
        );
        reconstructed.push_str(&slice_prepared_text(&prepared, start, range.end));
        start = range.end;
        streamed.push(line(&materialized));
    }
    // Source range cursors include skipped separators, even when painting omits
    // them. This verifies that the streaming API neither duplicates nor loses text.
    assert_eq!(
        reconstructed,
        prepared.segments().concat(),
        "range reconstruction for {text:?}"
    );
    let metrics_only = engine.prepare(text, font, options).unwrap();
    assert_eq!(
        metrics,
        PretextLayout::layout(&metrics_only, width, line_height)
    );
    json!({
        "input": input,
        "segments": prepared.segments(),
        "widths": prepared.widths(),
        "kinds": prepared.kinds().iter().map(|kind| format!("{kind:?}")).collect::<Vec<_>>(),
        "lineEndFitAdvances": prepared.line_end_fit_advances(),
        "lineEndPaintAdvances": prepared.line_end_paint_advances(),
        "breakableWidths": prepared.breakable_widths(),
        "breakablePrefixWidths": prepared.breakable_prefix_widths(),
        "chunks": prepared.chunks().iter().map(|chunk| json!({
            "startSegmentIndex": chunk.start_segment_index,
            "endSegmentIndex": chunk.end_segment_index,
            "consumedEndSegmentIndex": chunk.consumed_end_segment_index,
        })).collect::<Vec<_>>(),
        "segmentLevels": prepared.segment_levels(),
        "discretionaryHyphenWidth": prepared.discretionary_hyphen_width(),
        "tabStopAdvance": prepared.tab_stop_advance(),
        "simpleLineWalkFastPath": prepared.simple_line_walk_fast_path(),
        "naturalWidth": PretextLayout::measure_natural_width(&prepared),
        "stats": { "lineCount": stats.line_count, "maxLineWidth": stats.max_line_width },
        "metrics": { "lineCount": metrics.line_count, "height": metrics.height },
        "lines": { "lineCount": lines.line_count, "height": lines.height, "lines": lines.lines.iter().map(line).collect::<Vec<_>>() },
        "streamed": streamed,
    })
}

fn compare(actual: &Value, expected: &Value, path: &str, differences: &mut Vec<String>) {
    match (actual, expected) {
        (Value::Number(actual), Value::Number(expected)) => {
            let actual = actual.as_f64().unwrap();
            let expected = expected.as_f64().unwrap();
            if (actual - expected).abs() > 1e-9 * expected.abs().max(1.0) {
                differences.push(format!("{path}: {actual} != {expected}"));
            }
        }
        (Value::Array(actual), Value::Array(expected)) if actual.len() == expected.len() => {
            for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                compare(actual, expected, &format!("{path}[{index}]"), differences);
            }
        }
        (Value::Object(actual), Value::Object(expected)) if actual.len() == expected.len() => {
            for (key, expected) in expected {
                compare(
                    actual.get(key).unwrap_or(&Value::Null),
                    expected,
                    &format!("{path}.{key}"),
                    differences,
                );
            }
        }
        _ if actual == expected => {}
        _ => differences.push(format!("{path}: {actual} != {expected}")),
    }
}

#[test]
fn unchanged_source_reference_corpus_matches_all_prepared_and_layout_outputs() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/reference.json")).unwrap();
    assert_eq!(
        fixture["sourceCommit"],
        "e70a544e43f7ecffee76d2d22a0224213d7fbba3"
    );
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 240);
    let mut differences = Vec::new();
    for (index, expected) in cases.iter().enumerate() {
        compare(
            &actual_case(expected),
            expected,
            &format!("case[{index}]"),
            &mut differences,
        );
    }
    assert!(
        differences.is_empty(),
        "{} mismatches:\n{}",
        differences.len(),
        differences
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
