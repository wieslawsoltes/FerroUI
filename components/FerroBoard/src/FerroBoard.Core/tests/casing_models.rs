use ferroboard_core::{keys::*, layouts::*, text::*};
use serde_json::Value;
use std::rc::Rc;
use std::sync::Arc;
fn units(s: &str) -> Utf16Text {
    Utf16Text::from_units(if s.is_empty() {
        vec![]
    } else {
        s.split(',').map(|n| n.parse().unwrap()).collect::<Vec<_>>()
    })
}
fn encoded(text: &Utf16Text) -> String {
    text.units()
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
#[test]
fn unchanged_csharp_auto_key_corpus() {
    for line in include_str!("../../../tests/fixtures/auto-keys.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let source = Rc::new(AutoTextKeyData {
            base: TextKeyData::from_fields(TextKeyDataFields {
                code: row["code"].as_i64().unwrap() as i32,
                label: Some(units(row["text"].as_str().unwrap())),
                group_id: 3,
                ..Default::default()
            }),
        });
        let original: TextKey = source.clone();
        let evaluator = KeyEvaluator::from_values(KeyEvaluatorValues {
            shift_state: ShiftState::ShiftedManual,
            culture_override: Some(TextCulture::get(row["culture"].as_str().unwrap()).unwrap()),
            ..Default::default()
        });
        let computed = source.compute(&evaluator).unwrap().unwrap();
        assert_eq!(
            computed.data().code(),
            row["resultCode"].as_i64().unwrap() as i32,
            "{line}"
        );
        assert_eq!(
            encoded(computed.data().label().as_ref().unwrap()),
            row["label"].as_str().unwrap(),
            "{line}"
        );
        assert_eq!(
            Rc::ptr_eq(&computed, &original),
            row["same"].as_bool().unwrap(),
            "{line}"
        );
        assert_eq!(
            computed.data().code() == KeyCode::MultipleCodePoints,
            row["multi"].as_bool().unwrap(),
            "{line}"
        );
        assert_eq!(computed.data().group_id(), 3);
        let value = computed.as_string(false);
        if row["error"].is_null() {
            assert_eq!(
                encoded(&value.unwrap().unwrap()),
                row["value"].as_str().unwrap(),
                "{line}"
            );
        } else {
            assert!(value.is_err(), "{line}");
        }
    }
}
#[test]
fn casing_tables_and_all_culture_profiles_are_well_formed() {
    let data: Value = serde_json::from_str(include_str!("../text/data/casing.json")).unwrap();
    for (name, profile) in data["names"].as_object().unwrap() {
        let culture = TextCulture::get(name).unwrap();
        assert_eq!(culture.name(), name);
        let table = &data["tables"][profile.as_u64().unwrap() as usize];
        let mut previous = None;
        for pair in table.as_array().unwrap() {
            let cp = pair[0].as_u64().unwrap() as u32;
            assert!(previous.is_none_or(|p| p < cp));
            previous = Some(cp);
            let source = Utf16Text::from(char::from_u32(cp).unwrap().to_string().as_str());
            let expected = char::from_u32(pair[1].as_u64().unwrap() as u32)
                .unwrap()
                .to_string();
            assert_eq!(culture.to_upper(&source).to_utf8().unwrap(), expected);
        }
    }
    assert!(TextCulture::get("unmeasured-culture").is_err());
}
#[test]
fn auto_with_retains_override_and_unshifted_identity() {
    let key = Rc::new(AutoTextKeyData {
        base: TextKeyData::from_fields(TextKeyDataFields {
            code: 105,
            label: Some("i".into()),
            ..Default::default()
        }),
    });
    let original: TextKey = key.clone();
    let copy = key.with(KeyChanges::default());
    let eval = KeyEvaluator::default();
    assert!(Rc::ptr_eq(&original, &key.compute(&eval).unwrap().unwrap()));
    let eval = KeyEvaluator::from_values(KeyEvaluatorValues {
        shift_state: ShiftState::CapsLock,
        culture_override: Some(TextCulture::get("tr-TR").unwrap()),
        ..Default::default()
    });
    assert_eq!(
        copy.compute(&eval)
            .unwrap()
            .unwrap()
            .as_string(false)
            .unwrap()
            .unwrap()
            .to_utf8()
            .unwrap(),
        "İ"
    );
    assert_eq!(
        TextUtils::get_code_points(&units("55296,65,55357,56832,57343")),
        [0xd800, 65, 0x1f600, 0xdfff]
    );
    assert!(Arc::ptr_eq(
        &KeyEvaluator::shared_default(),
        &KeyEvaluator::shared_default()
    ));
}
#[test]
fn component_names_preserve_first_separator_and_utf16() {
    for (input, ext, id) in [
        ("abc", "fallback", "abc"),
        ("a:b:c", "a", "b:c"),
        (":", "", ""),
        ("", "fallback", ""),
    ] {
        let name = ComponentName::parse(&input.into(), &"fallback".into());
        assert_eq!(name.extension_id, Some(ext.into()));
        assert_eq!(name.component_id, Some(id.into()));
    }
    let name = ComponentName::parse(&units("55296,58,57343"), &"".into());
    assert_eq!(name.to_text(), units("55296,58,57343"));
}
#[test]
fn model_defaults_aliases_live_lists_and_currency_overflow() {
    for layout in LayoutType::ALL {
        assert_eq!(
            LayoutType::try_parse(&layout.to_id().unwrap().to_uppercase()),
            Some(layout)
        );
    }
    assert_eq!(LayoutType::try_parse(" numeric"), None);
    let subtype = Subtype::shared_default();
    assert_eq!(subtype.id().to_utf8().unwrap(), "en-US/qwerty");
    assert_eq!(subtype.popup_mapping().component_id, Some("en".into()));
    assert!(Arc::ptr_eq(&subtype, &Subtype::shared_default()));
    assert_eq!(Subtype::new("-tr".into()).short_label(), "-TR".into());
    assert_eq!(
        Subtype::new("ı-TR".into()).short_label(),
        TextCulture::invariant().to_upper(&"ı".into())
    );
    assert!(
        SubtypeLayoutMap::default()
            .get(LayoutType::Extension)
            .is_err()
    );
    let row = ResourceObjectList::<Key>::new(vec![]);
    let rows = ResourceObjectList::new(vec![ArrangementRow::from(row.clone())]);
    let arrangement = Rc::new(LayoutArrangement::new(rows.clone().into()));
    let key: TextKey = TextKeyData::space_key();
    row.push(TextKeyData::space_key());
    assert_eq!(
        arrangement
            .rows
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap()
            .unwrap()
            .count()
            .unwrap(),
        1
    );
    assert!(arrangement.rows.as_ref().unwrap().ptr_eq(&rows.into()));
    let slots = ResourceObjectList::new(vec![key.clone()]);
    let set = CurrencySet::new("test".into(), slots.clone());
    assert!(Rc::ptr_eq(&set.get_slot(-801).unwrap().unwrap(), &key));
    for code in [i32::MIN, i32::MAX, -802, -800, 0] {
        assert!(set.get_slot(code).unwrap().is_none());
    }
    slots.push(key);
    assert!(set.get_slot(-802).unwrap().is_some());
    let mapping = PopupMapping::default();
    let popup = Rc::new(PopupSet::default());
    let labels = ResourceDictionary::default();
    labels.set("a".into(), Some(popup.clone()));
    mapping
        .mappings()
        .as_ref()
        .unwrap()
        .as_concrete()
        .unwrap()
        .set(KeyVariation::Normal, Some(labels.clone().into()));
    assert!(Rc::ptr_eq(
        &mapping
            .get(KeyVariation::Normal, &"a".into())
            .unwrap()
            .unwrap(),
        &popup
    ));
    assert!(
        mapping
            .get(KeyVariation::All, &"a".into())
            .unwrap()
            .is_none()
    );
    labels.clear();
    assert!(
        mapping
            .get(KeyVariation::Normal, &"a".into())
            .unwrap()
            .is_none()
    );
}
#[test]
fn unchanged_csharp_model_corpus() {
    for line in include_str!("../../../tests/fixtures/models.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        match row["kind"].as_str().unwrap() {
            "name" => {
                let name = ComponentName::parse(
                    &units(row["input"].as_str().unwrap()),
                    &"fallback".into(),
                );
                assert_eq!(
                    encoded(name.extension_id.as_ref().unwrap()),
                    row["extension"].as_str().unwrap()
                );
                assert_eq!(
                    encoded(name.component_id.as_ref().unwrap()),
                    row["component"].as_str().unwrap()
                );
                assert_eq!(encoded(&name.to_text()), row["value"].as_str().unwrap());
            }
            "subtype" => {
                let sub = Subtype::new(units(row["input"].as_str().unwrap()));
                assert_eq!(encoded(&sub.id()), row["id"].as_str().unwrap());
                assert_eq!(
                    encoded(&sub.short_label()),
                    row["shortLabel"].as_str().unwrap()
                );
            }
            "layout" => {
                let layout = LayoutType::ALL[row["index"].as_u64().unwrap() as usize];
                assert_eq!(layout.to_id().unwrap(), row["id"].as_str().unwrap());
                let value = SubtypeLayoutMap::default()
                    .get(layout)
                    .ok()
                    .map(|c| c.to_text().to_utf8().unwrap());
                assert_eq!(value.as_deref(), row["value"].as_str());
            }
            "parse" => {
                let value = LayoutType::try_parse(row["input"].as_str().unwrap());
                assert_eq!(value.is_some(), row["success"].as_bool().unwrap());
                assert_eq!(
                    i64::from(value.unwrap_or(LayoutType::Characters).0),
                    row["index"].as_i64().unwrap()
                );
            }
            kind => panic!("Unknown corpus kind {kind}"),
        }
    }
}

#[test]
fn actual_runtime_ascii_allocation_flags_match_for_every_bundled_casing_culture() {
    let text = Utf16Text::from("ABC 123-_");
    let mut count = 0;
    for line in include_str!("../../../tests/fixtures/ascii-casing.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let culture = TextCulture::get(row["culture"].as_str().unwrap()).unwrap();
        let actual = culture.to_upper(&text);
        assert_eq!(actual, text, "{line}");
        assert_eq!(
            actual.ptr_eq(&text),
            row["same"].as_bool().unwrap(),
            "{line}"
        );
        count += 1;
    }
    assert_eq!(count, 1086);
}
