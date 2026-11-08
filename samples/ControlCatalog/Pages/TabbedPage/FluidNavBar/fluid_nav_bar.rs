//! Port of `Pages/TabbedPage/FluidNavBar/FluidNavBar.cs`.

use super::FluidNavItem;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::Event;
use ferroui_base::input::{
    Cursor, InputElementImpl, InputElementImplExt, PointerPressedEventArgs, StandardCursorType,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    immutable::{ImmutablePen, ImmutableSolidColorBrush},
    FillRule, PenLineCap, PenLineJoin,
};
use ferroui_base::media::{Color, Colors, DrawingContext, ImmediateDrawingContext};
use ferroui_base::platform::{IGeometryImpl, IPlatformRenderInterface};
use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
use ferroui_base::rendering::ICustomHitTest;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Rect,
    Ref, Size, StyledElementImpl, StyledProperty, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use ferroui_base::{FerroLocator, LocatorExtensions, Matrix};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

pub(crate) const NOMINAL_HEIGHT: f64 = 56.0;
pub(crate) const CIRCLE_RADIUS: f64 = 25.0;
/// The distance the circle rises.
pub(crate) const ACTIVE_FLOAT: f64 = 16.0;
/// The scale of the icon within the circle.
pub(crate) const ICON_DRAW_SCALE: f64 = 0.9;
pub(crate) const SCALE_CURVE_SCALE: f64 = 0.50;
pub(crate) const FLOAT_LINEAR_P_IN: f64 = 0.28;
pub(crate) const FILL_LINEAR_P_IN: f64 = 0.25;
/// The duration of the travel of the bump, in seconds.
pub(crate) const X_ANIM_DURATION: f64 = 0.620;
/// The duration of the dip down, in seconds.
pub(crate) const Y_DIP_DURATION: f64 = 0.300;
/// The wait before the bounce, in seconds.
pub(crate) const Y_BOUNCE_DELAY: f64 = 0.500;
/// The duration of the elastic bounce up, in seconds.
pub(crate) const Y_BOUNCE_DURATION: f64 = 1.200;
/// The duration of the rise of the circle, in seconds.
pub(crate) const FLOAT_UP_DURATION: f64 = 1.666;
/// The duration of the fall of the circle, in seconds.
pub(crate) const FLOAT_DOWN_DURATION: f64 = 0.833;

/// A fluid navigation bar that replicates the Flutter fluid_nav_bar vignette. The bar
/// background has a bezier "dip" that travels to the selected tab. Each icon is drawn
/// progressively by trimming its path for the fill animation.
#[repr(C)]
pub struct FluidNavBar {
    base: Control,
    /// `-1`: not yet initialised.
    x_current: Cell<f64>,
    /// Tracks width changes for the resize correction.
    last_width: Cell<f64>,
    x_start: Cell<f64>,
    x_target: Cell<f64>,
    x_anim_start_sec: Cell<f64>,
    /// 0 = deepest dip, 1 = flat.
    y_value: Cell<f64>,
    y_dip_start_sec: Cell<f64>,
    y_bounce_started: Cell<bool>,
    y_bounce_start_sec: Cell<f64>,

    // Per item (the length is the count of the items after `on_items_changed`).
    float_progress: RefCell<Vec<f64>>,
    float_start_sec: RefCell<Vec<f64>>,
    float_going_up: RefCell<Vec<bool>>,

    /// The parsed paths of the icons, shared with the draw operations.
    parsed_paths: RefCell<SharedPaths>,

    anim_timer: RefCell<Option<Rc<DispatcherTimer>>>,
    clock: Instant,
    animating: Cell<bool>,

    selection_changed: Event<i32>,
}

ferro_class!(FluidNavBar: Control);
ferro_class_info!(FluidNavBar {
    new: FluidNavBar::new
});
ferro_impl_classes!(FluidNavBar: StyledElementImpl, InteractiveImpl, ControlImpl);

