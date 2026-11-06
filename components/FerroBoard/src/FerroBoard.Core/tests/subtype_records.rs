//! Unchanged C# Subtype/LayoutMap observations; selected Core state, UTF16 and reentrant layout behavior.
use ferroboard_core::layouts::{Subtype, SubtypeLayoutMap, SubtypeLayoutMapValues, SubtypeValues};
use ferroboard_core::{
    layouts::{ComponentName, ResourceError},
    text::Utf16Text,
};
use serde_json::{Value, json};
use std::sync::Arc;
#[test]
fn original_nine_null_layout_maps_keep_layout_computer_mode_and_resource_error_order() {
    let _scope = TEST_SCOPE.lock().unwrap();
    use ferroboard_core::{
        keys::KeyboardMode,
        layouts::{KeyboardResources, LayoutComputer},
    };
    let source: Vec<Value> = include_str!("../../../tests/fixtures/layout-computer-api.jsonl")
        .lines()
        .map(|v| {
            let mut row = serde_json::from_str(v).unwrap();
            remove_record_printing(&mut row);
            row
        })
        .collect();
    let mut count = 0;
    for resource_case in ["empty", "bundled", "null"] {
        let resources = match resource_case {
            "empty" => Some(std::rc::Rc::new(KeyboardResources::create_empty())),
            "bundled" => Some(std::rc::Rc::new(
                KeyboardResources::create_default().unwrap(),
            )),
            _ => None,
        };
        let computer = LayoutComputer::new_nullable(resources);
        for e in source.iter().filter(|v| {
            v["tag"] == "null-subtype"
                && v["subtypeCase"] == "null-layouts"
                && v["resourceCase"] == resource_case
        }) {
            let subtype = Arc::new(Subtype::from_values(SubtypeValues {
                layouts: None,
                ..Subtype::shared_default().values()
            }));
            let actual = match computer.compute(
                KeyboardMode(e["mode"].as_i64().unwrap() as i32),
                subtype,
                false,
            ) {
                Err(ResourceError::NullReference) => {
                    json!({"error":"NullReferenceException","parameter":null})
                }
                Err(ResourceError::KeyboardMode(mode)) => {
                    json!({"error":"ArgumentOutOfRangeException","parameter":mode.parameter_name()})
                }
                Err(e) => panic!("unexpected {e}"),
                Ok(_) => panic!("source null-map case must fail"),
            };
            assert_eq!(actual, e["result"]);
            count += 1;
        }
    }
    assert_eq!(count, 9);
}
#[test]
fn twenty_source_reentrant_layout_provider_mutations_preserve_map_capture_and_later_reads() {
    let _scope = TEST_SCOPE.lock().unwrap();
    use ferroboard_core::{
        keys::KeyboardMode,
        layouts::{
            KeyComputeContext, KeyComputeContextValues, KeyboardResources, LayoutComputer,
            LayoutJson, PopupMapping,
        },
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    let mut count = 0;
    for e in rows()
        .iter()
        .filter(|v| v["tag"] == "layout-reentrant-init")
    {
        let mutation = e["mutation"].as_str().unwrap().to_owned();
        let mode = KeyboardMode(e["mode"].as_i64().unwrap() as i32);
        let number_row = e["numberRow"].as_bool().unwrap();
        let old_map = Arc::new(SubtypeLayoutMap::from_values(SubtypeLayoutMapValues {
            characters: ComponentName::new("probe", "main"),
            numeric: ComponentName::new("probe", "main"),
            symbols: ComponentName::new("probe", "oldSymbols"),
            numeric_row: ComponentName::new("probe", "oldNumbers"),
            ..Default::default()
        }));
        let new_map = Arc::new(SubtypeLayoutMap::from_values(SubtypeLayoutMapValues {
            symbols: ComponentName::new("probe", "newSymbols"),
            numeric_row: ComponentName::new("probe", "newNumbers"),
            ..old_map.values()
        }));
        let subtype = Arc::new(Subtype::from_values(SubtypeValues {
            language_tag: Some("en-US".into()),
            layouts: Some(old_map.clone()),
            ..Default::default()
        }));
        let resources = Rc::new(KeyboardResources::create_empty());
        let calls = Rc::new(RefCell::new(Vec::<String>::new()));
        let changed = Rc::new(Cell::new(false));
        let popup = Rc::new(PopupMapping::default());
        popup.set_extension_id(Some("probe".into()));
        popup.set_id(Some("newPopup".into()));
        resources.add_popup_mapping(popup.clone());
        let manifest=LayoutJson::read_extension_manifest(r#"{"meta":{"id":"probe"},"layouts":{"characters":[{"id":"main"}],"numeric":[{"id":"main"}],"symbols":[{"id":"oldSymbols"},{"id":"newSymbols"}],"numericRow":[{"id":"oldNumbers"},{"id":"newNumbers"}]}}"#).unwrap();
        let provider_subtype = subtype.clone();
        let provider_old = old_map.clone();
        let provider_new = new_map.clone();
        let provider_calls = calls.clone();
        let provider_changed = changed.clone();
        resources
            .add_extension(
                Rc::new(manifest),
                Rc::new(move |path| {
                    let path = path.to_utf8().unwrap();
                    provider_calls.borrow_mut().push(path.clone());
                    if path.ends_with("/main.json") && !provider_changed.replace(true) {
                        match mutation.as_str() {
                            "replace-layouts" => {
                                provider_subtype.set_layouts(Some(provider_new.clone()))
                            }
                            "mutate-map" => {
                                provider_old.set_symbols(ComponentName::new("probe", "newSymbols"));
                                provider_old
                                    .set_numeric_row(ComponentName::new("probe", "newNumbers"));
                            }
                            "null-layouts" => provider_subtype.set_layouts(None),
                            "null-language" => provider_subtype.set_language_tag(None),
                            "replace-popup" => provider_subtype
                                .set_popup_mapping(ComponentName::new("probe", "newPopup")),
                            _ => panic!("unknown mutation"),
                        }
                    }
                    let code = if path.contains("Numbers") {
                        if path.contains("newNumbers") { 50 } else { 49 }
                    } else if path.contains("Symbols") {
                        if path.contains("newSymbols") { 64 } else { 33 }
                    } else {
                        97
                    };
                    Ok(Some(Utf16Text::from(
                        format!("[[{{\"code\":{code}}}]]").as_str(),
                    )))
                }),
            )
            .unwrap();
        let result = match LayoutComputer::new(resources).compute(mode, subtype.clone(), number_row)
        {
            Ok(keyboard) => {
                keyboard
                    .compute(Rc::new(KeyComputeContext::from_values(
                        KeyComputeContextValues {
                            mode,
                            ..Default::default()
                        },
                    )))
                    .unwrap();
                json!({"value":{"rows":keyboard.rows().snapshot().unwrap().into_iter().map(|row|row.snapshot().unwrap().into_iter().map(|k|json!({"source":units(k.source().unwrap().as_string(true).unwrap()),"label":units(k.label()),"hint":units(k.hint_label())})).collect::<Vec<_>>()).collect::<Vec<_>>(),"subtypeSame":Arc::ptr_eq(&subtype,&keyboard.subtype()),"popupSame":keyboard.popup_mapping().is_some_and(|v|Rc::ptr_eq(&v,&popup))}})
            }
            Err(ResourceError::NullReference) => {
                json!({"error":"NullReferenceException","parameter":null})
            }
            Err(e) => panic!("unexpected {e}"),
        };
        assert_eq!(result, e["result"], "{e}");
        assert_eq!(json!(*calls.borrow()), e["calls"]);
        assert_eq!(state(&subtype), e["subtype"]);
        assert_eq!(map(Some(old_map)), e["oldMap"]);
        assert_eq!(map(Some(new_map)), e["newMap"]);
        count += 1;
    }
    assert_eq!(count, 20);
}
static TEST_SCOPE: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct RestoreDefault(Arc<Subtype>, SubtypeValues);
impl Drop for RestoreDefault {
    fn drop(&mut self) {
        self.0.set_language_tag(self.1.language_tag.clone());
        self.0.set_layouts(self.1.layouts.clone());
    }
}

fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/subtype-records.jsonl")
        .lines()
        .map(|v| {
            let mut row = serde_json::from_str(v).unwrap();
            remove_record_printing(&mut row);
            row
        })
        .collect()
}
// Generated record display is retired. Source fixtures themselves stay unchanged.
fn remove_record_printing(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.remove("printed");
            for value in fields.values_mut() {
                remove_record_printing(value);
            }
        }
        Value::Array(values) => values.iter_mut().for_each(remove_record_printing),
        _ => {}
    }
}
fn units(value: Option<Utf16Text>) -> Value {
    json!(value.map(|v| v.units().to_vec()))
}
fn name(value: ComponentName) -> Value {
    json!({"extension":units(value.extension_id),"component":units(value.component_id)})
}
fn map(value: Option<Arc<SubtypeLayoutMap>>) -> Value {
    match value {
        None => json!({"isNull":true}),
        Some(v) => {
            json!({"isNull":false,"characters":name(v.characters()),"symbols":name(v.symbols()),"symbols2":name(v.symbols2()),"numeric":name(v.numeric()),"numericAdvanced":name(v.numeric_advanced()),"numericRow":name(v.numeric_row()),"phone":name(v.phone()),"phone2":name(v.phone2())})
        }
    }
}
fn attempt(value: Result<Utf16Text, ResourceError>) -> Value {
    match value {
        Ok(v) => json!({"value":v.units()}),
        Err(ResourceError::NullReference) => {
            json!({"error":"NullReferenceException","parameter":null})
        }
        Err(e) => panic!("unexpected {e}"),
    }
}
fn state(v: &Subtype) -> Value {
    json!({"fields":{"language":units(v.language_tag()),"display":units(v.display_name()),"composer":name(v.composer()),"currency":name(v.currency_set()),"popup":name(v.popup_mapping()),"punctuation":name(v.punctuation_rule()),"layouts":map(v.layouts())},"id":attempt(v.try_id()),"shortLabel":attempt(v.try_short_label())})
}

