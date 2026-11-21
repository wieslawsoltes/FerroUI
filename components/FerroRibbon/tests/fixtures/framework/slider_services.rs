//! Framework-only source-mapping repro and native Slider extension dispatch.
use ferroui_base::{
    Point, Rect, Ref, Size,
    input::{KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType},
    layout::Orientation,
    threading::CancellationToken,
};
use ferroui_controls::{
    AppBuilder, Application, Button, Control, Slider, Window,
    primitives::{Thumb, Track},
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
fn run(f: impl FnOnce() + Send + 'static) {
    let s = HeadlessUnitTestSession::start_new(build);
    s.dispatch(
        move || {
            ferroui_controls::register_types();
            f();
        },
        CancellationToken::none(),
    )
    .wait()
    .unwrap();
    s.dispose();
}
fn mount(s: &Slider) -> (Ref<Window>, Ref<Button>) {
    s.set_minimum(3.);
    s.set_maximum(13.);
    s.set_range_value(3.);
    s.set_tick_frequency(2.);
    s.set_is_snap_to_tick_enabled(true);
    s.set_width(120.);
    s.set_height(20.);
    s.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
        |_, scope| {
            let track = Track::new();
            track.set_orientation(Orientation::Horizontal);
            track.set_minimum(3.);
            track.set_maximum(13.);
            track.set_range_value(3.);
            let thumb = Thumb::new();
            thumb.set_width(20.);
            thumb.set_height(20.);
            track.set_thumb(Some(thumb));
            let up = Button::new();
            let down = Button::new();
            track.set_increase_button(Some(up.clone()));
            track.set_decrease_button(Some(down.clone()));
            scope.register("PART_Track", track.clone().upcast());
            scope.register("PART_IncreaseButton", up.upcast());
            scope.register("PART_DecreaseButton", down.upcast());
            track.upcast()
        },
    )));
    let w = Window::new();
    w.set_width(120.);
    w.set_height(20.);
    w.set_content(Some(Control::boxed(s.to_ref())));
    w.show();
    w.measure(Size::new(120., 20.));
    w.arrange(Rect::new(0., 0., 120., 20.));
    let button = s
        .get_visual_descendants()
        .find_map(|p| p.cast::<Button>())
        .unwrap();
    (w, button)
}
fn press(s: &Slider, w: &Window, button: &Button) {
    let e = PointerPressedEventArgs::new(
        button.to_ref(),
        Pointer::new(800, PointerType::Mouse, true),
        w,
        Point::new(20.25, 10.),
        0,
        PointerPointProperties {
            is_left_button_pressed: true,
            ..Default::default()
        },
        KeyModifiers::NONE,
        1,
    );
    button.raise_event(&e);
    assert!(s.track().is_some());
}
#[test]
fn native_slider_source_math_needs_an_explicit_adapter() {
    run(|| {
        let s = Slider::new();
        let (w, b) = mount(&s);
        press(&s, &w, &b);
        assert_eq!(
            s.value(),
            5.,
            "default native tick policy must remain unchanged"
        );
        w.close();
    });
}

mod overrides {
    use super::*;
    use ferroui_base::{
        FerroObjectImpl, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
        ferro_impl_classes,
        input::{InputElementImpl, PointerEventArgs, VectorEventArgs},
        instantiate,
        interactivity::InteractiveImpl,
        layout::LayoutableImpl,
    };
    use ferroui_controls::{
        ControlImpl, SliderImpl,
        primitives::{RangeBase, TemplatedControlImpl},
    };
    use std::cell::Cell;
    #[repr(C)]
    struct PolicySlider {
        base: Slider,
        point: Cell<f64>,
        snaps: Cell<u32>,
        delta: Cell<f64>,
    }
    ferro_class!(PolicySlider:Slider);
    ferro_class_info!(PolicySlider {
        new: PolicySlider::new
    });
    ferro_impl_classes!(PolicySlider:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
    impl PolicySlider {
        fn new() -> Ref<Self> {
            instantiate(Self {
                base: Slider::construct(),
                point: Cell::new(0.),
                snaps: Cell::new(0),
                delta: Cell::new(0.),
            })
        }
    }
    impl SliderImpl for PolicySlider {
        fn snap_to_tick(c: &Self, v: f64) -> f64 {
            c.snaps.set(c.snaps.get() + 1);
            v.floor()
        }
        fn move_to_next_tick(c: &Self, amount: f64) {
            c.set_current_value(RangeBase::value_property(), c.value() + amount);
        }
        fn move_to_pointer(c: &Self, e: &PointerEventArgs) {
            let track = c.track().unwrap();
            let p = e.try_get_position_precise(Some(&track)).unwrap();
            c.point.set(p.x);
            let value = c.minimum() + ((p.x - 10.) / 100.) * (c.maximum() - c.minimum());
            c.set_current_value(RangeBase::value_property(), c.snap_to_tick(value));
        }
        fn on_thumb_drag_delta(c: &Self, e: &VectorEventArgs) {
            c.delta.set(e.vector.x);
        }
    }
    #[test]
    fn independent_subclass_overrides_pointer_snap_step_and_thumb_delta() {
        run(|| {
            let s = PolicySlider::new();
            let (w, b) = mount(&s);
            press(&s, &w, &b);
            assert_eq!(s.point.get(), 20.25);
            assert_eq!(s.snaps.get(), 1);
            assert_eq!(s.value(), 4.);
            s.move_to_next_tick(0.5);
            assert_eq!(s.value(), 4.5);
            let thumb = s.track().unwrap().thumb().unwrap();
            let mut e = VectorEventArgs::new();
            e.vector = ferroui_base::Vector::new(0.25, 0.5);
            e.set_routed_event(Some(Thumb::drag_delta_event()));
            thumb.raise_event(&e);
            assert_eq!(s.delta.get(), 0.25);
            w.close();
        });
    }
}
