use super::ink_stencil_input::InkStencilInput;
use super::*;
use ferroui_controls::{Control, ControlImpl};
use ferroui_base::input::{
    IPointer, InputElement, InputElementImpl, InputElementImplExt, PointerCaptureLostEventArgs,
    PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{
    Color, DrawingContext, FillRule, FlowDirection, FormattedText, Geometry, IBrush, IPen,
    StreamGeometry, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

struct CachedGeometry {
    polygons: Rc<Vec<Vec<Point>>>,
    geometry: Ref<Geometry>,
}

struct CachedStencilLabel {
    tenths: u64,
    color: Color,
    text: Rc<FormattedText>,
}

/// A pressure-sensitive ink surface that renders entirely through FerroUI.
#[repr(C)]
pub struct InkCanvas {
    base: Control,
    presenter: Rc<InkPresenter>,
    geometries: RefCell<BTreeMap<u32, CachedGeometry>>,
    captured: RefCell<BTreeMap<i32, Rc<dyn IPointer>>>,
    stencil_input: RefCell<InkStencilInput>,
    stencil_labels: RefCell<[Option<CachedStencilLabel>; 2]>,
    color_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(InkCanvas: Control);
ferroui_base::ferro_class_info!(InkCanvas {
    new: InkCanvas::new,
    markup: { properties: [InkPresenter: Rc<InkPresenter> { get: InkCanvas::ink_presenter }] },
});
ferro_impl_classes!(InkCanvas: StyledElementImpl, InteractiveImpl);

impl FerroObjectImpl for InkCanvas {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        this.presenter.changed(move || {
            if let Some(canvas) = weak.upgrade() {
                canvas
                    .stencil_input
                    .borrow_mut()
                    .reconcile(&canvas.presenter);
                canvas.release_inactive_pointers();
                canvas.invalidate_visual();
            }
        });
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == InputElement::is_effectively_enabled_property().as_property()
            && !this.is_effectively_enabled()
        {
            this.stencil_input.borrow_mut().clear();
            this.presenter.cancel_all();
        }
    }
}

impl ControlImpl for InkCanvas {
    fn on_create_automation_peer(this: &Self) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        InkCanvasAutomationPeer::new(this).upcast()
    }
}

impl LayoutableImpl for InkCanvas {
    fn measure_override(_this: &Self, _available_size: Size) -> Size {
        Size::default()
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.presenter.ruler().initialize_position(final_size);
        this.presenter.protractor().initialize_position(final_size);
        this.presenter.set_size(final_size);
        final_size
    }
}

impl VisualImpl for InkCanvas {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        if let Some(settings) = this
            .presentation_source()
            .and_then(|source| source.platform_settings())
        {
            let weak = this.to_ref().downgrade();
            let subscription = settings.color_values_changed(Rc::new(move |_| {
                if let Some(canvas) = weak.upgrade() {
                    canvas.invalidate_visual();
                }
            }));
            let old = this.color_subscription.replace(Some(subscription));
            if let Some(old) = old {
                old.dispose();
            }
        }
    }
    fn render(this: &Self, context: &mut DrawingContext) {
        // A transparent draw establishes the hit-test surface even when it is empty.
        let transparent: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(0));
        let bounds = Rect::from_size(this.bounds().size());
        context.draw_rectangle(
            Some(&transparent),
            None,
            bounds,
            0.0,
            0.0,
            &Default::default(),
        );
        let clip = context.push_clip(bounds);
        let strokes = this.presenter.visible_strokes();
        let ids: BTreeSet<_> = strokes.iter().map(|s| s.id()).collect();
        this.geometries
            .borrow_mut()
            .retain(|id, _| ids.contains(id));
        for stroke in &strokes {
            this.render_stroke(context, stroke);
        }
        this.render_stencils(context);
        context.pop(clip);
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.stencil_input.borrow_mut().clear();
        this.presenter.cancel_all();
        let subscription = this.color_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        this.geometries.borrow_mut().clear();
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl InputElementImpl for InkCanvas {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if e.handled() || !this.is_effectively_enabled() {
            return;
        }
        let point = e.get_current_point(Some(this));
        let stencil = this.stencil_input.borrow_mut().press(
            &this.presenter,
            e.pointer().id(),
            e.pointer().type_(),
            point.position,
            point.properties,
            e.key_modifiers(),
        );
        if stencil {
            this.captured
                .borrow_mut()
                .insert(e.pointer().id(), e.pointer().clone());
            e.pointer()
                .capture(Some(&this.to_ref().upcast::<InputElement>()));
            e.prevent_gesture_recognition();
            e.set_handled(true);
            this.invalidate_visual();
            return;
        }
        if this.feed(InkPointerPhase::Pressed, e) {
            if this.presenter.has_active_pointer(e.pointer().id()) {
                this.captured
                    .borrow_mut()
                    .insert(e.pointer().id(), e.pointer().clone());
                e.pointer()
                    .capture(Some(&this.to_ref().upcast::<InputElement>()));
            }
            e.prevent_gesture_recognition();
            e.set_handled(true);
        }
    }
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        if !this.is_effectively_enabled() {
            return;
        }
        if this.stencil_input.borrow().contains(e.pointer().id()) {
            this.move_stencil(e);
            e.prevent_gesture_recognition();
            e.set_handled(true);
            return;
        }
        let mut handled = false;
        for point in e.get_intermediate_points(Some(this)) {
            let args = Self::sample(e, point.position, point.properties);
            match this
                .presenter
                .process_pointer(InkPointerPhase::Moved, &args)
            {
                Ok(value) => handled |= value,
                Err(_) => {
                    this.presenter.cancel_pointer(e.pointer().id());
                    break;
                }
            }
        }
        if handled {
            e.prevent_gesture_recognition();
            e.set_handled(true);
        }
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if this.stencil_input.borrow().contains(e.pointer().id()) {
            this.move_stencil(e);
            this.stencil_input
                .borrow_mut()
                .release(&this.presenter, e.pointer().id());
            this.release_inactive_pointers();
            this.invalidate_visual();
            e.prevent_gesture_recognition();
            e.set_handled(true);
            return;
        }
        if this.feed(InkPointerPhase::Released, e) {
            e.set_handled(true);
        }
        if e.pointer()
            .captured()
            .is_some_and(|v| v == this.to_ref().upcast::<InputElement>())
        {
            e.pointer().capture(None);
        }
    }
    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);
        if this.feed(InkPointerPhase::Entered, e) {
            e.set_handled(true);
        }
    }
    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        if this.feed(InkPointerPhase::Exited, e) {
            e.set_handled(true);
        }
    }
    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        this.captured.borrow_mut().remove(&e.pointer().id());
        this.stencil_input
            .borrow_mut()
            .release(&this.presenter, e.pointer().id());
        this.presenter.cancel_pointer(e.pointer().id());
        this.invalidate_visual();
        Self::parent_on_pointer_capture_lost(this, e);
    }
}