#[test]
fn layout_map_defaults_get_bounds_and_native_setters_match_source() {
    let _scope = TEST_SCOPE.lock().unwrap();
    let source = rows();
    let original = Arc::new(SubtypeLayoutMap::default());
    let copy = Arc::new((*original).clone());
    let e = source.iter().find(|v| v["tag"] == "map-default").unwrap();
    assert_eq!(map(Some(original.clone())), e["fields"]);

    assert_eq!(!Arc::ptr_eq(&original, &copy), e["cloneFresh"]);
    assert_eq!(original == copy, e["cloneEqual"]);
    let mut count = 1;
    for e in source.iter().filter(|v| v["tag"] == "map-get") {
        let actual = match original.get(ferroboard_core::layouts::LayoutType(
            e["raw"].as_i64().unwrap() as i32,
        )) {
            Ok(v) => json!({"value":name(v)}),
            Err(v) => json!({"error":"ArgumentOutOfRangeException","parameter":v.parameter_name()}),
        };
        assert_eq!(actual, e["result"]);
        count += 1;
    }
    for e in source.iter().filter(|v| v["tag"] == "map-init") {
        let original = Arc::new(SubtypeLayoutMap::default());
        let alias = original.clone();
        let copy = Arc::new((*original).clone());
        let value = ComponentName::new_nullable(None, Some("custom".into()));
        match e["field"].as_str().unwrap() {
            "Characters" => original.set_characters(value),
            "Symbols" => original.set_symbols(value),
            "Symbols2" => original.set_symbols2(value),
            "Numeric" => original.set_numeric(value),
            "NumericAdvanced" => original.set_numeric_advanced(value),
            "NumericRow" => original.set_numeric_row(value),
            "Phone" => original.set_phone(value),
            "Phone2" => original.set_phone2(value),
            _ => panic!("unexpected map field"),
        }
        assert_eq!(Arc::ptr_eq(&original, &alias), e["aliasSame"]);
        assert_eq!(!Arc::ptr_eq(&original, &copy), e["cloneFresh"]);
        assert_eq!(map(Some(original.clone())), e["original"]);
        assert_eq!(map(Some(alias)), e["alias"]);
        assert_eq!(map(Some(copy.clone())), e["clone"]);
        assert_eq!(original == copy, e["equalsClone"]);

        count += 1;
    }
    assert_eq!(count, 25);
}
#[test]
fn utf16_null_getters_and_native_shallow_clone_match112_source_states() {
    let _scope = TEST_SCOPE.lock().unwrap();
    let languages: Vec<Option<Utf16Text>> = vec![
        None,
        Some("".into()),
        Some("en-US".into()),
        Some("de-DE".into()),
        Some("-en".into()),
        Some("en_US".into()),
        Some("i-TR".into()),
        Some("\u{131}-TR".into()),
        Some("\u{df}".into()),
        Some("\u{130}".into()),
        Some("a\u{301}-ZZ".into()),
        Some(Utf16Text::from_units(vec![0xd800])),
        Some(Utf16Text::from_units(vec![0xdc00])),
        Some("a\r\nb".into()),
    ];
    let mut count = 0;
    for expected in rows().iter().filter(|r| r["tag"] == "subtype-state") {
        let language = expected["language"].as_u64().unwrap() as usize;
        let layout = match expected["layoutCase"].as_str().unwrap() {
            "default" => Some(Arc::new(SubtypeLayoutMap::default())),
            "custom" => Some(Arc::new(SubtypeLayoutMap::from_values(
                SubtypeLayoutMapValues {
                    characters: ComponentName::new("probe", "custom"),
                    ..Default::default()
                },
            ))),
            "null-components" => Some(Arc::new(SubtypeLayoutMap::from_values(
                SubtypeLayoutMapValues {
                    characters: ComponentName::default(),
                    ..Default::default()
                },
            ))),
            "null" => None,
            _ => panic!("unexpected map input"),
        };
        let s = Arc::new(Subtype::from_values(SubtypeValues {
            language_tag: languages[language].clone(),
            display_name: (expected["displayCase"] == "label").then(|| "label".into()),
            layouts: layout,
            ..Default::default()
        }));
        let clone = Arc::new((*s).clone());
        let equal = Arc::new(Subtype::from_values(SubtypeValues {
            layouts: s.layouts().map(|m| Arc::new((*m).clone())),
            ..s.values()
        }));
        assert_eq!(
            state(&s),
            expected["state"],
            "language={language};map={}",
            expected["layoutCase"]
        );
        assert_eq!(Arc::ptr_eq(&s, &clone), expected["cloneSame"]);
        assert_eq!(s == clone, expected["cloneEqual"]);
        assert_eq!(s == equal, expected["equalFreshMap"]);
        let same_map = match (s.layouts(), clone.layouts()) {
            (Some(a), Some(b)) => Arc::ptr_eq(&a, &b),
            (None, None) => true,
            _ => false,
        };
        assert_eq!(same_map, expected["cloneMapSame"]);
        // Native PartialEq/Clone preserve the value and sharing checks; CLR record entry points are retired.
        count += 1;
    }
    assert_eq!(count, 112);
}
#[test]
fn source_default_and_setter_assignments_preserve_aliases_and_clone_fields() {
    let _scope = TEST_SCOPE.lock().unwrap();
    let rows = rows();
    let expected = rows
        .iter()
        .find(|v| v["tag"] == "runtime-default-construction")
        .unwrap();
    let v = Subtype::default();
    let other = Subtype::default();
    assert_eq!(state(&v), expected["state"]);
    assert!(!Arc::ptr_eq(
        &v.layouts().unwrap(),
        &other.layouts().unwrap()
    ));
    assert_eq!(*v.layouts().unwrap(), SubtypeLayoutMap::default());
    for expected in rows.iter().filter(|v| v["tag"] == "subtype-init") {
        let map = Arc::new(SubtypeLayoutMap::default());
        let original = Arc::new(Subtype::from_values(SubtypeValues {
            language_tag: Some("en-US".into()),
            layouts: Some(map.clone()),
            ..Default::default()
        }));
        let alias = original.clone();
        let copy = Arc::new((*original).clone());
        let changed = ComponentName::new_nullable(None, Some("changed".into()));
        match expected["field"].as_str().unwrap() {
            "LanguageTag" => original.set_language_tag(None),
            "DisplayName" => original.set_display_name(Some("new label".into())),
            "Composer" => original.set_composer(changed),
            "CurrencySet" => original.set_currency_set(changed),
            "PopupMapping" => original.set_popup_mapping(changed),
            "PunctuationRule" => original.set_punctuation_rule(changed),
            "Layouts" => original.set_layouts(None),
            _ => panic!("unexpected init field"),
        }
        assert!(Arc::ptr_eq(&original, &alias));
        assert!(!Arc::ptr_eq(&original, &copy));
        assert_eq!(state(&original), expected["state"]);
        assert_eq!(state(&alias), expected["aliasState"]);
        assert_eq!(state(&copy), expected["cloneState"]);
        assert_eq!(original == copy, expected["equalsClone"]);
        assert_eq!(
            original.layouts().is_some_and(|v| Arc::ptr_eq(&v, &map)),
            expected["originalMapSameAsInitial"]
        );
        assert!(Arc::ptr_eq(&copy.layouts().unwrap(), &map));
    }
}
#[test]
fn original_shared_map_replacement_worker_and_default_identity_are_observed_without_snapshot_substitution()
 {
    let _scope = TEST_SCOPE.lock().unwrap();
    let source = rows();
    let map = Arc::new(SubtypeLayoutMap::default());
    let original = Arc::new(Subtype::from_values(SubtypeValues {
        language_tag: Some("en-US".into()),
        layouts: Some(map.clone()),
        ..Default::default()
    }));
    let copy = Arc::new((*original).clone());
    map.set_characters(ComponentName::new("probe", "mutated"));
    let e = source
        .iter()
        .find(|v| v["tag"] == "shared-map-init")
        .unwrap();
    assert_eq!(state(&original), e["original"]);
    assert_eq!(state(&copy), e["clone"]);
    assert!(Arc::ptr_eq(
        &original.layouts().unwrap(),
        &copy.layouts().unwrap()
    ));
    original.set_layouts(Some(Arc::new(SubtypeLayoutMap::default())));
    let e = source
        .iter()
        .find(|v| v["tag"] == "replace-map-init")
        .unwrap();
    assert_eq!(state(&original), e["original"]);
    assert_eq!(state(&copy), e["clone"]);
    assert_eq!(original == copy, e["equal"]);
    let worker_original = original.clone();
    let worker_copy = copy.clone();
    let worker = std::thread::spawn(move || {
        worker_original.set_language_tag(Some("de-DE".into()));
        worker_copy
            .layouts()
            .unwrap()
            .set_characters(ComponentName::default());
        (state(&worker_original), state(&worker_copy))
    })
    .join()
    .unwrap();
    let e = source.iter().find(|v| v["tag"] == "worker-init").unwrap();
    assert_eq!(worker.0, e["worker"]["original"]);
    assert_eq!(worker.1, e["worker"]["clone"]);
    assert_eq!(state(&original), e["callerOriginal"]);
    assert_eq!(state(&copy), e["callerClone"]);
    let v = Subtype::shared_default();
    let saved = v.values();
    let _restore = RestoreDefault(v.clone(), saved.clone());
    let e = source
        .iter()
        .find(|v| v["tag"] == "shared-default")
        .unwrap();
    assert!(Arc::ptr_eq(&v, &Subtype::shared_default()));
    assert_eq!(state(&v), e["state"]);
    v.set_language_tag(None);
    v.set_layouts(None);
    let worker_v = v.clone();
    let worker = std::thread::spawn(move || {
        let actual = Subtype::shared_default();
        (Arc::ptr_eq(&actual, &worker_v), state(&actual))
    })
    .join()
    .unwrap();
    let e = source
        .iter()
        .find(|v| v["tag"] == "shared-default-init")
        .unwrap();
    assert!(worker.0);
    assert_eq!(state(&v), e["state"]);
    assert_eq!(worker.1, e["worker"]["state"]);
    v.set_language_tag(saved.language_tag);
    v.set_layouts(saved.layouts);
    let e = source
        .iter()
        .find(|v| v["tag"] == "shared-default-restored")
        .unwrap();
    assert_eq!(state(&v), e["state"]);
}
