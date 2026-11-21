use crate::{
    primitives::{IRibbonScrollContent, RibbonScrollPanel},
    tests::run,
};
use ferroui_base::input::{
    InputElementImpl, KeyModifiers, KeyboardNavigation, Pointer, PointerPointProperties,
    PointerType, PointerWheelEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::RectangleGeometry;
use ferroui_base::{
    FerroObject, FerroObjectImpl, Point, Rect, Ref, Size, StyledElementImpl, VisualImpl,
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate,
};
use ferroui_controls::{Button, Control, ControlImpl};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[repr(C)]
struct FixedContent {
    base: Control,
    natural: Cell<f64>,
    extent: Cell<f64>,
    measures: RefCell<Vec<Size>>,
    on_extent: RefCell<Option<Rc<dyn Fn()>>>,
}
ferro_class!(FixedContent:Control);
ferro_impl_classes!(FixedContent:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(FixedContent {interfaces:[Rc<dyn IRibbonScrollContent> => |value| Rc::new(ScrollContent(value)) as Rc<dyn IRibbonScrollContent>]});
impl FixedContent {
    fn new(natural: f64, extent: f64) -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            natural: Cell::new(natural),
            extent: Cell::new(extent),
            measures: RefCell::new(Vec::new()),
            on_extent: RefCell::new(None),
        })
    }
}
impl LayoutableImpl for FixedContent {
    fn measure_override(this: &Self, size: Size) -> Size {
        this.measures.borrow_mut().push(size);
        Size::new(this.natural.get().min(size.width), 17.25)
    }
}
struct ScrollContent(Ref<FixedContent>);
impl IRibbonScrollContent for ScrollContent {
    fn extent_width(&self) -> f64 {
        let callback = self.0.on_extent.borrow().clone();
        if let Some(callback) = callback {
            callback()
        }
        self.0.extent.get()
    }
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
}
fn arrange(panel: &RibbonScrollPanel, width: f64, height: f64) {
    panel.arrange(Rect::new(0., 0., width, height));
}
fn rect(rect: Rect) -> Value {
    json!([rect.x, rect.y, rect.width, rect.height])
}
fn same(actual: Value, expected: &Value) {
    match (&actual, expected) {
        (Value::Number(a), Value::Number(b)) => assert_eq!(a.as_f64(), b.as_f64()),
        (Value::Array(a), Value::Array(b)) => {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                same(a.clone(), b)
            }
        }
        _ => assert_eq!(&actual, expected),
    }
}
fn compare(panel: &RibbonScrollPanel, expected: &Value) {
    same(json!(panel.offset()), &expected["offset"]);
    assert_eq!(json!(panel.has_overflow()), expected["overflow"]);
    same(
        json!([panel.desired_size().width, panel.desired_size().height]),
        &expected["desired"],
    );
    same(
        panel
            .content()
            .map(|c| rect(c.bounds()))
            .unwrap_or(Value::Null),
        &expected["content"],
    );
    let buttons = panel
        .children()
        .snapshot()
        .iter()
        .filter_map(|c| c.cast::<Button>())
        .collect::<Vec<_>>();
    for (button, row) in buttons.iter().zip(expected["buttons"].as_array().unwrap()) {
        assert_eq!(json!(button.is_visible()), row["visible"]);
        assert_eq!(
            json!(KeyboardNavigation::get_is_tab_stop(button)),
            row["tab"]
        );
        // Native collapsed controls have no arranged bounds. WinUI adapter records the requested rect.
        if button.is_visible() {
            same(rect(button.bounds()), &row["bounds"]);
        }
    }
    same(
        rect(
            panel
                .clip()
                .unwrap()
                .cast::<RectangleGeometry>()
                .unwrap()
                .rect(),
        ),
        &expected["clip"],
    );
}
fn wheel(panel: &Ref<RibbonScrollPanel>, delta: f64) -> bool {
    let args = PointerWheelEventArgs::new(
        panel.clone(),
        Pointer::new(0, PointerType::Mouse, true),
        panel,
        Point::new(0., 0.),
        0,
        PointerPointProperties::default(),
        KeyModifiers::NONE,
        ferroui_base::Vector::new(0., delta),
    );
    panel.raise_event(&args);
    args.handled()
}
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/scroll-panel.json")).unwrap()
}
const WHEEL_UNITS: f64 = if cfg!(target_os = "macos") { 50. } else { 120. };
#[test]
fn original_win32_wheel_protocol_corpus_reaches_actual_native_handler() {
    run(|| {
        use ferroui_base::Vector;
        use ferroui_base::input::PlatformWheelDelta;
        use ferroui_base::media::FlowDirection;
        let corpus: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/win32-wheel.json")).unwrap();
        let exact = |input: &Value, name: &str| {
            f64::from_bits(u64::from_str_radix(input[name].as_str().unwrap(), 16).unwrap())
        };
        for direction in [FlowDirection::LeftToRight, FlowDirection::RightToLeft] {
            for case in corpus["corpus"].as_array().unwrap() {
                let panel = RibbonScrollPanel::new();
                panel.set_use_layout_rounding(false);
                panel.set_flow_direction(direction);
                panel.set_measure_infinite(true);
                let width = if case["overflow"].as_bool().unwrap() {
                    1000.
                } else {
                    200.
                };
                let content = FixedContent::new(width, width);
                content.set_use_layout_rounding(false);
                panel.set_content(&content);
                for _ in 0..4 {
                    panel.measure(Size::new(200., 40.));
                    arrange(&panel, 200., 40.);
                    if panel.is_measure_valid() && panel.is_arrange_valid() {
                        break;
                    }
                }
                assert_eq!(panel.has_overflow(), case["overflow"].as_bool().unwrap());
                panel.scroll_by(200.);
                assert_eq!(panel.offset(), case["before"].as_f64().unwrap());
                let input = &case["input"];
                let word = u64::from_str_radix(input["wparam"].as_str().unwrap(), 16).unwrap();
                let raw = f64::from((word >> 16) as u16 as i16);
                assert_eq!(raw, case["scalar"].as_f64().unwrap());
                let delta = if input["horizontal"].as_bool().unwrap() {
                    Vector::new(raw, 0.)
                } else {
                    Vector::new(0., raw)
                };
                let event = PointerWheelEventArgs::new(
                    content.clone(),
                    Pointer::new(0, PointerType::Mouse, true),
                    &panel,
                    Point::new(0., 0.),
                    0,
                    PointerPointProperties::default(),
                    KeyModifiers::NONE,
                    Vector::new(exact(input, "native_x_bits"), exact(input, "native_y_bits")),
                )
                .with_platform_delta(PlatformWheelDelta::Win32 { delta });
                content.raise_event(&event);
                assert_eq!(panel.offset(), case["after"].as_f64().unwrap(), "{case}");
                assert_eq!(
                    event.handled(),
                    case["handled"].as_bool().unwrap(),
                    "{case}"
                );
                assert!(panel.content().unwrap().ptr_eq(&content));
            }
        }
    });
}
#[cfg(target_os = "macos")]
#[test]
fn original_macos_wheel_corpus_preserves_raw_integer_policy_and_routing() {
    run(|| {
        use ferroui_base::input::PlatformWheelDelta;
        use ferroui_base::media::FlowDirection;
        let corpus: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/scroll-wheel.json")).unwrap();
        let exact = |input: &Value, name: &str| {
            f64::from_bits(u64::from_str_radix(input[name].as_str().unwrap(), 16).unwrap())
        };
        for direction in [FlowDirection::LeftToRight, FlowDirection::RightToLeft] {
            for case in corpus["corpus"].as_array().unwrap() {
                let panel = RibbonScrollPanel::new();
                panel.set_use_layout_rounding(false);
                panel.set_flow_direction(direction);
                panel.set_measure_infinite(true);
                let width = if case["overflow"].as_bool().unwrap() {
                    1000.
                } else {
                    200.
                };
                let content = FixedContent::new(width, width);
                content.set_use_layout_rounding(false);
                panel.set_content(&content);
                panel.measure(Size::new(200., 40.));
                arrange(&panel, 200., 40.);
                panel.scroll_by(200.);
                arrange(&panel, 200., 40.);
                assert_eq!(panel.offset(), case["before"].as_f64().unwrap());
                // Arrow visibility can invalidate the native parent during
                // arrange. Finish those layout passes before observing input.
                for _ in 0..4 {
                    if panel.is_arrange_valid() {
                        break;
                    }
                    panel.measure(Size::new(200., 40.));
                    arrange(&panel, 200., 40.);
                }
                assert!(panel.is_arrange_valid(), "{case}");
                let input = &case["input"];
                let args = PointerWheelEventArgs::new(
                    panel.clone(),
                    Pointer::new(0, PointerType::Mouse, true),
                    &panel,
                    Point::new(0., 0.),
                    0,
                    PointerPointProperties::default(),
                    KeyModifiers::NONE,
                    ferroui_base::Vector::new(
                        exact(input, "native_x_bits"),
                        exact(input, "native_y_bits"),
                    ),
                )
                .with_platform_delta(PlatformWheelDelta::MacOS {
                    delta: ferroui_base::Vector::new(
                        exact(input, "raw_x_bits"),
                        exact(input, "raw_y_bits"),
                    ),
                    is_precise: input["precise"].as_bool().unwrap(),
                });
                panel.raise_event(&args);
                assert_eq!(panel.offset(), case["after"].as_f64().unwrap(), "{case}");
                assert_eq!(args.handled(), case["handled"].as_bool().unwrap(), "{case}");
                assert_eq!(
                    !panel.is_arrange_valid(),
                    case["invalidations"].as_u64().unwrap() != 0,
                    "{case}"
                );
                assert!(panel.content().unwrap().ptr_eq(&content));
            }
        }
    });
}
#[test]
fn unchanged_scroll_source_body_boundaries_and_repeated_resize() {
    run(|| {
        for case in corpus()["cases"].as_array().unwrap() {
            let panel = RibbonScrollPanel::new();
            panel.set_use_layout_rounding(false);
            panel.set_measure_infinite(case["infinite"].as_bool().unwrap());
            let content = FixedContent::new(
                200.5,
                if case["adaptive"].as_bool().unwrap() {
                    200.5
                } else {
                    0.
                },
            );
            content.set_use_layout_rounding(false);
            panel.set_content(&content);
            let width = case["width"].as_f64().unwrap();
            panel.measure(Size::new(width, 40.));
            arrange(&panel, width, 40.);
            let rows = case["traces"].as_array().unwrap();
            compare(&panel, &rows[0]);
            for (i, delta) in [0.5, 0.0001, 120., -40., 1000., -1000.]
                .into_iter()
                .enumerate()
            {
                panel.scroll_by(delta);
                arrange(&panel, width, 40.);
                compare(&panel, &rows[i + 1]);
            }
            let button = panel.children().get(2).cast::<Button>().unwrap();
            button.raise_event(&RoutedEventArgs::with_event_and_source(
                Button::click_event(),
                &button,
            ));
            arrange(&panel, width, 40.);
            compare(&panel, &rows[7]);
            assert_eq!(json!(wheel(&panel, 15. / WHEEL_UNITS)), case["handled"]);
            arrange(&panel, width, 40.);
            compare(&panel, &rows[8]);
            arrange(&panel, 400., 40.);
            compare(&panel, &rows[9]);
            // Source adapter always calls Measure; native caches repeated equal constraints.
            let measured = content.measures.borrow()[0];
            let row = &case["measures"][0];
            if row[0].is_string() {
                assert_eq!(measured.width, f64::INFINITY)
            } else {
                assert_eq!(measured.width, row[0].as_f64().unwrap())
            }
            assert_eq!(measured.height, row[1].as_f64().unwrap());
        }
    });
}
#[cfg(target_os = "macos")]
#[test]
fn original_wheel_data_reaches_ribbon_through_native_mouse_device_capture() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::original_wheel_data_reaches_ribbon_through_native_mouse_device_capture"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        use ferroui_base::Vector;
        use ferroui_base::input::raw::RawMouseWheelEventArgs;
        use ferroui_base::input::{
            IInputDevice, IPointer, InputElement, MouseDevice, PlatformWheelDelta,
            RawInputModifiers,
        };
        use ferroui_controls::Window;
        let panel = RibbonScrollPanel::new();
        panel.set_use_layout_rounding(false);
        panel.set_measure_infinite(true);
        let content = FixedContent::new(1000., 1000.);
        panel.set_content(&content);
        let window = Window::new();
        window.set_width(200.);
        window.set_height(40.);
        window.set_content(Some(Control::boxed(panel.clone().upcast::<Control>())));
        window.show();
        window.update_layout();
        assert!(panel.has_overflow());
        let device = MouseDevice::new();
        device
            .pointer()
            .capture(Some(&panel.clone().upcast::<InputElement>()));
        let below = f64::from_bits(15.0_f64.to_bits() - 1);
        for (raw_y, expected) in [(15., 185.), (below, 186.)] {
            panel.scroll_by(-10000.);
            panel.scroll_by(200.);
            let event = RawMouseWheelEventArgs::new(
                device.clone(),
                0,
                window.input_root(),
                Point::new(100., 20.),
                Vector::new(0., raw_y / 50.),
                RawInputModifiers::NONE,
            )
            .with_platform_delta(PlatformWheelDelta::MacOS {
                delta: Vector::new(0., raw_y),
                is_precise: true,
            });
            device.process_raw_event(&event);
            assert!(event.handled());
            assert_eq!(panel.offset(), expected);
        }
        device.pointer().capture(None);
        window.close();
    });
}
#[test]
fn bring_into_view_replacement_and_stale_extent_match_source() {
    run(|| {
        let panel = RibbonScrollPanel::new();
        panel.set_use_layout_rounding(false);
        panel.set_measure_infinite(true);
        let content = ferroui_controls::Panel::new();
        content.set_use_layout_rounding(false);
        content.set_width(300.);
        content.set_height(30.);
        let item = Control::new();
        item.set_use_layout_rounding(false);
        content.children().add(item.clone());
        panel.set_content(&content);
        panel.measure(Size::new(100., 30.));
        arrange(&panel, 100., 30.);
        item.arrange(Rect::new(150., 0., 20., 30.));
        panel.bring_into_view(&item);
        same(json!(panel.offset()), &corpus()["bring"][0]);
        arrange(&panel, 100., 30.);
        item.arrange(Rect::new(5., 0., 20., 30.));
        panel.bring_into_view(&item);
        same(json!(panel.offset()), &corpus()["bring"][1]);
        let unrelated = Control::new();
        unrelated.arrange(Rect::new(80., 0., 20., 30.));
        panel.bring_into_view(&unrelated);
        same(json!(panel.offset()), &corpus()["bring"][2]);
        panel.scroll_by(1000.);
        panel.set_content(None);
        panel.measure(Size::new(100., 30.));
        arrange(&panel, 100., 30.);
        compare(&panel, &corpus()["cleared"]);
        assert!(content.parent().is_none());
        assert_eq!(panel.children().count(), 2);
    });
}
#[test]
fn localized_arrows_routed_input_and_weak_parent_lifetime() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::localized_arrows_routed_input_and_weak_parent_lifetime"
    )) {
        return;
    }
    run(|| {
        use ferroribbon_core::{
            RibbonString,
            localization::{RibbonCulture, RibbonStrings},
        };
        use ferroui_controls::automation::AutomationProperties;
        let previous = RibbonStrings::current();
        RibbonStrings::set_current(Some(
            RibbonStrings::for_culture(Some(RibbonCulture::new(
                RibbonString::from("fr-FR"),
                RibbonString::from("fr"),
            )))
            .unwrap(),
        ))
        .unwrap();
        let panel = RibbonScrollPanel::new();
        let buttons = panel.children().snapshot();
        let names = buttons
            .iter()
            .map(|c| AutomationProperties::get_name(c))
            .collect::<Vec<_>>();
        assert_eq!(json!(names), corpus()["names"]);
        RibbonStrings::current()
            .override_string(Some("ScrollLeft".into()), Some("Changed".into()))
            .unwrap();
        assert_eq!(
            json!(
                buttons
                    .iter()
                    .map(|c| AutomationProperties::get_name(c))
                    .collect::<Vec<_>>()
            ),
            corpus()["after"]
        );
        assert!(!wheel(&panel, 1.));
        let content = FixedContent::new(500., 500.);
        panel.set_content(&content);
        panel.measure(Size::new(100., 30.));
        arrange(&panel, 100., 30.);
        assert!(wheel(&panel, -1.));
        assert_eq!(panel.offset(), WHEEL_UNITS);
        let args = PointerWheelEventArgs::new(
            &panel,
            Pointer::new(0, PointerType::Mouse, true),
            &panel,
            Point::new(0., 0.),
            0,
            PointerPointProperties::default(),
            KeyModifiers::NONE,
            ferroui_base::Vector::new(1., 0.),
        );
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            panel.raise_event(&args);
            assert!(args.handled());
            assert_eq!(panel.offset(), 0.);
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let rejected =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| panel.raise_event(&args)));
            assert!(rejected.is_err());
            assert!(!args.handled());
            assert_eq!(panel.offset(), WHEEL_UNITS);
        }
        // The retained event args intentionally own their Source. Release them before checking handlers.
        drop(args);

        let weak = panel.downgrade();
        drop(panel);
        assert!(weak.upgrade().is_none());
        // Retaining an internal arrow does not retain its old panel through Click.
        buttons[1].raise_event(&RoutedEventArgs::with_event(Button::click_event()));
        RibbonStrings::set_current(Some(previous)).unwrap();
    });
}

