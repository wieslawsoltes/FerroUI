//! Embeddable FerroUI custom Visual Controls styled with the host's stock FerroUI theme:
//! - [`GraphMapperControl`]: Interactive Bezier / Sine / Parabola / Gaussian curve shaper
//! - [`MdSliderControl`]: Interactive 2D XY Multi-Dimensional joystick pad (`[0..1] × [0..1]`)

use crate::types::GraphCurveKind;
use ferroui_base::input::{
    InputElement, InputElementImpl, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::{ImmutableDashStyle, ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{
    BoxShadows, DrawingContext, IBrush, IImmutableBrush, IPen, PenLineCap, StreamGeometry,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect,
    Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Application, Control, ControlImpl};
use ferroui_view::ViewClass;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn is_control_dark(ctrl: &Control) -> bool {
    ctrl.get_value(ThemeVariant::actual_theme_variant_property())
        .or_else(|| Application::current().and_then(|app| app.actual_theme_variant()))
        .is_some_and(|v| v == ThemeVariant::dark())
}

// =============================================================================
// 1. Interactive Graph Mapper Control
// =============================================================================

#[repr(C)]
pub struct GraphMapperControl {
    base: Control,
    curve: Cell<GraphCurveKind>,
    p1: Cell<[f64; 2]>,
    p2: Cell<[f64; 2]>,
    dragging_handle: Cell<Option<u8>>,
    on_changed: RefCell<Option<Rc<dyn Fn(GraphCurveKind, [f64; 2], [f64; 2])>>>,
    bg_light: Rc<dyn IBrush>,
    bg_dark: Rc<dyn IBrush>,
    grid_light: Rc<dyn IPen>,
    grid_dark: Rc<dyn IPen>,
    border_light: Rc<dyn IPen>,
    border_dark: Rc<dyn IPen>,
    curve_light: Rc<dyn IPen>,
    curve_dark: Rc<dyn IPen>,
    guide_pen: Rc<dyn IPen>,
    handle_fill: Rc<dyn IBrush>,
    handle1_ring: Rc<dyn IPen>,
    handle2_ring: Rc<dyn IPen>,
}

ferro_class!(GraphMapperControl: Control);
ferro_impl_classes!(
    GraphMapperControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl
);
ferro_class_info!(GraphMapperControl { new: GraphMapperControl::new });

impl ViewClass for GraphMapperControl {
    fn create() -> Ref<Self> {
        Self::new()
    }
}

impl GraphMapperControl {
    pub fn construct() -> Self {
        let dash = Rc::new(ImmutableDashStyle::new(Some(&[3.0, 2.5]), 0.0));
        let guide_br: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(0xAA787878));
        Self {
            base: Control::construct(),
            curve: Cell::new(GraphCurveKind::Bezier),
            p1: Cell::new([0.25, 0.80]),
            p2: Cell::new([0.75, 0.20]),
            dragging_handle: Cell::new(None),
            on_changed: RefCell::new(None),
            bg_light: Rc::new(ImmutableSolidColorBrush::from_uint32(0x14000000)),
            bg_dark: Rc::new(ImmutableSolidColorBrush::from_uint32(0x24000000)),
            grid_light: Rc::new(ImmutablePen::from_uint32(0x28000000, 1.0)),
            grid_dark: Rc::new(ImmutablePen::from_uint32(0x24FFFFFF, 1.0)),
            border_light: Rc::new(ImmutablePen::from_uint32(0x44000000, 1.0)),
            border_dark: Rc::new(ImmutablePen::from_uint32(0x38FFFFFF, 1.0)),
            curve_light: Rc::new(ImmutablePen::from_uint32(0xFF1F1F1F, 2.2)),
            curve_dark: Rc::new(ImmutablePen::from_uint32(0xFF60CDFF, 2.2)),
            guide_pen: Rc::new(ImmutablePen::new(Some(guide_br), 1.1, Some(dash), PenLineCap::Round, Default::default(), 4.0)),
            handle_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFFFFFFFF)),
            handle1_ring: Rc::new(ImmutablePen::from_uint32(0xFF28A745, 2.2)),
            handle2_ring: Rc::new(ImmutablePen::from_uint32(0xFFD9383A, 2.2)),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_clip_to_bounds(true);
        this
    }

    pub fn configure(
        &self,
        curve: GraphCurveKind,
        p1: [f64; 2],
        p2: [f64; 2],
        on_changed: Rc<dyn Fn(GraphCurveKind, [f64; 2], [f64; 2])>,
    ) {
        self.curve.set(curve);
        self.p1.set(p1);
        self.p2.set(p2);
        *self.on_changed.borrow_mut() = Some(on_changed);
        self.invalidate_visual();
    }

    pub fn set_curve(&self, curve: GraphCurveKind) {
        self.curve.set(curve);
        self.invalidate_visual();
    }

    fn norm_to_px(&self, p: [f64; 2]) -> Point {
        let b = self.bounds();
        let pad = 8.0;
        let w = (b.width - pad * 2.0).max(1.0);
        let h = (b.height - pad * 2.0).max(1.0);
        Point::new(pad + p[0].clamp(0.0, 1.0) * w, pad + (1.0 - p[1].clamp(0.0, 1.0)) * h)
    }

    fn px_to_norm(&self, pt: Point) -> [f64; 2] {
        let b = self.bounds();
        let pad = 8.0;
        let w = (b.width - pad * 2.0).max(1.0);
        let h = (b.height - pad * 2.0).max(1.0);
        [
            ((pt.x - pad) / w).clamp(0.02, 0.98),
            (1.0 - (pt.y - pad) / h).clamp(0.02, 0.98),
        ]
    }
}

