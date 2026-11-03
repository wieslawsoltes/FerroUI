use ferroboard_core::{
    editor::EnterAction,
    keys::*,
    layouts::*,
    settings::*,
    text::{TextCulture, Utf16Text},
};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
fn rows() -> impl Iterator<Item = Value> {
    include_str!("../../../tests/fixtures/compute-context-state.jsonl")
        .lines()
        .map(|v| serde_json::from_str(v).unwrap())
}
fn units(v: Option<Utf16Text>) -> Value {
    json!(v.map(|v| v.units().to_vec()))
}
enum Reference {
    Culture(TextCulture),
    Currency(Rc<CurrencySet>),
}
fn id(refs: &mut Vec<Reference>, value: Option<Reference>) -> usize {
    let Some(value) = value else {
        return 0;
    };
    let found = refs.iter().position(|old| match (old, &value) {
        (Reference::Culture(a), Reference::Culture(b)) => a == b,
        (Reference::Currency(a), Reference::Currency(b)) => Rc::ptr_eq(a, b),
        _ => false,
    });
    if let Some(index) = found {
        index + 1
    } else {
        refs.push(value);
        refs.len()
    }
}
fn snapshot(c: &KeyComputeContext, refs: &mut Vec<Reference>) -> Value {
    let culture = id(refs, c.culture().map(Reference::Culture));
    let currency = id(refs, c.currency_set().map(Reference::Currency));
    json!([
        c.mode().0,
        c.shift_state().0,
        c.variation().0,
        c.direction().0,
        c.is_char_half_width(),
        c.is_kana_kata(),
        culture,
        c.enter_action().0,
        currency,
        c.has_multiple_subtypes(),
        c.utility_key_action().0,
        c.space_bar_mode().0,
        units(c.space_bar_label()),
        c.hint_mode().0,
        c.show_symbol_hints(),
        c.show_number_hints()
    ])
}
fn write(
    c: &KeyComputeContext,
    field: &str,
    index: usize,
    cultures: &[Option<TextCulture>],
    currencies: &[Option<Rc<CurrencySet>>],
) {
    let raw = [i32::MIN, -1, 0, 1, 99, i32::MAX][index.min(5)];
    match field {
        "Mode" => c.set_mode(KeyboardMode(raw)),
        "ShiftState" => c.set_shift_state(ShiftState(raw)),
        "Variation" => c.set_variation(KeyVariation(raw)),
        "Direction" => c.set_direction(LayoutDirection(raw)),
        "IsCharHalfWidth" => c.set_is_char_half_width(index == 1),
        "IsKanaKata" => c.set_is_kana_kata(index == 1),
        "Culture" => c.set_culture(cultures[index]),
        "EnterAction" => c.set_enter_action(EnterAction(raw)),
        "CurrencySet" => c.set_currency_set(currencies[index].clone()),
        "HasMultipleSubtypes" => c.set_has_multiple_subtypes(index == 1),
        "UtilityKeyAction" => c.set_utility_key_action(UtilityKeyAction(raw)),
        "SpaceBarMode" => c.set_space_bar_mode(SpaceBarMode(raw)),
        "SpaceBarLabel" => c.set_space_bar_label(
            [
                None,
                Some("".into()),
                Some("English".into()),
                Some(Utf16Text::from_units(vec![0xd800, 0xd83d, 0xde00, 0xdfff])),
                Some("אב\r\n".into()),
            ][index]
                .clone(),
        ),
        "HintMode" => c.set_hint_mode(KeyHintMode(raw)),
        "ShowSymbolHints" => c.set_show_symbol_hints(index == 1),
        "ShowNumberHints" => c.set_show_number_hints(index == 1),
        _ => panic!("unknown field {field}"),
    }
}
#[test]
fn all_sixteen_live_context_properties_aliases_interface_and_copies_match_source() {
    let slots =
        ResourceObjectList::new(vec![TextKeyData::char("¤".into(), None).unwrap() as TextKey]);
    let currencies = [
        None,
        Some(Rc::new(CurrencySet::new("money".into(), slots.clone()))),
        Some(Rc::new(CurrencySet::new("money".into(), slots))),
    ];
    // Distinct equal-data cultures measure original-reference retention in context copies.
    let cultures = [
        None,
        Some(TextCulture::invariant()),
        Some(TextCulture::get("en-US").unwrap()),
        Some(TextCulture::get("en-US").unwrap()),
    ];
    let mut defaults = 0;
    let mut writes = 0;
    for row in rows() {
        match row["tag"].as_str().unwrap() {
            "property" => {
                assert_eq!(
                    snapshot(&KeyComputeContext::default(), &mut Vec::new()),
                    row["defaults"]
                );
                assert!(row["init"].as_bool().unwrap());
                defaults += 1;
            }
            "write" => {
                if row["property"] == "Culture" {
                    continue;
                }
                let c = Rc::new(KeyComputeContext::default());
                let mut refs = Vec::new();
                let before = snapshot(&c, &mut refs);
                let alias = c.clone();
                let face: &dyn IKeyEvaluator = &*c;
                let copy = (*c).clone();
                let field = row["property"].as_str().unwrap();
                let index = row["index"].as_u64().unwrap() as usize;
                write(&alias, field, index, &cultures, &currencies);
                let after = snapshot(&c, &mut refs);
                let copied = snapshot(&copy, &mut refs);
                let copy_after = (*c).clone();
                let copied_after = snapshot(&copy_after, &mut refs);
                let unshifted = c.unshifted();
                let unshifted_snapshot = snapshot(&unshifted, &mut refs);
                let same_culture = match (c.culture(), copy_after.culture()) {
                    (None, None) => true,
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                };
                let same_currency = match (c.currency_set(), copy_after.currency_set()) {
                    (None, None) => true,
                    (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
                    _ => false,
                };

                let actual = json!({"tag":"write","property":field,"index":index,"before":before,"after":after,"copied":copied,"copiedAfter":copied_after,"unshifted":unshifted_snapshot,"copyAfterDistinct":!std::ptr::eq(&*c,&copy_after),"unshiftedDistinct":!std::ptr::eq(&*c,&unshifted),"copyAfterCultureSame":same_culture,"copyAfterCurrencySame":same_currency,"aliasSame":Rc::ptr_eq(&alias,&c),"interfaceSame":std::ptr::addr_eq(face as *const dyn IKeyEvaluator,Rc::as_ptr(&c)),"mode":face.mode().0,"shift":face.shift_state().0,"variation":face.variation().0,"direction":face.direction().0,"half":face.is_char_half_width(),"kana":face.is_kana_kata(),"cultureSame":match(face.culture(),c.culture()){(None,None)=>true,(Some(a),Some(b))=>a==b,_=>false},"copyDistinct":!std::ptr::eq(&*c,&copy),"copyCultureSame":copy.culture()==Some(TextCulture::invariant())});
                assert_eq!(actual, row);
                writes += 1;
            }
            _ => {}
        }
    }
    assert_eq!(defaults, 16);
    assert_eq!(writes, 66);
}
#[test]
fn runtime_null_label_and_culture_preserve_source_key_display_and_casing() {
    let mut count = 0;
    for row in rows() {
        match row["tag"].as_str().unwrap() {
            "null-label" => {
                let c = KeyComputeContext::default();
                let label = row["label"].as_array().map(|v| {
                    Utf16Text::from_units(
                        v.iter()
                            .map(|v| v.as_u64().unwrap() as u16)
                            .collect::<Vec<_>>(),
                    )
                });
                c.set_space_bar_label(label.clone());
                let code = row["code"].as_i64().unwrap() as i32;
                let key = TextKeyData::function(code, KeyType::Function, None);
                let (display, icon) = KeyLabels::get_display(&*key, &c).unwrap();
                assert_eq!(
                    json!({"tag":"null-label","code":code,"label":units(label),"display":units(display),"icon":icon.0}),
                    row
                );
                count += 1;
            }
            "null-culture" => {
                let c = KeyComputeContext::default();
                if row["shifted"] == true {
                    c.set_shift_state(ShiftState::ShiftedManual);
                }
                if row["nullCulture"] == true {
                    c.set_culture(None);
                }
                let key = Rc::new(AutoTextKeyData {
                    base: TextKeyData::from_fields(TextKeyDataFields {
                        code: 97,
                        label: Some("a".into()),
                        ..Default::default()
                    }),
                });
                let result = key.compute(&c).unwrap().unwrap();
                assert_eq!(units(result.data().label()), row["result"]);
                assert!(row["error"].is_null());
                count += 1;
            }
            _ => {}
        }
    }
    assert_eq!(count, 16);
}
#[test]
fn source_compute_callbacks_see_original_context_and_changes_in_later_reads() {
    struct Callback {
        context: Rc<KeyComputeContext>,
        trace: Rc<RefCell<Vec<Value>>>,
    }
    impl AbstractKeyData for Callback {
        fn compute_nullable(
            self: Rc<Self>,
            e: Option<&dyn IKeyEvaluator>,
        ) -> Result<Option<TextKey>, KeyDataError> {
            let e = e.unwrap();
            let call = self.trace.borrow().len() + 1;
            self.trace.borrow_mut().push(json!({"call":call,"same":std::ptr::addr_eq(e as *const dyn IKeyEvaluator,Rc::as_ptr(&self.context)),"mode":e.mode().0,"shift":e.shift_state().0}));
            if call == 1 {
                self.context.set_mode(KeyboardMode::Phone);
                self.context.set_enter_action(EnterAction::Search);
            } else {
                self.context.set_enter_action(EnterAction::Send);
            }
            Ok(Some(TextKeyData::function(
                KeyCode::Enter,
                KeyType::Function,
                None,
            )))
        }
        fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
            Ok(Some("".into()))
        }
    }
    let context = Rc::new(KeyComputeContext::from_values(KeyComputeContextValues {
        shift_state: ShiftState::CapsLock,
        ..Default::default()
    }));
    let trace = Rc::new(RefCell::new(Vec::new()));
    let key = Rc::new(Callback {
        context: context.clone(),
        trace: trace.clone(),
    }) as Key;
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    resources.add_layout(Rc::new(KeyboardLayout::new(
        LayoutType::Characters,
        "context".into(),
        "test".into(),
        Rc::new(LayoutArrangement::from_rows(vec![vec![key]])),
    )));
    let computer = LayoutComputer::new(resources);
    computer.set_default_modifier("missing:missing".into());
    let subtype = Arc::new(Subtype::from_values(
        ferroboard_core::layouts::SubtypeValues {
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    characters: ComponentName::new("context", "test"),
                    ..Default::default()
                },
            ))),
            ..(*Subtype::shared_default()).values()
        },
    ));
    let keyboard = computer
        .compute(KeyboardMode::Characters, subtype, false)
        .unwrap();
    keyboard.compute(context.clone()).unwrap();
    let key = keyboard.rows().get(0).unwrap().get(0).unwrap();
    let actual = json!({"tag":"reentrant","trace":trace.borrow().clone(),"icon":key.icon().0,"label":units(key.label()),"contextSame":Rc::ptr_eq(&context,&keyboard.context().unwrap()),"after":snapshot(&context,&mut Vec::new())});
    assert_eq!(actual, rows().find(|v| v["tag"] == "reentrant").unwrap());
}
