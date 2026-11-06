//! Actual source raw key enums, serialization loss, selector/popup fallback and input routing.
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
use ferroboard_core::{editor::*, engine::*, keys::*, layouts::*, settings::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/key-enums.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
fn output(value: Result<Option<Utf16Text>, KeyDataError>) -> Value {
    match value {
        Ok(v) => json!({"value":v.as_ref().map(Utf16Text::units)}),
        Err(KeyDataError::InvalidCodePoint(_) | KeyDataError::InvalidSurrogate(_)) => {
            json!({"error":"ArgumentOutOfRangeException"})
        }
        Err(e) => panic!("unexpected key error: {e:?}"),
    }
}
fn text(v: Utf16Text) -> String {
    v.to_utf8().unwrap()
}
fn popup_strings(p: &PopupSet) -> Vec<String> {
    p.relevant()
        .unwrap()
        .as_concrete()
        .unwrap()
        .iter()
        .map(|v| text(v.as_string(false).unwrap().unwrap()))
        .collect()
}
fn keyboard(
    key: Key,
    primary: Option<Rc<PopupMapping>>,
    fallback: Option<Rc<PopupMapping>>,
) -> ComputedKeyboard {
    let r = Rc::new(KeyboardResources::create_empty());
    if let Some(p) = primary {
        r.add_popup_mapping(p);
    }
    if let Some(p) = fallback {
        r.add_popup_mapping(p);
    }
    r.add_layout(Rc::new(KeyboardLayout::new(
        LayoutType::Numeric,
        "e".into(),
        "key".into(),
        Rc::new(LayoutArrangement::from_rows(vec![vec![key]])),
    )));
    let subtype = Arc::new(Subtype::from_values(
        ferroboard_core::layouts::SubtypeValues {
            popup_mapping: ComponentName::new("e", "p"),
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    numeric: ComponentName::new("e", "key"),
                    ..Default::default()
                },
            ))),
            ..(*Subtype::shared_default()).values()
        },
    ));
    LayoutComputer::new(r)
        .compute(KeyboardMode::Numeric, subtype, false)
        .unwrap()
}
#[test]
fn source_key_type_raw_values_exact_utf16_names_and_serialization_roundtrip_loss() {
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "type" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                assert_eq!(
                    json!({"kind":"type","raw":raw,"json":KeyType(raw).to_json_string()}),
                    row
                );
            }
            "parse" => {
                let value: Option<Vec<u16>> = serde_json::from_value(row["units"].clone()).unwrap();
                let value = value.map(Utf16Text::from_units);
                assert_eq!(
                    KeyType::parse_key_type_utf16(value.as_ref()).0,
                    row["value"]
                );
                if let Some(Ok(utf8)) = value.as_ref().map(Utf16Text::to_utf8) {
                    assert_eq!(KeyType::parse_key_type(Some(&utf8)).0, row["value"]);
                }
            }
            "key" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let code = row["code"].as_i64().unwrap() as i32;
                let key = TextKeyData::function(code, KeyType(raw), Some("label".into()));
                let original: TextKey = key.clone();
                let copy = key.with(KeyChanges::default());
                let document = LayoutJson::write_arrangement(
                    &LayoutArrangement::from_rows(vec![vec![key.clone()]]),
                    false,
                )
                .unwrap();
                let parsed = LayoutJson::read_arrangement(&document)
                    .unwrap()
                    .rows
                    .as_ref()
                    .unwrap()
                    .get(0)
                    .unwrap()
                    .unwrap()
                    .get(0)
                    .unwrap()
                    .unwrap()
                    .compute(&KeyEvaluator::default())
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    json!({"kind":"key","raw":raw,"code":code,"type":key.key_type().0,"IsSpace":key.is_space(),"ProducesText":key.produces_text(),"same":Rc::ptr_eq(&original,&key.clone().compute(&KeyEvaluator::default()).unwrap().unwrap()),"copySame":Rc::ptr_eq(&original,&copy),"copyType":copy.data().key_type().0,"output":output(key.as_string(false)),"display":output(key.as_string(true)),"document":serde_json::from_str::<Value>(&document).unwrap(),"parsedType":parsed.data().key_type().0}),
                    row
                );
            }
            _ => {}
        }
    }
}
#[test]
fn source_variation_selectors_and_popup_exact_uri_all_fallback_keep_raw_values() {
    type Mapping = ResourceDictionary<KeyVariation, PopupLabelMappings>;
    fn put(m: &Mapping, v: KeyVariation, label: &str) {
        let labels = ResourceDictionary::default();
        labels.set(
            "a".into(),
            Some(Rc::new(PopupSet::from_parts(
                None,
                (ResourceObjectList::new(vec![
                    TextKeyData::char(label.into(), None).unwrap() as Key
                ]))
                .into(),
            ))),
        );
        m.set(v, Some(labels.into()));
    }
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "selector" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let mask = row["mask"].as_u64().unwrap();
                let leaves = (0..5)
                    .map(|i| {
                        Rc::new(TextKeyData::from_fields(TextKeyDataFields {
                            code: 97 + i,
                            ..Default::default()
                        }))
                    })
                    .collect::<Vec<_>>();
                let get = |i: usize| {
                    if mask & (1 << i) != 0 {
                        Some(leaves[i].clone() as Key)
                    } else {
                        None
                    }
                };
                let s = Rc::new(VariationSelector::from_parts(
                    get(0),
                    get(1),
                    get(2),
                    get(3),
                    get(4),
                ));
                let evaluator = KeyEvaluator::from_values(KeyEvaluatorValues {
                    variation: KeyVariation(raw),
                    ..Default::default()
                });
                let result = s.compute(&evaluator).unwrap();
                let identity = result
                    .as_ref()
                    .and_then(|value| {
                        leaves.iter().position(|v| {
                            Rc::as_ptr(v).cast::<()>() == Rc::as_ptr(value).cast::<()>()
                        })
                    })
                    .map_or(-1, |v| v as i32);
                assert_eq!(
                    json!({"kind":"selector","raw":raw,"mask":mask,"retained":evaluator.variation().0,"code":result.map(|v|v.data().code()),"identity":identity}),
                    row
                );
            }
            "popup" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let mask = row["mask"].as_u64().unwrap();
                let primary = Mapping::default();
                let fallback = Mapping::default();
                if mask & 1 != 0 {
                    put(&primary, KeyVariation(raw), "x");
                }
                if mask & 2 != 0 {
                    put(&fallback, KeyVariation(raw), "y");
                }
                if mask & 4 != 0 {
                    put(&primary, KeyVariation::All, "z");
                }
                if mask & 8 != 0 {
                    put(&fallback, KeyVariation::All, "w");
                }
                if mask & 16 != 0 {
                    put(&primary, KeyVariation::Uri, "v");
                }
                let p = Rc::new(PopupMapping::from_values(PopupMappingValues {
                    extension_id: Some("e".into()),
                    id: Some("p".into()),
                    mappings: Some(primary.into()),
                    ..Default::default()
                }));
                let d = Rc::new(PopupMapping::from_values(PopupMappingValues {
                    extension_id: Some(Subtype::CORE_LOCALIZATION.into()),
                    id: Some("default".into()),
                    mappings: Some(fallback.into()),
                    ..Default::default()
                }));
                let direct = p
                    .get(KeyVariation(raw), &"a".into())
                    .unwrap()
                    .map(|v| popup_strings(&v));
                let k = keyboard(
                    TextKeyData::char("a".into(), None).unwrap(),
                    Some(p),
                    Some(d),
                );
                k.compute(Rc::new(KeyComputeContext::from_values(
                    ferroboard_core::layouts::KeyComputeContextValues {
                        variation: KeyVariation(raw),
                        ..Default::default()
                    },
                )))
                .unwrap();
                let key = k.keys().map(|key| key.unwrap().unwrap()).next().unwrap();
                assert_eq!(
                    json!({"kind":"popup","raw":raw,"mask":mask,"direct":direct,"keys":key.popup_keys().iter().map(|v|text(v.unwrap().as_string(false).unwrap().unwrap())).collect::<Vec<_>>(),"index":key.popup_default_index()}),
                    row
                );
            }
            "mapping-json" => {
                let raw = row["raw"].as_i64().unwrap() as i32;
                let p=LayoutJson::read_popup_mapping(r#"{"99":123,"all":{"a":["z"]},"Email":{"a":["u"]},"email":{"a":["e"]},"emailAddress":{"a":["f"]}}"#,"p".into(),"e".into(),None).unwrap();
                assert_eq!(
                    json!({"kind":"mapping-json","raw":raw,"count":p.mappings().as_ref().unwrap().count().unwrap(),"values":p.get(KeyVariation(raw),&"a".into()).unwrap().map(|v|popup_strings(&v))}),
                    row
                );
            }
            _ => {}
        }
    }
}
#[test]
fn source_computed_key_flags_labels_character_filter_and_exact_flow_geometry() {
    for row in rows().into_iter().filter(|v| v["kind"] == "computed") {
        let raw = row["raw"].as_i64().unwrap() as i32;
        let mode = row["mode"].as_i64().unwrap() as i32;
        let code = row["code"].as_i64().unwrap() as i32;
        let k = keyboard(
            TextKeyData::function(code, KeyType(raw), Some("label".into())),
            None,
            None,
        );
        k.compute(Rc::new(KeyComputeContext::from_values(
            ferroboard_core::layouts::KeyComputeContextValues {
                mode: KeyboardMode(mode),
                ..Default::default()
            },
        )))
        .unwrap();
        k.layout(211.375, 91.25, None).unwrap();
        let key = k.keys().map(|key| key.unwrap().unwrap()).next().unwrap();
        let b = key.touch_bounds();
        assert_eq!(
            json!({"kind":"computed","raw":raw,"mode":mode,"code":code,"type":key.data().data().key_type().0,"IsVisible":key.is_visible(),"IsEnabled":key.is_enabled(),"charKeys":k.character_keys().unwrap().len(),"label":key.label().map(|v|v.units().to_vec()),"sub":key.sub_label().map(|v|v.units().to_vec()),"icon":(key.icon()).0,"width":bits(key.width_factor()),"grow":bits(key.grow()),"shrink":bits(key.shrink()),"bounds":[bits(b.x),bits(b.y),bits(b.width),bits(b.height)]}),
            row
        );
    }
}
#[test]
fn source_input_key_routing_uses_produces_text_and_function_codes_for_raw_types() {
    for row in rows().into_iter().filter(|v| v["kind"] == "input") {
        let raw = row["raw"].as_i64().unwrap() as i32;
        let code = row["code"].as_i64().unwrap() as i32;
        let s = Rc::new(KeyboardSettings::new());
        s.set_glide_typing(false);
        s.set_auto_capitalization(false);
        s.set_show_suggestions(false);
        s.set_next_word_prediction(false);
        s.set_emoji_suggestions(false);
        s.set_clipboard_suggestion(false);
        let e = KeyboardEngine::new_explicit(
            s,
            Rc::new(KeyboardResources::create_empty()),
            LanguageModels::new(),
        )
        .unwrap();
        e.set_clock(Rc::new(|| fixture_clock::time(639028224000000000)));
        let target = Rc::new(TextBufferTarget::new(
            "ab".into(),
            Some(Arc::new(InputAttributes::from_values(
                ferroboard_core::editor::InputAttributesValues {
                    capitalization: CapitalizationMode::None,
                    ..Default::default()
                },
            ))),
        ));
        e.attach(target.clone()).unwrap();
        let events = Rc::new(RefCell::new(Vec::new()));
        for (name, event) in [
            ("keyboard", e.keyboard_changed()),
            ("state", e.state_changed()),
            ("suggestions", e.suggestions_changed()),
        ] {
            let log = events.clone();
            event.add(Rc::new(move |_| log.borrow_mut().push(json!(name))));
        }
        let log = events.clone();
        e.text_committed().add(Rc::new(move |v| {
            log.borrow_mut()
                .push(json!({"kind":"text","units":v.units()}))
        }));
        let log = events.clone();
        e.feedback_requested().add(Rc::new(move |v| {
            log.borrow_mut()
                .push(json!({"kind":"feedback","value":v.0}))
        }));
        let result = match e
            .input_key(TextKeyData::function(code, KeyType(raw), Some("label".into())).as_ref())
        {
            Ok(()) => json!({"value":null}),
            Err(e) => panic!("unexpected input error: {e:?}"),
        };
        assert_eq!(
            json!({"kind":"input","raw":raw,"code":code,"result":result,"text":target.text().units(),"start":target.selection_start(),"length":target.selection_length(),"shift":e.shift_state().0,"mode":e.mode().0,"actions":target.performed_actions().borrow().iter().map(|v|(v).0).collect::<Vec<_>>(),"events":*events.borrow()}),
            row
        );
        e.dispose().unwrap();
    }
}