impl InputElementImpl for GraphMapperControl {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        e.set_handled(true);
        let input: Ref<InputElement> = this.to_ref().upcast();
        e.pointer().capture(Some(&input));
        let pt = e.get_position(Some(this));
        let h1 = this.norm_to_px(this.p1.get());
        let h2 = this.norm_to_px(this.p2.get());
        let d1 = (pt.x - h1.x).powi(2) + (pt.y - h1.y).powi(2);
        let d2 = (pt.x - h2.x).powi(2) + (pt.y - h2.y).powi(2);
        let pick = if d1 <= d2 { 1 } else { 2 };
        this.dragging_handle.set(Some(pick));
        let n = this.px_to_norm(pt);
        if pick == 1 {
            this.p1.set(n);
        } else {
            this.p2.set(n);
        }
        if let Some(cb) = this.on_changed.borrow().as_ref() {
            cb(this.curve.get(), this.p1.get(), this.p2.get());
        }
        this.invalidate_visual();
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        if let Some(pick) = this.dragging_handle.get() {
            e.set_handled(true);
            let n = this.px_to_norm(e.get_position(Some(this)));
            if pick == 1 {
                this.p1.set(n);
            } else {
                this.p2.set(n);
            }
            if let Some(cb) = this.on_changed.borrow().as_ref() {
                cb(this.curve.get(), this.p1.get(), this.p2.get());
            }
            this.invalidate_visual();
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if this.dragging_handle.take().is_some() {
            e.set_handled(true);
            e.pointer().capture(None);
        }
    }
}

impl VisualImpl for GraphMapperControl {
    fn render(this: &Self, dc: &mut DrawingContext) {
        let b = this.bounds();
        let w = b.width.max(20.0);
        let h = b.height.max(20.0);
        let is_light = !is_control_dark(this);
        let bg = if is_light { &this.bg_light } else { &this.bg_dark };
        let grid_pen = if is_light { &this.grid_light } else { &this.grid_dark };
        let border_pen = if is_light { &this.border_light } else { &this.border_dark };
        let curve_pen = if is_light { &this.curve_light } else { &this.curve_dark };

        dc.draw_rectangle(Some(bg), Some(border_pen), Rect::new(0.0, 0.0, w, h), 4.0, 4.0, &BoxShadows::default());

        // Crisp 4x4 Grasshopper drafting grid
        for i in 1..4 {
            let t = i as f64 / 4.0;
            let gx = 8.0 + (w - 16.0) * t;
            let gy = 8.0 + (h - 16.0) * t;
            dc.draw_line(grid_pen, Point::new(gx, 8.0), Point::new(gx, h - 8.0));
            dc.draw_line(grid_pen, Point::new(8.0, gy), Point::new(w - 8.0, gy));
        }

        let curve = this.curve.get();
        let p1 = this.p1.get();
        let p2 = this.p2.get();

        // Sampled smooth curve
        let geom = StreamGeometry::new();
        {
            let mut ctx = geom.open();
            for step in 0..=40 {
                let x = step as f64 / 40.0;
                let y = curve.evaluate(x, p1, p2);
                let pt = this.norm_to_px([x, y]);
                if step == 0 {
                    ctx.begin_figure(pt, false);
                } else {
                    ctx.line_to(pt, true);
                }
            }
            ctx.end_figure(false);
            ctx.dispose();
        }
        dc.draw_geometry(None, Some(curve_pen), &geom.upcast());

        let hp1 = this.norm_to_px(p1);
        let hp2 = this.norm_to_px(p2);
        if curve == GraphCurveKind::Bezier {
            dc.draw_line(&this.guide_pen, this.norm_to_px([0.0, 0.0]), hp1);
            dc.draw_line(&this.guide_pen, this.norm_to_px([1.0, 1.0]), hp2);
        }
        dc.draw_ellipse_at(Some(&this.handle_fill), Some(&this.handle1_ring), hp1, 5.2, 5.2);
        dc.draw_ellipse_at(Some(&this.handle_fill), Some(&this.handle2_ring), hp2, 5.2, 5.2);
    }
}

// =============================================================================
// 2. Authentic Rhino Grasshopper 2D MD Slider Control
// =============================================================================

