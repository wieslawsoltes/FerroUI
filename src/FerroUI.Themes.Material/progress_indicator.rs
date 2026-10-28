//! Material linear/circular progress rendering and the 1.75s/6s Android timing curves.
//!
//! Tokens and timing: AndroidX Compose Material3 1.5.0-beta02,
//! e4d933c98df7baa7ea2b83eea6e086915961e0ff, ProgressIndicator.kt and
//! WavyProgressIndicator.kt and internal/LinearWavyProgressModifiers.kt,
//! Copyright The Android Open Source Project,
//! Apache-2.0. This implementation uses FerroUI drawing and its UI clock.

use crate::motion::animations_enabled;
use ferroui_base::animation::{IGlobalClock, TimeSpan};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{
    BoxShadows, BrushExtensions, DrawingContext, IBrush, IPen, PenLineCap, PenLineJoin,
    StreamGeometry, SweepDirection,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::reactive::{AnonymousObserver, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroLocator,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    LocatorExtensions, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty, Visual,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::f64::consts::{PI, TAU};
use std::rc::Rc;

#[path = "rounded_geometry.rs"]
pub(crate) mod rounded_geometry;
use rounded_geometry::{path_bounds, Cubic, Morph, PathMeasure, Polygon, Rounding, V};

/// Renderer used by the Material ProgressBar control themes. The enclosing
/// ProgressBar owns range semantics and automation; this noninteractive visual
/// consumes its percentage and state without changing that control's behavior.
#[repr(C)]
pub struct MaterialProgressIndicator {
    base: Control,
    clock_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    visibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    elapsed: Cell<f64>,
    epoch: Cell<Option<f64>>,
    amplitude: Cell<f64>,
    amplitude_from: Cell<f64>,
    amplitude_to: Cell<f64>,
    amplitude_started: Cell<f64>,
    circular_shapes: RefCell<Option<CircularShapes>>,
}
ferro_class!(MaterialProgressIndicator: Control);
ferro_class_info!(MaterialProgressIndicator {
    new: MaterialProgressIndicator::new
});
ferro_impl_classes!(MaterialProgressIndicator: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl FerroObjectImpl for MaterialProgressIndicator {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        this.invalidate_visual();
        if change.property() == Self::is_circular_property().as_property() {
            this.invalidate_mirror_transform();
        }
        if change.property() == Self::is_circular_property().as_property()
            || change.property() == Self::is_wavy_property().as_property()
            || change.property() == Self::orientation_property().as_property()
            || change.property() == Self::stroke_thickness_property().as_property()
        {
            this.invalidate_measure();
        }
        if change.property() == Self::percentage_property().as_property()
            || change.property() == Self::is_wavy_property().as_property()
            || change.property() == Self::is_indeterminate_property().as_property()
            || change.property() == Self::reduced_motion_property().as_property()
        {
            this.retarget_amplitude();
            this.update_clock();
        }
    }
}

impl LayoutableImpl for MaterialProgressIndicator {
    fn measure_override(this: &Self, available: Size) -> Size {
        let thickness = finite_nonnegative(this.stroke_thickness(), 4.0);
        let desired = if this.is_circular() {
            let diameter = if this.is_wavy() { 48.0 } else { 40.0 };
            Size::new(diameter, diameter)
        } else {
            let cross = if this.is_wavy() {
                thickness + 6.0
            } else {
                thickness
            };
            if this.orientation() == Orientation::Horizontal {
                Size::new(240.0, cross)
            } else {
                Size::new(cross, 240.0)
            }
        };
        Size::new(
            desired.width.min(available.width),
            desired.height.min(available.height),
        )
    }
}

impl VisualImpl for MaterialProgressIndicator {
    fn bypass_flow_direction_policies(this: &Self) -> bool {
        this.is_circular()
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
        this.retarget_amplitude();
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
        let bounds = this.bounds().size();
        if bounds.width <= 0.0
            || bounds.height <= 0.0
            || !bounds.width.is_finite()
            || !bounds.height.is_finite()
        {
            return;
        }
        let thickness =
            finite_nonnegative(this.stroke_thickness(), 4.0).min(bounds.width.min(bounds.height));
        if thickness <= 0.0 {
            return;
        }
        let gap = finite_nonnegative(this.gap_size(), 4.0);
        let phase = if !animations_enabled(this.reduced_motion()) {
            1.0
        } else {
            this.elapsed.get()
        };
        if this.is_circular() {
            this.draw_circular(dc, bounds, thickness, gap, phase);
        } else {
            this.draw_linear(dc, bounds, thickness, gap, phase);
        }
    }
}

ferro_properties! { impl MaterialProgressIndicator {
    /// Progress percentage, clamped to 0–100 when rendered.
    pub fn percentage_property() -> StyledProperty<f64> {
        FerroProperty::register::<MaterialProgressIndicator, _>("Percentage", 0.0)
    }
    /// Whether progress is unknown.
    pub fn is_indeterminate_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialProgressIndicator, _>("IsIndeterminate", false)
    }
    /// Use a circular rather than linear indicator.
    pub fn is_circular_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialProgressIndicator, _>("IsCircular", false)
    }
    /// Use the expressive wave form.
    pub fn is_wavy_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialProgressIndicator, _>("IsWavy", false)
    }
    /// Draw the required terminal stop dot on a linear determinate track.
    pub fn show_stop_indicator_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialProgressIndicator, _>("ShowStopIndicator", true)
    }
    /// Stop all continuous animation and display a static indeterminate state.
    pub fn reduced_motion_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialProgressIndicator, _>("ReducedMotion", false)
    }
    /// Direction of the linear indicator.
    pub fn orientation_property() -> StyledProperty<Orientation> {
        FerroProperty::register::<MaterialProgressIndicator, _>("Orientation", Orientation::Horizontal)
    }
    /// Thickness of the active and inactive strokes in device independent pixels.
    pub fn stroke_thickness_property() -> StyledProperty<f64> {
        FerroProperty::register::<MaterialProgressIndicator, _>("StrokeThickness", 4.0)
    }
    /// Visible space between the active and inactive tracks.
    pub fn gap_size_property() -> StyledProperty<f64> {
        FerroProperty::register::<MaterialProgressIndicator, _>("GapSize", 4.0)
    }
    /// Active indicator brush.
    pub fn active_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<MaterialProgressIndicator, _>("ActiveBrush", None)
    }
    /// Inactive track brush.
    pub fn track_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<MaterialProgressIndicator, _>("TrackBrush", None)
    }
} }