impl InkCanvas {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            presenter: InkPresenter::new(),
            geometries: RefCell::new(BTreeMap::new()),
            captured: RefCell::new(BTreeMap::new()),
            stencil_input: RefCell::new(InkStencilInput::default()),
            stencil_labels: RefCell::new([None, None]),
            color_subscription: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn ink_presenter(&self) -> Rc<InkPresenter> {
        self.presenter.clone()
    }

    fn release_inactive_pointers(&self) {
        let stale: Vec<_> = self
            .captured
            .borrow()
            .iter()
            .filter(|(id, _)| {
                !self.presenter.has_active_pointer(**id)
                    && !self.stencil_input.borrow().contains(**id)
            })
            .map(|(id, pointer)| (*id, pointer.clone()))
            .collect();
        {
            let mut captured = self.captured.borrow_mut();
            for (id, _) in &stale {
                captured.remove(id);
            }
        }
        for (_, pointer) in stale {
            if pointer
                .captured()
                .is_some_and(|owner| owner == self.to_ref().upcast::<InputElement>())
            {
                pointer.capture(None);
            }
        }
    }

    fn move_stencil(&self, event: &PointerEventArgs) {
        let update = self.stencil_input.borrow_mut().moved(
            &self.presenter,
            event.pointer().id(),
            event.get_position(Some(self)),
        );
        if let Some(update) = update {
            if update.apply(&self.presenter).is_err() {
                self.stencil_input
                    .borrow_mut()
                    .release(&self.presenter, event.pointer().id());
            }
        }
        self.release_inactive_pointers();
        self.invalidate_visual();
    }

