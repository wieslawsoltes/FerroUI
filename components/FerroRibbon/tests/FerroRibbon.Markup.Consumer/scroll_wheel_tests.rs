//! Actual inherited wheel routing from compiled/runtime library consumers.
//! Original-data events are synthetic; OS delivery is verified separately.
use ferroribbon::primitives::RibbonScrollPanel;
use ferroui_base::input::{
    KeyModifiers, PlatformWheelDelta, Pointer, PointerPointProperties, PointerType,
    PointerWheelEventArgs,
};
use ferroui_base::media::FlowDirection;
use ferroui_base::{Point, Rect, Ref, Size, Vector};
use ferroui_controls::{Border, StackPanel};
use serde_json::Value;
use std::{cell::Cell, rc::Rc};

#[test]
fn source_wheel_policy_survives_compiled_runtime_subclass_and_bubbling() {
    replay(include_str!("../fixtures/scroll-wheel.json"), false);
}
#[test]
fn source_win32_wheel_policy_survives_compiled_runtime_subclass_and_bubbling() {
    replay(include_str!("../fixtures/win32-wheel.json"), true);
}
fn replay(fixture: &'static str, windows: bool) {
    super::tests::run(move || {
        let cases: Value = serde_json::from_str(fixture).unwrap();
        let exact = |input: &Value, key: &str| {
            f64::from_bits(u64::from_str_radix(input[key].as_str().unwrap(), 16).unwrap())
        };
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ScrollWheel.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ScrollWheel.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let root = value.downcast_ref::<Ref<StackPanel>>().unwrap();
            let bubbled = Rc::new(Cell::new(0));
            let observed = bubbled.clone();
            root.pointer_wheel_changed(move |_, _| observed.set(observed.get() + 1));
            for child in root.children().snapshot().iter() {
                let panel = child.cast::<RibbonScrollPanel>().unwrap();
                let content = panel.content().unwrap();
                for direction in [FlowDirection::LeftToRight, FlowDirection::RightToLeft] {
                    panel.set_flow_direction(direction);
                    for case in cases["corpus"].as_array().unwrap() {
                        let width = if case["overflow"].as_bool().unwrap() {
                            1000.
                        } else {
                            200.
                        };
                        if let Some(content) = content.cast::<Border>() {
                            content.set_width(width);
                        } else {
                            content
                                .cast::<crate::scroll_consumer::ConsumerAdaptive>()
                                .unwrap()
                                .set_extent_width(width);
                        }
                        // This consumer deliberately has no layout root. Finish
                        // native passes scheduled by child/arrow changes before
                        // injecting the next input event.
                        for _ in 0..4 {
                            panel.measure(Size::new(200., 40.));
                            panel.arrange(Rect::new(0., 0., 200., 40.));
                            if panel.is_measure_valid() && panel.is_arrange_valid() {
                                break;
                            }
                        }
                        assert!(panel.is_measure_valid() && panel.is_arrange_valid());
                        assert_eq!(panel.has_overflow(), case["overflow"].as_bool().unwrap());
                        panel.scroll_by(-10000.);
                        panel.scroll_by(200.);
                        assert_eq!(
                            panel.offset(),
                            case["before"].as_f64().unwrap(),
                            "before input: runtime={runtime} direction={direction:?} width={} content_width={} desired={} overflow={} measure_valid={} content_measure_valid={} case={case}",
                            panel.bounds().width,
                            content.width(),
                            content.desired_size().width,
                            panel.has_overflow(),
                            panel.is_measure_valid(),
                            content.is_measure_valid(),
                        );
                        let input = &case["input"];
                        let original = if windows {
                            let word =
                                u64::from_str_radix(input["wparam"].as_str().unwrap(), 16).unwrap();
                            let raw = f64::from((word >> 16) as u16 as i16);
                            let delta = if input["horizontal"].as_bool().unwrap() {
                                Vector::new(raw, 0.)
                            } else {
                                Vector::new(0., raw)
                            };
                            PlatformWheelDelta::Win32 { delta }
                        } else {
                            PlatformWheelDelta::MacOS {
                                delta: Vector::new(
                                    exact(input, "raw_x_bits"),
                                    exact(input, "raw_y_bits"),
                                ),
                                is_precise: input["precise"].as_bool().unwrap(),
                            }
                        };
                        let event = PointerWheelEventArgs::new(
                            content.clone(),
                            Pointer::new(0, PointerType::Mouse, true),
                            &panel,
                            Point::new(0., 0.),
                            0,
                            PointerPointProperties::default(),
                            KeyModifiers::NONE,
                            Vector::new(
                                exact(input, "native_x_bits"),
                                exact(input, "native_y_bits"),
                            ),
                        )
                        .with_platform_delta(original);
                        let before_bubble = bubbled.get();
                        content.raise_event(&event);
                        assert_eq!(panel.offset(), case["after"].as_f64().unwrap(), "{case}");
                        let handled = case["handled"].as_bool().unwrap();
                        assert_eq!(event.handled(), handled, "{case}");
                        assert_eq!(bubbled.get() - before_bubble, usize::from(!handled));
                        assert!(panel.content().unwrap().ptr_eq(&content));
                        assert!(panel.children().get(0).ptr_eq(&content));
                    }
                }
                if let Some(subclass) = child.cast::<crate::scroll_consumer::ConsumerScroll>() {
                    assert!(subclass.arrangements() > 0);
                }
            }
        }
    });
}
