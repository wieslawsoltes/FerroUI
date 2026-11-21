use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{RibbonString, model::*};
use ferroui_system::{component_model::INotifyPropertyChanged, invariant_numbers::InvariantF64};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};

fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/icons.json")).unwrap()
}
fn text(value: &Value) -> Option<RibbonString> {
    value.as_array().map(|a| {
        RibbonString::from_utf16(
            a.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn units(value: Option<RibbonString>) -> Value {
    value.map_or(Value::Null, |s| json!(s.as_utf16()))
}
fn bits(value: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(value.as_str().unwrap(), 16).unwrap())
}
fn number(actual: f64, expected: &Value) {
    let expected = bits(expected);
    assert!(
        actual.to_bits() == expected.to_bits() || actual.is_nan() && expected.is_nan(),
        "{actual:?} != {expected:?}"
    );
}
fn layer(actual: &RibbonIconLayer, expected: &Value) {
    assert_eq!(units(actual.data()), expected["data"]);
    assert_eq!(units(actual.color()), expected["color"]);
    number(actual.stroke_thickness(), &expected["stroke"]);
    number(actual.opacity(), &expected["opacity"]);
    assert_eq!(units(actual.to_text()), expected["text"]);
}

#[test]
fn original_icon_parse_and_viewbox_corpus_including_invalid_utf16() {
    for case in corpus()["parse"].as_array().unwrap() {
        let input = text(&case["input"]);
        match RibbonIconLayer::parse_view_box(input.as_ref()) {
            Some(v) => number(v, &case["viewbox"]),
            None => assert!(case["viewbox"].is_null(), "{case}"),
        }
        let actual = RibbonIconLayer::parse(input.as_ref());
        let expected = case["layers"].as_array().unwrap();
        assert_eq!(actual.count() as usize, expected.len(), "{case}");
        for (a, e) in actual.iter().zip(expected) {
            layer(&a.unwrap(), e);
        }
    }
}

#[test]
fn original_layer_formatting_and_numeric_edge_cases() {
    let corpus = corpus();
    for case in corpus["numbers"].as_array().unwrap() {
        assert_eq!(
            InvariantF64(bits(&case["bits"])).to_string(),
            case["text"].as_str().unwrap()
        );
    }
    for case in corpus["layers"].as_array().unwrap() {
        let actual = RibbonIconLayer::with_options(
            text(&case["data"]),
            bits(&case["stroke"]),
            text(&case["color"]),
            bits(&case["opacity"]),
        );
        layer(&actual, case);
    }
}

#[test]
fn original_screentip_titles_preserve_null_whitespace_and_surrogates() {
    for case in corpus()["titles"].as_array().unwrap() {
        let title = text(&case["title"]);
        let shortcut = text(&case["shortcut"]);
        assert_eq!(
            units(Some(RibbonScreenTip::format_title(
                title.as_ref(),
                shortcut.as_ref()
            ))),
            case["result"]
        );
    }
}

#[test]
fn original_icon_factories() {
    let factories = [
        RibbonIcon::glyph_with_options(Some("x".into()), None, None),
        RibbonIcon::glyph_with_options(None, Some("font".into()), Some("red".into())),
        RibbonIcon::path_with_options(Some("M".into()), 32., Some("blue".into())),
        RibbonIcon::stroke_with_options(Some("M".into()), 1.5, 24., None),
        RibbonIcon::image(Some("uri".into())),
        RibbonIcon::text_with_foreground(Some("👩‍💻".into()), Some("yellow".into())),
        RibbonIcon::glyph_with_options(Some("x".into()), None, None)
            .with_foreground(Some("red".into())),
        RibbonIcon::layers_from_items(
            24.,
            &[
                RibbonIconLayer::with_options(None, 0., None, 1.),
                RibbonIconLayer::with_options(Some("x".into()), 0., None, 1.),
            ],
        ),
        Rc::new(RibbonIcon::from("z")),
    ];
    let corpus = corpus();
    let expected = corpus["factories"].as_array().unwrap();
    assert_eq!(factories.len(), expected.len());
    for (icon, expected) in factories.iter().zip(expected) {
        assert_eq!(json!(icon.kind.raw()), expected["kind"]);
        assert_eq!(units(icon.value.clone()), expected["value"]);
        assert_eq!(units(icon.font_family.clone()), expected["font"]);
        assert_eq!(units(icon.foreground.clone()), expected["foreground"]);
        number(icon.view_box_size, &expected["viewbox"]);
    }
}

#[test]
fn screentip_notification_trace_matches_source() {
    let tip = RibbonScreenTip::with_text(
        Some("Title".into()),
        Some("Description".into()),
        Some("Ctrl+T".into()),
    );
    let trace = Rc::new(RefCell::new(Vec::new()));
    let t = trace.clone();
    let weak = Rc::downgrade(&tip);
    tip.property_changed().add(Rc::new(move |name| {
        t.borrow_mut().push(format!(
            "{name}:{}",
            weak.upgrade()
                .unwrap()
                .title()
                .map_or_else(|| "<null>".into(), |s| s.to_utf8().unwrap())
        ));
    }));
    tip.set_title(Some("Title".into()));
    tip.set_title(None);
    tip.set_title(Some("".into()));
    tip.set_image(Some(RibbonIcon::glyph_with_options(
        Some("x".into()),
        None,
        None,
    )));
    tip.set_image(Some(RibbonIcon::glyph_with_options(
        Some("x".into()),
        None,
        None,
    )));
    tip.set_image(Some(RibbonIcon::glyph_with_options(
        Some("y".into()),
        None,
        None,
    )));
    tip.set_extended_description(Some("More".into()));
    tip.set_extended_image(Some(RibbonIcon::image(Some("uri".into()))));
    tip.set_help_text(Some("Help".into()));
    tip.set_disabled_reason(Some("Unavailable".into()));
    tip.set_shortcut(Some("".into()));
    tip.set_description(None);
    assert_eq!(json!(*trace.borrow()), corpus()["state"]);
}

#[test]
fn model_icon_implicit_conversion_and_factories() {
    let icon: RibbonIcon = RibbonIcons::Bold.into();
    assert_eq!(icon.kind, RibbonIconKind::Glyph);
    assert_eq!(
        RibbonIcon::path_with_options(Some("M0,0 L1,1".into()), 24., None).kind,
        RibbonIconKind::Path
    );
    assert_eq!(
        RibbonIcon::glyph_with_options(Some("x".into()), None, None)
            .with_foreground(Some("#FF0000".into()))
            .foreground,
        Some("#FF0000".into())
    );
}

#[test]
fn icon_layers_round_trip() {
    let icon = RibbonIcon::layers_from_items(
        32.,
        &[
            RibbonIconLayer::with_options(Some("M4,28 L28,4".into()), 1.8, None, 1.),
            RibbonIconLayer::with_options(
                Some("M2,26 h4 v4 h-4 Z".into()),
                0.,
                Some("#3DA9F5".into()),
                0.8,
            ),
        ],
    );
    assert_eq!(icon.kind, RibbonIconKind::Path);
    assert_eq!(icon.view_box_size, 32.);
    let layers = RibbonIconLayer::parse(icon.value.as_ref());
    assert_eq!(layers.count(), 2);
    assert_eq!(layers.get(0).unwrap().stroke_thickness(), 1.8);
    assert!(layers.get(0).unwrap().color().is_none());
    assert_eq!(layers.get(1).unwrap().color(), Some("#3DA9F5".into()));
    assert_eq!(layers.get(1).unwrap().opacity(), 0.8);
    assert_eq!(
        layers.get(1).unwrap().data(),
        Some("M2,26 h4 v4 h-4 Z".into())
    );
    assert_eq!(
        RibbonIconLayer::parse(
            RibbonIcon::stroke_with_options(Some("M0,0 L10,10".into()), 1.5, 24., None)
                .value
                .as_ref()
        )
        .get(0)
        .unwrap()
        .stroke_thickness(),
        1.5
    );
    assert_eq!(RibbonIconLayer::parse(Some(&"M1,1 L2,2".into())).count(), 1);
    assert!(
        icon.value
            .as_ref()
            .unwrap()
            .to_utf8()
            .unwrap()
            .starts_with("[stroke=1.8]")
    );
    let xaml = RibbonIconLayer::parse(Some(
        &"[viewbox=32;stroke=2]M0,0 L4,4|{color=#FF0000}M1,1 h2 v2 Z".into(),
    ));
    assert_eq!(xaml.get(0).unwrap().stroke_thickness(), 2.);
    assert_eq!(xaml.get(1).unwrap().color(), Some("#FF0000".into()));
    assert_eq!(
        RibbonIconLayer::parse_view_box(Some(&"[viewbox=32;stroke=2]M0,0 L4,4".into())),
        Some(32.)
    );
    assert_eq!(
        RibbonIconLayer::parse_view_box(Some(&"M0,0 L4,4".into())),
        None
    );
    assert!(RibbonIconLayer::parse(Some(&"  ".into())).count() == 0);
}

#[test]
fn structural_icon_equality_suppresses_assignment_but_screentip_uses_identity() {
    let tip = RibbonScreenTip::new();
    let first = RibbonIcon::new_with_options(RibbonIconKind::Path, None, None, None, f64::NAN);
    tip.set_image(Some(first.clone()));
    let second = RibbonIcon::new_with_options(
        RibbonIconKind::Path,
        None,
        None,
        None,
        f64::from_bits(0x7ff8_0000_0000_0001),
    );
    let weak = Rc::downgrade(&second);
    tip.set_image(Some(second));
    assert!(weak.upgrade().is_none());
    assert!(Rc::ptr_eq(&tip.image().unwrap(), &first));
    let copy = first.with_foreground(None);
    assert_eq!(first, copy);
    assert!(!Rc::ptr_eq(&first, &copy));
    assert!(tip == tip.clone());
    assert!(tip != RibbonScreenTip::new());
}

#[test]
fn utf16_storage_is_lossless_and_utf8_boundary_is_explicit() {
    let value = RibbonString::from_utf16(vec![0xd800, 0x61, 0xdc00]);
    let alias = value.clone();
    assert!(value.ptr_eq(&alias));
    assert_eq!(value.as_utf16(), [0xd800, 0x61, 0xdc00]);
    assert!(value.to_utf8().is_err());
    let value = RibbonString::from("👩‍💻");
    assert_eq!(value.len(), 5);
    assert_eq!(value.to_utf8().unwrap(), "👩‍💻");
}

#[test]
fn source_icon_kind_display_including_unnamed_values() {
    let source: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/model-leaf.json")).unwrap();
    for case in source["corpus"]["enumNames"].as_array().unwrap() {
        assert_eq!(
            RibbonIconKind::from_raw(case["value"].as_i64().unwrap() as i32).to_string(),
            case["text"]
        );
    }
}

#[test]
fn original_parse_list_aliasing_null_entries_and_enumerator_invalidation() {
    let source: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/icon-sequence.json")).unwrap();
    let a = RibbonIconLayer::parse(Some(&"M0,0 L1,1|[stroke=2]M2,2 L3,3".into()));
    let b = RibbonIconLayer::parse(Some(&"M0,0 L1,1|[stroke=2]M2,2 L3,3".into()));
    let alias = a.clone();
    let mutable = (a.clone() as Rc<dyn std::any::Any>)
        .downcast::<ferroribbon_core::collections::RibbonItems<Option<Rc<RibbonIconLayer>>>>()
        .expect("source Parse returns List");
    let first = a.get(0).unwrap();
    let same_item = Rc::ptr_eq(&first, &mutable.get(0).unwrap());
    mutable.set(0, Some(RibbonIconLayer::new(Some("changed".into()))));
    mutable.add(None);
    assert_eq!(
        json!({"sameAlias":Rc::ptr_eq(&a, &alias),"differentParse":Rc::ptr_eq(&a, &b),"sameItem":same_item,"changed":a.get(0).unwrap().data().unwrap().to_utf8().unwrap(),"aliasCount":alias.count(),"otherCount":b.count(),"nullItem":a.get(2).is_none()}),
        source["corpus"]["shared"]
    );
    let mut iterator = mutable.iter();
    assert_eq!(iterator.next().is_some(), source["corpus"]["firstMove"]);
    mutable.add(Some(RibbonIconLayer::new(Some("added".into()))));
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| iterator.next())).is_err());
    assert_eq!(
        source["corpus"]["invalidation"]["type"],
        "InvalidOperationException"
    );
    let empty = RibbonIconLayer::parse(None);
    let other = RibbonIconLayer::parse(None);
    assert_eq!(
        json!({"same":Rc::ptr_eq(&empty, &other),"count":empty.count()}),
        source["corpus"]["nullEmpty"]
    );
    let array = RibbonFixedItems::new(vec![None]);
    let array_view =
        array as ferroribbon_core::collections::RibbonReadOnlyList<Option<Rc<RibbonIconLayer>>>;
    assert!(
        (array_view as Rc<dyn std::any::Any>)
            .downcast::<ferroribbon_core::collections::RibbonItems<Option<Rc<RibbonIconLayer>>>>()
            .is_err(),
        "Array must not pretend to be List"
    );
}

