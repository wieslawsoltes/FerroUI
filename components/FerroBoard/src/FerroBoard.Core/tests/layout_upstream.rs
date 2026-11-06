//! Direct assertion counterparts of all sixteen pinned LayoutTests.cs methods.
//! Fresh, explicitly owned resources replace the source's process-global test registry.
use ferroboard_core::{input::KeyboardTouchProcessor, keys::*, layouts::*};
use std::{rc::Rc, sync::Arc};
fn context(mode: KeyboardMode, shift_state: ShiftState) -> Rc<KeyComputeContext> {
    Rc::new(KeyComputeContext::from_values(
        ferroboard_core::layouts::KeyComputeContextValues {
            mode,
            shift_state,
            space_bar_label: Some("English".into()),
            ..Default::default()
        },
    ))
}
fn keyboard(mode: KeyboardMode, number_row: bool) -> ComputedKeyboard {
    LayoutComputer::new(Rc::new(KeyboardResources::create_default().unwrap()))
        .compute(mode, Subtype::shared_default(), number_row)
        .unwrap()
}
fn computed() -> ComputedKeyboard {
    let k = keyboard(KeyboardMode::Characters, false);
    k.compute(context(KeyboardMode::Characters, ShiftState::Unshifted))
        .unwrap();
    k
}
#[test]
fn default_resources_contain_bundled_florisboard_data() {
    let r = KeyboardResources::create_default().unwrap();
    assert!(r.get_layout_metadata(LayoutType::Characters).unwrap().len() >= 70);
    assert!(r.subtype_presets().len() >= 60);
    assert!(r.currency_sets().len() >= 10);
    assert!(
        r.composers()
            .iter()
            .any(|c| c.id() == "hangul-unicode".into())
    );
    assert!(
        r.get_popup_mapping(&ComponentName::new("org.florisboard.localization", "de"))
            .unwrap()
            .is_some()
    );
}
#[test]
fn every_bundled_layout_parses() {
    let r = KeyboardResources::create_default().unwrap();
    let mut count = 0;
    for kind in LayoutType::ALL {
        for name in r.get_layout_names(kind) {
            assert!(r.get_layout(kind, &name).unwrap().is_some());
            count += 1;
        }
    }
    assert!(count > 120, "Only {count} layouts found");
}
#[test]
fn every_popup_mapping_parses() {
    let r = KeyboardResources::create_default().unwrap();
    for name in r.popup_mapping_names() {
        assert!(r.get_popup_mapping(&name).unwrap().is_some());
    }
}
#[test]
fn every_subtype_preset_computes_all_modes() {
    let r = Rc::new(KeyboardResources::create_default().unwrap());
    let computer = LayoutComputer::new(r.clone());
    for subtype in r.subtype_presets() {
        for mode in [
            KeyboardMode::Characters,
            KeyboardMode::Symbols,
            KeyboardMode::Symbols2,
            KeyboardMode::Numeric,
            KeyboardMode::NumericAdvanced,
            KeyboardMode::Phone,
            KeyboardMode::Phone2,
        ] {
            let k = computer.compute(mode, subtype.clone(), true).unwrap();
            k.compute(context(mode, ShiftState::Unshifted)).unwrap();
            k.layout(800.0, 250.0, None).unwrap();
            assert!(k.row_count() > 0, "{:?} {mode:?} has no rows", subtype.id());
        }
    }
}
#[test]
fn qwerty_is_merged_with_modifier_layout() {
    let k = computed();
    let rows = k.rows();
    assert_eq!(k.row_count(), 4);
    assert_eq!(
        rows.get(0).unwrap().get(0).unwrap().label(),
        Some("q".into())
    );
    let third = rows.get(2).unwrap();
    assert_eq!(third.get(0).unwrap().code(), KeyCode::Shift);
    assert_eq!(third.get(third.len() - 1).unwrap().code(), KeyCode::Delete);
    let last = rows.get(3).unwrap().snapshot().unwrap();
    assert!(last.iter().any(|k| k.code() == KeyCode::Space));
    assert!(last.iter().any(|k| k.code() == KeyCode::Enter));
}
#[test]
fn number_row_adds_a_row() {
    let k = keyboard(KeyboardMode::Characters, true);
    k.compute(context(KeyboardMode::Characters, ShiftState::Unshifted))
        .unwrap();
    assert_eq!(k.row_count(), 5);
    assert_eq!(
        k.rows().get(0).unwrap().get(0).unwrap().label(),
        Some("1".into())
    );
}
#[test]
fn shift_uppercases_auto_text_keys() {
    let k = keyboard(KeyboardMode::Characters, false);
    k.compute(context(KeyboardMode::Characters, ShiftState::ShiftedManual))
        .unwrap();
    assert_eq!(
        k.rows().get(0).unwrap().get(0).unwrap().label(),
        Some("Q".into())
    );
    assert_eq!(
        k.rows().get(2).unwrap().get(0).unwrap().icon(),
        KeyIcon::ShiftActive
    );
    k.compute(context(KeyboardMode::Characters, ShiftState::CapsLock))
        .unwrap();
    assert_eq!(
        k.rows().get(2).unwrap().get(0).unwrap().icon(),
        KeyIcon::CapsLock
    );
}
#[test]
fn keys_have_hints_and_popups() {
    let k = computed();
    let q = k.rows().get(0).unwrap().get(0).unwrap();
    assert_eq!(q.hint_label(), Some("1".into()));
    assert!(
        q.popup_keys()
            .snapshot()
            .unwrap()
            .iter()
            .any(|p| p.as_string(false).unwrap() == Some("1".into()))
    );
    let a = k.rows().get(1).unwrap().get(0).unwrap();
    assert_eq!(a.label(), Some("a".into()));
    assert!(
        a.popup_keys()
            .snapshot()
            .unwrap()
            .iter()
            .any(|p| p.as_string(false).unwrap() == Some("á".into()))
    );
    assert_eq!(a.hint_label(), Some("@".into()));
}
#[test]
fn popups_are_uppercased_when_shifted() {
    let k = keyboard(KeyboardMode::Characters, false);
    k.compute(context(KeyboardMode::Characters, ShiftState::ShiftedManual))
        .unwrap();
    let a = k.rows().get(1).unwrap().get(0).unwrap();
    assert!(
        a.popup_keys()
            .snapshot()
            .unwrap()
            .iter()
            .any(|p| p.as_string(false).unwrap() == Some("Á".into()))
    );
}
fn equal_to_three_decimal_places(a: f64, b: f64) {
    // xUnit Assert.Equal(double,double,3) rounds both operands, rather than using a tolerance.
    assert_eq!(
        (a * 1000.0).round_ties_even(),
        (b * 1000.0).round_ties_even()
    );
}
#[test]
fn layout_fills_width_without_overlap() {
    let k = computed();
    k.layout(
        1000.0,
        240.0,
        Some(&KeyboardGeometryOptions::from_values(
            KeyboardGeometryOptionsValues {
                key_spacing_horizontal: 4.0,
                key_spacing_vertical: 8.0,
                ..Default::default()
            },
        )),
    )
    .unwrap();
    for row in k.rows().snapshot().unwrap() {
        let visible = row
            .snapshot()
            .unwrap()
            .into_iter()
            .filter(|k| k.is_visible())
            .collect::<Vec<_>>();
        equal_to_three_decimal_places(0.0, visible[0].touch_bounds().left());
        equal_to_three_decimal_places(1000.0, visible.last().unwrap().touch_bounds().right());
        for pair in visible.windows(2) {
            equal_to_three_decimal_places(
                pair[0].touch_bounds().right(),
                pair[1].touch_bounds().left(),
            );
            assert!(pair[1].visible_bounds().left() >= pair[0].visible_bounds().right());
        }
    }
    assert!(
        k.find_key(KeyCode::Space)
            .unwrap()
            .unwrap()
            .touch_bounds()
            .width
            > 300.0,
        "space should grow"
    );
}
#[test]
fn split_layout_inserts_a_gap() {
    let k = computed();
    k.layout(
        1000.0,
        240.0,
        Some(&KeyboardGeometryOptions::from_values(
            KeyboardGeometryOptionsValues {
                split_gap: 200.0,
                ..Default::default()
            },
        )),
    )
    .unwrap();
    let t = k.rows().get(0).unwrap().get(4).unwrap();
    let y = k.rows().get(0).unwrap().get(5).unwrap();
    assert!(y.touch_bounds().left() - t.touch_bounds().right() >= 199.0);
    assert!(Rc::ptr_eq(
        &y,
        &k.hit_test(y.touch_bounds().center_x(), y.touch_bounds().center_y())
            .unwrap()
            .unwrap()
    ));
    for key in k
        .visible_keys()
        .map(|key| key.unwrap().unwrap())
        .filter(|key| key.code() != KeyCode::Space)
    {
        assert!(
            !(key.visible_bounds().left() < 599.5 && key.visible_bounds().right() > 400.5),
            "key {} crosses the split gap",
            key.code()
        );
    }
    assert!(
        k.visible_keys()
            .map(|key| key.unwrap().unwrap())
            .all(|key| key.touch_bounds().right() <= 1000.01)
    );
}
#[test]
fn hit_test_returns_key_under_point() {
    let k = computed();
    k.layout(1000.0, 240.0, None).unwrap();
    let w = k.rows().get(0).unwrap().get(1).unwrap();
    assert!(Rc::ptr_eq(
        &w,
        &k.hit_test(w.touch_bounds().center_x(), w.touch_bounds().center_y())
            .unwrap()
            .unwrap()
    ));
}
#[test]
fn utility_key_visibility_follows_settings() {
    let k = keyboard(KeyboardMode::Characters, false);
    for multiple in [false, true] {
        k.compute(Rc::new(KeyComputeContext::from_values(
            ferroboard_core::layouts::KeyComputeContextValues {
                has_multiple_subtypes: multiple,
                ..context(KeyboardMode::Characters, ShiftState::Unshifted).values()
            },
        )))
        .unwrap();
        assert_eq!(
            k.find_key(KeyCode::ImeUiModeMedia).unwrap().is_some(),
            !multiple
        );
        assert_eq!(
            k.find_key(KeyCode::LanguageSwitch).unwrap().is_some(),
            multiple
        );
    }
}
#[test]
fn currency_slots_are_resolved() {
    let r = Rc::new(KeyboardResources::create_default().unwrap());
    let k = LayoutComputer::new(r.clone())
        .compute(KeyboardMode::Symbols, Subtype::shared_default(), false)
        .unwrap();
    k.compute(Rc::new(KeyComputeContext::from_values(
        ferroboard_core::layouts::KeyComputeContextValues {
            currency_set: r
                .get_currency_set(&ComponentName::new("org.florisboard.currencysets", "euro")),
            ..context(KeyboardMode::Symbols, ShiftState::Unshifted).values()
        },
    )))
    .unwrap();
    assert!(
        k.keys()
            .map(|key| key.unwrap().unwrap())
            .any(|key| key.label() == Some("€".into()))
    );
    assert!(
        !k.keys()
            .map(|key| key.unwrap().unwrap())
            .any(|key| KeyCode::is_currency_slot(key.code()))
    );
}
#[test]
fn custom_layout_json_roundtrips() {
    const JSON: &str = r#"{"type":"characters","id":"abc","label":"ABC test","arrangement":[["a","b","c",{"code":100,"label":"d","popup":{"main":{"code":101,"label":"e"}}}],[{"$":"case_selector","lower":{"code":102,"label":"f"},"upper":{"code":70,"label":"F"}}]]}"#;
    let r = Rc::new(KeyboardResources::create_default().unwrap());
    let layout = r.add_layout_json(JSON, None).unwrap();
    assert_eq!(
        layout
            .arrangement()
            .unwrap()
            .rows
            .as_ref()
            .unwrap()
            .count()
            .unwrap(),
        2
    );
    let written = LayoutJson::write_layout_document(&layout, true).unwrap();
    let reparsed = LayoutJson::read_layout_document(&written, "custom".into()).unwrap();
    assert_eq!(
        reparsed
            .arrangement()
            .unwrap()
            .rows
            .as_ref()
            .unwrap()
            .get(0)
            .unwrap()
            .unwrap()
            .count()
            .unwrap(),
        4
    );
    let subtype = Arc::new(Subtype::from_values(
        ferroboard_core::layouts::SubtypeValues {
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    characters: layout.name(),
                    ..Default::default()
                },
            ))),
            ..(*Subtype::shared_default()).values()
        },
    ));
    let k = LayoutComputer::new(r)
        .compute(KeyboardMode::Characters, subtype, false)
        .unwrap();
    k.compute(context(KeyboardMode::Characters, ShiftState::Unshifted))
        .unwrap();
    assert_eq!(
        k.rows().get(0).unwrap().get(0).unwrap().label(),
        Some("a".into())
    );
    assert!(
        k.keys()
            .map(|key| key.unwrap().unwrap())
            .any(|key| key.code() == KeyCode::Space)
    );
}
#[test]
fn popup_arrangement_places_default_above_key() {
    let k = computed();
    k.layout(1000.0, 240.0, None).unwrap();
    let e = k.rows().get(0).unwrap().get(2).unwrap();
    let (bounds, selected) = KeyboardTouchProcessor::arrange_popup(
        &e,
        e.popup_keys().len() as i32,
        e.popup_default_index(),
        1000.0,
        6,
        f64::MIN,
    )
    .unwrap();
    let bounds = bounds.borrow();
    assert_eq!(bounds.len(), e.popup_keys().len());
    assert!(bounds[selected as usize].bottom() <= e.visible_bounds().top() + 0.01);
    assert!(
        bounds
            .iter()
            .all(|b| b.left() >= 0.0 && b.right() <= 1000.01)
    );
    for (i, a) in bounds.iter().enumerate() {
        for b in &bounds[i + 1..] {
            assert!(
                !(a.left() < b.right() - 0.01
                    && b.left() < a.right() - 0.01
                    && a.top() < b.bottom() - 0.01
                    && b.top() < a.bottom() - 0.01)
            );
        }
    }
}