#[test]
fn extent_getter_reentry_property_invalidation_and_resource_theme() {
    run(|| {
        use ferroui_base::styling::ControlTheme;
        use ferroui_controls::Application;
        let app = Application::current().unwrap();
        let key = "RibbonScrollButtonStyle".into();
        let previous = app.resources().try_get_resource(&key, None);
        let theme = ControlTheme::for_type::<Button>();
        app.resources()
            .set(key.clone(), Some(Rc::new(theme.clone())));
        let panel = RibbonScrollPanel::new();
        panel.set_use_layout_rounding(false);
        for button in panel.children().snapshot().iter() {
            assert_eq!(button.theme(), Some(theme.clone()));
        }
        if let Some(previous) = previous {
            app.resources().set(key, previous)
        } else {
            app.resources().remove(&key);
        }
        let content = FixedContent::new(200.5, 0.);
        content.set_use_layout_rounding(false);
        let weak = content.downgrade();
        *content.on_extent.borrow_mut() = Some(Rc::new(move || {
            let content = weak.upgrade().unwrap();
            content.invalidate_measure();
            content.measure(Size::new(80., 40.));
        }));
        panel.set_content(&content);
        panel.measure(Size::new(100., 40.));
        same(
            json!([panel.desired_size().width, panel.desired_size().height]),
            &corpus()["reentrantDesired"],
        );
        assert!(panel.is_measure_valid());
        panel.set_measure_infinite(true);
        assert!(panel.is_measure_valid());
        let old = panel.content().unwrap();
        panel.set_content(Control::new());
        assert!(!panel.is_measure_valid());
        assert!(old.parent().is_none());
        assert_eq!(panel.offset(), 0.);
    });
}
