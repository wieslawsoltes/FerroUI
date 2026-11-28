//! Source WinUI slider input adaptation over native Slider services.
//! Pointer/range differences are tracked separately
use super::slider_math;
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Matrix,
    Point, Ref, StyledElementImpl, StyledProperty, StyledPropertyMetadata, TypeInfo, VisualImpl,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        IPointer, InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers,
        NavigationMethod, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
        VectorEventArgs,
    },
    instantiate,
    interactivity::{InteractiveImpl, RoutingStrategies},
    layout::LayoutableImpl,
    reactive::IDisposable,
};
use ferroui_controls::{
    ControlImpl, Slider, SliderImpl, SliderImplExt, TopLevel,
    primitives::{
        RangeBase, RangeBaseValueChangedEventArgs, TemplateAppliedEventArgs, TemplatedControl,
        TemplatedControlImpl, TemplatedControlImplExt,
    },
};
use std::{
    cell::{Cell, RefCell},
    cmp::Ordering,
    rc::{Rc, Weak},
};
struct Reset<'a>(&'a Cell<bool>);
impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
#[repr(C)]
pub struct RibbonInputSlider {
    base: Slider,
    uncoerced_value: Cell<f64>,
    processing: Cell<bool>,
    thumb_dragging: Cell<bool>,
    drag_value: Cell<f64>,
    previous_point: Cell<Option<Point>>,
    drag_space: RefCell<Option<(ferroui_base::WeakRef<TopLevel>, Matrix)>>,
    press_origin: RefCell<Option<(ferroui_base::WeakRef<TopLevel>, Point)>>,
    pending_point: Cell<Option<Point>>,
    pointer: RefCell<Option<Weak<dyn IPointer>>>,
    capture_owner: RefCell<Option<ferroui_base::WeakRef<InputElement>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}
