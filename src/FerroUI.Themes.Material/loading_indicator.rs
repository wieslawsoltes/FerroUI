//! Android Material3 Expressive loading indicators, including the seven-shape
//! indeterminate sequence, spring morphs, and progress-controlled circle/burst.
//! Ported from AndroidX LoadingIndicator.kt and MaterialShapes.kt at
//! e4d933c98df7baa7ea2b83eea6e086915961e0ff (Apache-2.0).
//! Copyright 2024 The Android Open Source Project.

use crate::motion::{animations_enabled, MaterialSpringSpec};
use crate::progress_indicator::rounded_geometry::{
    loading_polygons, path_bounds, Morph, Polygon, V,
};
use ferroui_base::animation::{IGlobalClock, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{BoxShadows, DrawingContext, IBrush, StreamGeometry};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    DirectProperty, FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Point, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::automation::peers::{AutomationPeer, ProgressBarAutomationPeer};
use ferroui_controls::primitives::{RangeBase, TemplatedControlImpl};
use ferroui_controls::ControlImpl;
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;
use std::sync::OnceLock;

/// A Material loading indicator with native 48dp container and 38dp active size.
/// `Value`, `Minimum`, `Maximum`, and range accessibility come from RangeBase;
/// `Percentage` provides a convenient 0–100 alias independent of that range.
#[repr(C)]
pub struct MaterialLoadingIndicator {
    base: RangeBase,
    percentage: Cell<f64>,
    clock_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    elapsed: Cell<f64>,
    epoch: Cell<Option<f64>>,
}
ferro_class!(MaterialLoadingIndicator: RangeBase);
ferro_class_info!(MaterialLoadingIndicator {
    new: MaterialLoadingIndicator::new
});
ferro_impl_classes!(MaterialLoadingIndicator: StyledElementImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl);
impl ControlImpl for MaterialLoadingIndicator {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ProgressBarAutomationPeer::new(this).upcast()
    }
}
impl FerroObjectImpl for MaterialLoadingIndicator {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.invalidate_visual();
        let property = change.property();
        if property == RangeBase::value_property().as_property()
            || property == RangeBase::minimum_property().as_property()
            || property == RangeBase::maximum_property().as_property()
        {
            let range = this.maximum() - this.minimum();
            let percentage = if range > 0.0 {
                ((this.value() - this.minimum()) / range * 100.0).clamp(0.0, 100.0)
            } else {
                0.0
            };
            this.set_and_raise_cell(Self::percentage_property(), &this.percentage, percentage);
        }
        if property == Self::is_indeterminate_property().as_property()
            || property == Self::reduced_motion_property().as_property()
        {
            this.update_clock();
        }
    }
}
impl LayoutableImpl for MaterialLoadingIndicator {
    fn measure_override(_this: &Self, available: Size) -> Size {
        Size::new(
            48.0_f64.min(available.width),
            48.0_f64.min(available.height),
        )
    }
}
impl VisualImpl for MaterialLoadingIndicator {
    fn bypass_flow_direction_policies(_this: &Self) -> bool {
        true
    }
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        let weak = this.to_ref().downgrade();
        *this.visibility_subscription.borrow_mut() =
            Some(this.is_effectively_visible_changed(move || {
                if let Some(this) = weak.upgrade() {
                    this.update_clock();
                }
            }));
        this.update_clock();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.stop_clock();
        if let Some(subscription) = this.visibility_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, e);
    }
    fn render(this: &Self, dc: &mut DrawingContext) {
        let size = this.bounds().size();
        let side = size.width.min(size.height);
        if !side.is_finite() || side <= 0.0 {
            return;
        }
        let container = this.container_brush();
        dc.draw_rectangle(
            container.as_ref(),
            None,
            Rect::new(0.0, 0.0, size.width, size.height),
            side / 2.0,
            side / 2.0,
            &BoxShadows::default(),
        );
        let active = this.active_brush();
        if active.is_none() {
            return;
        }
        let shapes = loading_shapes();
        let (curves, scale, rotation) = if this.is_indeterminate() {
            let time = if !animations_enabled(this.reduced_motion()) {
                0.0
            } else {
                this.elapsed.get()
            };
            let frame = loading_indeterminate_frame(time);
            (
                shapes.sequence[frame.0].cubics(frame.1),
                shapes.indeterminate_scale,
                frame.2,
            )
        } else {
            let progress = (this.percentage() / 100.0).clamp(0.0, 1.0);
            (
                shapes.determinate.cubics(progress),
                shapes.determinate_scale,
                -progress * PI,
            )
        };
        let scale = side * scale * 38.0 / 48.0;
        let scaled: Vec<_> = curves
            .into_iter()
            .map(|c| c.transformed(|p| p * scale))
            .collect();
        let bounds = path_bounds(&scaled);
        let path_center = V::new((bounds[0] + bounds[2]) / 2.0, (bounds[1] + bounds[3]) / 2.0);
        let point = |p: V| {
            let p =
                (p - path_center).rotate(rotation) + V::new(size.width / 2.0, size.height / 2.0);
            Point::new(p.x, p.y)
        };
        let geometry = StreamGeometry::new();
        let mut path = geometry.open();
        path.begin_figure(point(scaled[0].0[0]), true);
        for cubic in &scaled {
            path.cubic_bezier_to(
                point(cubic.0[1]),
                point(cubic.0[2]),
                point(cubic.0[3]),
                true,
            );
        }
        path.end_figure(true);
        path.dispose();
        dc.draw_geometry(active.as_ref(), None, &geometry.upcast());
    }
}
ferro_properties! {impl MaterialLoadingIndicator {
    pub fn percentage_property()->DirectProperty<MaterialLoadingIndicator,f64>{
        FerroProperty::register_direct::<MaterialLoadingIndicator,_>("Percentage",|o|o.percentage(),Some(|o,value|o.set_percentage(value)),0.0)
    }
    pub fn is_indeterminate_property()->StyledProperty<bool>{FerroProperty::register::<MaterialLoadingIndicator,_>("IsIndeterminate",true)}
    pub fn reduced_motion_property()->StyledProperty<bool>{FerroProperty::register::<MaterialLoadingIndicator,_>("ReducedMotion",false)}
    pub fn active_brush_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<MaterialLoadingIndicator,_>("ActiveBrush",None)}
    pub fn container_brush_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<MaterialLoadingIndicator,_>("ContainerBrush",None)}
}}
impl MaterialLoadingIndicator {
    fn static_constructor() {
        Visual::affects_render::<MaterialLoadingIndicator>(&[
            Self::active_brush_property().as_property(),
            Self::container_brush_property().as_property(),
        ]);
    }
    pub fn construct() -> Self {
        Self {
            base: RangeBase::construct(),
            percentage: Cell::new(0.0),
            clock_subscription: RefCell::new(None),
            visibility_subscription: RefCell::new(None),
            elapsed: Cell::new(0.0),
            epoch: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_is_hit_test_visible(false);
        this
    }
    pub fn percentage(&self) -> f64 {
        self.percentage.get()
    }
    pub fn set_percentage(&self, value: f64) {
        let fraction = if value.is_nan() {
            0.0
        } else {
            (value / 100.0).clamp(0.0, 1.0)
        };
        self.set_range_value(self.minimum() + (self.maximum() - self.minimum()) * fraction);
    }
    pub fn is_indeterminate(&self) -> bool {
        self.get_value(Self::is_indeterminate_property())
    }
    pub fn set_is_indeterminate(&self, value: bool) {
        self.set_value(Self::is_indeterminate_property(), value);
    }
    pub fn reduced_motion(&self) -> bool {
        self.get_value(Self::reduced_motion_property())
    }
    pub fn set_reduced_motion(&self, value: bool) {
        self.set_value(Self::reduced_motion_property(), value);
    }
    pub fn active_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::active_brush_property())
    }
    pub fn set_active_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::active_brush_property(), value);
    }
    pub fn container_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::container_brush_property())
    }
    pub fn set_container_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::container_brush_property(), value);
    }
    fn update_clock(&self) {
        if !self.is_attached_to_visual_tree()
            || !self.is_effectively_visible()
            || !self.is_indeterminate()
            || !animations_enabled(self.reduced_motion())
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
        let weak = self.to_ref().downgrade();
        self.epoch.set(None);
        *self.clock_subscription.borrow_mut() = Some(clock.subscribe(Rc::new(
            AnonymousObserver::new(move |time: TimeSpan| {
                if let Some(this) = weak.upgrade() {
                    if !animations_enabled(this.reduced_motion()) {
                        this.stop_clock();
                        this.invalidate_visual();
                        return;
                    }
                    let time = time.total_seconds();
                    if !time.is_finite() {
                        return;
                    }
                    let epoch = this.epoch.get().unwrap_or_else(|| {
                        let epoch = time - this.elapsed.get();
                        this.epoch.set(Some(epoch));
                        epoch
                    });
                    this.elapsed.set((time - epoch).max(0.0));
                    this.invalidate_visual();
                }
            }),
        )));
    }
    fn stop_clock(&self) {
        if let Some(subscription) = self.clock_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.epoch.set(None);
    }
}
impl Drop for MaterialLoadingIndicator {
    fn drop(&mut self) {
        if let Some(subscription) = self.clock_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
struct LoadingShapes {
    sequence: Vec<Morph>,
    determinate: Morph,
    indeterminate_scale: f64,
    determinate_scale: f64,
}
fn loading_shapes() -> &'static LoadingShapes {
    static SHAPES: OnceLock<LoadingShapes> = OnceLock::new();
    SHAPES.get_or_init(|| {
        let polygons = loading_polygons();
        let indeterminate_scale = polygons
            .iter()
            .fold(1.0_f64, |scale, p| scale.min(p.rotation_safe_scale()));
        let circle = Polygon::circle(10)
            .normalized()
            .transformed(|p| p.rotate(PI / 10.0));
        let determinate_scale = 1.0_f64
            .min(circle.rotation_safe_scale())
            .min(polygons[0].rotation_safe_scale());
        let sequence = (0..polygons.len())
            .map(|i| {
                Morph::new(
                    &polygons[i].normalized(),
                    &polygons[(i + 1) % polygons.len()].normalized(),
                )
            })
            .collect();
        LoadingShapes {
            sequence,
            determinate: Morph::new(&circle.normalized(), &polygons[0].normalized()),
            indeterminate_scale,
            determinate_scale,
        }
    })
}
/// Shape index, spring progress, and clockwise rotation in radians. New morphs
/// start every 650ms; continuous global rotation takes 4666ms per revolution.
/// Compose computes a 297ms settling duration for this spring at visibility
/// threshold 0.1, then snaps to its target and holds until the next interval.
pub fn loading_indeterminate_frame(seconds: f64) -> (usize, f64, f64) {
    let time = if seconds.is_finite() {
        seconds.max(0.0)
    } else {
        0.0
    };
    let iteration = (time / 0.65).floor();
    let local = time.rem_euclid(0.65);
    let progress = if local >= 0.297 {
        1.0
    } else {
        MaterialSpringSpec::new(0.6, 200.0)
            .sample(local, 0.0, 1.0, 0.0)
            .value
    };
    let rotation =
        (90.0 * (iteration + 1.0) + progress * 90.0 + time.rem_euclid(4.666) / 4.666 * 360.0)
            .to_radians();
    ((iteration as usize) % 7, progress, rotation)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loading_sequence_uses_native_shapes_and_timing() {
        assert_eq!(7, loading_shapes().sequence.len());
        assert_eq!((0, 0.0, PI / 2.0), loading_indeterminate_frame(0.0));
        assert_eq!(1, loading_indeterminate_frame(0.65).0);
        assert_eq!(6, loading_indeterminate_frame(4.0).0);
        assert!(
            loading_indeterminate_frame(0.28).1 > 1.0,
            "native spring must preserve expressive overshoot"
        );
        assert_eq!(1.0, loading_indeterminate_frame(0.6).1);
    }
}