impl MaterialProgressIndicator {
    fn static_constructor() {
        Visual::affects_render::<MaterialProgressIndicator>(&[
            Self::active_brush_property().as_property(),
            Self::track_brush_property().as_property(),
        ]);
    }
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            clock_subscription: RefCell::new(None),
            visibility_subscription: RefCell::new(None),
            elapsed: Cell::new(0.0),
            epoch: Cell::new(None),
            amplitude: Cell::new(0.0),
            amplitude_from: Cell::new(0.0),
            amplitude_to: Cell::new(0.0),
            amplitude_started: Cell::new(0.0),
            circular_shapes: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_is_hit_test_visible(false);
        this
    }
    /// Progress percentage, clamped to 0–100 when rendered.
    pub fn percentage(&self) -> f64 {
        self.get_value(Self::percentage_property())
    }
    pub fn set_percentage(&self, value: f64) {
        self.set_value(Self::percentage_property(), value);
    }
    /// Whether progress is unknown.
    pub fn is_indeterminate(&self) -> bool {
        self.get_value(Self::is_indeterminate_property())
    }
    pub fn set_is_indeterminate(&self, value: bool) {
        self.set_value(Self::is_indeterminate_property(), value);
    }
    /// Use a circular rather than linear indicator.
    pub fn is_circular(&self) -> bool {
        self.get_value(Self::is_circular_property())
    }
    pub fn set_is_circular(&self, value: bool) {
        self.set_value(Self::is_circular_property(), value);
    }
    /// Use the expressive wave form.
    pub fn is_wavy(&self) -> bool {
        self.get_value(Self::is_wavy_property())
    }
    pub fn set_is_wavy(&self, value: bool) {
        self.set_value(Self::is_wavy_property(), value);
    }
    /// Draw the required terminal stop dot on a linear determinate track.
    pub fn show_stop_indicator(&self) -> bool {
        self.get_value(Self::show_stop_indicator_property())
    }
    pub fn set_show_stop_indicator(&self, value: bool) {
        self.set_value(Self::show_stop_indicator_property(), value);
    }
    /// Stop all continuous animation and display a static indeterminate state.
    pub fn reduced_motion(&self) -> bool {
        self.get_value(Self::reduced_motion_property())
    }
    pub fn set_reduced_motion(&self, value: bool) {
        self.set_value(Self::reduced_motion_property(), value);
    }
    /// Direction of the linear indicator.
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value);
    }
    /// Thickness of the active and inactive strokes in device independent pixels.
    pub fn stroke_thickness(&self) -> f64 {
        self.get_value(Self::stroke_thickness_property())
    }
    pub fn set_stroke_thickness(&self, value: f64) {
        self.set_value(Self::stroke_thickness_property(), value);
    }
    /// Visible space between the active and inactive tracks.
    pub fn gap_size(&self) -> f64 {
        self.get_value(Self::gap_size_property())
    }
    pub fn set_gap_size(&self, value: f64) {
        self.set_value(Self::gap_size_property(), value);
    }
    /// Active indicator brush.
    pub fn active_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::active_brush_property())
    }
    pub fn set_active_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::active_brush_property(), value);
    }
    /// Inactive track brush.
    pub fn track_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::track_brush_property())
    }
    pub fn set_track_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::track_brush_property(), value);
    }

    fn normalized_progress(&self) -> f64 {
        normalized_progress(self.percentage())
    }
    fn target_amplitude(&self) -> f64 {
        if !self.is_wavy() {
            0.0
        } else if self.is_indeterminate() {
            1.0
        } else {
            wave_amplitude(self.normalized_progress())
        }
    }
    fn retarget_amplitude(&self) {
        let target = self.target_amplitude();
        if !animations_enabled(self.reduced_motion()) || !self.is_attached_to_visual_tree() {
            self.amplitude.set(target);
            self.amplitude_from.set(target);
            self.amplitude_to.set(target);
        } else if target != self.amplitude_to.get() {
            self.amplitude_from.set(self.amplitude.get());
            self.amplitude_to.set(target);
            self.amplitude_started.set(self.elapsed.get());
        }
    }
    fn animation_needed(&self) -> bool {
        animations_enabled(self.reduced_motion())
            && (self.is_indeterminate()
                || (self.is_wavy()
                    && (self.amplitude.get() > 0.0 || self.amplitude_to.get() > 0.0)))
    }
    fn update_clock(&self) {
        if !self.is_attached_to_visual_tree()
            || !self.is_effectively_visible()
            || !self.animation_needed()
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
        self.epoch.set(None);
        let weak = self.to_ref().downgrade();
        let subscription =
            clock.subscribe(Rc::new(AnonymousObserver::new(move |time: TimeSpan| {
                if let Some(this) = weak.upgrade() {
                    this.tick(time.total_seconds());
                }
            })));
        *self.clock_subscription.borrow_mut() = Some(subscription);
    }
    fn stop_clock(&self) {
        if let Some(subscription) = self.clock_subscription.borrow_mut().take() {
            subscription.dispose();
        }
        self.epoch.set(None);
    }
    fn tick(&self, time: f64) {
        if !animations_enabled(self.reduced_motion()) {
            self.amplitude.set(self.target_amplitude());
            self.stop_clock();
            self.invalidate_visual();
            return;
        }
        if !time.is_finite() {
            return;
        }
        let epoch = self.epoch.get().unwrap_or_else(|| {
            let epoch = time - self.elapsed.get();
            self.epoch.set(Some(epoch));
            epoch
        });
        self.elapsed.set((time - epoch).max(0.0));
        let fraction = ((self.elapsed.get() - self.amplitude_started.get()) / 0.5).clamp(0.0, 1.0);
        let from = self.amplitude_from.get();
        let target = self.amplitude_to.get();
        let eased = if target > from {
            cubic_bezier(fraction, 0.2, 0.0, 0.0, 1.0)
        } else {
            cubic_bezier(fraction, 0.3, 0.0, 0.8, 0.15)
        };
        self.amplitude.set(from + (target - from) * eased);
        self.invalidate_visual();
        if !self.animation_needed() {
            self.stop_clock();
        }
    }

    fn draw_linear(
        &self,
        dc: &mut DrawingContext,
        size: Size,
        thickness: f64,
        gap: f64,
        phase: f64,
    ) {
        let vertical = self.orientation() == Orientation::Vertical;
        let length = if vertical { size.height } else { size.width };
        let cross = if vertical { size.width } else { size.height };
        let active = self.active_brush();
        let track = self.track_brush();
        let mut segments = if self.is_indeterminate() {
            linear_indeterminate_segments(phase).to_vec()
        } else {
            vec![(0.0, self.normalized_progress())]
        };
        segments.retain(|(start, end)| end > start);
        segments.sort_by(|a, b| a.0.total_cmp(&b.0));
        // Subtract active segments and the gap from the inactive track, rather
        // than painting a fake gap in the application's background color.
        let mut cursor: f64 = 0.0;
        let gap_fraction = gap / length;
        for &(start, end) in &segments {
            let inactive_end = (start - gap_fraction).max(0.0);
            if inactive_end > cursor {
                draw_linear_segment(
                    dc,
                    track.as_ref(),
                    cursor * length,
                    inactive_end * length,
                    size,
                    thickness,
                    vertical,
                );
            }
            cursor = cursor.max((end + gap_fraction).min(1.0));
        }
        if cursor < 1.0 {
            draw_linear_segment(
                dc,
                track.as_ref(),
                cursor * length,
                length,
                size,
                thickness,
                vertical,
            );
        }
        let amplitude = if !animations_enabled(self.reduced_motion()) {
            self.target_amplitude()
        } else {
            self.amplitude.get()
        };
        for &(start, end) in &segments {
            if self.is_wavy() && amplitude > 0.001 {
                let wavelength = if self.is_indeterminate() { 20.0 } else { 40.0 };
                draw_linear_wave(
                    dc,
                    active.as_ref(),
                    start * length,
                    end * length,
                    size,
                    thickness,
                    vertical,
                    wavelength,
                    amplitude,
                    phase,
                );
            } else {
                draw_linear_segment(
                    dc,
                    active.as_ref(),
                    start * length,
                    end * length,
                    size,
                    thickness,
                    vertical,
                );
            }
        }
        if self.show_stop_indicator()
            && !self.is_indeterminate()
            && self.normalized_progress() < 1.0
        {
            // The terminal dot is always part of the active color. Its extent is
            // inside the track, and is naturally covered as progress completes.
            let dot = 4.0_f64.min(thickness);
            let center = if vertical {
                Point::new(cross / 2.0, dot / 2.0)
            } else {
                Point::new(length - dot / 2.0, cross / 2.0)
            };
            dc.draw_ellipse(
                active.as_ref(),
                None,
                Rect::new(center.x - dot / 2.0, center.y - dot / 2.0, dot, dot),
            );
        }
    }

    fn draw_circular(
        &self,
        dc: &mut DrawingContext,
        size: Size,
        thickness: f64,
        gap: f64,
        phase: f64,
    ) {
        let diameter = size.width.min(size.height);
        let radius = (diameter - thickness) / 2.0;
        if radius <= 0.0 {
            return;
        }
        let center = Point::new(size.width / 2.0, size.height / 2.0);
        let (rotation, progress) = if self.is_indeterminate() {
            circular_indeterminate_frame(phase)
        } else {
            (-90.0, self.normalized_progress())
        };
        if !self.is_wavy() {
            let sweep = progress * TAU;
            let gap_angle = ((gap + thickness) / (PI * diameter) * TAU).min(sweep);
            let start = rotation.to_radians();
            draw_arc(
                dc,
                self.track_brush().as_ref(),
                center,
                radius,
                start + sweep + gap_angle,
                (TAU - sweep - 2.0 * gap_angle).max(0.0),
                thickness,
            );
            if sweep > 0.0 {
                draw_arc(
                    dc,
                    self.active_brush().as_ref(),
                    center,
                    radius,
                    start,
                    sweep,
                    thickness,
                );
            }
            return;
        }
        let amplitude = if !animations_enabled(self.reduced_motion()) {
            self.target_amplitude()
        } else {
            self.amplitude.get()
        };
        let count = (TAU * radius / 15.0).round().max(5.0) as usize;
        let mut cache = self.circular_shapes.borrow_mut();
        if cache.as_ref().map(|shapes| shapes.count) != Some(count) {
            *cache = Some(CircularShapes::new(count));
        }
        let shapes = cache.as_mut().unwrap();
        let paths = shapes.paths(size, thickness, amplitude);
        let stop = progress * paths.active.length;
        let cap = if size.height > size.width {
            0.0
        } else {
            thickness / 2.0
        };
        let spacing = stop.min(cap) * 2.0 + stop.min(gap);
        let rotation = if self.is_indeterminate() {
            (rotation + 90.0).to_radians()
        } else {
            0.0
        };
        let pivot = V::new(center.x, center.y);
        let track = paths.track.segment(
            progress * paths.track.length + spacing,
            paths.track.length - spacing,
        );
        draw_cubic_stroke(
            dc,
            self.track_brush().as_ref(),
            &track,
            thickness,
            pivot,
            rotation,
        );
        if stop > 0.0 {
            // One wavelength per second; one revolution contains `count` waves.
            // Trim the repeated path by length, then counter-rotate its offset,
            // exactly as CircularProgressDrawingCache.updateDrawPaths does.
            let offset = if amplitude > 0.0 && animations_enabled(self.reduced_motion()) {
                (phase / count as f64).rem_euclid(1.0)
            } else {
                0.0
            };
            let active = paths.active.segment(
                offset * paths.active.length,
                stop + offset * paths.active.length,
            );
            draw_cubic_stroke(
                dc,
                self.active_brush().as_ref(),
                &active,
                thickness,
                pivot,
                rotation - offset * TAU,
            );
        }
    }
}