ferro_properties! {
    impl FluidNavBar {
        pub fn items_property() -> StyledProperty<FerroList<Rc<FluidNavItem>>> {
            FerroProperty::register::<FluidNavBar, _>("Items", FerroList::new())
        }

        pub fn selected_index_property() -> StyledProperty<i32> {
            FerroProperty::register::<FluidNavBar, _>("SelectedIndex", 0)
        }

        pub fn bar_color_property() -> StyledProperty<Color> {
            FerroProperty::register::<FluidNavBar, _>("BarColor", Colors::WHITE)
        }

        pub fn button_color_property() -> StyledProperty<Color> {
            FerroProperty::register::<FluidNavBar, _>("ButtonColor", Colors::WHITE)
        }

        pub fn active_icon_color_property() -> StyledProperty<Color> {
            FerroProperty::register::<FluidNavBar, _>("ActiveIconColor", Colors::BLACK)
        }

        pub fn inactive_icon_color_property() -> StyledProperty<Color> {
            FerroProperty::register::<FluidNavBar, _>("InactiveIconColor", Color::from_argb(140, 120, 120, 120))
        }
    }
}

impl FerroObjectImpl for FluidNavBar {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::items_property().as_property() {
            this.on_items_changed();
        } else if change.property() == Self::selected_index_property().as_property() {
            let (old_index, new_index) = change.get_old_and_new_value::<i32>();
            this.on_selected_index_changed(old_index, new_index);
        } else if change.property() == Self::bar_color_property().as_property()
            || change.property() == Self::button_color_property().as_property()
            || change.property() == Self::active_icon_color_property().as_property()
            || change.property() == Self::inactive_icon_color_property().as_property()
        {
            this.invalidate_visual();
        }
    }
}

impl ICustomHitTest for FluidNavBar {
    fn hit_test(&self, point: Point) -> bool {
        point.x >= 0.0
            && point.x <= self.bounds().width
            && point.y >= -(ACTIVE_FLOAT + CIRCLE_RADIUS)
            && point.y <= self.bounds().height
    }
}

impl InputElementImpl for FluidNavBar {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        let n = this.items().count() as i32;
        if n == 0 || this.bounds().width <= 0.0 {
            return;
        }

        let pos = e.get_position(Some(this));
        let index = ((pos.x / (this.bounds().width / f64::from(n))) as i32).clamp(0, n - 1);

        if index != this.selected_index() {
            this.set_current_value(Self::selected_index_property(), index);
            this.selection_changed.raise(&index);
        }

        e.set_handled(true);
    }
}

impl LayoutableImpl for FluidNavBar {
    fn measure_override(_this: &Self, available_size: Size) -> Size {
        let w = if available_size.width == f64::INFINITY {
            300.0
        } else {
            available_size.width
        };
        Size::new(w, NOMINAL_HEIGHT)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let w = final_size.width;
        if w > 0.0 {
            if this.x_current.get() < 0.0 || this.last_width.get() < 0.0 {
                // First layout: everything snaps to the current selection.
                this.x_current
                    .set(this.index_to_x(this.selected_index(), w));
                this.x_target.set(this.x_current.get());
                this.x_start.set(this.x_current.get());
            } else if (w - this.last_width.get()).abs() > 0.5 {
                // The width changed (resize): the pixel positions scale proportionally so the
                // bump stays over the correct slot.
                let ratio = w / this.last_width.get();
                this.x_current.set(this.x_current.get() * ratio);
                this.x_start.set(this.x_start.get() * ratio);
                this.x_target.set(this.index_to_x(this.selected_index(), w));
                this.invalidate_visual();
            }

            this.last_width.set(w);
        }

        Size::new(if w > 0.0 { w } else { 300.0 }, NOMINAL_HEIGHT)
    }
}

impl VisualImpl for FluidNavBar {
    fn custom_hit_test(this: &Self, point: Point) -> Option<bool> {
        Some(ICustomHitTest::hit_test(this, point))
    }

