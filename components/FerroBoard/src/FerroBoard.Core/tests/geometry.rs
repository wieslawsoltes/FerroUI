use ferroboard_core::{editor::EnterAction, keys::*, layouts::*, settings::*, text::Utf16Text};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::rc::Rc;
use std::sync::Arc;
fn units(t: Option<Utf16Text>) -> Value {
    t.map_or(Value::Null, |t| json!(t.units()))
}
fn bits(f: f64) -> String {
    format!("{:016x}", f.to_bits())
}
fn rect(r: KeyRect) -> Value {
    json!([bits(r.x), bits(r.y), bits(r.width), bits(r.height)])
}
fn point(k: &ComputedKeyboard, x: f64, y: f64) -> Value {
    k.hit_test(x, y)
        .unwrap()
        .map_or(Value::Null, |h| json!([h.row(), h.column()]))
}
fn snapshot(k: &ComputedKeyboard) -> Value {
    json!([
        k.mode().0,
        k.direction().0,
        k.row_count(),
        k.version(),
        bits(k.width()),
        bits(k.height()),
        k.keys()
            .map(|key| key.unwrap().unwrap())
            .map(|x| json!([
                x.row(),
                x.column(),
                x.code(),
                x.key_type().0,
                units(x.label()),
                (x.icon()).0,
                units(x.hint_label()),
                units(x.sub_label()),
                units(Some(x.output().unwrap().unwrap())),
                x.popup_default_index(),
                x.is_visible(),
                x.is_enabled(),
                bits(x.width_factor()),
                bits(x.grow()),
                bits(x.shrink()),
                rect(x.touch_bounds()),
                rect(x.visible_bounds()),
                x.popup_keys()
                    .snapshot()
                    .unwrap()
                    .iter()
                    .map(|p| json!([
                        p.data().code(),
                        p.data().key_type().0,
                        units(p.data().label().clone()),
                        units(Some(p.as_string(false).unwrap().unwrap()))
                    ]))
                    .collect::<Vec<_>>()
            ]))
            .collect::<Vec<_>>(),
        k.character_keys()
            .unwrap()
            .iter()
            .map(|x| json!([x.row(), x.column()]))
            .collect::<Vec<_>>(),
        k.keys()
            .map(|key| key.unwrap().unwrap())
            .map(|x| {
                let t = x.touch_bounds();
                json!([
                    point(k, t.left(), t.top()),
                    point(k, t.right(), t.bottom()),
                    point(k, t.center_x(), t.center_y()),
                    point(k, -100.0, t.center_y()),
                    point(k, k.width() + 100.0, t.center_y())
                ])
            })
            .collect::<Vec<_>>()
    ])
}
/// `value` with every number written as its bits that is a NaN written as the default NaN of ARM
/// (positive, quiet), which the reference digests were recorded with.
fn arm_nan(value: Value) -> Value {
    match value {
        Value::String(text) => match u64::from_str_radix(&text, 16) {
            Ok(bits) if text.len() == 16 && f64::from_bits(bits).is_nan() => {
                Value::String(format!("{:016x}", f64::NAN.to_bits()))
            }
            _ => Value::String(text),
        },
        Value::Array(items) => Value::Array(items.into_iter().map(arm_nan).collect()),
        Value::Object(map) => {
            Value::Object(map.into_iter().map(|(k, v)| (k, arm_nan(v))).collect())
        }
        other => other,
    }
}
fn verify(k: &ComputedKeyboard, expected: &Value) {
    let snapshot = snapshot(k);
    let digest =
        |value: &Value| format!("{:x}", Sha256::digest(serde_json::to_vec(value).unwrap()));
    let mut hash = digest(&snapshot);
    if hash != expected["digest"].as_str().unwrap() {
        // A NaN computed from operands that are not NaN is negative on x86-64 and positive on
        // ARM, where the reference was recorded: compare the computed NaNs as ARM's.
        hash = digest(&arm_nan(snapshot));
    }
    assert_eq!(hash, expected["digest"].as_str().unwrap(), "{expected}");
    assert_eq!(
        k.keys().collect::<Result<Vec<_>, _>>().unwrap().len(),
        expected["count"].as_u64().unwrap() as usize
    );
}
const MODES: [KeyboardMode; 7] = [
    KeyboardMode::Characters,
    KeyboardMode::Symbols,
    KeyboardMode::Symbols2,
    KeyboardMode::Numeric,
    KeyboardMode::NumericAdvanced,
    KeyboardMode::Phone,
    KeyboardMode::Phone2,
];
const SHIFTS: [ShiftState; 4] = [
    ShiftState::Unshifted,
    ShiftState::ShiftedManual,
    ShiftState::ShiftedAutomatic,
    ShiftState::CapsLock,
];
#[test]
fn unchanged_source_bundled_layouts_all_modes_and_stateful_geometry() {
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    let computer = LayoutComputer::new(resources.clone());
    let subtypes = resources.subtype_presets();
    let mut resizing = None;
    let mut null_keyboard = None;
    for line in include_str!("../../../tests/fixtures/geometry.jsonl").lines() {
        let expected: Value = serde_json::from_str(line).unwrap();
        match expected["kind"].as_str().unwrap() {
            "bundled" => {
                let subtype = subtypes[expected["subtype"].as_u64().unwrap() as usize].clone();
                let mode = MODES[expected["mode"].as_u64().unwrap() as usize];
                let keyboard = computer.compute(mode, subtype.clone(), true).unwrap();
                let ctx = Rc::new(KeyComputeContext::from_values(
                    ferroboard_core::layouts::KeyComputeContextValues {
                        mode,
                        space_bar_label: Some("English".into()),
                        currency_set: resources.get_currency_set(&subtype.currency_set()),
                        ..Default::default()
                    },
                ));
                keyboard.compute(ctx.clone()).unwrap();
                assert!(Rc::ptr_eq(keyboard.context().as_ref().unwrap(), &ctx));
                keyboard.layout(800.0, 250.0, None).unwrap();
                verify(&keyboard, &expected);
            }
            "resize" => {
                let mode = MODES[expected["mode"].as_u64().unwrap() as usize];
                if expected["step"] == 0 {
                    let keyboard = computer
                        .compute(mode, Subtype::shared_default(), false)
                        .unwrap();
                    keyboard
                        .compute(Rc::new(KeyComputeContext::from_values(
                            ferroboard_core::layouts::KeyComputeContextValues {
                                mode,
                                shift_state: SHIFTS[expected["shift"].as_u64().unwrap() as usize],
                                hint_mode: KeyHintMode(expected["hint"].as_i64().unwrap() as i32),
                                space_bar_label: Some("English".into()),
                                has_multiple_subtypes: true,
                                variation: KeyVariation::EmailAddress,
                                enter_action: EnterAction::Search,
                                ..Default::default()
                            },
                        )))
                        .unwrap();
                    resizing = Some(keyboard);
                }
                let keyboard = resizing.as_ref().unwrap();
                let c = &expected["c"];
                let f = |k: &str| c[k].as_f64().unwrap();
                keyboard
                    .layout(
                        f("width"),
                        f("height"),
                        Some(&KeyboardGeometryOptions::from_values(
                            KeyboardGeometryOptionsValues {
                                split_gap: f("gap"),
                                units_per_row: f("units"),
                                key_spacing_horizontal: f("sh"),
                                key_spacing_vertical: f("sv"),
                            },
                        )),
                    )
                    .unwrap();
                verify(keyboard, &expected);
            }
            "custom" => {
                let resources = Rc::new(KeyboardResources::create_empty());
                resources
                    .add_layout_json(expected["main"].as_str().unwrap(), Some("e".into()))
                    .unwrap();
                resources
                    .add_layout_json(expected["modifier"].as_str().unwrap(), Some("e".into()))
                    .unwrap();
                let computer = LayoutComputer::new(resources);
                computer.set_default_modifier("e:default".into());
                let subtype = Arc::new(Subtype::from_values(
                    ferroboard_core::layouts::SubtypeValues {
                        layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                            ferroboard_core::layouts::SubtypeLayoutMapValues {
                                characters: "e:x".into(),
                                ..Default::default()
                            },
                        ))),
                        ..Subtype::new("x".into()).values()
                    },
                ));
                let keyboard = computer
                    .compute(KeyboardMode::Characters, subtype, false)
                    .unwrap();
                keyboard
                    .compute(Rc::new(KeyComputeContext::default()))
                    .unwrap();
                keyboard
                    .layout(
                        180.5,
                        99.25,
                        Some(&KeyboardGeometryOptions::from_values(
                            KeyboardGeometryOptionsValues {
                                split_gap: 30.0,
                                ..Default::default()
                            },
                        )),
                    )
                    .unwrap();
                verify(&keyboard, &expected);
            }
            "nonfinite" => {
                let v: Vec<_> = expected["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| f64::from_bits(u64::from_str_radix(s.as_str().unwrap(), 16).unwrap()))
                    .collect();
                let k = computer
                    .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
                    .unwrap();
                k.compute(Rc::new(KeyComputeContext::default())).unwrap();
                k.layout(800.0, 250.0, None).unwrap();
                k.layout(
                    v[0],
                    v[1],
                    Some(&KeyboardGeometryOptions::from_values(
                        KeyboardGeometryOptionsValues {
                            split_gap: v[2],
                            units_per_row: v[3],
                            key_spacing_horizontal: v[4],
                            key_spacing_vertical: v[5],
                        },
                    )),
                )
                .unwrap();
                verify(&k, &expected);
            }
            "rect" => {
                let v: Vec<_> = expected["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|s| f64::from_bits(u64::from_str_radix(s.as_str().unwrap(), 16).unwrap()))
                    .collect();
                let r = KeyRect {
                    x: v[0],
                    y: v[1],
                    width: v[2],
                    height: v[3],
                };
                assert_eq!(
                    json!([
                        rect(r.deflate(-0.0, 1.0, 0.0, 2.0)),
                        rect(r.deflate(0.0, 0.0, 0.0, 0.0)),
                        r.eq(&KeyRect { ..r }),
                        r.is_empty(),
                        r.contains(0.0, 0.5),
                        bits(r.center_distance_squared(1.0, 2.0)),
                        bits(r.right()),
                        bits(r.bottom())
                    ]),
                    expected["result"],
                    "{expected}"
                );
            }
            "null-transition" => {
                if null_keyboard.is_none() {
                    let r = Rc::new(KeyboardResources::create_empty());
                    r.add_layout_json(expected["input"].as_str().unwrap(), Some("e".into()))
                        .unwrap();
                    let c = LayoutComputer::new(r);
                    null_keyboard = Some(
                        c.compute(
                            KeyboardMode::Phone,
                            Arc::new(Subtype::from_values(
                                ferroboard_core::layouts::SubtypeValues {
                                    layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                                        ferroboard_core::layouts::SubtypeLayoutMapValues {
                                            phone: "e:x".into(),
                                            ..Default::default()
                                        },
                                    ))),
                                    ..Subtype::new("x".into()).values()
                                },
                            )),
                            false,
                        )
                        .unwrap(),
                    );
                }
                let k = null_keyboard.as_ref().unwrap();
                k.compute(Rc::new(KeyComputeContext::from_values(
                    ferroboard_core::layouts::KeyComputeContextValues {
                        mode: KeyboardMode::Phone,
                        shift_state: SHIFTS[expected["shift"].as_u64().unwrap() as usize],
                        ..Default::default()
                    },
                )))
                .unwrap();
                k.layout(100.0, 100.0, None).unwrap();
                verify(k, &expected);
            }
            "labels" => {
                let r = Rc::new(KeyboardResources::create_empty());
                r.add_layout_json(expected["document"].as_str().unwrap(), Some("e".into()))
                    .unwrap();
                let c = LayoutComputer::new(r);
                let k = c
                    .compute(
                        KeyboardMode::Numeric,
                        Arc::new(Subtype::from_values(
                            ferroboard_core::layouts::SubtypeValues {
                                layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                                    ferroboard_core::layouts::SubtypeLayoutMapValues {
                                        numeric: "e:x".into(),
                                        ..Default::default()
                                    },
                                ))),
                                ..Subtype::new("x".into()).values()
                            },
                        )),
                        false,
                    )
                    .unwrap();
                let ctx = &expected["context"];
                let i = |name: &str| ctx[name].as_u64().unwrap() as usize;
                let enter = [
                    EnterAction::Default,
                    EnterAction::NewLine,
                    EnterAction::Go,
                    EnterAction::Search,
                    EnterAction::Send,
                    EnterAction::Next,
                    EnterAction::Previous,
                    EnterAction::Done,
                ][i("enter")];
                k.compute(Rc::new(KeyComputeContext::from_values(
                    ferroboard_core::layouts::KeyComputeContextValues {
                        mode: MODES[i("mode")],
                        shift_state: SHIFTS[i("shift")],
                        enter_action: enter,
                        utility_key_action: UtilityKeyAction(i("utility") as i32),
                        has_multiple_subtypes: ctx["multiple"].as_bool().unwrap(),
                        space_bar_mode: SpaceBarMode(i("space") as i32),
                        space_bar_label: Some(ctx["label"].as_str().unwrap().into()),
                        is_char_half_width: ctx["half"].as_bool().unwrap(),
                        is_kana_kata: ctx["kata"].as_bool().unwrap(),
                        ..Default::default()
                    },
                )))
                .unwrap();
                k.layout(800.0, 250.0, None).unwrap();
                verify(&k, &expected);
            }
            "partial-failure" => {
                let r = Rc::new(KeyboardResources::create_empty());
                let fail = Rc::new(FailKey(std::cell::Cell::new(false)));
                let rows = vec![vec![
                    Rc::new(AutoTextKeyData {
                        base: TextKeyData::from_fields(TextKeyDataFields {
                            code: 97,
                            label: Some("a".into()),
                            ..Default::default()
                        }),
                    }) as Key,
                    fail.clone(),
                ]];
                r.add_layout(Rc::new(KeyboardLayout::new(
                    LayoutType::Characters,
                    "e".into(),
                    "x".into(),
                    Rc::new(LayoutArrangement::from_rows(rows)),
                )));
                let k = LayoutComputer::new(r)
                    .compute(
                        KeyboardMode::Characters,
                        Arc::new(Subtype::from_values(
                            ferroboard_core::layouts::SubtypeValues {
                                layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                                    ferroboard_core::layouts::SubtypeLayoutMapValues {
                                        characters: "e:x".into(),
                                        ..Default::default()
                                    },
                                ))),
                                ..Subtype::new("x".into()).values()
                            },
                        )),
                        false,
                    )
                    .unwrap();
                k.compute(Rc::new(KeyComputeContext::default())).unwrap();
                k.layout(300.0, 120.0, None).unwrap();
                fail.0.set(true);
                let context = Rc::new(KeyComputeContext::from_values(
                    ferroboard_core::layouts::KeyComputeContextValues {
                        shift_state: ShiftState::CapsLock,
                        ..Default::default()
                    },
                ));
                assert!(k.compute(context.clone()).is_err());
                assert_eq!(
                    Rc::ptr_eq(&context, k.context().as_ref().unwrap()),
                    expected["contextSame"].as_bool().unwrap()
                );
                verify(&k, &expected);
            }
            other => panic!("Unknown corpus kind {other}"),
        }
    }
}
#[test]
fn source_qwerty_shift_hints_and_identity() {
    let r = Rc::new(KeyboardResources::create_default().unwrap());
    let computer = LayoutComputer::new(r.clone());
    assert!(Rc::ptr_eq(&computer.resources(), &r));
    let keyboard = computer
        .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
        .unwrap();
    let rows = keyboard.rows();
    let q = rows.get(0).unwrap().get(0).unwrap();
    keyboard
        .compute(Rc::new(KeyComputeContext::from_values(
            ferroboard_core::layouts::KeyComputeContextValues {
                space_bar_label: Some("English".into()),
                ..Default::default()
            },
        )))
        .unwrap();
    assert_eq!(keyboard.row_count(), 4);
    assert_eq!(q.label(), Some("q".into()));
    assert_eq!(q.hint_label(), Some("1".into()));
    assert!(
        q.popup_keys()
            .snapshot()
            .unwrap()
            .iter()
            .any(|k| k.as_string(false).unwrap().unwrap() == "1".into())
    );
    let a = keyboard.find_key(97).unwrap().unwrap();
    assert_eq!(a.hint_label(), Some("@".into()));
    assert!(
        a.popup_keys()
            .snapshot()
            .unwrap()
            .iter()
            .any(|k| k.as_string(false).unwrap().unwrap() == "á".into())
    );
    keyboard
        .compute(Rc::new(KeyComputeContext::from_values(
            ferroboard_core::layouts::KeyComputeContextValues {
                shift_state: ShiftState::CapsLock,
                ..Default::default()
            },
        )))
        .unwrap();
    assert_eq!(q.label(), Some("Q".into()));
    assert_eq!(
        keyboard.find_key(KeyCode::Shift).unwrap().unwrap().icon(),
        KeyIcon::CapsLock
    );
    assert!(rows.ptr_eq(&keyboard.rows()));
    assert!(Rc::ptr_eq(&q, &keyboard.find_key(81).unwrap().unwrap()));
    keyboard.layout(800.0, 250.0, None).unwrap();
    let b = q.touch_bounds();
    assert!(Rc::ptr_eq(
        &q,
        &keyboard
            .hit_test(b.center_x(), b.center_y())
            .unwrap()
            .unwrap()
    ));
    assert!(keyboard.hit_test(1.0, -1.0).unwrap().is_none());
    assert!(keyboard.hit_test(1.0, 250.0).unwrap().is_none());
}

struct FailKey(std::cell::Cell<bool>);
impl AbstractKeyData for FailKey {
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        if self.0.get() {
            Err(KeyDataError::EmptyCharacter)
        } else {
            Ok(Some(TextKeyData::char("b".into(), None)?))
        }
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        Ok(Some("b".into()))
    }
}