impl Drop for MaterialProgressIndicator {
    fn drop(&mut self) {
        if let Some(subscription) = self.clock_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(subscription) = self.visibility_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

fn finite_nonnegative(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        fallback
    }
}
fn normalized_progress(percentage: f64) -> f64 {
    if percentage.is_nan() {
        0.0
    } else {
        (percentage / 100.0).clamp(0.0, 1.0)
    }
}
fn wave_amplitude(progress: f64) -> f64 {
    if progress <= 0.1 || progress >= 0.95 {
        0.0
    } else {
        1.0
    }
}

/// Start/end fractions of Android's two indeterminate line segments. The
/// 1750ms cycle, head/tail durations and delays are ported from ProgressIndicator.kt.
pub fn linear_indeterminate_segments(seconds: f64) -> [(f64, f64); 2] {
    let time = if seconds.is_finite() {
        seconds.max(0.0) % 1.75
    } else {
        0.0
    };
    let fraction = |delay: f64, duration: f64| {
        cubic_bezier(
            ((time - delay) / duration).clamp(0.0, 1.0),
            0.3,
            0.0,
            0.8,
            0.15,
        )
    };
    [
        (fraction(0.25, 1.0), fraction(0.0, 1.0)),
        (fraction(0.9, 0.85), fraction(0.65, 0.85)),
    ]
}

/// Rotation in degrees and sweep fraction for the 6-second circular cycle.
/// Android's keyframes add four 90-degree turns with a 300ms advancing phase
/// and a 1200ms hold; the global rotation runs continuously over six seconds.
pub fn circular_indeterminate_frame(seconds: f64) -> (f64, f64) {
    let time = if seconds.is_finite() {
        seconds.max(0.0) % 6.0
    } else {
        0.0
    };
    let turn = (time / 1.5).floor();
    let additional = 90.0 * (turn + ((time % 1.5) / 0.3).clamp(0.0, 1.0));
    // In the source keyframe spec, the first segment uses its default linear
    // easing and the keyframe at 3000ms sets Standard easing for the return.
    let progress = if time <= 3.0 {
        0.1 + 0.77 * time / 3.0
    } else {
        0.87 - 0.77 * cubic_bezier((time - 3.0) / 3.0, 0.2, 0.0, 0.0, 1.0)
    };
    (time * 60.0 + additional, progress)
}

fn cubic_bezier(progress: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    if progress <= 0.0 {
        return 0.0;
    }
    if progress >= 1.0 {
        return 1.0;
    }
    let evaluate = |t: f64, a: f64, b: f64| {
        3.0 * (1.0 - t).powi(2) * t * a + 3.0 * (1.0 - t) * t * t * b + t * t * t
    };
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..32 {
        let t = (low + high) / 2.0;
        if evaluate(t, x1, x2) < progress {
            low = t;
        } else {
            high = t;
        }
    }
    evaluate((low + high) / 2.0, y1, y2)
}

fn draw_linear_segment(
    dc: &mut DrawingContext,
    brush: Option<&Rc<dyn IBrush>>,
    start: f64,
    end: f64,
    size: Size,
    thickness: f64,
    vertical: bool,
) {
    if end <= start {
        return;
    }
    let rect = if vertical {
        Rect::new(
            (size.width - thickness) / 2.0,
            size.height - end,
            thickness,
            end - start,
        )
    } else {
        Rect::new(
            start,
            (size.height - thickness) / 2.0,
            end - start,
            thickness,
        )
    };
    dc.draw_rectangle(
        brush,
        None,
        rect,
        thickness / 2.0,
        thickness / 2.0,
        &BoxShadows::default(),
    );
}
fn pen(brush: Option<&Rc<dyn IBrush>>, thickness: f64) -> Rc<dyn IPen> {
    Rc::new(ImmutablePen::new(
        brush.map(BrushExtensions::to_immutable),
        thickness,
        None,
        PenLineCap::Round,
        PenLineJoin::Round,
        10.0,
    ))
}
// Android's linear wave is an alternating quadratic path, not a sine. Its
// PathMeasure trims by arc length, using the full-amplitude path even while
// amplitude is animated. Invert that same arc-length parameter analytically,
// then emit the exact quadratic subcurves through de Casteljau subdivision.
fn quadratic_wave_arc(t: f64, half_wave: f64, max_amplitude: f64) -> f64 {
    let k = 4.0 * max_amplitude / half_wave;
    if k.abs() < 1e-10 {
        return t * half_wave;
    }
    let primitive = |u: f64| 0.5 * (u * (1.0 + (k * u).powi(2)).sqrt() + (k * u).asinh() / k);
    half_wave * 0.5 * (primitive(1.0) - primitive(1.0 - 2.0 * t))
}
fn wave_parameter(fraction: f64, half_wave: f64, max_amplitude: f64) -> f64 {
    if fraction <= 0.0 {
        return 0.0;
    }
    if fraction >= 1.0 {
        return 1.0;
    }
    let target = fraction * quadratic_wave_arc(1.0, half_wave, max_amplitude);
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..32 {
        let t = (low + high) / 2.0;
        if quadratic_wave_arc(t, half_wave, max_amplitude) < target {
            low = t;
        } else {
            high = t;
        }
    }
    (low + high) / 2.0
}
fn draw_linear_wave(
    dc: &mut DrawingContext,
    brush: Option<&Rc<dyn IBrush>>,
    start: f64,
    end: f64,
    size: Size,
    thickness: f64,
    vertical: bool,
    wavelength: f64,
    amplitude: f64,
    phase: f64,
) {
    if brush.is_none() || end <= start {
        return;
    }
    let length = if vertical { size.height } else { size.width };
    let cross = if vertical { size.width } else { size.height };
    let max_amplitude = ((cross - thickness) / 2.0).max(0.0);
    let cap = thickness / 2.0;
    let shift = phase.rem_euclid(1.0) * wavelength;
    let a = start.clamp(cap, length - cap) + shift;
    let b = end.clamp(cap, length - cap) + shift;
    if b <= a {
        return;
    }
    let half = wavelength / 2.0;
    let first = (a / half).floor() as usize;
    let last = (b / half).floor() as usize;
    let geometry = StreamGeometry::new();
    let mut path = geometry.open();
    let point = |x: f64, y: f64| {
        if vertical {
            Point::new(cross / 2.0 + y, length - x)
        } else {
            Point::new(x, cross / 2.0 + y)
        }
    };
    for segment in first..=last {
        let offset = segment as f64 * half;
        let t0 = wave_parameter(((a - offset) / half).clamp(0.0, 1.0), half, max_amplitude);
        let t1 = wave_parameter(((b - offset) / half).clamp(0.0, 1.0), half, max_amplitude);
        if t1 <= t0 {
            continue;
        }
        let sign = if segment % 2 == 0 { 1.0 } else { -1.0 };
        let a_y = 4.0 * max_amplitude * amplitude * sign;
        let x0 = offset + half * t0 - shift;
        let y0 = a_y * t0 * (1.0 - t0);
        if segment == first {
            path.begin_figure(point(x0, y0), false);
        }
        let dt = t1 - t0;
        let control = point(x0 + half * dt / 2.0, y0 + a_y * (1.0 - 2.0 * t0) * dt / 2.0);
        let end = point(offset + half * t1 - shift, a_y * t1 * (1.0 - t1));
        path.quadratic_bezier_to(control, end, true);
    }
    path.end_figure(false);
    path.dispose();
    dc.draw_geometry(None, Some(&pen(brush, thickness)), &geometry.upcast());
}

// Android's circular waveform is a normalized rounded star, and intermediate
// amplitudes are corner-matched Morphs from an equally segmented rounded circle.
struct CircularShapes {
    count: usize,
    circle: Polygon,
    star: Polygon,
    morph: Option<Morph>,
    paths: Option<CircularPaths>,
}
struct CircularPaths {
    size: Size,
    thickness: f64,
    amplitude: f64,
    active: PathMeasure,
    track: PathMeasure,
}
impl CircularShapes {
    fn new(count: usize) -> Self {
        Self {
            count,
            circle: Polygon::circle(count).normalized(),
            star: Polygon::star(
                count,
                0.75,
                Rounding::new(0.35, 0.4),
                Rounding::new(0.5, 0.0),
            )
            .normalized(),
            morph: None,
            paths: None,
        }
    }
    fn paths(&mut self, size: Size, thickness: f64, amplitude: f64) -> &CircularPaths {
        if self
            .paths
            .as_ref()
            .is_some_and(|p| p.size == size && p.thickness == thickness && p.amplitude == amplitude)
        {
            return self.paths.as_ref().unwrap();
        }
        if amplitude > 0.0 && amplitude < 1.0 && self.morph.is_none() {
            self.morph = Some(Morph::new(&self.circle, &self.star));
        }
        let (active, pivot) = if let Some(morph) = &self.morph {
            (morph.cubics(amplitude), V::new(0.5, 0.5))
        } else if amplitude == 1.0 {
            (self.star.cubics(), self.star.center)
        } else {
            (self.circle.cubics(), self.circle.center)
        };
        self.paths = Some(CircularPaths {
            size,
            thickness,
            amplitude,
            active: PathMeasure::new(prepare_circular_path(active, pivot, size, thickness)),
            track: PathMeasure::new(prepare_circular_path(
                self.circle.cubics(),
                self.circle.center,
                size,
                thickness,
            )),
        });
        self.paths.as_ref().unwrap()
    }
}
fn prepare_circular_path(cubics: Vec<Cubic>, pivot: V, size: Size, thickness: f64) -> Vec<Cubic> {
    let anchor = cubics[0].0[0] - pivot;
    let rotation = 3.0 * PI / 2.0 - anchor.y.atan2(anchor.x);
    let mut cubics: Vec<_> = cubics
        .into_iter()
        .map(|c| {
            c.transformed(|p| {
                let p = p.rotate(rotation);
                V::new(
                    p.x * (size.width - thickness),
                    p.y * (size.height - thickness),
                )
            })
        })
        .collect();
    let bounds = path_bounds(&cubics);
    let offset = V::new(
        (size.width - bounds[0] - bounds[2]) / 2.0,
        (size.height - bounds[1] - bounds[3]) / 2.0,
    );
    for cubic in &mut cubics {
        *cubic = cubic.transformed(|p| p + offset);
    }
    cubics
}
fn draw_cubic_stroke(
    dc: &mut DrawingContext,
    brush: Option<&Rc<dyn IBrush>>,
    cubics: &[Cubic],
    thickness: f64,
    pivot: V,
    rotation: f64,
) {
    if brush.is_none() || cubics.is_empty() {
        return;
    }
    let point = |p: V| {
        let p = (p - pivot).rotate(rotation) + pivot;
        Point::new(p.x, p.y)
    };
    let geometry = StreamGeometry::new();
    let mut path = geometry.open();
    path.begin_figure(point(cubics[0].0[0]), false);
    for cubic in cubics {
        path.cubic_bezier_to(
            point(cubic.0[1]),
            point(cubic.0[2]),
            point(cubic.0[3]),
            true,
        );
    }
    path.end_figure(false);
    path.dispose();
    dc.draw_geometry(None, Some(&pen(brush, thickness)), &geometry.upcast());
}
fn draw_arc(
    dc: &mut DrawingContext,
    brush: Option<&Rc<dyn IBrush>>,
    center: Point,
    radius: f64,
    start: f64,
    sweep: f64,
    thickness: f64,
) {
    if brush.is_none() || sweep <= 0.0 {
        return;
    }
    let geometry = StreamGeometry::new();
    let mut path = geometry.open();
    let point = |angle: f64| {
        Point::new(
            center.x + radius * angle.cos(),
            center.y + radius * angle.sin(),
        )
    };
    path.begin_figure(point(start), false);
    if sweep >= TAU {
        path.arc_to(
            point(start + PI),
            Size::new(radius, radius),
            0.0,
            false,
            SweepDirection::Clockwise,
            true,
        );
        path.arc_to(
            point(start + TAU),
            Size::new(radius, radius),
            0.0,
            false,
            SweepDirection::Clockwise,
            true,
        );
        path.end_figure(true);
    } else {
        path.arc_to(
            point(start + sweep),
            Size::new(radius, radius),
            0.0,
            sweep > PI,
            SweepDirection::Clockwise,
            true,
        );
        path.end_figure(false);
    }
    path.dispose();
    dc.draw_geometry(None, Some(&pen(brush, thickness)), &geometry.upcast());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linear_segments_remain_ordered_and_repeat() {
        for step in 0..=3500 {
            let time = step as f64 / 1000.0;
            for (start, end) in linear_indeterminate_segments(time) {
                assert!((0.0..=1.0).contains(&start));
                assert!((start..=1.0).contains(&end));
            }
        }
        assert_eq!([(0.0, 0.0); 2], linear_indeterminate_segments(0.0));
        let a = linear_indeterminate_segments(0.5);
        let b = linear_indeterminate_segments(2.25);
        assert_eq!(a, b);
        assert_eq!(0.0, a[1].1);
    }
    #[test]
    fn circular_keyframes_match_source_landmarks() {
        assert_eq!((0.0, 0.1), circular_indeterminate_frame(0.0));
        let (rotation, progress) = circular_indeterminate_frame(3.0);
        assert!((rotation - 360.0).abs() < 1e-9);
        assert!((progress - 0.87).abs() < 1e-9);
        assert_eq!(
            circular_indeterminate_frame(0.0),
            circular_indeterminate_frame(6.0)
        );
    }
    #[test]
    fn quadratic_wave_arc_inverse_preserves_native_path_measure_parameterization() {
        let length = quadratic_wave_arc(1.0, 20.0, 3.0);
        assert!(length > 20.0);
        for i in 0..=100 {
            let fraction = i as f64 / 100.0;
            let t = wave_parameter(fraction, 20.0, 3.0);
            assert!((quadratic_wave_arc(t, 20.0, 3.0) / length - fraction).abs() < 1e-8);
        }
        assert!((wave_parameter(0.5, 20.0, 3.0) - 0.5).abs() < 1e-8);
    }
    #[test]
    fn progress_and_wave_extremes_are_safe() {
        assert_eq!(0.0, normalized_progress(f64::NAN));
        assert_eq!(1.0, normalized_progress(f64::INFINITY));
        assert_eq!(0.0, normalized_progress(-10.0));
        assert_eq!(1.0, normalized_progress(110.0));
        assert_eq!(0.0, wave_amplitude(0.1));
        assert_eq!(1.0, wave_amplitude(0.5));
        assert_eq!(0.0, wave_amplitude(0.95));
    }
}
