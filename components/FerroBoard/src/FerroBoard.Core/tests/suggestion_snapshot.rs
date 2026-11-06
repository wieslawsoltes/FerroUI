use ferroboard_core::text::{Suggestion, SuggestionKind, SuggestionValues, Utf16Text};
use serde_json::{Value, json};
fn text(v: &Value) -> Option<Utf16Text> {
    v.as_array().map(|v| {
        Utf16Text::from_units(
            v.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn state(s: &Suggestion) -> Value {
    json!({"text":s.text().as_ref().map(Utf16Text::units),"kind":s.kind().0,"confidence":format!("{:016x}",s.confidence().to_bits()),"auto":s.is_auto_commit(),"description":s.description().as_ref().map(Utf16Text::units),"removable":s.is_removable()})
}
#[test]
fn native_snapshot_preserves_source_fields_and_shared_utf16() {
    let mut matched = 0;
    let mut source_signature = 0;
    for line in include_str!("../../../tests/fixtures/deconstruct.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        // The original by-ref signature is retired; retain the 320 field-value cases.
        if row["tag"] == "signature" {
            source_signature += 1;
            continue;
        }
        let before = &row["before"];
        let s = Suggestion::new_nullable(
            text(&before["text"]),
            SuggestionKind(before["kind"].as_i64().unwrap() as i32),
            f64::from_bits(
                u64::from_str_radix(before["confidence"].as_str().unwrap(), 16).unwrap(),
            ),
            before["auto"].as_bool().unwrap(),
        );
        s.set_description(text(&before["description"]));
        s.set_is_removable(before["removable"].as_bool().unwrap());
        let SuggestionValues {
            text: output,
            kind,
            confidence,
            is_auto_commit: auto,
            ..
        } = s.values();
        let shared = match (s.text(), output.as_ref()) {
            (Some(a), Some(b)) => a.ptr_eq(b),
            (None, None) => true,
            _ => false,
        };
        let actual = json!({"tag":"case","index":matched,"before":state(&s),"text":output.as_ref().map(Utf16Text::units),"kind":kind.0,"confidence":format!("{:016x}",confidence.to_bits()),"auto":auto,"textShared":shared,"after":state(&s)});
        assert_eq!(actual, row, "source out-parameter row {matched}");
        matched += 1;
    }
    assert_eq!((matched, source_signature), (320, 1));
}
