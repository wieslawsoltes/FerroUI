use ferroboard_core::{glide::*, layouts::KeyRect, text::Utf16Text};
use serde_json::{Value, json};
use std::sync::Arc;
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/glide.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn number(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn text(v: &Value) -> Utf16Text {
    Utf16Text::from_units(
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
fn keys(row: &Value) -> Vec<(u16, KeyRect)> {
    row["keys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let r = &v["r"];
            (
                v["c"].as_u64().unwrap() as u16,
                KeyRect {
                    x: number(&r[0]),
                    y: number(&r[1]),
                    width: number(&r[2]),
                    height: number(&r[3]),
                },
            )
        })
        .collect()
}
fn setup(row: &Value) -> GlideTypingClassifier {
    let g = GlideTypingClassifier::default();
    g.set_layout(keys(row)).unwrap();
    g.set_words(
        row["words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (text(&p["word"]), p["frequency"].as_i64().unwrap() as i32)),
    )
    .unwrap();
    g
}
fn points(row: &Value) -> Vec<GesturePoint> {
    row["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| GesturePoint {
            x: number(&p[0]),
            y: number(&p[1]),
        })
        .collect()
}
fn result(v: &[Utf16Text]) -> Value {
    json!(v.iter().map(|s| s.units()).collect::<Vec<_>>())
}
#[test]
fn unchanged_source_glide_traces_match_rankings_counts_and_cache_identity() {
    let rows = rows();
    let mut traces = 0;
    for row in &rows {
        match row["kind"].as_str().unwrap() {
            "trace" | "special" => {
                let g = setup(&rows[0]);
                let n = row["max"].as_i64().unwrap_or(3) as i32;
                let r = g.classify(points(row), n).unwrap();
                assert_eq!(result(&r), row["result"], "{row}");
                assert_eq!(g.point_count(), row["count"].as_u64().unwrap() as usize);
                assert_eq!(
                    Arc::ptr_eq(&r, &g.get_suggestions(n).unwrap()),
                    row["same"].as_bool().unwrap(),
                    "cache {row}"
                );
                traces += 1;
            }
            _ => {}
        }
    }
    assert!(traces > 400);
}
#[test]
fn source_cache_clear_equal_layout_rebuild_and_duplicate_failure() {
    let rows = rows();
    let g = setup(&rows[0]);
    let trace = rows
        .iter()
        .find(|r| {
            r["kind"] == "trace" && r["word"] == "hello" && r["steps"] == 10 && r["noise"] == 0.0
        })
        .unwrap();
    let p = points(trace);
    let a = g.classify(p.clone(), 3).unwrap();
    g.clear();
    for p in p {
        g.add_point(p.x, p.y);
    }
    let b = g.get_suggestions(3).unwrap();
    g.set_layout(keys(&rows[0])).unwrap();
    let c = g.get_suggestions(3).unwrap();
    g.set_words(
        rows[0]["words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (text(&p["word"]), p["frequency"].as_i64().unwrap() as i32)),
    )
    .unwrap();
    let d = g.get_suggestions(3).unwrap();
    let row = rows.iter().find(|r| r["kind"] == "cache").unwrap();
    assert_eq!(
        json!({"kind":"cache","clearSame":Arc::ptr_eq(&a,&b),"layoutSame":Arc::ptr_eq(&b,&c),"wordsSame":Arc::ptr_eq(&c,&d)}),
        *row
    );
    let rect = KeyRect {
        x: 0.0,
        y: 0.0,
        width: 40.0,
        height: 40.0,
    };
    assert!(matches!(
        g.set_layout([(65, rect), (97, rect)]),
        Err(GlideError::DuplicateLowercaseKey(97))
    ));
    let row = rows.iter().find(|r| r["kind"] == "duplicate").unwrap();
    assert_eq!(g.is_ready(), row["ready"].as_bool().unwrap());
    assert_eq!(
        result(&g.classify(points(trace), 3).unwrap()),
        row["result"]
    );
}

#[test]
fn original_computed_keyboard_and_english_dictionary_glide_test() {
    use ferroboard_core::{keys::KeyboardMode, layouts::*, text::WordDictionary};
    use std::rc::Rc;
    let keyboard = LayoutComputer::new(Rc::new(KeyboardResources::create_default().unwrap()))
        .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
        .unwrap();
    keyboard
        .compute(Rc::new(KeyComputeContext::default()))
        .unwrap();
    keyboard.layout(1000.0, 240.0, None).unwrap();
    let classifier = GlideTypingClassifier::default();
    let keys = keyboard.character_keys().unwrap();
    classifier
        .set_computed_layout(keys.iter().map(|k| k.as_ref()))
        .unwrap();
    classifier
        .set_dictionary(&WordDictionary::load_bundled_english().unwrap())
        .unwrap();
    for row in rows().iter().filter(|r| r["kind"] == "computed") {
        let result = classifier.classify(points(row), 3).unwrap();
        assert_eq!(self::result(&result), row["result"], "{row}");
        assert_eq!(
            result.first().unwrap().to_utf8().unwrap(),
            row["word"].as_str().unwrap()
        );
    }
}
