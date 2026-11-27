use super::*;
use crate::controls::{Ribbon, RibbonGroup, RibbonTab};
use ferroui_base::{
    controls::ResourceDictionary,
    data::core::ValueTypes,
    input::{InputElement, Key, KeyEventArgs, KeyModifiers},
    metadata::MarkupDelegate,
    styling::ControlTheme,
    threading::Dispatcher,
};
use ferroui_controls::Window;
use serde_json::{Value, json};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/slider.json")).unwrap()
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
fn mount(c: &RibbonSlider) -> (Ref<Ribbon>, Ref<Window>) {
    let value = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Slider.xaml")
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
        .try_get_resource(&RibbonSlider::TYPE.into(), None)
        .flatten()
        .unwrap();
    c.set_resources(d);
    c.set_theme(
        ValueTypes::as_object(&*theme)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap(),
    );
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
fn part(c: &RibbonSlider) -> Ref<RibbonInputSlider> {
    c.current_part().cast::<RibbonInputSlider>().unwrap()
}
fn event_log(c: &RibbonSlider, log: &Rc<RefCell<Vec<String>>>, committed: bool) {
    let out = log.clone();
    let h = Some(MarkupDelegate::new(move |args| {
        let v = *args[1].as_ref().unwrap().downcast_ref::<f64>().unwrap();
        out.borrow_mut().push(format!(
            "{}:{}",
            if committed { "committed" } else { "changed" },
            bits(v)
        ));
        None
    }));
    if committed {
        c.add_value_committed(h);
    } else {
        c.add_value_changed(h);
    }
}
fn command(c: &RibbonSlider, r: &Ribbon, log: &Rc<RefCell<Vec<String>>>) {
    let out = log.clone();
    c.set_command(Some(crate::commands::adapt_core_command(
        ferroribbon_core::commands::RibbonRelayCommand::new_with_parameter(
            move |p| {
                out.borrow_mut().push(format!(
                    "execute:{}",
                    p.unwrap().downcast_ref::<f64>().unwrap()
                ))
            },
            None,
        ),
    )));
    let out = log.clone();
    r.add_item_invoked(Some(MarkupDelegate::new(move |args| {
        let a = args[1]
            .as_ref()
            .unwrap()
            .downcast_ref::<Rc<crate::controls::RibbonItemInvokedEventArgs>>()
            .unwrap();
        out.borrow_mut().push(format!(
            "invoke:{}",
            a.parameter.as_ref().unwrap().downcast_ref::<f64>().unwrap()
        ));
        None
    })));
}
fn snapshot(c: &RibbonSlider, log: &Rc<RefCell<Vec<String>>>) -> Value {
    json!({"value":bits(c.value()),"part":bits(part(c).value()),"trace":*log.borrow()})
}
#[test]
fn original_runtime_slider_programmatic_changes_do_not_execute_user_command() {
    crate::group_tests::run(|| {
        let v = case("programmatic_and_user");
        let c = RibbonSlider::new();
        c.set_numeric_value(30.);
        let log = Rc::new(RefCell::new(vec![]));
        event_log(&c, &log, false);
        event_log(&c, &log, true);
        assert_eq!(json!(bits(c.value())), v["rows"][0]["value"]);
        assert!(c.slider.borrow().is_none());
        let (r, w) = mount(&c);
        command(&c, &r, &log);
        assert_eq!(snapshot(&c, &log), v["rows"][1]);
        c.set_numeric_value(60.);
        assert_eq!(snapshot(&c, &log), v["rows"][2]);
        c.set_maximum(50.);
        assert_eq!(snapshot(&c, &log), v["rows"][3]);
        part(&c).set_numeric_value(35.);
        assert_eq!(snapshot(&c, &log), v["rows"][4]);
        w.close();
    });
}
#[test]
fn original_slider_outer_value_event_keeps_snapshot_after_nested_coercion() {
    crate::group_tests::run(|| {
        let c = RibbonSlider::new();
        c.set_maximum(10.);
        let (r, w) = mount(&c);
        let log = Rc::new(RefCell::new(vec![]));
        command(&c, &r, &log);
        event_log(&c, &log, false);
        event_log(&c, &log, true);
        c.set_value(RibbonSlider::value_property(), 25.);
        let v = case("bound_outer_snapshot");
        assert_eq!(
            snapshot(&c, &log),
            json!({"value":v["value"],"part":v["part"],"trace":v["trace"]})
        );
        w.close();
    });
}
#[test]
fn source_slider_keyboard_small_change_ignores_step_frequency_and_keeps_page_keys_unhandled() {
    crate::group_tests::run(|| {
        for v in corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["name"] == "keyboard")
        {
            let c = RibbonSlider::new();
            c.set_minimum(3.);
            c.set_maximum(13.);
            c.set_numeric_value(3.);
            c.set_step_frequency(2.);
            let (_, w) = mount(&c);
            let p = part(&c);
            p.set_is_direction_reversed(v["reversed"].as_bool().unwrap());
            if v["rtl"].as_bool().unwrap() {
                p.set_flow_direction(ferroui_base::media::FlowDirection::RightToLeft);
            }
            for (k, row) in [
                Key::Right,
                Key::PageUp,
                Key::Home,
                Key::End,
                Key::Left,
                Key::Up,
                Key::Down,
                Key::PageDown,
            ]
            .into_iter()
            .zip(v["rows"].as_array().unwrap())
            {
                let mut e = KeyEventArgs::new();
                e.key = k;
                e.key_modifiers = KeyModifiers::CONTROL;
                e.set_routed_event(Some(InputElement::key_down_event()));
                p.raise_event(&e);
                assert_eq!(json!(e.handled()), row["Handled"], "{row}");
                assert_eq!(json!(bits(p.value())), row["value"], "{row}");
            }
            w.close();
        }
    });
}
#[test]
fn source_slider_user_reentry_commits_original_argument_and_resets_sync_after_error() {
    crate::group_tests::run(|| {
        let c = RibbonSlider::new();
        c.set_numeric_value(2.);
        let (r, w) = mount(&c);
        let log = Rc::new(RefCell::new(vec![]));
        event_log(&c, &log, false);
        let weak = c.downgrade();
        c.add_value_changed(Some(MarkupDelegate::new(move |args| {
            if *args[1].as_ref().unwrap().downcast_ref::<f64>().unwrap() == 5. {
                weak.upgrade().unwrap().set_numeric_value(8.);
            }
            None
        })));
        command(&c, &r, &log);
        event_log(&c, &log, true);
        part(&c).set_numeric_value(5.);
        let v = case("user_reentrant_value");
        assert_eq!(
            snapshot(&c, &log),
            json!({"value":v["value"],"part":v["part"],"trace":v["trace"]})
        );
        w.close();
        let c = RibbonSlider::new();
        let (_, w) = mount(&c);
        let log = Rc::new(RefCell::new(vec![]));
        event_log(&c, &log, false);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| c.set_minimum(f64::NAN)))
                .is_err()
        );
        assert!(!c.pushing.get());
        c.set_minimum(0.);
        c.set_numeric_value(5.);
        let v = case("sync_error_flag_reset");
        assert_eq!(v["error"], "ArgumentException");
        assert_eq!(
            snapshot(&c, &log),
            json!({"value":v["value"],"part":v["part"],"trace":v["trace"]})
        );
        w.close();
    });
}