#[repr(C)]
pub struct MdSliderControl {
    base: Control,
    pos: Cell<[f64; 2]>,
    dragging: Cell<bool>,
    on_changed: RefCell<Option<Rc<dyn Fn(f64, f64)>>>,
    bg_light: Rc<dyn IBrush>,
    bg_dark: Rc<dyn IBrush>,
    grid_light: Rc<dyn IPen>,
    grid_dark: Rc<dyn IPen>,
    cross_light: Rc<dyn IPen>,
    cross_dark: Rc<dyn IPen>,
    border_light: Rc<dyn IPen>,
    border_dark: Rc<dyn IPen>,
    puck_glow: Rc<dyn IBrush>,
    puck_core: Rc<dyn IBrush>,
    ring_pen: Rc<dyn IPen>,
}

ferro_class!(MdSliderControl: Control);
ferro_impl_classes!(
    MdSliderControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl
);
ferro_class_info!(MdSliderControl { new: MdSliderControl::new });

impl ViewClass for MdSliderControl {
    fn create() -> Ref<Self> {
        Self::new()
    }
}

impl MdSliderControl {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            pos: Cell::new([0.5, 0.5]),
            dragging: Cell::new(false),
            on_changed: RefCell::new(None),
            bg_light: Rc::new(ImmutableSolidColorBrush::from_uint32(0x14000000)),
            bg_dark: Rc::new(ImmutableSolidColorBrush::from_uint32(0x24000000)),
            grid_light: Rc::new(ImmutablePen::from_uint32(0x28000000, 1.0)),
            grid_dark: Rc::new(ImmutablePen::from_uint32(0x24FFFFFF, 1.0)),
            cross_light: Rc::new(ImmutablePen::from_uint32(0xBB2E2E2E, 1.2)),
            cross_dark: Rc::new(ImmutablePen::from_uint32(0x9960CDFF, 1.2)),
            border_light: Rc::new(ImmutablePen::from_uint32(0x44000000, 1.0)),
            border_dark: Rc::new(ImmutablePen::from_uint32(0x38FFFFFF, 1.0)),
            puck_glow: Rc::new(ImmutableSolidColorBrush::from_uint32(0x38D9383A)),
            puck_core: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFFD9383A)),
            ring_pen: Rc::new(ImmutablePen::from_uint32(0xFFFFFFFF, 1.6)),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_clip_to_bounds(true);
        this
    }

    pub fn configure(&self, x: f64, y: f64, on_changed: Rc<dyn Fn(f64, f64)>) {
        self.pos.set([x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)]);
        *self.on_changed.borrow_mut() = Some(on_changed);
        self.invalidate_visual();
    }

    fn update_from_px(&self, pt: Point) {
        let b = self.bounds();
        let pad = 7.0;
        let w = (b.width - pad * 2.0).max(1.0);
        let h = (b.height - pad * 2.0).max(1.0);
        let nx = ((pt.x - pad) / w).clamp(0.0, 1.0);
        let ny = ((pt.y - pad) / h).clamp(0.0, 1.0);
        self.pos.set([nx, ny]);
        if let Some(cb) = self.on_changed.borrow().as_ref() {
            cb(nx, ny);
        }
        self.invalidate_visual();
    }
}

impl InputElementImpl for MdSliderControl {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        e.set_handled(true);
        let input: Ref<InputElement> = this.to_ref().upcast();
        e.pointer().capture(Some(&input));
        this.dragging.set(true);
        this.update_from_px(e.get_position(Some(this)));
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        if this.dragging.get() {
            e.set_handled(true);
            this.update_from_px(e.get_position(Some(this)));
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if this.dragging.replace(false) {
            e.set_handled(true);
            e.pointer().capture(None);
        }
    }
}

impl VisualImpl for MdSliderControl {
    fn render(this: &Self, dc: &mut DrawingContext) {
        let b = this.bounds();
        let w = b.width.max(20.0);
        let h = b.height.max(20.0);
        let is_light = !is_control_dark(this);
        let bg = if is_light { &this.bg_light } else { &this.bg_dark };
        let grid_pen = if is_light { &this.grid_light } else { &this.grid_dark };
        let cross_pen = if is_light { &this.cross_light } else { &this.cross_dark };
        let border_pen = if is_light { &this.border_light } else { &this.border_dark };

        dc.draw_rectangle(Some(bg), Some(border_pen), Rect::new(0.0, 0.0, w, h), 4.0, 4.0, &BoxShadows::default());

        for i in 1..4 {
            let t = i as f64 / 4.0;
            let gx = 7.0 + (w - 14.0) * t;
            let gy = 7.0 + (h - 14.0) * t;
            dc.draw_line(grid_pen, Point::new(gx, 7.0), Point::new(gx, h - 7.0));
            dc.draw_line(grid_pen, Point::new(7.0, gy), Point::new(w - 7.0, gy));
        }

        let [nx, ny] = this.pos.get();
        let px = 7.0 + nx * (w - 14.0);
        let py = 7.0 + ny * (h - 14.0);
        dc.draw_line(cross_pen, Point::new(px, 7.0), Point::new(px, h - 7.0));
        dc.draw_line(cross_pen, Point::new(7.0, py), Point::new(w - 7.0, py));
        dc.draw_ellipse_at(Some(&this.puck_glow), None, Point::new(px, py), 10.0, 10.0);
        dc.draw_ellipse_at(Some(&this.puck_core), Some(&this.ring_pen), Point::new(px, py), 5.0, 5.0);
    }
}
