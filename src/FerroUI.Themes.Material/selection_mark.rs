/*
 * Copyright 2024 The Android Open Source Project
 * Copyright 2026 FerroUI contributors
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at https://www.apache.org/licenses/LICENSE-2.0
 * Unless required by applicable law or agreed to in writing, software distributed
 * under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
 * CONDITIONS OF ANY KIND, either express or implied. See the License for the
 * specific language governing permissions and limitations under the License.
 */
//! Path-length check revelation and check/dash gravitation from AndroidX
//! Checkbox.kt, e4d933c98df7baa7ea2b83eea6e086915961e0ff. The 18dp geometry is
//! the latest Material3 styling branch (isCheckboxStylingFixEnabled=true).

use crate::motion::{
    animations_enabled, MaterialMotionRole, MaterialMotionScheme, MaterialSpringState,
};
use ferroui_base::animation::{IGlobalClock, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{
    BrushExtensions, DrawingContext, IBrush, IPen, PenLineCap, PenLineJoin, StreamGeometry,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroLocator,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    LocatorExtensions, Point, Ref, Size, StyledElementImpl, StyledProperty, Visual, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct MaterialSelectionMark {
    base: Control,
    motion: RefCell<MarkMotion>,
    clock_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    last_tick: Cell<Option<f64>>,
}
ferro_class!(MaterialSelectionMark:Control);
ferro_class_info!(MaterialSelectionMark {
    new: MaterialSelectionMark::new
});
ferro_impl_classes!(MaterialSelectionMark:StyledElementImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl FerroObjectImpl for MaterialSelectionMark {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::is_checked_property().as_property()
            || property == Self::reduced_motion_property().as_property()
            || property == Self::motion_scheme_property().as_property()
        {
            this.retarget();
        }
        this.invalidate_visual();
    }
}
impl LayoutableImpl for MaterialSelectionMark {
    fn measure_override(_this: &Self, available: Size) -> Size {
        Size::new(
            18.0_f64.min(available.width),
            18.0_f64.min(available.height),
        )
    }
}
impl VisualImpl for MaterialSelectionMark {
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
    fn on_attached_to_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, event);
        let weak = this.to_ref().downgrade();
        *this.visibility_subscription.borrow_mut() =
            Some(this.is_effectively_visible_changed(move || {
                if let Some(this) = weak.upgrade() {
                    this.update_clock();
                }
            }));
        this.retarget();
    }
    fn on_detached_from_visual_tree(this: &Self, event: &VisualTreeAttachmentEventArgs) {
        this.stop_clock();
        if let Some(subscription) = this.visibility_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, event);
    }
    fn render(this: &Self, dc: &mut DrawingContext) {
        let size = this.bounds().size();
        let width = size.width.min(size.height);
        let brush = this.fill();
        if !width.is_finite() || width <= 0.0 || brush.is_none() {
            return;
        }
        let motion = this.motion.borrow();
        let (fraction, gravity) = if animations_enabled(this.reduced_motion()) {
            (motion.draw.current().value, motion.gravity.current().value)
        } else {
            mark_targets(this.is_checked())
        };
        let points = mark_path(fraction, gravity, width);
        if points.len() < 2 {
            return;
        }
        let offset = Point::new((size.width - width) / 2.0, (size.height - width) / 2.0);
        let point = |p: Point| Point::new(p.x + offset.x, p.y + offset.y);
        let geometry = StreamGeometry::new();
        let mut path = geometry.open();
        path.begin_figure(point(points[0]), false);
        for p in points.iter().skip(1) {
            path.line_to(point(*p), true);
        }
        path.end_figure(false);
        path.dispose();
        let pen: Rc<dyn IPen> = Rc::new(ImmutablePen::new(
            brush.as_ref().map(BrushExtensions::to_immutable),
            2.0,
            None,
            PenLineCap::Square,
            PenLineJoin::Miter,
            4.0,
        ));
        dc.draw_geometry(None, Some(&pen), &geometry.upcast());
    }
}
ferro_properties! {impl MaterialSelectionMark {
    pub fn is_checked_property()->StyledProperty<Option<bool>>{FerroProperty::register::<MaterialSelectionMark,_>("IsChecked",Some(false))}
    pub fn fill_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<MaterialSelectionMark,_>("Fill",None)}
    pub fn reduced_motion_property()->StyledProperty<bool>{FerroProperty::register::<MaterialSelectionMark,_>("ReducedMotion",false)}
    pub fn motion_scheme_property()->StyledProperty<MaterialMotionScheme>{FerroProperty::register::<MaterialSelectionMark,_>("MotionScheme",MaterialMotionScheme::Expressive)}
}}
impl MaterialSelectionMark {
    fn static_constructor() {
        Visual::affects_render::<MaterialSelectionMark>(&[Self::fill_property().as_property()]);
    }
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            motion: RefCell::new(MarkMotion::new()),
            clock_subscription: RefCell::new(None),
            visibility_subscription: RefCell::new(None),
            last_tick: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_is_hit_test_visible(false);
        this
    }
    pub fn is_checked(&self) -> Option<bool> {
        self.get_value(Self::is_checked_property())
    }
    pub fn set_is_checked(&self, value: Option<bool>) {
        self.set_value(Self::is_checked_property(), value);
    }
    pub fn fill(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::fill_property())
    }
    pub fn set_fill(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::fill_property(), value);
    }
    pub fn reduced_motion(&self) -> bool {
        self.get_value(Self::reduced_motion_property())
    }
    pub fn set_reduced_motion(&self, value: bool) {
        self.set_value(Self::reduced_motion_property(), value);
    }
    pub fn motion_scheme(&self) -> MaterialMotionScheme {
        self.get_value(Self::motion_scheme_property())
    }
    pub fn set_motion_scheme(&self, value: MaterialMotionScheme) {
        self.set_value(Self::motion_scheme_property(), value);
    }
    fn retarget(&self) {
        let reduced =
            !animations_enabled(self.reduced_motion()) || !self.is_attached_to_visual_tree();
        self.motion
            .borrow_mut()
            .retarget(self.is_checked(), self.motion_scheme(), reduced);
        self.update_clock();
    }
    fn update_clock(&self) {
        if !self.is_attached_to_visual_tree()
            || !self.is_effectively_visible()
            || !animations_enabled(self.reduced_motion())
            || !self.motion.borrow().running()
        {
            self.stop_clock();
            return;
        }
        if self.clock_subscription.borrow().is_some() {
            return;
        }
        let Some(clock) = FerroLocator::current().get_service::<dyn IGlobalClock>() else {
            return;
        };
        self.last_tick.set(None);
        let weak = self.to_ref().downgrade();
        *self.clock_subscription.borrow_mut() = Some(clock.subscribe(Rc::new(
            AnonymousObserver::new(move |time: TimeSpan| {
                if let Some(this) = weak.upgrade() {
                    if !animations_enabled(this.reduced_motion()) {
                        this.retarget();
                        this.invalidate_visual();
                        return;
                    }
                    let time = time.total_seconds();
                    if !time.is_finite() {
                        return;
                    }
                    let delta = this
                        .last_tick
                        .replace(Some(time))
                        .map_or(0.0, |previous| (time - previous).max(0.0));
                    this.motion.borrow_mut().advance(delta);
                    this.invalidate_visual();
                    this.update_clock();
                }
            }),
        )));
    }
    fn stop_clock(&self) {
        if let Some(subscription) = self.clock_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.last_tick.set(None);
    }
}
impl Drop for MaterialSelectionMark {
    fn drop(&mut self) {
        if let Some(subscription) = self.clock_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
fn mark_targets(state: Option<bool>) -> (f64, f64) {
    match state {
        Some(false) => (0.0, 0.0),
        Some(true) => (1.0, 0.0),
        None => (1.0, 1.0),
    }
}
struct MarkMotion {
    state: Option<bool>,
    draw: MaterialSpringState,
    gravity: MaterialSpringState,
    off_delay: Option<f64>,
}
impl MarkMotion {
    fn new() -> Self {
        let spec = MaterialMotionScheme::Expressive.spring(MaterialMotionRole::DefaultSpatial);
        Self {
            state: Some(false),
            draw: MaterialSpringState::new(spec, 0.0),
            gravity: MaterialSpringState::new(spec, 0.0),
            off_delay: None,
        }
    }
    fn retarget(&mut self, state: Option<bool>, scheme: MaterialMotionScheme, reduced: bool) {
        let (draw, gravity) = mark_targets(state);
        let previous = self.state;
        let spec = scheme.spring(MaterialMotionRole::DefaultSpatial);
        self.draw.set_spec(spec);
        self.gravity.set_spec(spec);
        self.draw.set_reduced_motion(reduced);
        self.gravity.set_reduced_motion(reduced);
        if reduced {
            self.draw.retarget(draw);
            self.gravity.retarget(gravity);
            self.draw.finish();
            self.gravity.finish();
            self.off_delay = None;
        } else if state == Some(false) && previous != Some(false) {
            self.off_delay = Some(0.1);
        } else if state != Some(false) {
            self.off_delay = None;
            self.draw.retarget(draw);
            self.gravity.retarget(gravity);
            if previous == Some(false) {
                self.gravity.finish();
            }
        }
        self.state = state;
    }
    fn advance(&mut self, delta: f64) {
        if let Some(remaining) = self.off_delay {
            if remaining - delta <= 1e-9 {
                self.off_delay = None;
                self.draw.retarget(0.0);
                self.gravity.retarget(0.0);
                self.draw.finish();
                self.gravity.finish();
            } else {
                self.off_delay = Some(remaining - delta);
            }
        } else {
            self.draw.advance(delta);
            self.gravity.advance(delta);
            if self.draw.is_at_rest(0.01, 0.625) {
                self.draw.finish();
            }
            if self.gravity.is_at_rest(0.01, 0.625) {
                self.gravity.finish();
            }
        }
    }
    fn running(&self) -> bool {
        self.off_delay.is_some()
            || !self.draw.is_at_rest(0.01, 0.625)
            || !self.gravity.is_at_rest(0.01, 0.625)
    }
}
fn mark_path(fraction: f64, gravity: f64, width: f64) -> Vec<Point> {
    let points = [
        Point::new(width * 0.25, width * 0.5),
        Point::new(
            width * (0.4 + 0.1 * gravity),
            width * (0.65 - 0.15 * gravity),
        ),
        Point::new(width * 0.75, width * (0.3 + 0.2 * gravity)),
    ];
    let length = |a: Point, b: Point| (b.x - a.x).hypot(b.y - a.y);
    let first = length(points[0], points[1]);
    let second = length(points[1], points[2]);
    let mut remaining = (first + second) * fraction.clamp(0.0, 1.0);
    if remaining <= 0.0 {
        return Vec::new();
    }
    let mut output = vec![points[0]];
    for i in 0..2 {
        let span = if i == 0 { first } else { second };
        let t = (remaining / span).clamp(0.0, 1.0);
        output.push(Point::new(
            points[i].x + (points[i + 1].x - points[i].x) * t,
            points[i].y + (points[i + 1].y - points[i].y) * t,
        ));
        remaining -= span;
        if remaining <= 0.0 {
            break;
        }
    }
    output
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_check_reveals_by_length_and_gravitates_to_dash() {
        let check = mark_path(1.0, 0.0, 18.0);
        assert_eq!(3, check.len());
        assert_eq!(Point::new(4.5, 9.0), check[0]);
        assert!((check[1].y - 11.7).abs() < 1e-12);
        assert_eq!(13.5, check[2].x);
        assert!((check[2].y - 5.4).abs() < 1e-12);
        let dash = mark_path(1.0, 1.0, 18.0);
        assert!(dash.iter().all(|p| (p.y - 9.0).abs() < 1e-12));
        let half = mark_path(0.5, 1.0, 18.0);
        assert_eq!(Point::new(9.0, 9.0), *half.last().unwrap());
    }
    #[test]
    fn uncheck_holds_mark_for_native_100ms_then_snaps() {
        let mut motion = MarkMotion::new();
        motion.retarget(Some(true), MaterialMotionScheme::Expressive, true);
        motion.retarget(Some(false), MaterialMotionScheme::Expressive, false);
        motion.advance(0.099);
        assert_eq!(1.0, motion.draw.current().value);
        motion.advance(0.001);
        assert_eq!(0.0, motion.draw.current().value);
        assert!(!motion.running());
    }
    #[test]
    fn indeterminate_check_morph_and_reduced_motion_do_not_leave_clock_work() {
        let mut motion = MarkMotion::new();
        motion.retarget(Some(true), MaterialMotionScheme::Expressive, true);
        motion.retarget(None, MaterialMotionScheme::Expressive, false);
        motion.advance(0.04);
        assert!(motion.gravity.current().value > 0.0 && motion.gravity.current().value < 1.0);
        motion.retarget(None, MaterialMotionScheme::Expressive, true);
        assert_eq!(1.0, motion.gravity.current().value);
        assert!(!motion.running());
    }
}
