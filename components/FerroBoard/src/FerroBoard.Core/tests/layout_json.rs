use ferroboard_core::{keys::*, layouts::*};
use serde_json::Value;
#[test]
fn unchanged_csharp_layout_json_corpus() {
    let mut bundled = 0;
    let mut popups = 0;
    for line in include_str!("../../../tests/fixtures/layout-json.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let input = row["input"].as_str().unwrap();
        if row["kind"] == "popup" {
            popups += 1;
            let map =
                LayoutJson::read_popup_mapping(input, "test".into(), "extension".into(), None)
                    .unwrap();
            let expected = row["output"].as_object().unwrap();
            assert_eq!(
                map.mappings().as_ref().unwrap().count().unwrap() as usize,
                expected.len()
            );
            for entry in map.mappings().as_ref().unwrap().iter().unwrap() {
                let entry = entry.unwrap().unwrap();
                let variation = entry.key.unwrap();
                let labels = entry.value.unwrap();
                let expected = &expected[&(variation).0.to_string()];
                assert_eq!(
                    labels.count().unwrap() as usize,
                    expected.as_object().unwrap().len()
                );
                for entry in labels.iter().unwrap() {
                    let entry = entry.unwrap().unwrap();
                    let label = entry.key.unwrap();
                    let popup = entry.value.unwrap();
                    let expected = &expected[label.to_utf8().unwrap()];
                    let wrap = |keys: Vec<Key>| LayoutArrangement::from_rows(vec![keys]);
                    if let Some(main) = &popup.main() {
                        let actual =
                            LayoutJson::write_arrangement(&wrap(vec![main.clone()]), false)
                                .unwrap();
                        assert_eq!(
                            LayoutJson::parse(&actual).unwrap(),
                            LayoutJson::parse(expected["main"].as_str().unwrap()).unwrap()
                        );
                    } else {
                        assert!(expected["main"].is_null());
                    }
                    let actual = LayoutJson::write_arrangement(
                        &wrap(popup.relevant().unwrap().as_concrete().unwrap().snapshot()),
                        false,
                    )
                    .unwrap();
                    assert_eq!(
                        LayoutJson::parse(&actual).unwrap(),
                        LayoutJson::parse(expected["relevant"].as_str().unwrap()).unwrap()
                    );
                }
            }
            continue;
        }

        if row["source"].is_string() {
            bundled += 1;
        }
        let actual = if row["kind"] == "layout" {
            LayoutJson::read_layout_document(input, "custom".into())
                .and_then(|v| LayoutJson::write_layout_document(&v, false))
        } else {
            LayoutJson::read_arrangement(input)
                .and_then(|v| LayoutJson::write_arrangement(&v, false))
        };
        if row.get("error").is_some() {
            assert!(actual.is_err(), "accepted {input}");
            continue;
        }
        let actual = actual.unwrap_or_else(|e| panic!("{}: {e:?}", row["source"]));
        // Ordered DOM comparison verifies property ordering as well as semantic values.
        assert_eq!(
            LayoutJson::parse(&actual).unwrap(),
            LayoutJson::parse(row["output"].as_str().unwrap()).unwrap(),
            "{}: {input}",
            row["source"]
        );
    }
    assert_eq!(bundled, 136);
    assert_eq!(popups, 57);
}
#[test]
fn source_selector_json_retains_dynamic_behavior_after_roundtrip() {
    let json = r#"[[{"$":"shift_state_selector","unshifted":"a","shifted":"b","capsLock":{"$":"multi_text_key","codePoints":[67,68]}}]]"#;
    let a = LayoutJson::read_arrangement(json).unwrap();
    let json = LayoutJson::write_arrangement(&a, true).unwrap();
    let b = LayoutJson::read_arrangement(&json).unwrap();
    for (shift, expected) in [
        (ShiftState::Unshifted, "a"),
        (ShiftState::ShiftedManual, "B"),
        (ShiftState::CapsLock, "CD"),
    ] {
        let key = b
            .rows
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap()
            .unwrap()
            .get(0)
            .unwrap()
            .unwrap();
        let eval = KeyEvaluator::from_values(KeyEvaluatorValues {
            shift_state: shift,
            ..Default::default()
        });
        assert_eq!(
            key.compute(&eval)
                .unwrap()
                .unwrap()
                .as_string(false)
                .unwrap()
                .unwrap()
                .to_utf8()
                .unwrap(),
            expected
        );
    }
}
#[test]
fn popup_alias_order_and_subtype_precedence_are_preserved() {
    let map = LayoutJson::read_popup_mapping(
        r#"{"email":{"x":["a"]},"emailAddress":{"y":["b"]},"email":{"z":["c"]}}"#,
        "en".into(),
        "ext".into(),
        None,
    )
    .unwrap();
    assert!(
        map.get(KeyVariation::EmailAddress, &"x".into())
            .unwrap()
            .is_none()
    );
    assert!(
        map.get(KeyVariation::EmailAddress, &"y".into())
            .unwrap()
            .is_none()
    );
    assert!(
        map.get(KeyVariation::EmailAddress, &"z".into())
            .unwrap()
            .is_some()
    );
    let subtype = LayoutJson::read_subtype(
        r#"{"preferred":{"characters":"first"},"layouts":{"characters":"second"}}"#,
    )
    .unwrap();
    assert_eq!(
        subtype.layouts().unwrap().characters().component_id,
        Some("first".into())
    );
    assert!(LayoutJson::read_subtype(r#"{"preferred":null,"layouts":{}}"#).is_err());
}
#[test]
fn comments_commas_depth_and_error_cases() {
    for invalid in ["[,]", "[[,]]", "{,}", "/*", "[[\"a\"]] garbage", "[[true]]"] {
        assert!(LayoutJson::read_arrangement(invalid).is_err(), "{invalid}");
    }
    let valid = "/*begin*/[/*row*/[\"a/*x*/,]\",/*end*/],]//tail";
    assert!(LayoutJson::read_arrangement(valid).is_ok());
    assert!(LayoutJson::parse(&format!("{}0{}", "[".repeat(65), "]".repeat(65))).is_err());
    assert!(LayoutJson::parse(&format!("{}0{}", "[".repeat(64), "]".repeat(64))).is_ok());
}
#[test]
fn manifests_parse_all_bundled_extensions() {
    let base=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reference/ScreenKeyboardControl/src/ScreenKeyboard.Core/Resources/florisboard/keyboard");
    let mut count = 0;
    let mut layouts = 0;
    let mut subtypes = 0;
    for entry in std::fs::read_dir(base).unwrap() {
        let path = entry.unwrap().path().join("extension.json");
        if !path.exists() {
            continue;
        }
        let manifest =
            LayoutJson::read_extension_manifest(&std::fs::read_to_string(&path).unwrap())
                .unwrap_or_else(|e| panic!("{path:?}: {e:?}"));
        count += 1;
        layouts += manifest.layouts.borrow().len();
        subtypes += manifest.subtype_presets.borrow().len();
    }
    assert!(count >= 4);
    assert!(layouts > 120);
    assert!(subtypes >= 60);
    let minimal=LayoutJson::read_extension_manifest(r#"{"meta":{"id":"x"},"punctuationRules":[{}],"currencySets":[{"slots":[]}],"layouts":{"characters":[{}]}}"#).unwrap();
    assert_eq!(
        minimal.punctuation_rules.borrow()[0]
            .symbols_preceding_auto_space()
            .unwrap(),
        "".into()
    );
    assert_eq!(
        minimal.punctuation_rules.borrow()[0]
            .symbols_terminating_sentence()
            .unwrap(),
        ".?!".into()
    );
    assert!(minimal.currency_sets.borrow()[0].id().is_none());
    assert!(minimal.layouts.borrow()[0].id.is_none());
}