#[test]
fn original_layers_nullable_array_and_items_join_and_numeric_bits() {
    let source: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/icon-sequence.json")).unwrap();
    let snapshot = |icon: &RibbonIcon| json!({"kind":icon.kind.raw(),"value":icon.value.as_ref().map(|v|v.as_utf16()),"viewbox":format!("{:016x}",icon.view_box_size.to_bits())});
    let array = RibbonFixedItems::new(vec![
        Some(RibbonIconLayer::new(None)),
        Some(RibbonIconLayer::new(Some("x".into()))),
        Some(RibbonIconLayer::new(None)),
    ]);
    assert_eq!(
        snapshot(&RibbonIcon::layers(32., Some(&array)).unwrap()),
        source["corpus"]["joined"]
    );
    assert_eq!(
        snapshot(&RibbonIcon::layers(-0., Some(&RibbonFixedItems::new(Vec::new()))).unwrap()),
        source["corpus"]["empty"]
    );
    assert_eq!(
        RibbonIcon::layers(24., None).unwrap_err(),
        RibbonIconError::NullLayers
    );
    assert_eq!(
        source["corpus"]["errors"]["nullArray"],
        json!({"type":"ArgumentNullException","parameter":"layers"})
    );
    let array = RibbonFixedItems::new(vec![Some(RibbonIconLayer::new(Some("x".into()))), None]);
    assert_eq!(
        RibbonIcon::layers(24., Some(&array)).unwrap_err(),
        RibbonIconError::NullLayer
    );
    assert_eq!(
        source["corpus"]["errors"]["nullItem"],
        json!({"type":"NullReferenceException","parameter":null})
    );
    assert_eq!(
        snapshot(&RibbonIcon::glyph(None)),
        source["corpus"]["convertedNull"]
    );
    assert_eq!(
        snapshot(&RibbonIcon::glyph(Some("plain".into()))),
        source["corpus"]["convertedText"]
    );
}
