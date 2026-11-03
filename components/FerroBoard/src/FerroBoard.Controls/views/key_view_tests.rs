use super::*;
use crate::themes::KeyboardThemes;
use ferroboard_core::{editor::EnterAction, layouts::*};
use ferroui_base::media::IBrush;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
fn bits(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}
fn float(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn text(s: Option<String>) -> Value {
    s.map_or(Value::Null, |s| json!(s.encode_utf16().collect::<Vec<_>>()))
}
fn brush_color(brush: Option<Rc<dyn IBrush>>) -> Value {
    brush.map_or(Value::Null, |b| {
        json!(KeyboardTheme::to_hex(
            b.as_solid_color_brush().unwrap().color()
        ))
    })
}
fn thickness(t: Thickness) -> Value {
    json!([bits(t.left), bits(t.top), bits(t.right), bits(t.bottom)])
}
fn corner(c: CornerRadius) -> Value {
    json!([
        bits(c.top_left),
        bits(c.top_right),
        bits(c.bottom_right),
        bits(c.bottom_left)
    ])
}
fn property(c: &Control, name: &str) -> Value {
    match name {
        "Width" => json!(bits(c.width())),
        "Height" => json!(bits(c.height())),
        "MaxWidth" => json!(bits(c.max_width())),
        "Left" => json!(bits(Canvas::get_left(c))),
        "Top" => json!(bits(Canvas::get_top(c))),
        "Visibility" => json!(if c.is_visible() {
            "Visible"
        } else {
            "Collapsed"
        }),
        "IsHitTestVisible" => json!(c.is_hit_test_visible()),
        "IsTabStop" => json!(c.is_tab_stop()),
        "HorizontalAlignment" => json!(format!("{:?}", c.horizontal_alignment())),
        "VerticalAlignment" => json!(format!("{:?}", c.vertical_alignment())),
        "Margin" => thickness(c.margin()),
        "AutomationName" => text(AutomationProperties::get_name(c)),
        "Background" => brush_color(c.downcast_ref::<Border>().unwrap().background()),
        "BorderBrush" => brush_color(c.downcast_ref::<Border>().unwrap().border_brush()),
        "BorderThickness" => thickness(c.downcast_ref::<Border>().unwrap().border_thickness()),
        "CornerRadius" => corner(c.downcast_ref::<Border>().unwrap().corner_radius()),
        "FontSize" => json!(bits(c.downcast_ref::<TextBlock>().unwrap().font_size())),
        "FontFamily" => json!(c.downcast_ref::<TextBlock>().unwrap().font_family().name()),
        "Foreground" => brush_color(c.downcast_ref::<TextBlock>().unwrap().foreground()),
        "Text" => text(c.downcast_ref::<TextBlock>().unwrap().text()),
        "TextAlignment" => json!(format!(
            "{:?}",
            c.downcast_ref::<TextBlock>().unwrap().text_alignment()
        )),
        "TextWrapping" => json!(format!(
            "{:?}",
            c.downcast_ref::<TextBlock>().unwrap().text_wrapping()
        )),
        "TextTrimming" => json!(
            c.downcast_ref::<TextBlock>()
                .unwrap()
                .text_trimming()
                .to_string()
        ),
        "Fill" => brush_color(c.downcast_ref::<Path>().unwrap().fill()),
        "Stretch" => json!(format!("{:?}", c.downcast_ref::<Path>().unwrap().stretch())),
        _ => panic!("unmapped written source property {name}"),
    }
}
fn written(c: &Control, expected: &Value) -> Value {
    Value::Object(
        expected
            .as_object()
            .unwrap()
            .keys()
            .map(|name| (name.clone(), property(c, name)))
            .collect(),
    )
}
fn snapshot(view: &KeyView, expected: &Value) -> Value {
    let icon = view.icon.borrow().clone();
    json!({"view":written(view,&expected["view"]),"shadow":written(&view.shadow,&expected["shadow"]),
        "face":written(&view.face,&expected["face"]),"label":written(&view.label,&expected["label"]),
        "hint":written(&view.hint,&expected["hint"]),"sub":written(&view.sub_label,&expected["sub"]),
        "icon":icon.map(|icon|written(&icon,&expected["icon"])),
        "children":view.face_content.children().snapshot().iter().map(|c|c.get_type().name()).collect::<Vec<_>>(),
        "iconKind":view.icon_kind.get().0,"iconSize":bits(view.icon_size.get())})
}
fn style(s: KeyStyle) -> Value {
    json!({"background":KeyboardTheme::to_hex(s.background),"foreground":KeyboardTheme::to_hex(s.foreground),"font":bits(s.font_size),"corner":bits(s.corner_radius)})
}
fn mode(i: u64) -> KeyboardMode {
    [
        KeyboardMode::Characters,
        KeyboardMode::Symbols,
        KeyboardMode::Symbols2,
        KeyboardMode::Numeric,
        KeyboardMode::NumericAdvanced,
        KeyboardMode::Phone,
        KeyboardMode::Phone2,
    ][i as usize]
}
fn shift(i: u64) -> ShiftState {
    [
        ShiftState::Unshifted,
        ShiftState::ShiftedManual,
        ShiftState::ShiftedAutomatic,
        ShiftState::CapsLock,
    ][i as usize]
}
fn difference(a: &Value, b: &Value, path: String) -> String {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for k in a.keys().chain(b.keys()) {
                if a.get(k) != b.get(k) {
                    return difference(
                        a.get(k).unwrap_or(&Value::Null),
                        b.get(k).unwrap_or(&Value::Null),
                        format!("{path}/{k}"),
                    );
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            for i in 0..a.len().max(b.len()) {
                if a.get(i) != b.get(i) {
                    return difference(
                        a.get(i).unwrap_or(&Value::Null),
                        b.get(i).unwrap_or(&Value::Null),
                        format!("{path}/{i}"),
                    );
                }
            }
        }
        _ => (),
    }
    format!("{path}: actual {a} expected {b}")
}
#[test]
fn unchanged_key_style_view_update_and_strings_reference() {
    crate::test_support::run(reference);
}
fn reference() {
    let resources = Rc::new(KeyboardResources::create_default().unwrap());
    let computer = LayoutComputer::new(resources);
    let mut styles = HashMap::new();
    let mut keyboards = HashMap::new();
    let mut views: HashMap<(u64, usize), Ref<KeyView>> = HashMap::new();
    let mut count = 0;
    for line in include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/key-view.jsonl"
    ))
    .lines()
    {
        let row: Value = serde_json::from_str(line).unwrap();
        let input = &row["input"];
        let expected = &row["result"];
        assert!(
            row["error"].is_null(),
            "reference unexpectedly failed {input}"
        );
        let actual = match input["op"].as_str().unwrap() {
            "darken" => json!(KeyboardTheme::to_hex(KeyStyles::darken(
                KeyboardTheme::hex(input["color"].as_str().unwrap()).unwrap(),
                float(&input["factor"])
            ))),
            "style" => {
                let (m, s) = (
                    input["mode"].as_u64().unwrap(),
                    input["shift"].as_u64().unwrap(),
                );
                let keyboard = styles.entry((m, s)).or_insert_with(|| {
                    let k = computer
                        .compute(mode(m), Subtype::shared_default(), true)
                        .unwrap();
                    k.compute(Rc::new(KeyComputeContext::from_values(
                        ferroboard_core::layouts::KeyComputeContextValues {
                            mode: mode(m),
                            shift_state: shift(s),
                            space_bar_label: Some("English".into()),
                            has_multiple_subtypes: true,
                            enter_action: EnterAction::Search,
                            ..Default::default()
                        },
                    )))
                    .unwrap();
                    k.layout(800.5, 250.25, None).unwrap();
                    k
                });
                let key = keyboard
                    .keys()
                    .map(|key| key.unwrap().unwrap())
                    .nth(input["key"].as_u64().unwrap() as usize)
                    .unwrap();
                let theme = KeyboardThemes::get(input["theme"].as_str().unwrap()).unwrap();
                let mut value = style(
                    KeyStyles::resolve(
                        &theme,
                        &key,
                        mode(m),
                        shift(s),
                        input["pressed"].as_bool().unwrap(),
                        float(&input["scale"]),
                    )
                    .unwrap(),
                );
                value["function"] = json!(KeyStyles::is_function_key(&key));
                value
            }
            "customStyle" | "customRules" => {
                let r = Rc::new(KeyboardResources::create_empty());
                r.add_layout_json(input["document"].as_str().unwrap(), Some("e".into()))
                    .unwrap();
                let subtype = Subtype::new("x".into());
                subtype.set_layouts(Some(Arc::new(SubtypeLayoutMap::from_values(
                    ferroboard_core::layouts::SubtypeLayoutMapValues {
                        numeric: ComponentName::new("e", "x"),
                        ..Default::default()
                    },
                ))));
                let keyboard = LayoutComputer::new(r)
                    .compute(KeyboardMode::Numeric, Arc::new(subtype), false)
                    .unwrap();
                keyboard
                    .compute(Rc::new(KeyComputeContext::from_values(
                        ferroboard_core::layouts::KeyComputeContextValues {
                            mode: KeyboardMode::Numeric,
                            ..Default::default()
                        },
                    )))
                    .unwrap();
                let key = keyboard
                    .keys()
                    .map(|key| key.unwrap().unwrap())
                    .nth(input["key"].as_u64().unwrap() as usize)
                    .unwrap();
                let rules = input["op"] == "customRules";
                let theme = if rules {
                    KeyboardTheme::from_json(input["theme"].as_str().unwrap(), None).unwrap()
                } else {
                    KeyboardThemes::floris_day()
                };
                style(
                    KeyStyles::resolve(
                        &theme,
                        &key,
                        KeyboardMode::Numeric,
                        ShiftState::Unshifted,
                        if rules {
                            input["pressed"].as_bool().unwrap()
                        } else {
                            false
                        },
                        if rules { 1.25 } else { 1.0 },
                    )
                    .unwrap(),
                )
            }
            "view" => {
                let m = input["mode"].as_u64().unwrap();
                let s = input["shift"].as_u64().unwrap();
                let step = input["step"].as_u64().unwrap();
                let index = input["key"].as_u64().unwrap() as usize;
                let keyboard = keyboards.entry(m).or_insert_with(|| {
                    computer
                        .compute(mode(m), Subtype::shared_default(), true)
                        .unwrap()
                });
                if index == 0 {
                    keyboard
                        .compute(Rc::new(KeyComputeContext::from_values(
                            ferroboard_core::layouts::KeyComputeContextValues {
                                mode: mode(m),
                                shift_state: shift(s),
                                space_bar_label: Some(if step.is_multiple_of(2) {
                                    "English".into()
                                } else {
                                    Utf16Text::default()
                                }),
                                has_multiple_subtypes: true,
                                enter_action: if step.is_multiple_of(3) {
                                    EnterAction::Search
                                } else {
                                    EnterAction::Done
                                },
                                ..Default::default()
                            },
                        )))
                        .unwrap();
                    keyboard
                        .layout(float(&input["width"]), float(&input["height"]), None)
                        .unwrap();
                }
                let key = keyboard
                    .keys()
                    .map(|key| key.unwrap().unwrap())
                    .nth(index)
                    .unwrap();
                let view = views
                    .entry((m, index))
                    .or_insert_with(|| KeyView::new(key.clone()));
                assert!(Rc::ptr_eq(&key, &view.key()));
                let old = view.icon.borrow().clone();
                view.set_is_pressed(input["pressed"].as_bool().unwrap());
                let theme =
                    KeyboardTheme::from_json(input["theme"].as_str().unwrap(), None).unwrap();
                view.update(
                    &theme,
                    mode(m),
                    shift(s),
                    float(&input["scale"]),
                    input["font"].as_str().map(FontFamily::new),
                )
                .unwrap();
                json!({"state":snapshot(view,&expected["state"]),"iconReused":old.is_some() && old==*view.icon.borrow()})
            }
            "string" => KeyboardStrings::source_value(input["name"].as_str().unwrap())
                .unwrap()
                .map_or(Value::Null, |s| json!(s.units())),
            "iconFailure" => {
                let keyboard = computer
                    .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
                    .unwrap();
                keyboard
                    .compute(Rc::new(KeyComputeContext::default()))
                    .unwrap();
                keyboard.layout(800.0, 250.0, None).unwrap();
                let key = keyboard
                    .keys()
                    .map(|key| key.unwrap().unwrap())
                    .find(|key| key.code() == KeyCode::Shift)
                    .unwrap();
                let view = KeyView::new(key);
                let theme = KeyboardThemes::floris_day();
                view.update(
                    &theme,
                    KeyboardMode::Characters,
                    ShiftState::Unshifted,
                    1.0,
                    None,
                )
                .unwrap();
                IconFactory::register(KeyIcon::Shift, "M bad 0");
                let failure = match view.update(
                    &theme,
                    KeyboardMode::Characters,
                    ShiftState::Unshifted,
                    1.2,
                    None,
                ) {
                    Err(KeyViewError::Icon(IconError::InvalidNumber { .. })) => {
                        json!("System.FormatException")
                    }
                    other => panic!("{other:?}"),
                };
                let partial = snapshot(&view, &expected["partial"]);
                view.update(
                    &theme,
                    KeyboardMode::Characters,
                    ShiftState::Unshifted,
                    1.0,
                    None,
                )
                .unwrap();
                let final_state = snapshot(&view, &expected["final"]);
                IconFactory::register(KeyIcon::Shift, input["restore"].as_str().unwrap());
                json!({"failure":failure,"partial":partial,"final":final_state})
            }
            op => panic!("{op}"),
        };
        assert!(
            actual == *expected,
            "row {} input {}: {}",
            row["index"],
            input["op"],
            difference(&actual, expected, String::new())
        );
        count += 1;
    }
    assert_eq!(count, 1494);
}