fn inner_geometry(vertical: bool) -> (Ref<RibbonInputSlider>, Ref<Window>) {
    let p = RibbonInputSlider::new();
    p.set_minimum(3.);
    p.set_maximum(13.);
    p.set_numeric_value(3.);
    p.set_step_frequency(2.);
    p.set_width(120.);
    p.set_height(80.);
    if vertical {
        p.set_orientation(ferroui_base::layout::Orientation::Vertical);
    }
    let w = Window::new();
    w.set_width(120.);
    w.set_height(80.);
    w.set_content(Some(Control::boxed(p.clone())));
    w.show();
    w.measure(ferroui_base::Size::new(120., 80.));
    w.arrange(ferroui_base::Rect::new(0., 0., 120., 80.));
    Dispatcher::ui_thread().run_jobs(None);
    let thumb = p.track().unwrap().thumb().unwrap();
    thumb.set_min_width(0.);
    thumb.set_min_height(0.);
    thumb.set_width(20.);
    thumb.set_height(16.);
    w.measure(ferroui_base::Size::new(120., 80.));
    w.arrange(ferroui_base::Rect::new(0., 0., 120., 80.));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(
        if vertical {
            p.track().unwrap().bounds().height
        } else {
            p.track().unwrap().bounds().width
        },
        if vertical { 80. } else { 120. }
    );
    (p, w)
}
#[test]
fn source_slider_precise_track_points_use_zero_grid_lower_ties_and_intermediate_values() {
    crate::group_tests::run(|| {
        use ferroui_base::{
            Point, Vector,
            input::{Pointer, PointerEventArgs, PointerPointProperties, PointerType},
        };
        for v in corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["name"] == "track_points")
        {
            let (p, w) = inner_geometry(v["vertical"].as_bool().unwrap());
            p.set_is_direction_reversed(v["reversed"].as_bool().unwrap());
            let origin = p
                .track()
                .unwrap()
                .translate_point(Point::default(), &w)
                .unwrap();
            for row in v["rows"].as_array().unwrap() {
                let e = PointerEventArgs::new(
                    Some(InputElement::pointer_moved_event()),
                    p.clone(),
                    Pointer::new(901, PointerType::Mouse, true),
                    Some(&w),
                    origin + Vector::new(row["x"].as_f64().unwrap(), row["y"].as_f64().unwrap()),
                    0,
                    PointerPointProperties::NONE,
                    KeyModifiers::NONE,
                );
                p.move_to_pointer(&e);
                assert_eq!(json!(bits(p.value())), row["value"], "{row}");
                assert_eq!(
                    json!(bits(p.intermediate_value())),
                    row["intermediate"],
                    "{row}"
                );
            }
            w.close();
        }
    });
}
#[test]
fn source_slider_thumb_deltas_retain_unsnapped_accumulator_and_overshoot() {
    crate::group_tests::run(|| {
        use ferroui_base::input::VectorEventArgs;
        for v in corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["name"] == "thumb_deltas")
        {
            let vertical = v["vertical"].as_bool().unwrap();
            let (p, w) = inner_geometry(vertical);
            p.set_is_direction_reversed(v["reversed"].as_bool().unwrap());
            p.set_numeric_value(3.25);
            p.on_thumb_drag_started(&VectorEventArgs::new());
            for row in v["rows"].as_array().unwrap() {
                let delta = row["change"].as_f64().unwrap();
                let mut e = VectorEventArgs::new();
                e.vector = ferroui_base::Vector::new(
                    if vertical { 0. } else { delta },
                    if vertical { delta } else { 0. },
                );
                p.on_thumb_drag_delta(&e);
                assert_eq!(json!(bits(p.value())), row["value"], "{row}");
                assert_eq!(
                    json!(bits(p.intermediate_value())),
                    row["intermediate"],
                    "{row}"
                );
            }
            p.on_thumb_drag_completed(&VectorEventArgs::new());
            assert_eq!(json!(bits(p.intermediate_value())), v["finalIntermediate"]);
            w.close();
        }
    });
}
#[test]
fn source_slider_actual_headless_track_press_captures_container_and_commits_once() {
    crate::group_tests::run(|| {
        use ferroui_base::{
            Point, Vector,
            input::{MouseButton, RawInputModifiers},
        };
        use ferroui_headless::HeadlessWindowExtensions;
        let c = RibbonSlider::new();
        c.set_minimum(3.);
        c.set_maximum(13.);
        c.set_numeric_value(3.);
        c.set_step_frequency(2.);
        let (r, w) = mount(&c);
        let p = part(&c);
        let track = p.track().unwrap();
        let origin = track.translate_point(Point::default(), &w).unwrap();
        let log = Rc::new(RefCell::new(vec![]));
        command(&c, &r, &log);
        event_log(&c, &log, false);
        event_log(&c, &log, true);
        w.mouse_down(
            origin + Vector::new(20.25, 10.),
            MouseButton::Left,
            RawInputModifiers::NONE,
        );
        assert_eq!(p.value(), 4.);
        assert_eq!(c.value(), 4.);
        assert_eq!(track.pointer_captures().len(), 1);
        assert_eq!(
            log.borrow()
                .iter()
                .filter(|x| x.starts_with("execute:"))
                .count(),
            1
        );
        w.mouse_up(
            origin + Vector::new(20.25, 10.),
            MouseButton::Left,
            RawInputModifiers::NONE,
        );
        assert!(track.pointer_captures().is_empty());
        assert_eq!(
            log.borrow()
                .iter()
                .filter(|x| x.starts_with("execute:"))
                .count(),
            1
        );
        assert_eq!(p.intermediate_value(), 4.);
        w.close();
    });
}