    fn render(this: &Self, context: &mut DrawingContext) {
        let w = this.bounds().width;
        let h = this.bounds().height;
        let n = this.items().count();

        if w <= 0.0 || h <= 0.0 || n == 0 {
            return;
        }

        let selected_index = this.selected_index();

        // `x_current` is initialised here if the layout did not run yet.
        if this.x_current.get() < 0.0 {
            this.x_current.set(this.index_to_x(selected_index, w));
            this.x_target.set(this.x_current.get());
        }

        // The per-item animation state of this frame.
        let slot_centers: Vec<f64> = (0..n).map(|i| this.index_to_x(i as i32, w)).collect();
        let mut float_offsets = vec![0.0; n];
        let mut scale_y_values = vec![0.0; n];
        let mut fill_amounts = vec![0.0; n];

        {
            let float_progress = this.float_progress.borrow();
            let float_going_up = this.float_going_up.borrow();
            for i in 0..n {
                let is_selected = i as i32 == selected_index;
                let p =
                    float_progress
                        .get(i)
                        .copied()
                        .unwrap_or(if is_selected { 1.0 } else { 0.0 });
                let go_up = float_going_up.get(i).copied().unwrap_or(is_selected);

                // The float offset: `linear_point(0.28, 0)` delays the start, then elastic or
                // quintic easing.
                let linear_p = Self::linear_point(p, FLOAT_LINEAR_P_IN, 0.0);
                let float_eased = if go_up {
                    Self::elastic_out(linear_p, 0.38)
                } else {
                    Self::ease_in_quint(linear_p)
                };
                float_offsets[i] = ACTIVE_FLOAT * float_eased;

                // The squish of the vertical scale through the centered elastic curves.
                let centered = if go_up {
                    Self::centered_elastic_out(p, 0.6)
                } else {
                    Self::centered_elastic_in(p, 0.6)
                };
                scale_y_values[i] = 0.75 + centered * SCALE_CURVE_SCALE;

                // The fill of the icon: `linear_point(0.25, 1.0)` adds a slight draw delay
                // against the float.
                fill_amounts[i] = Self::linear_point(p, FILL_LINEAR_P_IN, 1.0);
            }
        }

        // The vertical scale is clamped to a sane range.
        for scale_y in &mut scale_y_values {
            *scale_y = 0.1_f64.max(1.5_f64.min(*scale_y));
        }

        let mut op = FluidNavBarRenderOp {
            background: None,
            bounds: Rect::new(
                0.0,
                -(ACTIVE_FLOAT + CIRCLE_RADIUS),
                w,
                h + ACTIVE_FLOAT + CIRCLE_RADIUS,
            ),
            w: w as f32,
            h: h as f32,
            x_center: this.x_current.get() as f32,
            norm_y: this.y_value.get() as f32,
            slots: slot_centers,
            float_off: float_offsets,
            scale_y: scale_y_values,
            fill: fill_amounts,
            paths: this.parsed_paths.borrow().clone(),
            bar: this.bar_color(),
            btn: this.button_color(),
            active: this.active_icon_color(),
            inactive: this.inactive_icon_color(),
        };
        op.background = Some(op.background_geometry());
        let op: std::sync::Arc<dyn ICustomDrawOperation> = std::sync::Arc::new(op);
        context.custom(&op);
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.stop_animation();
    }
}

