use crate::test_support::test_scope;
use crate::Slider;
use ferroui_base::layout::Orientation;

#[test]
fn default_orientation_should_be_horizontal() {
    let _scope = test_scope();
    let slider = Slider::new();
    assert_eq!(Orientation::Horizontal, slider.orientation());
}

#[test]
fn should_set_horizontal_class() {
    let _scope = test_scope();
    let slider = Slider::new();
    slider.set_orientation(Orientation::Horizontal);

    assert!(slider.classes().contains(":horizontal"));
}

#[test]
fn should_set_vertical_class() {
    let _scope = test_scope();
    let slider = Slider::new();
    slider.set_orientation(Orientation::Vertical);

    assert!(slider.classes().contains(":vertical"));
}

// Additional adapter extension coverage. Existing native policy remains the default.
use crate::{
    ControlImpl, SliderImpl,
    primitives::{RangeBase, TemplatedControlImpl},
};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes,
    input::{InputElementImpl, VectorEventArgs},
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use std::cell::Cell;
#[repr(C)]
struct AdapterSlider {
    base: Slider,
    snaps: Cell<u32>,
    deltas: Cell<u32>,
}
ferro_class!(AdapterSlider:Slider);
ferro_class_info!(AdapterSlider {
    new: AdapterSlider::new
});
ferro_impl_classes!(AdapterSlider:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl AdapterSlider {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Slider::construct(),
            snaps: Cell::new(0),
            deltas: Cell::new(0),
        })
    }
}
impl SliderImpl for AdapterSlider {
    fn snap_to_tick(c: &Self, v: f64) -> f64 {
        c.snaps.set(c.snaps.get() + 1);
        v.floor()
    }
    fn move_to_next_tick(c: &Self, direction: f64) {
        c.set_current_value(RangeBase::value_property(), c.value() + direction);
    }
    fn on_thumb_drag_delta(c: &Self, _: &VectorEventArgs) {
        c.deltas.set(c.deltas.get() + 1);
    }
}
#[test]
fn default_tick_policy_and_base_thumb_delta_are_unchanged() {
    let _scope = test_scope();
    let s = Slider::new();
    s.set_minimum(3.);
    s.set_maximum(13.);
    s.set_is_snap_to_tick_enabled(true);
    s.set_tick_frequency(2.);
    s.set_range_value(3.);
    assert_eq!(s.snap_to_tick(4.025), 5.);
    s.move_to_next_tick(1.);
    assert_eq!(s.value(), 5.);
    s.on_thumb_drag_delta(&VectorEventArgs::new());
    assert_eq!(s.value(), 5.);
}
#[test]
fn slider_reference_dispatches_adapted_snap_step_and_thumb_delta() {
    let _scope = test_scope();
    let s = AdapterSlider::new();
    let base: Ref<Slider> = s.clone().upcast();
    base.set_range_value(3.);
    assert_eq!(base.snap_to_tick(4.025), 4.);
    assert_eq!(s.snaps.get(), 1);
    base.move_to_next_tick(0.5);
    assert_eq!(base.value(), 3.5);
    base.on_thumb_drag_delta(&VectorEventArgs::new());
    assert_eq!(s.deltas.get(), 1);
}