#[test]
fn source_slider_range_coercion_retains_raw_values_and_precise_bound_notification_order() {
    crate::group_tests::run(|| {
        for (name, raw) in [
            ("range_retention_and_bound_order", false),
            ("raw_range_nonfinite_and_signed_zero", true),
        ] {
            let p = RibbonInputSlider::new();
            let events = Rc::new(RefCell::new(vec![]));
            let out = events.clone();
            let weak = p.downgrade();
            let sub=p.add_disposable_handler(RangeBase::value_changed_event(),move|_,e|{let p=weak.upgrade().unwrap();out.borrow_mut().push(if raw{json!({"old":bits(e.old_value()),"value":bits(e.new_value())})}else{json!({"old":bits(e.old_value()),"value":bits(e.new_value()),"minimum":bits(p.minimum()),"maximum":bits(p.maximum())})});});
            for row in case(name)["rows"].as_array().unwrap() {
                let value = f64::from_bits(
                    u64::from_str_radix(row["input"].as_str().unwrap(), 16).unwrap(),
                );
                match (row["property"].as_str().unwrap(), raw) {
                    ("Value", true) => p.set_value(RangeBase::value_property(), value),
                    ("Minimum", true) => p.set_value(RangeBase::minimum_property(), value),
                    ("Maximum", true) => p.set_value(RangeBase::maximum_property(), value),
                    ("Value", false) => p.set_numeric_value(value),
                    ("Minimum", false) => p.set_minimum(value),
                    ("Maximum", false) => p.set_maximum(value),
                    _ => unreachable!(),
                }
                assert_eq!(json!(bits(p.value())), row["value"], "{name}: {row}");
                assert_eq!(json!(bits(p.minimum())), row["minimum"], "{name}: {row}");
                assert_eq!(json!(bits(p.maximum())), row["maximum"], "{name}: {row}");
                assert_eq!(json!(*events.borrow()), row["events"], "{name}: {row}");
            }
            sub.dispose();
        }
    });
}

