use super::*;
use ferroui_base::{
    controls::ResourceDictionary, data::core::ValueTypes, styling::ControlTheme,
    threading::Dispatcher,
};
use ferroui_controls::Window;
use serde_json::{Value, json};
fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
fn number(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn state(c: &RibbonZoomControl) -> Value {
    json!({"value":bits(c.value()),"minimum":bits(c.minimum()),"maximum":bits(c.maximum()),"step":bits(c.step()),"text":c.value_text(),"quick":c.can_add_to_quick_access(),"tab":ferroui_base::input::KeyboardNavigation::get_is_tab_stop(c)})
}
fn normalize_nan(v: &mut Value) {
    match v {
        Value::Object(m) => {
            for (k, v) in m.iter_mut() {
                if ["value", "minimum", "maximum", "step", "input"].contains(&k.as_str())
                    && v.as_str().is_some_and(|s| {
                        s.len() == 16
                            && u64::from_str_radix(s, 16).is_ok_and(|n| f64::from_bits(n).is_nan())
                    })
                {
                    *v = json!("NaN");
                } else {
                    normalize_nan(v);
                }
            }
        }
        Value::Array(a) => {
            for v in a {
                normalize_nan(v);
            }
        }
        _ => {}
    }
}
fn expect_state(c: &RibbonZoomControl, expected: &Value) {
    let mut actual = state(c);
    let mut expected = expected.clone();
    normalize_nan(&mut actual);
    normalize_nan(&mut expected);
    assert_eq!(actual, expected);
}
fn mount(c: &RibbonZoomControl) -> Ref<Window> {
    let value = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Zoom.xaml")
        .unwrap()
        .unwrap();
    let d = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    d.add_merged_dictionary(
        crate::theming::RibbonThemeResources::new().upcast::<ResourceDictionary>(),
    );
    let theme = d
        .try_get_resource(&RibbonZoomControl::TYPE.into(), None)
        .flatten()
        .unwrap();
    c.set_resources(d);
    c.set_theme(
        ValueTypes::as_object(&*theme)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap(),
    );
    let w = Window::new();
    w.set_content(Some(Control::boxed(c.to_ref())));
    w.show();
    w.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    w
}
fn button(c: &RibbonZoomControl, name: &str) -> Ref<Button> {
    c.get_visual_descendants()
        .find_map(|p| {
            p.cast::<Button>()
                .filter(|p| p.name().as_deref() == Some(name))
        })
        .unwrap()
}
fn click(c: &RibbonZoomControl, name: &str) {
    button(c, name).raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
        Button::click_event(),
    ));
}
fn observe(c: &RibbonZoomControl, log: &Rc<RefCell<Vec<String>>>, text: bool) {
    let out = log.clone();
    c.value_changed(move |c, v| {
        out.borrow_mut().push(if text {
            format!("event:{}:{}", bits(v), c.value_text())
        } else {
            format!("event:{}", bits(v))
        })
    });
    let out = log.clone();
    c.zoom_dialog_requested(move |_| out.borrow_mut().push("dialog".into()));
    let out = log.clone();
    c.set_command(Some(crate::commands::adapt_core_command(
        ferroribbon_core::commands::RibbonRelayCommand::new_with_parameter(
            move |v| {
                out.borrow_mut().push(format!(
                    "command:{}",
                    bits(*v.unwrap().downcast_ref::<f64>().unwrap())
                ))
            },
            None,
        ),
    )));
}
#[test]
fn unchanged_zoom_source_numeric_commands_retemplate_and_reentrancy_corpus() {
    crate::group_tests::run(|| {
        let old = CultureInfo::current_formatting();
        CultureInfo::set_current_formatting(CultureInfo::invariant_culture());
        let fixture: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/zoom-control.json")).unwrap();
        let cases = fixture["corpus"]["cases"].as_array().unwrap();
        let log = Rc::new(RefCell::new(vec![]));
        let mut c = RibbonZoomControl::new();
        let mut w = None;
        let mut old_button = None;
        let mut old_slider = None;
        for case in cases {
            log.borrow_mut().clear();
            match case["name"].as_str().unwrap() {
                "defaults" => {
                    observe(&c, &log, true);
                }
                "programmatic" => c.set_numeric_value(number(&case["input"])),
                "button" => {
                    if w.is_none() {
                        c = RibbonZoomControl::new();
                        w = Some(mount(&c));
                        observe(&c, &log, true);
                    }
                    click(&c, case["part"].as_str().unwrap());
                }
                "slider_user" => c.slider_part().set_range_value(137.),
                "old_parts_detached" => {
                    old_button = Some(button(&c, "PART_ZoomIn"));
                    old_slider = Some(c.slider_part());
                    let template = c.template();
                    c.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
                        |_, _| ferroui_controls::Border::new().upcast(),
                    )));
                    c.apply_template();
                    c.set_template(template);
                    c.apply_template();
                    old_button.as_ref().unwrap().raise_event(
                        &ferroui_base::interactivity::RoutedEventArgs::with_event(
                            Button::click_event(),
                        ),
                    );
                    old_slider.as_ref().unwrap().set_range_value(200.);
                }
                "invoke" => {
                    assert!(c.invoke_core());
                    let names = ["PART_ZoomOut", "PART_ZoomIn", "PART_Percent"]
                        .map(|name| AutomationProperties::get_name(&button(&c, name)));
                    assert_eq!(
                        json!([
                            names[0],
                            names[1],
                            names[2],
                            AutomationProperties::get_name(&c.slider_part())
                        ]),
                        case["names"]
                    );
                }
                "inverted_range" => {
                    if let Some(w) = w.take() {
                        w.close();
                    }
                    c = RibbonZoomControl::new();
                    observe(&c, &log, false);
                    c.set_minimum(600.);
                }
                "step_boundary" => {
                    if let Some(w) = w.take() {
                        w.close();
                    }
                    c = RibbonZoomControl::new();
                    w = Some(mount(&c));
                    c.set_step(number(&case["step"]));
                    observe(&c, &log, false);
                    click(&c, "PART_ZoomIn");
                    assert!(case["error"].is_null());
                }
                "reentrant_programmatic_value" => {
                    if let Some(w) = w.take() {
                        w.close();
                    }
                    c = RibbonZoomControl::new();
                    w = Some(mount(&c));
                    let nested = Cell::new(false);
                    let out = log.clone();
                    c.value_changed(move |c, v| {
                        out.borrow_mut().push(format!("event:{}", bits(v)));
                        if !nested.replace(true) {
                            c.set_numeric_value(125.);
                        }
                    });
                    let out = log.clone();
                    c.set_command(Some(crate::commands::adapt_core_command(
                        ferroribbon_core::commands::RibbonRelayCommand::new_with_parameter(
                            move |v| {
                                out.borrow_mut().push(format!(
                                    "command:{}",
                                    bits(*v.unwrap().downcast_ref::<f64>().unwrap())
                                ))
                            },
                            None,
                        ),
                    )));
                    click(&c, "PART_ZoomIn");
                }
                other => panic!("unmapped reference case {other}"),
            }
            expect_state(&c, &case["state"]);
            if let Some(expected) = case.get("trace") {
                assert_eq!(json!(*log.borrow()), *expected, "{case}");
            }
        }
        if let Some(w) = w {
            w.close();
        }
        drop((old_button, old_slider));
        CultureInfo::set_current_formatting(old);
    });
}
#[test]
fn native_and_xaml_callbacks_share_order_removal_snapshot_and_weak_lifetimes() {
    crate::group_tests::run(|| {
        let c = RibbonZoomControl::new();
        let log = Rc::new(RefCell::new(vec![]));
        let out = log.clone();
        let token = c.value_changed(move |_, _| out.borrow_mut().push("rust"));
        let out = log.clone();
        let xaml = MarkupDelegate::new(move |_| {
            out.borrow_mut().push("xaml");
            None
        });
        c.add_value_changed(Some(xaml.clone()));
        c.set_numeric_value(110.);
        assert_eq!(*log.borrow(), ["rust", "xaml"]);
        token.dispose();
        c.remove_value_changed(Some(xaml));
        log.borrow_mut().clear();
        c.set_numeric_value(120.);
        assert!(log.borrow().is_empty());
        let second = Rc::new(RefCell::new(None::<Rc<dyn IDisposable>>));
        let remove = second.clone();
        let out = log.clone();
        let first = c.value_changed(move |_, _| {
            out.borrow_mut().push("first");
            if let Some(token) = remove.borrow_mut().take() {
                token.dispose();
            }
        });
        let out = log.clone();
        *second.borrow_mut() = Some(c.value_changed(move |_, _| out.borrow_mut().push("second")));
        c.set_numeric_value(130.);
        assert_eq!(*log.borrow(), ["first", "second"]);
        log.borrow_mut().clear();
        c.set_numeric_value(140.);
        assert_eq!(*log.borrow(), ["first"]);
        first.dispose();
        let marker = Rc::new(());
        let weak_marker = Rc::downgrade(&marker);
        let weak = c.downgrade();
        let token = c.value_changed(move |_, _| {
            let _ = &marker;
        });
        drop(c);
        assert!(weak.upgrade().is_none());
        assert!(weak_marker.upgrade().is_none());
        token.dispose();
    });
}
#[test]
fn missing_template_parts_are_supported_and_user_flag_resets_during_unwind() {
    crate::group_tests::run(|| {
        let c = RibbonZoomControl::new();
        c.set_numeric_value(112.5);
        assert_eq!(c.value_text(), "113%");
        assert!(!c.can_add_to_quick_access());
        assert!(!c.focusable());
        assert!(!ferroui_base::input::KeyboardNavigation::get_is_tab_stop(
            &c
        ));
        let token = c.value_changed(|_, _| panic!("subscriber failure"));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.set_user_value(130.)))
                .is_err()
        );
        assert!(!c.user_change.get());
        token.dispose();
        c.set_numeric_value(140.);
        assert_eq!(c.value_text(), "140%");
    });
}
