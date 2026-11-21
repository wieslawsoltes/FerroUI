use super::*;
use crate::controls::{Ribbon, RibbonGroup, RibbonTab};
use ferroui_base::{
    controls::ResourceDictionary,
    data::core::ValueTypes,
    input::{
        IPointer, MouseButton, Pointer, PointerPointProperties, PointerType, PointerWheelEventArgs,
    },
    styling::ControlTheme,
    threading::Dispatcher,
    utilities::NumberFormatInfo,
};
use ferroui_controls::Window;
use serde_json::{Value, json};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/spinner.json")).unwrap()
}
fn case(name: &str) -> Value {
    corpus()["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()
        .clone()
}
fn bits(v: f64) -> String {
    format!("{:016X}", v.to_bits())
}
fn numeric_info(v: &Value) -> NumberFormatInfo {
    let mut n = NumberFormatInfo::new();
    macro_rules! text{($($set:ident:$name:literal),*)=>{$(n.$set(v[$name].as_str().unwrap());)*};}
    text!(set_currency_decimal_separator:"CurrencyDecimalSeparator",set_currency_group_separator:"CurrencyGroupSeparator",set_currency_symbol:"CurrencySymbol",set_nan_symbol:"NaNSymbol",set_negative_infinity_symbol:"NegativeInfinitySymbol",set_negative_sign:"NegativeSign",set_number_decimal_separator:"NumberDecimalSeparator",set_number_group_separator:"NumberGroupSeparator",set_percent_decimal_separator:"PercentDecimalSeparator",set_percent_group_separator:"PercentGroupSeparator",set_percent_symbol:"PercentSymbol",set_per_mille_symbol:"PerMilleSymbol",set_positive_infinity_symbol:"PositiveInfinitySymbol",set_positive_sign:"PositiveSign");
    macro_rules! number{($($set:ident:$name:literal),*)=>{$(n.$set(v[$name].as_i64().unwrap() as i32);)*};}
    number!(set_currency_decimal_digits:"CurrencyDecimalDigits",set_currency_negative_pattern:"CurrencyNegativePattern",set_currency_positive_pattern:"CurrencyPositivePattern",set_number_decimal_digits:"NumberDecimalDigits",set_number_negative_pattern:"NumberNegativePattern",set_percent_decimal_digits:"PercentDecimalDigits",set_percent_negative_pattern:"PercentNegativePattern",set_percent_positive_pattern:"PercentPositivePattern");
    for (key, set) in [
        (
            "CurrencyGroupSizes",
            NumberFormatInfo::set_currency_group_sizes as fn(&mut NumberFormatInfo, &[i32]),
        ),
        ("NumberGroupSizes", NumberFormatInfo::set_number_group_sizes),
        (
            "PercentGroupSizes",
            NumberFormatInfo::set_percent_group_sizes,
        ),
    ] {
        set(
            &mut n,
            &v[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap() as i32)
                .collect::<Vec<_>>(),
        );
    }
    n
}
fn mount(c: &RibbonSpinner) -> (Ref<Ribbon>, Ref<Window>) {
    let v = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Spinner.xaml")
        .unwrap()
        .unwrap();
    let d = ValueTypes::as_object(&*v)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    d.add_merged_dictionary(
        crate::theming::RibbonThemeResources::new().upcast::<ResourceDictionary>(),
    );
    let t = d
        .try_get_resource(&RibbonSpinner::TYPE.into(), None)
        .flatten()
        .unwrap();
    c.set_theme(
        ValueTypes::as_object(&*t)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap(),
    );
    c.set_resources(d);
    let r = Ribbon::new();
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    let t = RibbonTab::new();
    let g = RibbonGroup::new();
    g.items().add(Some(c.to_ref().upcast()));
    t.groups().add(Some(g));
    r.tabs().add(Some(t));
    let w = crate::ribbon_tests::mount(&r);
    Dispatcher::ui_thread().run_jobs(None);
    (r, w)
}
fn editor(c: &RibbonSpinner) -> Ref<TextBox> {
    c.text_box.borrow().clone().unwrap()
}
fn key(c: &RibbonSpinner, k: Key) -> bool {
    let e = KeyEventArgs::new();
    let mut e = e;
    e.key = k;
    e.set_routed_event(Some(InputElement::key_down_event()));
    editor(c).raise_event(&e);
    e.handled()
}
fn log_event(slot: &RibbonSpinner, log: &Rc<RefCell<Vec<String>>>, commit: bool) {
    let out = log.clone();
    let h = Some(MarkupDelegate::new(move |args| {
        let e = args[1]
            .as_ref()
            .unwrap()
            .downcast_ref::<Rc<RibbonValueChangedEventArgs>>()
            .unwrap();
        out.borrow_mut().push(format!(
            "{}:{}>{}",
            if commit { "committed" } else { "changed" },
            bits(e.old_value()),
            bits(e.new_value())
        ));
        None
    }));
    if commit {
        slot.add_value_committed(h)
    } else {
        slot.add_value_changed(h)
    }
}
fn command(c: &RibbonSpinner, r: &Ribbon, log: &Rc<RefCell<Vec<String>>>) {
    let out = log.clone();
    c.set_command(Some(crate::commands::adapt_core_command(
        ferroribbon_core::commands::RibbonRelayCommand::new_with_parameter(
            move |p| {
                let value = *p.unwrap().downcast_ref::<f64>().unwrap();
                out.borrow_mut().push(format!(
                    "execute:{}",
                    format_arg_with(
                        &FormatArg::F64(value),
                        "",
                        &CultureInfo::invariant_culture()
                    )
                    .unwrap()
                ));
            },
            None,
        ),
    )));
    let out = log.clone();
    r.add_item_invoked(Some(MarkupDelegate::new(move |args| {
        let e = args[1]
            .as_ref()
            .unwrap()
            .downcast_ref::<Rc<crate::controls::RibbonItemInvokedEventArgs>>()
            .unwrap();
        let v = *e.parameter.as_ref().unwrap().downcast_ref::<f64>().unwrap();
        out.borrow_mut().push(format!(
            "invoke:{}",
            format_arg_with(&FormatArg::F64(v), "", &CultureInfo::invariant_culture()).unwrap()
        ));
        None
    })));
}

#[test]
fn original_spinner_format_value_matches_all_culture_double_cases() {
    crate::group_tests::run(|| {
        struct Restore(CultureInfo);
        impl Drop for Restore {
            fn drop(&mut self) {
                CultureInfo::set_current_formatting(self.0.clone());
            }
        }
        let _restore = Restore(CultureInfo::current_formatting());
        for format in corpus()["corpus"]["formats"].as_array().unwrap() {
            CultureInfo::set_current_formatting(
                CultureInfo::invariant_culture().with_number_format(numeric_info(&format["info"])),
            );
            let c = RibbonSpinner::new();
            for row in format["rows"].as_array().unwrap() {
                let value =
                    f64::from_bits(u64::from_str_radix(row["bits"].as_str().unwrap(), 16).unwrap());
                let f = row["format"].as_str();
                c.set_format(f.map(str::to_string));
                let actual = format_arg_with(
                    &FormatArg::F64(value),
                    f.unwrap_or(""),
                    &CultureInfo::current_formatting(),
                );
                if row["error"].is_null() {
                    assert_eq!(
                        json!(c.format_value(value)),
                        row["text"],
                        "culture {} bits {} format {:?}",
                        format["name"],
                        row["bits"],
                        f
                    );
                } else {
                    assert!(
                        actual.is_err(),
                        "{} {} {:?}",
                        format["name"],
                        row["bits"],
                        f
                    );
                }
            }
        }
    })
}
#[test]
fn source_spinner_units_keep_utf16_length_and_defaults() {
    crate::group_tests::run(|| {
        let c = RibbonSpinner::new();
        assert_eq!(c.input_width(), 64.);
        assert_eq!(c.value(), 0.);
        assert_eq!(c.minimum(), f64::MIN);
        assert_eq!(c.maximum(), f64::MAX);
        assert_eq!(c.increment(), 1.);
        assert_eq!(c.format().as_deref(), Some("0.##"));
        assert!(c.is_scrub_enabled());
        c.set_format(Some("0.0".into()));
        for row in corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["name"] == "units")
        {
            c.set_unit(row["unit"].as_str().map(str::to_string));
            assert_eq!(json!(c.format_value(2.5)), row["text"]);
        }
    })
}
#[test]
fn source_spinner_bound_coercion_crossed_ranges_and_notification_order() {
    crate::group_tests::run(|| {
        let c = RibbonSpinner::new();
        c.set_minimum(0.);
        c.set_maximum(10.);
        c.set_numeric_value(2.);
        let (_, w) = mount(&c);
        let log = Rc::new(RefCell::new(vec![]));
        log_event(&c, &log, false);
        log_event(&c, &log, true);
        c.set_value(RibbonSpinner::value_property(), 25.);
        c.set_maximum(5.);
        c.set_minimum(8.);
        c.set_maximum(20.);
        c.set_numeric_value(f64::NAN);
        let v = case("coercion");
        assert_eq!(json!(bits(c.value())), v["value"]);
        assert_eq!(json!(editor(&c).text()), v["Text"]);
        assert_eq!(json!(*log.borrow()), v["trace"]);
        w.close();
    })
}
#[test]
fn source_spinner_commit_parsing_units_invalid_boundary_and_command_order() {
    crate::group_tests::run(|| {
        for row in corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["name"] == "parse")
        {
            let c = RibbonSpinner::new();
            c.set_minimum(0.);
            c.set_maximum(10.);
            c.set_numeric_value(2.5);
            c.set_unit(Some("cm".into()));
            let (r, w) = mount(&c);
            let log = Rc::new(RefCell::new(vec![]));
            command(&c, &r, &log);
            log_event(&c, &log, true);
            editor(&c).set_text(row["input"].as_str());
            c.commit_text();
            assert_eq!(json!(bits(c.value())), row["value"], "{}", row["input"]);
            assert_eq!(json!(editor(&c).text()), row["Text"]);
            assert_eq!(json!(*log.borrow()), row["trace"]);
            w.close();
        }
    })
}
#[test]
fn source_spinner_keyboard_repeat_button_wheel_zero_and_retemplate_detach() {
    crate::group_tests::run(|| {
        let c = RibbonSpinner::new();
        c.set_minimum(0.);
        c.set_maximum(10.);
        c.set_numeric_value(4.);
        let (r, w) = mount(&c);
        let log = Rc::new(RefCell::new(vec![]));
        command(&c, &r, &log);
        log_event(&c, &log, true);
        let v = case("keyboard");
        for (k, row) in [Key::Up, Key::Down, Key::PageUp, Key::PageDown, Key::Escape]
            .into_iter()
            .zip(v["rows"].as_array().unwrap())
        {
            assert_eq!(key(&c, k), row["Handled"].as_bool().unwrap());
            assert_eq!(json!(bits(c.value())), row["value"]);
            assert_eq!(json!(editor(&c).text()), row["Text"]);
        }
        let e = PointerWheelEventArgs::new(
            editor(&c),
            Pointer::new(41, PointerType::Mouse, true),
            &w,
            Point::new(50., 20.),
            0,
            PointerPointProperties::NONE,
            KeyModifiers::NONE,
            ferroui_base::Vector::new(0., 0.),
        );
        editor(&c).raise_event(&e);
        assert!(e.handled());
        assert_eq!(json!(*log.borrow()), v["trace"]);
        assert_eq!(json!(bits(c.value())), v["value"]);
        let up = c
            .get_visual_descendants()
            .find_map(|p| {
                p.cast::<RepeatButton>()
                    .filter(|p| p.name().as_deref() == Some("PART_UpButton"))
            })
            .unwrap();
        let old = editor(&c);
        let template = ferroui_controls::templates::FuncControlTemplate::new(|_, _| {
            ferroui_controls::Border::new().upcast()
        });
        c.set_template(Some(template));
        c.apply_template();
        let before = c.value();
        let mut e = KeyEventArgs::new();
        e.key = Key::Up;
        e.set_routed_event(Some(InputElement::key_down_event()));
        old.raise_event(&e);
        let e = ferroui_base::interactivity::RoutedEventArgs::with_event(Button::click_event());
        up.raise_event(&e);
        assert_eq!(c.value(), before);
        w.close();
    })
}
#[test]
fn original_runtime_spinner_copy_executes_source_only_on_user_change() {
    crate::group_tests::run(|| {
        let c = RibbonSpinner::new();
        c.set_minimum(0.);
        c.set_maximum(10.);
        c.set_is_scrub_enabled(false);
        let (r, w) = mount(&c);
        let log = Rc::new(RefCell::new(vec![]));
        command(&c, &r, &log);
        let copy = c
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonSpinner>()
            .unwrap();
        c.set_numeric_value(4.);
        assert!(log.borrow().is_empty());
        assert_eq!(copy.value(), 4.);
        copy.step(1.);
        let v = case("copy");
        assert_eq!(json!(*log.borrow()), v["trace"]);
        assert_eq!(json!(bits(c.value())), v["value"]);
        assert_eq!(json!(bits(copy.value())), v["copyValue"]);
        assert_eq!(copy.is_scrub_enabled(), v["scrub"].as_bool().unwrap());
        RibbonItemHelper::unlink(&copy);
        c.set_numeric_value(9.);
        assert_eq!(copy.value(), 5.);
        w.close();
    })
}
#[test]
fn source_spinner_scrub_bankers_rounding_capture_and_release_once() {
    crate::group_tests::run(|| {
        let c = RibbonSpinner::new();
        c.set_minimum(0.);
        c.set_maximum(100.);
        c.set_numeric_value(20.);
        let (r, w) = mount(&c);
        let log = Rc::new(RefCell::new(vec![]));
        command(&c, &r, &log);
        let h = c.scrub_handle.borrow().clone().unwrap();
        let pointer: Rc<dyn IPointer> = Pointer::new(42, PointerType::Mouse, true);
        let press = PointerPressedEventArgs::new(
            h.clone(),
            pointer.clone(),
            &w,
            Point::new(0.25, 20.),
            0,
            PointerPointProperties::NONE,
            KeyModifiers::NONE,
            1,
        );
        h.raise_event(&press);
        assert!(press.handled());
        assert!(pointer.captured().is_some_and(|p| p.ptr_eq(&h)));
        let v = case("scrub");
        for row in v["rows"].as_array().unwrap() {
            let e = PointerEventArgs::new(
                Some(InputElement::pointer_moved_event()),
                h.clone(),
                pointer.clone(),
                Some(&w),
                Point::new(row["x"].as_f64().unwrap(), 20.),
                0,
                PointerPointProperties::NONE,
                KeyModifiers::NONE,
            );
            h.raise_event(&e);
            assert_eq!(json!(bits(c.value())), row["value"]);
        }
        let released = PointerReleasedEventArgs::new(
            h.clone(),
            pointer.clone(),
            &w,
            Point::new(10., 20.),
            0,
            PointerPointProperties::NONE,
            KeyModifiers::NONE,
            MouseButton::Left,
        );
        h.raise_event(&released);
        assert!(pointer.captured().is_none());
        assert_eq!(json!(*log.borrow()), v["trace"]);
        h.raise_event(&released);
        assert_eq!(json!(*log.borrow()), v["trace"]);
        w.close();
    })
}