#[test]
fn source_slider_copy_bidirectional_values_scalar_forwarding_unlink_and_lifetime() {
    crate::group_tests::run(|| {
        let source = RibbonSlider::new();
        source.set_numeric_value(2.);
        source.set_maximum(10.);
        source.set_input_width(155.);
        let weak = source.downgrade();
        let calls = Rc::new(RefCell::new(vec![]));
        let out = calls.clone();
        let target = weak.clone();
        source.set_command(Some(crate::commands::adapt_core_command(
            ferroribbon_core::commands::RibbonRelayCommand::new_with_parameter(
                move |p| {
                    let v = *p.unwrap().downcast_ref::<f64>().unwrap();
                    out.borrow_mut().push(v);
                    if v == 5. {
                        target.upgrade().unwrap().set_numeric_value(9.);
                    }
                },
                None,
            ),
        )));
        let committed = Rc::new(RefCell::new(vec![]));
        let out = committed.clone();
        let target = weak.clone();
        source.add_value_committed(Some(MarkupDelegate::new(move |args| {
            let v = *args[1].as_ref().unwrap().downcast_ref::<f64>().unwrap();
            out.borrow_mut()
                .push((v, target.upgrade().unwrap().value()));
            None
        })));
        let copy = source
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonSlider>()
            .unwrap();
        let (root, w) = mount(&copy);
        assert!(
            copy.command().is_none(),
            "source links do not copy the command"
        );
        assert_eq!(copy.input_width(), 155.);
        source.set_input_width(170.);
        assert_eq!(copy.input_width(), 170.);
        source.set_numeric_value(3.);
        assert_eq!(copy.value(), 3.);
        copy.set_numeric_value(4.);
        assert_eq!(source.value(), 4.);
        assert!(calls.borrow().is_empty());
        part(&copy).set_numeric_value(5.);
        assert_eq!(*calls.borrow(), vec![5.]);
        assert_eq!(
            *committed.borrow(),
            vec![(5., 9.)],
            "copy forwards incoming scalar even after command changes source Value"
        );
        assert_eq!(source.value(), 9.);
        assert_eq!(copy.value(), 9.);
        RibbonItemHelper::unlink(&copy);
        copy.set_numeric_value(6.);
        assert_eq!(source.value(), 9.);
        source.set_input_width(180.);
        assert_eq!(copy.input_width(), 170.);
        drop(source);
        assert!(weak.upgrade().is_some());
        part(&copy).set_numeric_value(7.);
        assert_eq!(*calls.borrow(), vec![5., 7.]);
        assert_eq!(*committed.borrow(), vec![(5., 9.), (7., 9.)]);
        w.close();
        w.set_content(None);
        drop(w);
        drop(root);
        drop(copy);
        assert!(
            weak.upgrade().is_some(),
            "source unload callback temporarily retains the root/copy graph"
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            weak.upgrade().is_none(),
            "copy-to-source callback is strong and acyclic"
        );
    });
}