impl FluidNavBar {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            x_current: Cell::new(-1.0),
            last_width: Cell::new(-1.0),
            x_start: Cell::new(0.0),
            x_target: Cell::new(0.0),
            x_anim_start_sec: Cell::new(0.0),
            y_value: Cell::new(1.0),
            y_dip_start_sec: Cell::new(0.0),
            y_bounce_started: Cell::new(false),
            y_bounce_start_sec: Cell::new(0.0),
            float_progress: RefCell::new(Vec::new()),
            float_start_sec: RefCell::new(Vec::new()),
            float_going_up: RefCell::new(Vec::new()),
            parsed_paths: RefCell::new(SharedPaths::default()),
            anim_timer: RefCell::new(None),
            clock: Instant::now(),
            animating: Cell::new(false),
            selection_changed: Event::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_clip_to_bounds(false);
        this.set_height(NOMINAL_HEIGHT);
        this.set_cursor(Some(Cursor::new(StandardCursorType::Hand)));
        this
    }

    pub fn items(&self) -> FerroList<Rc<FluidNavItem>> {
        self.get_value(Self::items_property())
    }

    pub fn set_items(&self, value: FerroList<Rc<FluidNavItem>>) {
        self.set_value(Self::items_property(), value)
    }

    pub fn selected_index(&self) -> i32 {
        self.get_value(Self::selected_index_property())
    }

    pub fn set_selected_index(&self, value: i32) {
        self.set_value(Self::selected_index_property(), value)
    }

    pub fn bar_color(&self) -> Color {
        self.get_value(Self::bar_color_property())
    }

    pub fn set_bar_color(&self, value: Color) {
        self.set_value(Self::bar_color_property(), value)
    }

    pub fn button_color(&self) -> Color {
        self.get_value(Self::button_color_property())
    }

    pub fn set_button_color(&self, value: Color) {
        self.set_value(Self::button_color_property(), value)
    }

    pub fn active_icon_color(&self) -> Color {
        self.get_value(Self::active_icon_color_property())
    }

    pub fn set_active_icon_color(&self, value: Color) {
        self.set_value(Self::active_icon_color_property(), value)
    }

    pub fn inactive_icon_color(&self) -> Color {
        self.get_value(Self::inactive_icon_color_property())
    }

    pub fn set_inactive_icon_color(&self, value: Color) {
        self.set_value(Self::inactive_icon_color_property(), value)
    }

    /// Raised with the new index when a press of the pointer changes the selection.
    pub fn selection_changed(&self) -> &Event<i32> {
        &self.selection_changed
    }

    /// The seconds the clock of the control has run.
    fn now(&self) -> f64 {
        self.clock.elapsed().as_secs_f64()
    }

    fn on_items_changed(&self) {
        let items = self.items().snapshot();
        let n = items.len();

        let parsed_paths: Vec<Option<Vec<std::sync::Arc<dyn IGeometryImpl>>>> = items
            .iter()
            .map(|item| {
                let svg = item.svg_path();
                if svg.is_empty() {
                    None
                } else {
                    parse_icon_contours(&svg)
                }
            })
            .collect();
        *self.parsed_paths.borrow_mut() = std::sync::Arc::new(std::sync::Mutex::new(parsed_paths));

        let sel = self.selected_index().clamp(0, 0.max(n as i32 - 1));
        *self.float_progress.borrow_mut() = (0..n)
            .map(|i| if i as i32 == sel { 1.0 } else { 0.0 })
            .collect();
        *self.float_start_sec.borrow_mut() = vec![0.0; n];
        *self.float_going_up.borrow_mut() = (0..n).map(|i| i as i32 == sel).collect();

        // Forces the initialisation again on the next arrange or render.
        self.x_current.set(-1.0);
        self.invalidate_visual();
    }

    fn on_selected_index_changed(&self, old_index: i32, new_index: i32) {
        let n = self.float_progress.borrow().len() as i32;
        if n == 0 {
            return;
        }

        let new_index = new_index.clamp(0, n - 1);
        let old_index = old_index.clamp(0, n - 1);
        if old_index == new_index {
            return;
        }

        let now = self.now();
        let width = self.bounds().width;

        // X: the bump slides from the old to the new position.
        if self.x_current.get() < 0.0 && width > 0.0 {
            self.x_current.set(self.index_to_x(old_index, width));
        }

        self.x_start.set(self.x_current.get());
        self.x_target.set(if width > 0.0 {
            self.index_to_x(new_index, width)
        } else {
            self.x_start.get()
        });
        self.x_anim_start_sec.set(now);

        // Y: the dip, then the elastic bounce.
        self.y_value.set(1.0);
        self.y_dip_start_sec.set(now);
        self.y_bounce_started.set(false);

        // The float of each button.
        {
            let mut float_going_up = self.float_going_up.borrow_mut();
            let mut float_start_sec = self.float_start_sec.borrow_mut();
            float_going_up[old_index as usize] = false;
            float_start_sec[old_index as usize] = now;
            float_going_up[new_index as usize] = true;
            float_start_sec[new_index as usize] = now;
        }

        self.start_animation();
    }

    fn start_animation(&self) {
        if self.animating.get() {
            return;
        }
        self.animating.set(true);
        // The timer is a field of the control: its handler holds the control weakly.
        let weak = self.to_ref().downgrade();
        let timer = DispatcherTimer::with_callback(
            Duration::from_secs_f64(1.0 / 60.0),
            DispatcherPriority::RENDER,
            move |_| {
                if let Some(this) = weak.upgrade() {
                    this.on_anim_tick();
                }
            },
        );
        timer.start();
        *self.anim_timer.borrow_mut() = Some(timer);
    }

    fn stop_animation(&self) {
        if let Some(timer) = self.anim_timer.borrow_mut().take() {
            timer.stop();
        }
        self.animating.set(false);
    }

    fn on_anim_tick(&self) {
        let now = self.now();
        let mut any_active = false;

        let x_elapsed = now - self.x_anim_start_sec.get();
        if x_elapsed < X_ANIM_DURATION {
            self.x_current.set(
                self.x_start.get()
                    + (self.x_target.get() - self.x_start.get()) * (x_elapsed / X_ANIM_DURATION),
            );
            any_active = true;
        } else {
            self.x_current.set(self.x_target.get());
        }

        let y_dip_elapsed = now - self.y_dip_start_sec.get();
        if y_dip_elapsed < Y_DIP_DURATION {
            self.y_value.set(1.0 - y_dip_elapsed / Y_DIP_DURATION);
            any_active = true;
        } else {
            self.y_value.set(0.0);

            if !self.y_bounce_started.get() && y_dip_elapsed >= Y_BOUNCE_DELAY {
                self.y_bounce_started.set(true);
                self.y_bounce_start_sec.set(now);
            }

            if self.y_bounce_started.get() {
                let bt = now - self.y_bounce_start_sec.get();
                if bt < Y_BOUNCE_DURATION {
                    self.y_value
                        .set(Self::elastic_out(bt / Y_BOUNCE_DURATION, 0.38));
                    any_active = true;
                } else {
                    self.y_value.set(1.0);
                }
            }
        }

        {
            let mut float_progress = self.float_progress.borrow_mut();
            let float_start_sec = self.float_start_sec.borrow();
            let float_going_up = self.float_going_up.borrow();
            for i in 0..float_progress.len() {
                let elapsed = now - float_start_sec[i];
                let duration = if float_going_up[i] {
                    FLOAT_UP_DURATION
                } else {
                    FLOAT_DOWN_DURATION
                };
                if elapsed < duration {
                    let t = elapsed / duration;
                    float_progress[i] = if float_going_up[i] { t } else { 1.0 - t };
                    any_active = true;
                } else {
                    float_progress[i] = if float_going_up[i] { 1.0 } else { 0.0 };
                }
            }
        }

        self.invalidate_visual();

        if !any_active {
            self.stop_animation();
        }
    }

    fn index_to_x(&self, index: i32, width: f64) -> f64 {
        let mut n = self.items().count() as i32;
        if n <= 0 {
            n = 1;
        }

        (f64::from(index) + 0.5) * (width / f64::from(n))
    }

    pub(crate) fn elastic_out(t: f64, period: f64) -> f64 {
        if t <= 0.0 {
            return 0.0;
        }
        if t >= 1.0 {
            return 1.0;
        }
        let s = period / 4.0;
        2.0_f64.powf(-10.0 * t) * ((t - s) * 2.0 * std::f64::consts::PI / period).sin() + 1.0
    }

    fn centered_elastic_out(t: f64, period: f64) -> f64 {
        2.0_f64.powf(-10.0 * t) * (t * 2.0 * std::f64::consts::PI / period).sin() + 0.5
    }

    fn centered_elastic_in(t: f64, period: f64) -> f64 {
        -(2.0_f64.powf(10.0 * (t - 1.0))) * ((t - 1.0) * 2.0 * std::f64::consts::PI / period).sin()
            + 0.5
    }

    pub(crate) fn linear_point(x: f64, p_in: f64, p_out: f64) -> f64 {
        if p_in <= 0.0 {
            return p_out;
        }
        let lower_scale = p_out / p_in;
        let upper_scale = (1.0 - p_out) / (1.0 - p_in);
        let upper_off = 1.0 - upper_scale;
        if x < p_in {
            x * lower_scale
        } else {
            x * upper_scale + upper_off
        }
    }

    fn ease_in_quint(t: f64) -> f64 {
        t * t * t * t * t
    }
}