#[test]
fn source_spinner_focused_editing_escapes_and_real_format_errors() {
    crate::group_tests::run(|| {
        let c = RibbonSpinner::new();
        c.set_numeric_value(2.);
        let (_, w) = mount(&c);
        let p = editor(&c);
        p.focus();
        p.set_text(Some("editing"));
        c.set_numeric_value(5.);
        assert_eq!(json!(p.text()), case("focus_detach")["editing"]);
        assert!(!key(&c, Key::Escape));
        assert_eq!(p.text().as_deref(), Some("5"));
        w.close();
        let c = RibbonSpinner::new();
        c.set_format(Some("D2".into()));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.format_value(2.))).is_err()
        );
        // Source nonfinite values bypass even invalid format parsing.
        assert_eq!(
            c.format_value(f64::INFINITY),
            CultureInfo::current_formatting()
                .number_format()
                .positive_infinity_symbol()
        );
    });
}

#[test]
fn source_spinner_copy_forwards_same_args_and_retains_source_after_unlink() {
    crate::group_tests::run(|| {
        let source = RibbonSpinner::new();
        let weak = source.downgrade();
        let source_args = Rc::new(RefCell::new(None::<Rc<RibbonValueChangedEventArgs>>));
        let out = source_args.clone();
        source.add_value_committed(Some(MarkupDelegate::new(move |args| {
            out.replace(Some(
                args[1]
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<Rc<RibbonValueChangedEventArgs>>()
                    .unwrap()
                    .clone(),
            ));
            None
        })));
        let copy = source
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonSpinner>()
            .unwrap();
        let out = source_args.clone();
        copy.add_value_committed(Some(MarkupDelegate::new(move |args| {
            let e = args[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<RibbonValueChangedEventArgs>>()
                .unwrap();
            assert!(Rc::ptr_eq(e, out.borrow().as_ref().unwrap()));
            None
        })));
        copy.step(1.);
        assert_eq!(source.value(), 1.);
        RibbonItemHelper::unlink(&copy);
        drop(source);
        assert!(weak.upgrade().is_some());
        copy.step(1.);
        assert_eq!(weak.upgrade().unwrap().value(), 1.);
        assert_eq!(source_args.borrow().as_ref().unwrap().new_value(), 2.);
        drop(copy);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn source_spinner_nonfinite_ranges_and_signed_zero_notification_boundaries() {
    crate::group_tests::run(|| {
        let c = RibbonSpinner::new();
        let log = Rc::new(RefCell::new(vec![]));
        let out = log.clone();
        c.add_value_changed(Some(MarkupDelegate::new(move |args| {
            let e = args[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<RibbonValueChangedEventArgs>>()
                .unwrap();
            out.borrow_mut()
                .push(format!("{}>{}", bits(e.old_value()), bits(e.new_value())));
            None
        })));
        for row in case("nonfinite_ranges")["rows"].as_array().unwrap() {
            let value =
                f64::from_bits(u64::from_str_radix(row["input"].as_str().unwrap(), 16).unwrap());
            match row["property"].as_str().unwrap() {
                "Minimum" => c.set_minimum(value),
                "Maximum" => c.set_maximum(value),
                "Value" => c.set_numeric_value(value),
                _ => panic!("unknown source operation"),
            };
            assert_eq!(json!(bits(c.value())), row["value"]);
            assert_eq!(json!(*log.borrow()), row["events"]);
        }
    });
}

#[test]
fn source_spinner_unit_suffix_uses_ordinal_utf16_comparison() {
    crate::group_tests::run(|| {
        for row in corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["name"] == "unicode_parse")
        {
            let c = RibbonSpinner::new();
            c.set_unit(row["unit"].as_str().map(str::to_string));
            let (_, w) = mount(&c);
            editor(&c).set_text(row["input"].as_str());
            c.commit_text();
            assert_eq!(json!(bits(c.value())), row["value"]);
            assert_eq!(json!(editor(&c).text()), row["Text"]);
            w.close();
        }
    });
}

#[test]
fn source_spinner_repeat_defaults_and_press_release_invoke_exactly_once() {
    crate::group_tests::run(|| {
        let c = RibbonSpinner::new();
        c.set_numeric_value(2.);
        let (r, w) = mount(&c);
        let log = Rc::new(RefCell::new(vec![]));
        command(&c, &r, &log);
        let up = c.up.borrow().clone().unwrap();
        assert_eq!(up.delay(), 250);
        assert_eq!(up.interval(), 250);
        assert_eq!(up.click_mode(), ferroui_controls::ClickMode::Press);
        let position = up.translate_point(Point::new(1., 1.), &w).unwrap();
        let pointer: Rc<dyn IPointer> = Pointer::new(48, PointerType::Mouse, true);
        let press = PointerPressedEventArgs::new(
            up.clone(),
            pointer.clone(),
            &w,
            position,
            0,
            PointerPointProperties {
                is_left_button_pressed: true,
                ..PointerPointProperties::NONE
            },
            KeyModifiers::NONE,
            1,
        );
        up.raise_event(&press);
        assert_eq!(c.value(), 3.);
        assert_eq!(*log.borrow(), vec!["execute:3", "invoke:3"]);
        let release = PointerReleasedEventArgs::new(
            up.clone(),
            pointer,
            &w,
            position,
            1,
            PointerPointProperties::NONE,
            KeyModifiers::NONE,
            MouseButton::Left,
        );
        up.raise_event(&release);
        assert_eq!(*log.borrow(), vec!["execute:3", "invoke:3"]);
        w.close();
    });
}

fn replace_editor(c: &RibbonSpinner) -> Ref<TextBox> {
    c.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
        |_, scope| {
            let p = TextBox::new();
            scope.register("PART_TextBox", p.clone().upcast());
            p.upcast()
        },
    )));
    c.apply_template();
    editor(c)
}

#[test]
fn source_spinner_commit_rereads_current_part_after_reentrant_callbacks() {
    crate::group_tests::run(|| {
        for row in corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["name"] == "commit_retemplate")
        {
            let stage = row["stage"].as_str().unwrap();
            let c = RibbonSpinner::new();
            c.set_numeric_value(2.);
            c.set_format(Some("0.0".into()));
            let (_, w) = mount(&c);
            let old = editor(&c);
            let calls = Rc::new(Cell::new(0));
            let commits = Rc::new(Cell::new(0));
            let weak = c.downgrade();
            let replace: Rc<dyn Fn()> = Rc::new(move || {
                replace_editor(&weak.upgrade().unwrap()).set_text(Some("pending"));
            });
            if stage == "changed" {
                let replace = replace.clone();
                c.add_value_changed(Some(MarkupDelegate::new(move |_| {
                    replace();
                    None
                })));
            }
            let count = calls.clone();
            let action = (stage == "command").then(|| replace.clone());
            c.set_command(Some(crate::commands::adapt_core_command(
                ferroribbon_core::commands::RibbonRelayCommand::new(move || {
                    count.set(count.get() + 1);
                    if let Some(action) = &action {
                        action();
                    }
                }),
            )));
            let count = commits.clone();
            let action = (stage == "committed").then(|| replace.clone());
            c.add_value_committed(Some(MarkupDelegate::new(move |_| {
                count.set(count.get() + 1);
                if let Some(action) = &action {
                    action();
                }
                None
            })));
            old.set_text(Some("7"));
            c.commit_text();
            assert_eq!(json!(old.text()), row["oldText"], "{stage}");
            assert_eq!(json!(editor(&c).text()), row["currentText"], "{stage}");
            assert_eq!(json!(bits(c.value())), row["value"]);
            assert_eq!(json!(calls.get()), row["calls"]);
            assert_eq!(json!(commits.get()), row["commits"]);
            w.close();
        }
        // The original final field dereference throws after command/event delivery
        // when reentry removes the current part. Do not turn that into silent success.
        let c = RibbonSpinner::new();
        c.set_numeric_value(2.);
        let (_, w) = mount(&c);
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let weak = c.downgrade();
        c.set_command(Some(crate::commands::adapt_core_command(
            ferroribbon_core::commands::RibbonRelayCommand::new(move || {
                count.set(count.get() + 1);
                let c = weak.upgrade().unwrap();
                c.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
                    |_, _| ferroui_controls::Border::new().upcast(),
                )));
                c.apply_template();
            }),
        )));
        let commits = Rc::new(Cell::new(0));
        let count = commits.clone();
        c.add_value_committed(Some(MarkupDelegate::new(move |_| {
            count.set(count.get() + 1);
            None
        })));
        editor(&c).set_text(Some("7"));
        let row = case("commit_part_removed");
        assert_eq!(row["error"], "NullReferenceException");
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.commit_text())).is_err()
        );
        assert_eq!(json!(bits(c.value())), row["value"]);
        assert_eq!(json!(calls.get()), row["calls"]);
        assert_eq!(json!(commits.get()), row["commits"]);
        w.close();
    });
}

#[test]
fn source_spinner_keytip_selects_current_part_after_focus_retemplates() {
    crate::group_tests::run(|| {
        let c = RibbonSpinner::new();
        c.set_numeric_value(2.);
        let (_, w) = mount(&c);
        let old = editor(&c);
        let weak = c.downgrade();
        let subscription =
            old.add_disposable_handler(InputElement::got_focus_event(), move |_, _| {
                replace_editor(&weak.upgrade().unwrap()).set_text(Some("replacement"));
            });
        c.on_key_tip_core();
        let row = case("keytip_retemplate");
        assert_eq!(old.selection_start(), old.selection_end());
        assert_eq!(row["oldSelections"], 0);
        let current = editor(&c);
        assert!(!current.ptr_eq(&old));
        assert_eq!(current.selection_start(), 0);
        assert_eq!(current.selected_text(), "replacement");
        assert_eq!(row["currentSelections"], 1);
        subscription.dispose();
        w.close();
    });
}