#[test]
fn source_slider_raw_thumb_fractional_moves_keep_source_drag_sequence_and_release_capture() {
    crate::group_tests::run(|| {
        use ferroui_base::{
            Point, Vector,
            input::{MouseButton, RawInputModifiers},
        };
        use ferroui_headless::HeadlessWindowExtensions;
        for case in corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["name"] == "thumb_deltas")
        {
            let vertical = case["vertical"].as_bool().unwrap();
            let (p, w) = inner_geometry(vertical);
            p.set_is_direction_reversed(case["reversed"].as_bool().unwrap());
            p.set_numeric_value(3.25);
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            let thumb = p.track().unwrap().thumb().unwrap();
            let origin = thumb
                .translate_point(
                    Point::new(thumb.bounds().width / 2., thumb.bounds().height / 2.),
                    &w,
                )
                .unwrap();
            w.mouse_down(origin, MouseButton::Left, RawInputModifiers::NONE);
            assert_eq!(
                p.value(),
                3.25,
                "press must start thumb drag without a track jump"
            );
            assert_eq!(thumb.pointer_captures().len(), 1);
            let mut position = origin;
            for row in case["rows"].as_array().unwrap() {
                let delta = row["change"].as_f64().unwrap();
                position += Vector::new(
                    if vertical { 0. } else { delta },
                    if vertical { delta } else { 0. },
                );
                w.mouse_move(position, RawInputModifiers::NONE);
                w.update_layout();
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(json!(bits(p.value())), row["value"], "{case}, {row}");
                assert_eq!(
                    json!(bits(p.intermediate_value())),
                    row["intermediate"],
                    "{case}, {row}"
                );
            }
            w.mouse_up(position, MouseButton::Left, RawInputModifiers::NONE);
            assert!(thumb.pointer_captures().is_empty());
            assert_eq!(
                json!(bits(p.intermediate_value())),
                case["finalIntermediate"]
            );
            w.close();
        }
    });
}