/// Keep separate contours so each icon component receives the same trim fraction.
///
/// The path markup is parsed into one geometry of the backend per figure, so it works in every
/// backend, whether or not its geometries expose their path data.
fn parse_icon_contours(svg: &str) -> Option<Vec<std::sync::Arc<dyn IGeometryImpl>>> {
    let backend = FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();
    let mut contours = ContourSplitter { backend: &*backend, current: None, contours: Vec::new() };
    ferroui_base::media::PathMarkupParser::new(&mut contours).parse(svg).ok()?;
    contours.finish();
    Some(contours.contours)
}

/// A geometry context that gives each figure a stream geometry of its own.
struct ContourSplitter<'a> {
    backend: &'a dyn IPlatformRenderInterface,
    current: Option<Box<dyn ferroui_base::platform::IStreamGeometryBuilder>>,
    contours: Vec<std::sync::Arc<dyn IGeometryImpl>>,
}

impl ContourSplitter<'_> {
    fn finish(&mut self) {
        if let Some(mut builder) = self.current.take() {
            self.contours.push(builder.to_geometry());
        }
    }
}

impl ferroui_base::platform::IGeometryContext for ContourSplitter<'_> {
    fn arc_to(
        &mut self,
        point: Point,
        size: Size,
        rotation_angle: f64,
        is_large_arc: bool,
        sweep_direction: ferroui_base::media::SweepDirection,
        is_stroked: bool,
    ) {
        if let Some(builder) = self.current.as_mut() {
            builder.arc_to(point, size, rotation_angle, is_large_arc, sweep_direction, is_stroked);
        }
    }
    fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
        self.finish();
        let mut builder = self.backend.create_stream_geometry_builder(None);
        builder.begin_figure(start_point, is_filled);
        self.current = Some(builder);
    }
    fn cubic_bezier_to(&mut self, control_point1: Point, control_point2: Point, end_point: Point, is_stroked: bool) {
        if let Some(builder) = self.current.as_mut() {
            builder.cubic_bezier_to(control_point1, control_point2, end_point, is_stroked);
        }
    }
    fn quadratic_bezier_to(&mut self, control_point: Point, end_point: Point, is_stroked: bool) {
        if let Some(builder) = self.current.as_mut() {
            builder.quadratic_bezier_to(control_point, end_point, is_stroked);
        }
    }
    fn line_to(&mut self, point: Point, is_stroked: bool) {
        if let Some(builder) = self.current.as_mut() {
            builder.line_to(point, is_stroked);
        }
    }
    fn end_figure(&mut self, is_closed: bool) {
        if let Some(builder) = self.current.as_mut() {
            builder.end_figure(is_closed);
        }
    }
    fn set_fill_rule(&mut self, _fill_rule: FillRule) {}
    fn dispose(&mut self) {
        self.finish();
    }
}