    fn sample(
        e: &PointerEventArgs,
        position: Point,
        properties: ferroui_base::input::PointerPointProperties,
    ) -> InkPointerEventArgs {
        let mut sample = InkPointerEventArgs::new(
            e.pointer().id(),
            e.pointer().type_(),
            InkPoint {
                position,
                pressure: properties.pressure,
                tilt_x: properties.x_tilt,
                tilt_y: properties.y_tilt,
                // FerroUI's platform-neutral timestamps are milliseconds. InkPoint
                // stores microseconds; coalesced points share the batch timestamp.
                twist: properties.twist,
                timestamp: e.timestamp().saturating_mul(1000),
            },
        );
        sample.is_eraser = properties.is_eraser || properties.is_inverted;
        sample.is_barrel_button_pressed = properties.is_barrel_button_pressed;
        sample.is_right_button_pressed = properties.is_right_button_pressed;
        sample
    }
    fn feed(&self, phase: InkPointerPhase, e: &PointerEventArgs) -> bool {
        let point = e.get_current_point(Some(self));
        let sample = Self::sample(e, point.position, point.properties);
        match self.presenter.process_pointer(phase, &sample) {
            Ok(handled) => handled,
            Err(_) => {
                self.presenter.cancel_pointer(e.pointer().id());
                false
            }
        }
    }
    fn render_stroke(&self, context: &mut DrawingContext, stroke: &Rc<InkStroke>) {
        let polygons = stroke.outline_polygons();
        let cached = self
            .geometries
            .borrow()
            .get(&stroke.id())
            .filter(|cached| Rc::ptr_eq(&cached.polygons, &polygons))
            .map(|cached| cached.geometry.clone());
        let geometry = cached.unwrap_or_else(|| {
            let geometry = Self::create_geometry(&polygons);
            self.geometries.borrow_mut().insert(
                stroke.id(),
                CachedGeometry {
                    polygons,
                    geometry: geometry.clone(),
                },
            );
            geometry
        });
        let attributes = stroke.drawing_attributes();
        let mut color = attributes.color;
        if let Some((background, system)) =
            super::color_names::high_contrast_colors(self, attributes.draw_as_highlighter)
        {
            color = super::color_names::adjust_high_contrast_color(
                color,
                background,
                system,
                self.presenter.high_contrast_adjustment(),
            );
        }
        // One nonzero-filled geometry per stroke: overlapping segments of a
        // translucent highlighter never accumulate opacity within that stroke.
        let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::with_opacity(
            color,
            (if attributes.draw_as_highlighter {
                0.5
            } else {
                1.0
            }) * (if attributes.kind == InkDrawingAttributesKind::Pencil {
                f64::from(attributes.pencil_opacity)
            } else {
                1.0
            }),
        ));
        context.draw_geometry(Some(&brush), None, &geometry);
    }
    fn create_geometry(polygons: &[Vec<Point>]) -> Ref<Geometry> {
        let geometry = StreamGeometry::new();
        let mut builder = geometry.open();
        builder.set_fill_rule(FillRule::NonZero);
        for polygon in polygons {
            if let Some(first) = polygon.first() {
                builder.begin_figure(*first, true);
                for point in &polygon[1..] {
                    builder.line_to(*point, false);
                }
                builder.end_figure(true);
            }
        }
        builder.dispose();
        geometry.upcast()
    }
    fn render_stencils(&self, context: &mut DrawingContext) {
        let ruler = self.presenter.ruler();
        if ruler.is_visible() {
            let fill: Rc<dyn IBrush> =
                Rc::new(ImmutableSolidColorBrush::new(ruler.background_color()));
            let foreground = Rc::new(ImmutableSolidColorBrush::new(ruler.foreground_color()));
            let scale = Self::stencil_scale(ruler.transform());
            let pen: Rc<dyn IPen> =
                Rc::new(ImmutablePen::with_brush(Some(foreground), 1.0 / scale));
            let opacity =
                context.push_opacity(self.stencil_opacity(InkPresenterStencilKind::Ruler));
            let state = context.push_transform(ruler.transform());
            context.draw_rectangle(
                Some(&fill),
                Some(&pen),
                Rect::new(0.0, 0.0, ruler.length(), ruler.width()),
                0.0,
                0.0,
                &Default::default(),
            );
            if ruler.are_tick_marks_visible() {
                for index in 0..=(ruler.length() / 10.0) as usize {
                    let x = index as f64 * 10.0;
                    let scale_y = ruler
                        .transform()
                        .m21
                        .hypot(ruler.transform().m22)
                        .max(1.0e-3);
                    let length = ((if index % 5 == 0 { 12.0_f64 } else { 6.0 }) / scale_y)
                        .min(ruler.width() * 0.3);
                    context.draw_line(&pen, Point::new(x, 0.0), Point::new(x, length));
                    context.draw_line(
                        &pen,
                        Point::new(x, ruler.width()),
                        Point::new(x, ruler.width() - length),
                    );
                }
            }
            context.pop(state);
            if ruler.is_compass_visible() {
                let label = self.stencil_label(0, ruler.angle(), ruler.foreground_color());
                let center = ruler.center() * ruler.transform();
                context.draw_text(
                    &label,
                    Point::new(
                        center.x - label.width() * 0.5,
                        center.y - label.height() * 0.5,
                    ),
                );
            }
            context.pop(opacity);
        }
        let protractor = self.presenter.protractor();
        if protractor.is_visible() {
            let fill: Rc<dyn IBrush> =
                Rc::new(ImmutableSolidColorBrush::new(protractor.background_color()));
            let foreground = Rc::new(ImmutableSolidColorBrush::new(protractor.foreground_color()));
            let accent = Rc::new(ImmutableSolidColorBrush::new(protractor.accent_color()));
            let scale = Self::stencil_scale(protractor.transform());
            let pen: Rc<dyn IPen> =
                Rc::new(ImmutablePen::with_brush(Some(foreground), 1.0 / scale));
            let accent_pen: Rc<dyn IPen> =
                Rc::new(ImmutablePen::with_brush(Some(accent), 1.5 / scale));
            let opacity =
                context.push_opacity(self.stencil_opacity(InkPresenterStencilKind::Protractor));
            let state = context.push_transform(protractor.transform());
            let radius = protractor.radius();
            let center = protractor.center();
            context.draw_ellipse(
                Some(&fill),
                Some(&pen),
                Rect::new(0.0, 0.0, radius * 2.0, radius * 2.0),
            );
            let radial_point = |angle: f64, distance: f64| {
                Point::new(
                    center.x + angle.cos() * distance,
                    center.y + angle.sin() * distance,
                )
            };
            if protractor.are_tick_marks_visible() {
                for tick in 0..72 {
                    let angle = f64::from(tick * 5).to_radians();
                    let length =
                        ((if tick % 3 == 0 { 12.0_f64 } else { 6.0 }) / scale).min(radius * 0.2);
                    context.draw_line(
                        &pen,
                        radial_point(angle, radius - length),
                        radial_point(angle, radius),
                    );
                }
            }
            let mark_length = (18.0 / scale).min(radius * 0.3);
            for angle in [0.0, std::f64::consts::PI] {
                context.draw_line(
                    &accent_pen,
                    radial_point(angle, radius - mark_length),
                    radial_point(angle, radius),
                );
            }
            if protractor.is_center_marker_visible() {
                let half = (8.0 / scale).min(radius * 0.2);
                context.draw_line(
                    &pen,
                    Point::new(center.x - half, center.y),
                    Point::new(center.x + half, center.y),
                );
                context.draw_line(
                    &pen,
                    Point::new(center.x, center.y - half),
                    Point::new(center.x, center.y + half),
                );
            }
            if protractor.are_rays_visible() {
                if let Some((start, end)) = protractor.rays() {
                    context.draw_line(&accent_pen, center, radial_point(start, radius));
                    context.draw_line(&accent_pen, center, radial_point(end, radius));
                }
            }
            context.pop(state);
            if protractor.is_angle_readout_visible() {
                if let Some(angle) = protractor.angle() {
                    let label = self.stencil_label(1, angle, protractor.foreground_color());
                    let center = center * protractor.transform();
                    context.draw_text(
                        &label,
                        Point::new(center.x - label.width() * 0.5, center.y + 12.0),
                    );
                }
            }
            context.pop(opacity);
        }
    }

    fn stencil_scale(transform: ferroui_base::Matrix) -> f64 {
        ((transform.m11.hypot(transform.m12) + transform.m21.hypot(transform.m22)) * 0.5)
            .max(1.0e-3)
    }

    fn stencil_opacity(&self, kind: InkPresenterStencilKind) -> f64 {
        if self.stencil_input.borrow().is_manipulating(kind) {
            0.5
        } else {
            0.75
        }
    }

    fn stencil_label(&self, index: usize, angle: f64, color: Color) -> Rc<FormattedText> {
        let tenths = (angle * 10.0).round() as u64;
        let cached = self.stencil_labels.borrow()[index]
            .as_ref()
            .filter(|v| v.tenths == tenths && v.color == color)
            .map(|v| v.text.clone());
        if let Some(cached) = cached {
            return cached;
        }
        let text = Rc::new(FormattedText::new(
            &format!("{}.{:01}°", tenths / 10, tenths % 10),
            CultureInfo::current_culture(),
            FlowDirection::LeftToRight,
            Typeface::default_typeface(),
            14.0,
            Some(Rc::new(ImmutableSolidColorBrush::new(color))),
        ));
        self.stencil_labels.borrow_mut()[index] = Some(CachedStencilLabel {
            tenths,
            color,
            text: text.clone(),
        });
        text
    }
}