ferro_class!(RibbonInputSlider:Slider);
ferro_class_info!(RibbonInputSlider{new:RibbonInputSlider::new,markup:{namespace:"FerroRibbon.Internal"}});
ferro_impl_classes!(RibbonInputSlider:VisualImpl,LayoutableImpl,InteractiveImpl,ControlImpl);
ferro_properties! {impl RibbonInputSlider{
 pub fn intermediate_value_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("IntermediateValue",0.)}
 pub fn step_frequency_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("StepFrequency",1.)}
}}
impl RibbonInputSlider {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Slider::construct(),
            uncoerced_value: Cell::new(0.),
            processing: Cell::new(false),
            thumb_dragging: Cell::new(false),
            drag_value: Cell::new(0.),
            previous_point: Cell::new(None),
            drag_space: RefCell::new(None),
            press_origin: RefCell::new(None),
            pending_point: Cell::new(None),
            pointer: RefCell::new(None),
            capture_owner: RefCell::new(None),
            subscriptions: RefCell::new(vec![]),
        })
    }
    fn static_constructor() {
        RangeBase::minimum_property().override_metadata::<Self>(
            StyledPropertyMetadata::new(None).with_coerce(|o, v: f64| {
                let c = o.to_ref().cast::<Self>().expect("Slider range owner");
                let old = c.minimum();
                let maximum = c.maximum();
                // Source !(v <= old) also accepts unordered (NaN) comparisons.
                if matches!(v.partial_cmp(&old), None | Some(Ordering::Greater)) && v > maximum {
                    c.set_value(RangeBase::maximum_property(), v);
                }
                v
            }),
        );
        RangeBase::maximum_property().override_metadata::<Self>(
            StyledPropertyMetadata::new(None).with_coerce(|o, v: f64| {
                let c = o.to_ref().cast::<Self>().expect("Slider range owner");
                if v >= c.maximum() {
                    v
                } else {
                    ferroui_base::utilities::math_utilities::max(v, c.minimum())
                }
            }),
        );
        RangeBase::value_property().override_metadata::<Self>(
            StyledPropertyMetadata::new(None)
                .with_default_binding_mode(ferroui_base::data::BindingMode::OneWay)
                .with_binding_writeback_uses_change_value(true)
                .with_coerce(|o, v| {
                    let c = o.to_ref().cast::<Self>().expect("Slider range owner");
                    c.uncoerced_value.set(v);
                    c.clamp_range(v)
                }),
        );
        Self::step_frequency_property()
            .changed()
            .add_class_handler::<Self>(|c, e| c.set_tick_frequency(e.get_new_value::<f64>()))
            .detach();
    }
    pub fn intermediate_value(&self) -> f64 {
        self.get_value(Self::intermediate_value_property())
    }
    fn set_intermediate_value(&self, v: f64) {
        self.set_value(Self::intermediate_value_property(), v);
    }
    pub fn step_frequency(&self) -> f64 {
        self.get_value(Self::step_frequency_property())
    }
    pub fn set_step_frequency(&self, v: f64) {
        self.set_value(Self::step_frequency_property(), v)
    }
    pub(crate) fn ensure_finite(v: f64) {
        assert!(
            v.is_finite(),
            "Invalid value supplied (source RangeBase.value)"
        );
    }
    fn clamp_range(&self, v: f64) -> f64 {
        use ferroui_base::utilities::math_utilities::{max, min};
        max(min(v, self.maximum()), self.minimum())
    }
    pub fn set_minimum(&self, v: f64) {
        Self::ensure_finite(v);
        self.set_value(RangeBase::minimum_property(), v);
    }
    pub fn set_maximum(&self, v: f64) {
        Self::ensure_finite(v);
        self.set_value(RangeBase::maximum_property(), v);
    }
    pub fn set_small_change(&self, v: f64) {
        Self::ensure_finite(v);
        self.set_value(RangeBase::small_change_property(), v);
    }
    pub fn set_large_change(&self, v: f64) {
        Self::ensure_finite(v);
        self.set_value(RangeBase::large_change_property(), v);
    }
    pub fn set_numeric_value(&self, v: f64) {
        Self::ensure_finite(v);
        self.set_value(RangeBase::value_property(), v);
    }
    fn step(&self, forward: bool) {
        let v = slider_math::step_value(
            self.minimum(),
            self.maximum(),
            self.value(),
            self.small_change(),
            forward,
        );
        self.set_numeric_value(v);
    }
}
impl RibbonInputSlider {
    fn point(&self, e: &PointerEventArgs) -> Option<Point> {
        self.track().map(|t| {
            e.try_get_position_precise(Some(&t))
                .expect("Slider requires continuous track coordinates")
        })
    }
    fn capture(&self, e: &PointerPressedEventArgs, owner: &InputElement) {
        if self
            .pointer
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some()
        {
            return;
        }
        if owner.capture_pointer(e.pointer()) {
            self.pointer.replace(Some(Rc::downgrade(e.pointer())));
            self.capture_owner.replace(Some(owner.to_ref().downgrade()));
        }
    }
    fn release_capture(&self) {
        let pointer = self.pointer.borrow_mut().take().and_then(|p| p.upgrade());
        let owner = self
            .capture_owner
            .borrow_mut()
            .take()
            .and_then(|p| p.upgrade());
        if let (Some(pointer), Some(owner)) = (pointer, owner)
            && pointer.captured().is_some_and(|c| c.ptr_eq(&owner))
        {
            pointer.capture(None);
        }
    }
    fn prepare_press(&self, e: &PointerPressedEventArgs) {
        if self.thumb_dragging.get()
            || !e
                .get_current_point(Some(self))
                .properties
                .is_left_button_pressed
        {
            return;
        }
        if let (Some(track), Some(source)) = (
            self.track(),
            e.source().and_then(|s| s.cast::<ferroui_base::Visual>()),
        ) && let Some(thumb) = track.thumb()
            && (source.ptr_eq(&thumb) || thumb.is_visual_ancestor_of(&source))
        {
            let root = TopLevel::get_top_level(Some(&track)).expect("Thumb drag root");
            let point = e
                .try_get_position_precise(Some(&root))
                .expect("Thumb drag requires continuous root coordinates");
            // Retain the original pointer point before capture can reenter.
            // The source computes its inverse after capture/pressed state,
            // immediately before Slider's DragStarted callback focuses it.
            self.press_origin.replace(Some((root.downgrade(), point)));
            self.capture(e, &thumb);
        }
    }
    fn drag_point(&self, e: &PointerEventArgs) -> Point {
        let space = self.drag_space.borrow();
        let (root, inverse) = space
            .as_ref()
            .expect("Thumb drag requires its press transform");
        let root = root.upgrade().expect("Thumb drag root was detached");
        inverse.transform(
            e.try_get_position_precise(Some(&root))
                .expect("Thumb drag requires continuous root coordinates"),
        )
    }
    fn release_track(&self, e: &PointerReleasedEventArgs) {
        if self.thumb_dragging.get() {
            return;
        }
        self.set_intermediate_value(self.value());
        // Native track release handlers live on buttons, whereas source capture lives
        // on the container. Reset native drag state through its existing virtual hook.
        Self::parent_on_thumb_drag_completed(self, &VectorEventArgs::new());
        let owned = self.pointer.borrow().as_ref().and_then(Weak::upgrade);
        if owned.is_some_and(|p| p.id() == e.pointer().id()) {
            self.release_capture();
        }
        e.set_handled(true);
    }
    fn move_thumb_to_point(&self, point: Point) {
        self.processing.set(true);
        let _reset = Reset(&self.processing);
        let Some(track) = self.track() else {
            return;
        };
        let horizontal = self.orientation() == ferroui_base::layout::Orientation::Horizontal;
        let length = if horizontal {
            track.bounds().width
        } else {
            track.bounds().height
        };
        let thumb = track.thumb().map_or(0., |t| {
            if horizontal {
                t.bounds().width
            } else {
                t.bounds().height
            }
        });
        let region = slider_math::max(length - thumb, 1.);
        let delta = if horizontal {
            point.x
        } else {
            length - point.y
        };
        let mut percent = (delta - thumb / 2.) / region;
        percent = slider_math::max(percent, 0.);
        percent = slider_math::min(percent, 1.);
        if self.is_direction_reversed() {
            percent = 1. - percent;
        }
        let value = self.minimum() + percent * (self.maximum() - self.minimum());
        self.set_intermediate_value(value);
        let next = self.snap_to_tick(value);
        if !slider_math::are_close(self.value(), next) {
            self.set_numeric_value(next);
        }
    }
}
impl TemplatedControlImpl for RibbonInputSlider {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        for s in std::mem::take(&mut *c.subscriptions.borrow_mut()) {
            s.dispose();
        }
        Self::parent_on_apply_template(c, e);
        if let Some(thumb) = c.track().and_then(|track| track.thumb()) {
            // Keep the same native template part, with the source Thumb press gate.
            thumb.set_drag_start_policy(
                ferroui_controls::primitives::ThumbDragStartPolicy::PrimaryPressWhenIdle,
            );
        }
        let weak = c.to_ref().downgrade();
        let pressed = c.add_disposable_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| {
                if let Some(c) = weak.upgrade() {
                    c.prepare_press(e);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = c.to_ref().downgrade();
        let released = c.add_disposable_handler_with(
            InputElement::pointer_released_event(),
            move |_, e| {
                if let Some(c) = weak.upgrade() {
                    c.release_track(e);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = c.to_ref().downgrade();
        let lost =
            c.add_disposable_handler(InputElement::pointer_capture_lost_event(), move |_, _| {
                if let Some(c) = weak.upgrade() {
                    c.pointer.replace(None);
                    c.capture_owner.replace(None);
                    c.thumb_dragging.set(false);
                    c.drag_space.replace(None);
                    c.press_origin.replace(None);
                    Self::parent_on_thumb_drag_completed(&c, &VectorEventArgs::new());
                }
            });
        c.subscriptions
            .borrow_mut()
            .extend([pressed.detach(), released.detach(), lost.detach()]);
    }
}
impl SliderImpl for RibbonInputSlider {
    fn snap_to_tick(c: &Self, v: f64) -> f64 {
        slider_math::closest_step(c.minimum(), c.maximum(), c.step_frequency(), v)
    }
    fn move_to_next_tick(c: &Self, v: f64) {
        c.set_numeric_value(slider_math::step_value(
            c.minimum(),
            c.maximum(),
            c.value(),
            v.abs(),
            v > 0.,
        ));
    }
    fn move_to_pointer(c: &Self, e: &PointerEventArgs) {
        if c.thumb_dragging.get() {
            c.pending_point.set(Some(c.drag_point(e)));
        } else {
            let Some(p) = c.point(e) else {
                return;
            };
            if e.routed_event() == Some(InputElement::pointer_pressed_event().as_routed_event()) {
                // Keep source container ownership instead of the native input system's
                // implicit capture of an individual track button.
                let pointer = e.pointer();
                if c.pointer
                    .borrow()
                    .as_ref()
                    .and_then(Weak::upgrade)
                    .is_none()
                {
                    let track = c.track().unwrap();
                    if track.capture_pointer(pointer) {
                        c.pointer.replace(Some(Rc::downgrade(pointer)));
                        c.capture_owner
                            .replace(Some(track.upcast::<InputElement>().downgrade()));
                    }
                }
                c.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE);
            }
            c.move_thumb_to_point(p);
            e.set_handled(true);
        }
    }
    fn on_thumb_drag_started(c: &Self, e: &VectorEventArgs) {
        Self::parent_on_thumb_drag_started(c, e);
        c.thumb_dragging.set(true);
        let origin = c.press_origin.borrow_mut().take();
        if let Some((root, point)) = origin {
            let root = root.upgrade().expect("Thumb drag root was detached");
            let inverse = c
                .track()
                .expect("Thumb drag track")
                .transform_to_visual(&root)
                .and_then(|m| m.try_invert())
                .expect("Thumb drag requires invertible parent transform");
            c.previous_point.set(Some(inverse.transform(point)));
            c.pending_point.set(None);
            c.drag_space.replace(Some((root.downgrade(), inverse)));
        }
        c.focus_with(NavigationMethod::Pointer, KeyModifiers::NONE);
        // Original Slider reads Value after focus callbacks (which can change it).
        c.drag_value.set(c.value());
    }
    fn on_thumb_drag_delta(c: &Self, e: &VectorEventArgs) {
        c.processing.set(true);
        let _reset = Reset(&c.processing);
        let Some(track) = c.track() else {
            return;
        };
        let Some(thumb) = track.thumb() else {
            return;
        };
        let change = if let Some(p) = c.pending_point.take() {
            let previous = c
                .previous_point
                .replace(Some(p))
                .expect("Thumb drag requires its press coordinate");
            (p - previous).into()
        } else {
            e.vector
        };
        let horizontal = c.orientation() == ferroui_base::layout::Orientation::Horizontal;
        let delta = if horizontal { change.x } else { -change.y };
        let length = if horizontal {
            track.bounds().width
        } else {
            track.bounds().height
        };
        let thumb_length = if horizontal {
            thumb.bounds().width
        } else {
            thumb.bounds().height
        };
        let offset =
            delta * (c.maximum() - c.minimum()) / slider_math::max(1., length - thumb_length);
        if offset.is_nan() || offset.is_infinite() {
            return;
        }
        let drag = c.drag_value.get()
            + if c.is_direction_reversed() {
                -offset
            } else {
                offset
            };
        c.drag_value.set(drag);
        let value = slider_math::min(c.maximum(), slider_math::max(c.minimum(), drag));
        c.set_intermediate_value(value);
        let next = c.snap_to_tick(value);
        if !slider_math::are_close(c.value(), next) {
            c.set_numeric_value(next);
        }
    }
    fn on_thumb_drag_completed(c: &Self, e: &VectorEventArgs) {
        Self::parent_on_thumb_drag_completed(c, e);
        c.thumb_dragging.set(false);
        c.drag_space.replace(None);
        c.press_origin.replace(None);
        c.set_intermediate_value(c.value());
        c.release_capture();
    }
}
impl StyledElementImpl for RibbonInputSlider {
    fn on_initialized(c: &Self) {
        <TemplatedControl as StyledElementImpl>::on_initialized(c);
    }

    fn style_key_override(_: &Self) -> &'static TypeInfo {
        Slider::TYPE
    }
}
impl FerroObjectImpl for RibbonInputSlider {
    fn on_property_changed(c: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        let p = e.property();
        if p == RangeBase::minimum_property().as_property()
            || p == RangeBase::maximum_property().as_property()
            || p == RangeBase::value_property().as_property()
        {
            // Source RangeBase performs dependent coercion in its metadata callback,
            // before protected notifications. Skip the native RangeBase dependency
            // policy, which is initialization-gated and can reenter a pending bound.
            <TemplatedControl as FerroObjectImpl>::on_property_changed(c, e);
            if p == RangeBase::value_property().as_property() {
                let (old, new) = e.get_old_and_new_value::<f64>();
                c.raise_event(&RangeBaseValueChangedEventArgs::new(
                    old,
                    new,
                    Some(RangeBase::value_changed_event()),
                ));
                if !c.processing.get() {
                    c.set_intermediate_value(new);
                }
            } else {
                let clamped = c.clamp_range(c.uncoerced_value.get());
                if clamped != c.value() {
                    c.coerce_value(RangeBase::value_property().as_property());
                }
            }
        } else {
            Self::parent_on_property_changed(c, e);
        }
    }
    fn constructed(c: &Self) {
        Self::parent_constructed(c);
        c.set_tick_frequency(1.);
        c.set_is_snap_to_tick_enabled(true);
    }
}
impl InputElementImpl for RibbonInputSlider {
    fn on_key_down(c: &Self, e: &KeyEventArgs) {
        // Skip the native Slider key policy: source has no PageUp/PageDown
        // action, does not filter modifiers, and uses SmallChange for arrows.
        <RangeBase as InputElementImpl>::on_key_down(c, e);
        if e.handled() || !c.is_effectively_enabled() {
            return;
        }
        let reversed = c.is_direction_reversed();
        let rtl = c.flow_direction() == ferroui_base::media::FlowDirection::RightToLeft;
        match e.key {
            Key::Left => c.step(rtl ^ reversed),
            Key::Right => c.step(!(rtl ^ reversed)),
            Key::Up => c.step(!reversed),
            Key::Down => c.step(reversed),
            Key::Home => c.set_numeric_value(c.minimum()),
            Key::End => c.set_numeric_value(c.maximum()),
            _ => return,
        }
        if !c.is_focused() {
            c.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
        }
        e.set_handled(true);
    }
}