/// The parsed icon paths, shared with the draw operations. An operation is
/// rendered on the render thread, hence the lock.
type SharedPaths =
    std::sync::Arc<std::sync::Mutex<Vec<Option<Vec<std::sync::Arc<dyn IGeometryImpl>>>>>>;

/// The draw operation of a frame of the bar.
struct FluidNavBarRenderOp {
    background: Option<std::sync::Arc<dyn IGeometryImpl>>,
    bounds: Rect,
    w: f32,
    h: f32,
    x_center: f32,
    norm_y: f32,
    slots: Vec<f64>,
    float_off: Vec<f64>,
    scale_y: Vec<f64>,
    fill: Vec<f64>,
    paths: SharedPaths,
    bar: Color,
    btn: Color,
    active: Color,
    inactive: Color,
}

impl ICustomDrawOperation for FluidNavBarRenderOp {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn hit_test(&self, _p: Point) -> bool {
        false
    }

    fn equals(&self, _other: &dyn ICustomDrawOperation) -> bool {
        false
    }

    fn dispose(&self) {}

    fn render(&self, context: &mut ImmediateDrawingContext<'_>) {
        let target = context.platform_impl();
        let original = target.transform();
        if let Some(background) = &self.background {
            target.draw_geometry(
                Some(&ImmutableSolidColorBrush::new(self.bar)),
                None,
                &**background,
            );
        }
        for i in 0..self.slots.len() {
            let cx = self.slots[i];
            let cy = f64::from(self.h) / 2.0 - self.float_off[i];
            target.draw_ellipse(
                Some(&ImmutableSolidColorBrush::new(self.btn)),
                None,
                Rect::new(
                    cx - CIRCLE_RADIUS,
                    cy - CIRCLE_RADIUS,
                    CIRCLE_RADIUS * 2.0,
                    CIRCLE_RADIUS * 2.0,
                ),
            );
            if let Some(Some(paths)) = self.paths.lock().unwrap().get(i) {
                target.set_transform(
                    Matrix::new(
                        ICON_DRAW_SCALE,
                        0.0,
                        0.0,
                        ICON_DRAW_SCALE * self.scale_y[i],
                        cx,
                        cy,
                    ) * original,
                );
                let pen = |c| {
                    ImmutablePen::new(
                        Some(Rc::new(ImmutableSolidColorBrush::new(c))),
                        2.4,
                        None,
                        PenLineCap::Round,
                        PenLineJoin::Round,
                        10.0,
                    )
                };
                for path in paths {
                    target.draw_geometry(None, Some(&pen(self.inactive)), &**path);
                    if self.fill[i] > 0.0 {
                        if let Some(segment) =
                            path.try_get_segment(0.0, path.contour_length() * self.fill[i], true)
                        {
                            target.draw_geometry(None, Some(&pen(self.active)), &*segment);
                        }
                    }
                }
                target.set_transform(original);
            }
        }
    }
}