#[test]
fn source_slider_right_button_on_thumb_does_not_start_drag_or_change_value() {
    crate::group_tests::run(|| {
        use ferroui_base::{
            Point, Vector,
            input::{MouseButton, RawInputModifiers},
        };
        use ferroui_headless::HeadlessWindowExtensions;
        let (p, w) = inner_geometry(false);
        p.set_numeric_value(3.25);
        w.update_layout();
        let thumb = p.track().unwrap().thumb().unwrap();
        let point = thumb.translate_point(Point::new(10., 8.), &w).unwrap();
        w.mouse_down(point, MouseButton::Right, RawInputModifiers::NONE);
        w.mouse_move(point + Vector::new(20., 0.), RawInputModifiers::NONE);
        assert_eq!(
            p.value(),
            3.25,
            "pinned Uno Thumb only starts while IsLeftButtonPressed"
        );
        w.mouse_up(
            point + Vector::new(20., 0.),
            MouseButton::Right,
            RawInputModifiers::NONE,
        );
        w.close();
    });
}

#[test]
fn source_thumb_keeps_press_coordinate_transform_when_track_transform_changes() {
    crate::group_tests::run(|| {
        use ferroui_base::{
            Point, Vector,
            input::{MouseButton, RawInputModifiers},
            media::ScaleTransform,
        };
        use ferroui_headless::HeadlessWindowExtensions;
        let (p, w) = inner_geometry(false);
        p.set_numeric_value(3.25);
        w.update_layout();
        let track = p.track().unwrap();
        let thumb = track.thumb().unwrap();
        let point = thumb.translate_point(Point::new(10., 8.), &w).unwrap();
        w.mouse_down(point, MouseButton::Left, RawInputModifiers::NONE);
        track.set_render_transform(Some(ScaleTransform::with_scale(2., 2.).into()));
        w.update_layout();
        w.mouse_move(point + Vector::new(0.25, 0.), RawInputModifiers::NONE);
        let expected = corpus()["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| {
                v["name"] == "thumb_deltas" && v["vertical"] == false && v["reversed"] == false
            })
            .unwrap()["rows"][0]
            .clone();
        assert_eq!(json!(bits(p.value())), expected["value"]);
        assert_eq!(
            json!(bits(p.intermediate_value())),
            expected["intermediate"],
            "original Thumb freezes parent-to-root inverse at press"
        );
        w.mouse_up(
            point + Vector::new(0.25, 0.),
            MouseButton::Left,
            RawInputModifiers::NONE,
        );
        w.close();
    });
}

#[test]
fn source_thumb_drag_start_reads_value_after_focus_reentry() {
    crate::group_tests::run(|| {
        use ferroui_base::input::{NavigationMethod, VectorEventArgs};
        let (p, w) = inner_geometry(false);
        p.set_numeric_value(3.25);
        w.focus_manager()
            .focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
        assert!(!p.is_focused());
        let count = Rc::new(Cell::new(0));
        let out = count.clone();
        let weak = p.downgrade();
        let subscription =
            p.add_disposable_handler(InputElement::got_focus_event(), move |_, _| {
                out.set(out.get() + 1);
                weak.upgrade().unwrap().set_numeric_value(8.);
            });
        p.on_thumb_drag_started(&VectorEventArgs::new());
        let mut e = VectorEventArgs::new();
        e.vector = ferroui_base::Vector::new(0.25, 0.);
        p.on_thumb_drag_delta(&e);
        let expected = case("thumb_start_focus_reentry");
        assert_eq!(json!(count.get()), expected["focuses"]);
        assert_eq!(json!(bits(p.value())), expected["value"]);
        assert_eq!(
            json!(bits(p.intermediate_value())),
            expected["intermediate"]
        );
        subscription.dispose();
        w.close();
    });
}
