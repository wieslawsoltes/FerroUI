use ferroboard_core::text::{TextCulture, TextUtils, Utf16Text};
use serde_json::Value;
fn text(v: &Value) -> Utf16Text {
    Utf16Text::from_units(
        v.as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
#[test]
fn unchanged_source_word_grapheme_and_capitalization_corpus() {
    for (n, line) in include_str!("../../../tests/fixtures/text.jsonl")
        .lines()
        .enumerate()
    {
        let v: Value = serde_json::from_str(line).unwrap();
        let input = text(&v["input"]);
        assert_eq!(
            TextUtils::first_grapheme_length(&input),
            v["first"].as_i64().unwrap() as i32,
            "first {n}"
        );
        assert_eq!(
            TextUtils::last_grapheme_length(&input),
            v["last"].as_i64().unwrap() as i32,
            "last {n}"
        );
        assert_eq!(
            TextUtils::get_current_word(&input),
            text(&v["word"]),
            "word {n}"
        );
        assert_eq!(
            TextUtils::capitalize(&input, TextCulture::invariant()),
            text(&v["capital"]),
            "capital {n}"
        );
        assert_eq!(
            TextUtils::get_code_points(&input),
            v["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_i64().unwrap() as i32)
                .collect::<Vec<_>>()
        );
        for (j, index) in v["indexes"].as_array().unwrap().iter().enumerate() {
            let i = index.as_i64().unwrap() as i32;
            assert_eq!(
                TextUtils::find_previous_word_boundary(&input, i),
                v["previous"][j].as_i64().unwrap() as i32,
                "previous {n}:{j}"
            );
            assert_eq!(
                TextUtils::find_next_word_boundary(&input, i),
                v["next"][j].as_i64().unwrap() as i32,
                "next {n}:{j}"
            );
        }
    }
}
#[test]
fn unchanged_source_case_pattern_corpus() {
    for line in include_str!("../../../tests/fixtures/match-case.jsonl").lines() {
        let v: Value = serde_json::from_str(line).unwrap();
        assert_eq!(
            TextUtils::match_case(
                &text(&v["word"]),
                &text(&v["pattern"]),
                TextCulture::get(v["culture"].as_str().unwrap()).unwrap()
            ),
            text(&v["result"]),
            "{line}"
        );
    }
}
#[test]
fn original_source_word_tests_and_utf16_boundaries() {
    for (input, index, expected) in [
        ("hello world", 11, 6),
        ("hello world  ", 13, 6),
        ("a, b", 4, 3),
        ("end...", 6, 3),
        ("hello", 5, 0),
    ] {
        assert_eq!(
            TextUtils::find_previous_word_boundary(&input.into(), index),
            expected
        );
    }
    assert_eq!(
        TextUtils::get_current_word(&"I don't".into()),
        "don't".into()
    );
    assert_eq!(TextUtils::get_current_word(&"word ".into()), "".into());
    assert_eq!(TextUtils::get_current_word(&"(über".into()), "über".into());
    // Source retains trailing joiners despite its doc comment; supplementary letters are two non-word chars.
    assert_eq!(
        TextUtils::get_current_word(&"word---".into()),
        "word---".into()
    );
    assert_eq!(TextUtils::get_current_word(&"𐐨".into()), "".into());
    assert_eq!(TextUtils::first_grapheme_length(&"👩🏽‍💻".into()), 7);
    assert_eq!(TextUtils::last_grapheme_length(&"🇵🇱".into()), 4);
    assert_eq!(TextUtils::find_previous_word_boundary(&"x".into(), -2), -2);
    assert_eq!(TextUtils::find_next_word_boundary(&"x".into(), 9), 9);
}

#[test]
fn unicode16_grapheme_inputs_follow_the_source_runtime_including_its_deviations() {
    let mut differences = 0;
    for line in include_str!("../../../tests/fixtures/uax16.jsonl").lines() {
        let v: Value = serde_json::from_str(line).unwrap();
        let input = text(&v["input"]);
        let mut offset = 0;
        let mut actual = vec![0];
        while offset < input.len() {
            let remainder = Utf16Text::from_units(input.units()[offset..].to_vec());
            let length = TextUtils::first_grapheme_length(&remainder) as usize;
            assert!(length > 0);
            offset += length;
            actual.push(offset);
        }
        let expected: Vec<_> = v["source"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as usize)
            .collect();
        assert_eq!(actual, expected, "{line}");
        if v["source"] != v["unicode"] {
            differences += 1;
        }
    }
    let summary: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/uax-summary.json")).unwrap();
    assert_eq!(differences, summary["sourceDifferences"].as_u64().unwrap());
}