#[test]
fn actual_canvas_geometry_localized_automation_and_explicit_encoding_boundary() {
    crate::test_support::run(|| {
        use ferroui_base::threading::Dispatcher;
        use ferroui_controls::Window;
        let resources = Rc::new(KeyboardResources::create_default().unwrap());
        let computer = LayoutComputer::new(resources);
        let keyboard = computer
            .compute(KeyboardMode::Characters, Subtype::shared_default(), false)
            .unwrap();
        keyboard
            .compute(Rc::new(KeyComputeContext::default()))
            .unwrap();
        keyboard.layout(800.5, 250.25, None).unwrap();
        let key = keyboard
            .keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.code() == 97)
            .unwrap();
        let expected = key.visible_bounds();
        let view = KeyView::new(key);
        let theme = KeyboardThemes::floris_day();
        view.update(
            &theme,
            KeyboardMode::Characters,
            ShiftState::Unshifted,
            1.0,
            None,
        )
        .unwrap();
        let canvas = Canvas::new();
        canvas.set_use_layout_rounding(false);
        view.set_use_layout_rounding(false);
        canvas.children().add(view.clone());
        let window = Window::new();
        window.set_use_layout_rounding(false);
        window.set_width(800.5);
        window.set_height(250.25);
        window.set_content(Some(Control::boxed(canvas)));
        window.show();
        Dispatcher::current_dispatcher().run_jobs(None);
        let actual = view.bounds();
        assert_eq!(
            [actual.x, actual.y, actual.width, actual.height],
            [expected.x, expected.y, expected.width, expected.height]
        );
        assert!(!view.is_hit_test_visible());
        assert!(!view.is_tab_stop());
        assert!(!view.focusable());
        assert_eq!(FontWeightsEx::semi_bold().0, 600);
        let shared = Brushes::get(theme.key_background());
        assert!(std::ptr::eq(
            view.face.background().unwrap().as_object().unwrap(),
            &**shared.upcast_ref::<ferroui_base::FerroObject>()
        ));
        let original = shared.color();
        shared.set_color(Color::from_uint32(0xff102030));
        assert_eq!(
            view.face
                .background()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            shared.color()
        );
        shared.set_color(original);
        window.close();

        struct Restore(Option<Utf16Text>);
        impl Drop for Restore {
            fn drop(&mut self) {
                KeyboardStrings::set_shift(self.0.clone());
            }
        }
        let _restore = Restore(KeyboardStrings::shift());
        let key = keyboard
            .keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.code() == KeyCode::Shift)
            .unwrap();
        let view = KeyView::new(key);
        let translated: Utf16Text = "Majuscule 👩‍💻".into();
        KeyboardStrings::set_shift(Some(translated.clone()));
        assert_eq!(
            KeyboardStrings::shift().unwrap().units().as_ptr(),
            translated.units().as_ptr()
        );
        view.update(
            &theme,
            KeyboardMode::Characters,
            ShiftState::Unshifted,
            1.0,
            None,
        )
        .unwrap();
        assert_eq!(
            AutomationProperties::get_name(&view).as_deref(),
            Some("Majuscule 👩‍💻")
        );
        KeyboardStrings::set_shift(None);
        view.update(
            &theme,
            KeyboardMode::Characters,
            ShiftState::Unshifted,
            1.0,
            None,
        )
        .unwrap();
        assert_eq!(AutomationProperties::get_name(&view), None);
        KeyboardStrings::set_shift(Some(Utf16Text::from_units(vec![0xd800])));
        assert!(matches!(
            view.update(
                &theme,
                KeyboardMode::Characters,
                ShiftState::Unshifted,
                1.0,
                None
            ),
            Err(KeyViewError::UnpairedSurrogate("automation name"))
        ));
    });
}