impl FluidNavBarRenderOp {
    fn background_geometry(&self) -> std::sync::Arc<dyn IGeometryImpl> {
        const R_TOP: f32 = 54.0;
        const R_BOT: f32 = 44.0;
        const HC_TOP: f32 = 0.6;
        const HC_BOT: f32 = 0.5;
        const PC_TOP: f32 = 0.35;
        const PC_BOT: f32 = 0.85;
        const T_Y: f32 = -10.0;
        const B_Y: f32 = 54.0;
        const T_D: f32 = 0.0;
        const B_D: f32 = 6.0;

        let norm = (Self::linear_point(f64::from(self.norm_y), 0.5, 2.0) / 2.0) as f32;

        let r = Self::lerp(R_TOP, R_BOT, norm);
        let anchr = Self::lerp(
            r * HC_TOP,
            r * HC_BOT,
            Self::linear_point(f64::from(norm), 0.5, 0.75) as f32,
        );
        let dipc = Self::lerp(
            r * PC_TOP,
            r * PC_BOT,
            Self::linear_point(f64::from(norm), 0.5, 0.80) as f32,
        );
        let y = Self::lerp(
            T_Y,
            B_Y,
            Self::linear_point(f64::from(norm), 0.2, 0.70) as f32,
        );
        let dist = Self::lerp(
            T_D,
            B_D,
            Self::linear_point(f64::from(norm), 0.5, 0.00) as f32,
        );
        let x0 = self.x_center - dist / 2.0;
        let x1 = self.x_center + dist / 2.0;

        let backend =
            FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();
        let mut path = backend.create_stream_geometry_builder(None);
        let p = |x: f32, y: f32| Point::new(f64::from(x), f64::from(y));
        path.set_fill_rule(FillRule::NonZero);
        path.begin_figure(p(0.0, 0.0), true);
        path.line_to(p(x0 - r, 0.0), true);
        path.cubic_bezier_to(p(x0 - r + anchr, 0.0), p(x0 - dipc, y), p(x0, y), true);
        path.line_to(p(x1, y), true);
        path.cubic_bezier_to(
            p(x1 + dipc, y),
            p(x1 + r - anchr, 0.0),
            p(x1 + r, 0.0),
            true,
        );
        path.line_to(p(self.w, 0.0), true);
        path.line_to(p(self.w, self.h), true);
        path.line_to(p(0.0, self.h), true);
        path.end_figure(true);
        path.to_geometry()
    }

    /// `float.Lerp(a, b, t)`.
    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a * (1.0 - t) + b * t
    }

    fn linear_point(x: f64, p_in: f64, p_out: f64) -> f64 {
        if p_in <= 0.0 {
            return p_out;
        }
        let lo = p_out / p_in;
        let hi = (1.0 - p_out) / (1.0 - p_in);
        if x < p_in {
            x * lo
        } else {
            x * hi + (1.0 - hi)
        }
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn elastic_out_is_clamped_at_its_ends() {
        assert_eq!(0.0, FluidNavBar::elastic_out(0.0, 0.4));
        assert_eq!(0.0, FluidNavBar::elastic_out(-1.0, 0.4));
        assert_eq!(1.0, FluidNavBar::elastic_out(1.0, 0.4));
        assert_eq!(1.0, FluidNavBar::elastic_out(2.0, 0.4));
    }

    #[test]
    fn linear_point_passes_through_its_point() {
        assert_eq!(0.0, FluidNavBar::linear_point(0.0, 0.25, 1.0));
        assert_eq!(1.0, FluidNavBar::linear_point(0.25, 0.25, 1.0));
        assert_eq!(1.0, FluidNavBar::linear_point(1.0, 0.25, 1.0));
        assert_eq!(0.0, FluidNavBar::linear_point(0.1, 0.28, 0.0));
        assert_eq!(0.75, FluidNavBar::linear_point(0.3, 0.0, 0.75));
    }

    #[test]
    fn quintic_ease_in() {
        assert_eq!(0.03125, FluidNavBar::ease_in_quint(0.5));
    }
}
