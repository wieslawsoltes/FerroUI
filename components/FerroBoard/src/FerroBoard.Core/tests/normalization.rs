use ferroboard_core::text::{NormalizationError, TextUtils, Utf16Text};
use serde_json::Value;
use sha2::{Digest, Sha256};
fn text(v: &Value) -> Utf16Text {
    Utf16Text::from_units(
        v.as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
fn scalar(cp: u32) -> Utf16Text {
    let mut b = [0; 2];
    Utf16Text::from_units(char::from_u32(cp).unwrap().encode_utf16(&mut b).to_vec())
}
fn add(hash: &mut Sha256, value: Result<Utf16Text, NormalizationError>) -> bool {
    match value {
        Ok(v) => {
            hash.update((v.len() as i32).to_le_bytes());
            for c in v.units() {
                hash.update(c.to_le_bytes());
            }
            false
        }
        Err(_) => {
            hash.update((-1_i32).to_le_bytes());
            true
        }
    }
}
#[test]
fn unchanged_source_normalization_focused_cases() {
    for line in include_str!("../../../tests/fixtures/normalization.jsonl").lines() {
        let v: Value = serde_json::from_str(line).unwrap();
        let result = TextUtils::remove_diacritics(&text(&v["input"]));
        if v["error"].is_null() {
            assert_eq!(result.unwrap(), text(&v["result"]), "{line}");
        } else {
            assert_eq!(v["error"], "ArgumentException");
            assert!(result.is_err(), "{line}");
        }
    }
    assert_eq!(
        TextUtils::remove_diacritics(&"x𝆅".into()).unwrap(),
        "x𝆅".into()
    );
    assert_eq!(
        TextUtils::base_char(0xd800),
        Err(NormalizationError::InvalidUtf16)
    );
    assert_eq!(
        TextUtils::base_char(0xfffe),
        Err(NormalizationError::InvalidCodePoint)
    );
}
#[test]
fn unchanged_source_exhaustive_and_normalization_conformance_digests() {
    let records: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/normalization-digests.json"
    ))
    .unwrap();
    for expected in records.as_array().unwrap() {
        let mut hash = Sha256::new();
        let mut errors = Vec::new();
        let mut count = 0;
        match expected["kind"].as_str().unwrap() {
            "scalars" => {
                for cp in 0..=0x10ffff {
                    if char::from_u32(cp).is_none() {
                        continue;
                    }
                    if add(&mut hash, TextUtils::remove_diacritics(&scalar(cp))) {
                        errors.push(cp);
                    }
                    count += 1;
                }
            }
            "baseChar" => {
                for cp in 0..=65535 {
                    if add(
                        &mut hash,
                        TextUtils::base_char(cp as u16).map(|c| Utf16Text::from_units(vec![c])),
                    ) {
                        errors.push(cp);
                    }
                    count += 1;
                }
            }
            "conformance" => {
                for line in
                    include_str!("../../../reference/unicode-16.0.0/NormalizationTest.txt").lines()
                {
                    let s = line.split('#').next().unwrap().trim();
                    if s.is_empty() || s.starts_with('@') {
                        continue;
                    }
                    for field in s.split(';').take(5) {
                        let units = field
                            .split_whitespace()
                            .flat_map(|cp| {
                                scalar(u32::from_str_radix(cp, 16).unwrap())
                                    .units()
                                    .to_vec()
                            })
                            .collect::<Vec<_>>();
                        assert!(!add(
                            &mut hash,
                            TextUtils::remove_diacritics(&Utf16Text::from_units(units))
                        ));
                        count += 1;
                    }
                }
            }
            other => panic!("unknown corpus {other}"),
        }
        assert_eq!(count, expected["count"].as_u64().unwrap());
        if let Some(expected_errors) = expected["errors"].as_array() {
            assert_eq!(
                errors,
                expected_errors
                    .iter()
                    .map(|n| n.as_u64().unwrap() as u32)
                    .collect::<Vec<_>>()
            );
        }
        assert_eq!(
            format!("{:x}", hash.finalize()),
            expected["sha256"].as_str().unwrap(),
            "{expected}"
        );
    }
}
