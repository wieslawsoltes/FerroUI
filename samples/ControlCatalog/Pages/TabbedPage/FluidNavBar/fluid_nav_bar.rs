//! Port of `Pages/TabbedPage/FluidNavBar/FluidNavBar.cs`.

use super::FluidNavItem;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::Event;
use ferroui_base::input::{Cursor, InputElementImpl, InputElementImplExt, PointerPressedEventArgs, StandardCursorType};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Color, Colors, DrawingContext, ImmediateDrawingContext};
use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
use ferroui_base::rendering::ICustomHitTest;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_skia::ISkiaApiLeaseFeature;
use skia_safe::{Canvas, Paint, PaintCap, PaintJoin, PaintStyle, Path, PathBuilder, PathMeasure};
use std::any::TypeId;
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
ferro_class_info!(FluidNavBar { new: FluidNavBar::new });
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
        let w = if available_size.width == f64::INFINITY { 300.0 } else { available_size.width };
        Size::new(w, NOMINAL_HEIGHT)
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let w = final_size.width;
        if w > 0.0 {
            if this.x_current.get() < 0.0 || this.last_width.get() < 0.0 {
                // First layout: everything snaps to the current selection.
                this.x_current.set(this.index_to_x(this.selected_index(), w));
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
                let p = float_progress.get(i).copied().unwrap_or(if is_selected { 1.0 } else { 0.0 });
                let go_up = float_going_up.get(i).copied().unwrap_or(is_selected);

                // The float offset: `linear_point(0.28, 0)` delays the start, then elastic or
                // quintic easing.
                let linear_p = Self::linear_point(p, FLOAT_LINEAR_P_IN, 0.0);
                let float_eased =
                    if go_up { Self::elastic_out(linear_p, 0.38) } else { Self::ease_in_quint(linear_p) };
                float_offsets[i] = ACTIVE_FLOAT * float_eased;

                // The squish of the vertical scale through the centered elastic curves.
                let centered =
                    if go_up { Self::centered_elastic_out(p, 0.6) } else { Self::centered_elastic_in(p, 0.6) };
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

        let op: std::sync::Arc<dyn ICustomDrawOperation> = std::sync::Arc::new(FluidNavBarRenderOp {
            bounds: Rect::new(0.0, -(ACTIVE_FLOAT + CIRCLE_RADIUS), w, h + ACTIVE_FLOAT + CIRCLE_RADIUS),
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
        });

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

        let parsed_paths: Vec<Option<Path>> = items
            .iter()
            .map(|item| {
                let svg = item.svg_path();
                if svg.is_empty() { None } else { Path::from_svg(&svg) }
            })
            .collect();
        *self.parsed_paths.borrow_mut() = std::sync::Arc::new(std::sync::Mutex::new(parsed_paths));

        let sel = self.selected_index().clamp(0, 0.max(n as i32 - 1));
        *self.float_progress.borrow_mut() = (0..n).map(|i| if i as i32 == sel { 1.0 } else { 0.0 }).collect();
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
        self.x_target.set(if width > 0.0 { self.index_to_x(new_index, width) } else { self.x_start.get() });
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
        let timer =
            DispatcherTimer::with_callback(Duration::from_secs_f64(1.0 / 60.0), DispatcherPriority::RENDER, move |_| {
                if let Some(this) = weak.upgrade() {
                    this.on_anim_tick();
                }
            });
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
            self.x_current
                .set(self.x_start.get() + (self.x_target.get() - self.x_start.get()) * (x_elapsed / X_ANIM_DURATION));
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
                    self.y_value.set(Self::elastic_out(bt / Y_BOUNCE_DURATION, 0.38));
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
                let duration = if float_going_up[i] { FLOAT_UP_DURATION } else { FLOAT_DOWN_DURATION };
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
        -(2.0_f64.powf(10.0 * (t - 1.0))) * ((t - 1.0) * 2.0 * std::f64::consts::PI / period).sin() + 0.5
    }

    pub(crate) fn linear_point(x: f64, p_in: f64, p_out: f64) -> f64 {
        if p_in <= 0.0 {
            return p_out;
        }
        let lower_scale = p_out / p_in;
        let upper_scale = (1.0 - p_out) / (1.0 - p_in);
        let upper_off = 1.0 - upper_scale;
        if x < p_in { x * lower_scale } else { x * upper_scale + upper_off }
    }

    fn ease_in_quint(t: f64) -> f64 {
        t * t * t * t * t
    }
}

/// The parsed icon paths, shared with the draw operations. An operation is
/// rendered on the render thread, and a Skia path may be sent to another
/// thread but not shared by reference, hence the lock.
type SharedPaths = std::sync::Arc<std::sync::Mutex<Vec<Option<Path>>>>;

/// The draw operation of a frame of the bar.
struct FluidNavBarRenderOp {
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
        let lease = context
            .try_get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
            .and_then(|feature| feature.downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>().cloned());
        let Some(lease) = lease else {
            return;
        };

        let l = lease.lease();
        l.with_sk_canvas(&mut |canvas| {
            let save = canvas.save();
            self.draw_background(canvas);
            for i in 0..self.slots.len() {
                self.draw_button(canvas, i);
            }
            canvas.restore_to_count(save);
        });
        l.dispose();
    }
}

impl FluidNavBarRenderOp {
    fn draw_background(&self, canvas: &Canvas) {
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
        let anchr = Self::lerp(r * HC_TOP, r * HC_BOT, Self::linear_point(f64::from(norm), 0.5, 0.75) as f32);
        let dipc = Self::lerp(r * PC_TOP, r * PC_BOT, Self::linear_point(f64::from(norm), 0.5, 0.80) as f32);
        let y = Self::lerp(T_Y, B_Y, Self::linear_point(f64::from(norm), 0.2, 0.70) as f32);
        let dist = Self::lerp(T_D, B_D, Self::linear_point(f64::from(norm), 0.5, 0.00) as f32);
        let x0 = self.x_center - dist / 2.0;
        let x1 = self.x_center + dist / 2.0;

        let mut path = PathBuilder::new();
        path.move_to((0.0_f32, 0.0_f32));
        path.line_to((x0 - r, 0.0_f32));
        path.cubic_to((x0 - r + anchr, 0.0_f32), (x0 - dipc, y), (x0, y));
        path.line_to((x1, y));
        path.cubic_to((x1 + dipc, y), (x1 + r - anchr, 0.0_f32), (x1 + r, 0.0_f32));
        path.line_to((self.w, 0.0_f32));
        path.line_to((self.w, self.h));
        path.line_to((0.0_f32, self.h));
        path.close();

        let mut paint = Paint::default();
        paint.set_color(Self::to_sk(self.bar));
        paint.set_anti_alias(true);
        canvas.draw_path(&path.detach(), &paint);
    }

    fn draw_button(&self, canvas: &Canvas, i: usize) {
        let cx = self.slots[i] as f32;
        let cy = self.h / 2.0;
        let fo = self.float_off[i] as f32;
        let sy = self.scale_y[i] as f32;
        let fa = self.fill[i] as f32;

        const R: f32 = CIRCLE_RADIUS as f32;

        // The circle is only translated up, not scaled.
        let mut cp = Paint::default();
        cp.set_color(Self::to_sk(self.btn));
        cp.set_anti_alias(true);
        canvas.draw_circle((cx, cy - fo), R, &cp);

        // The icon.
        if let Some(Some(path)) = self.paths.lock().unwrap().get(i) {
            self.draw_icon(canvas, path, cx, cy - fo, sy, fa);
        }
    }

    fn draw_icon(&self, canvas: &Canvas, path: &Path, cx: f32, cy: f32, scale_y: f32, fill_amount: f32) {
        const S: f32 = ICON_DRAW_SCALE as f32;

        let save = canvas.save();
        canvas.translate((cx, cy));
        canvas.scale((S, S * scale_y));

        // The grey background stroke (the full path, the unselected look).
        let mut bg = Paint::default();
        bg.set_style(PaintStyle::Stroke);
        bg.set_stroke_width(2.4);
        bg.set_stroke_cap(PaintCap::Round);
        bg.set_stroke_join(PaintJoin::Round);
        bg.set_color(Self::to_sk(self.inactive));
        bg.set_anti_alias(true);
        canvas.draw_path(path, &bg);

        // The foreground stroke, trimmed progressively.
        if fill_amount > 0.0 {
            let mut fg = Paint::default();
            fg.set_style(PaintStyle::Stroke);
            fg.set_stroke_width(2.4);
            fg.set_stroke_cap(PaintCap::Round);
            fg.set_stroke_join(PaintJoin::Round);
            fg.set_color(Self::to_sk(self.active));
            fg.set_anti_alias(true);
            Self::draw_trimmed_path(canvas, path, fill_amount, &fg);
        }

        canvas.restore_to_count(save);
    }

    /// Iterates all contours and draws each trimmed to `fill_amount` of its length.
    fn draw_trimmed_path(canvas: &Canvas, path: &Path, fill_amount: f32, paint: &Paint) {
        let mut measure = PathMeasure::new(path, false, None);
        loop {
            let len = measure.length();
            if len > 0.0 {
                let mut seg = PathBuilder::new();
                if measure.get_segment(0.0, len * fill_amount, &mut seg, true) {
                    canvas.draw_path(&seg.detach(), paint);
                }
            }

            if !measure.next_contour() {
                break;
            }
        }
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
        if x < p_in { x * lo } else { x * hi + (1.0 - hi) }
    }

    fn to_sk(c: Color) -> skia_safe::Color {
        skia_safe::Color::from_argb(c.a, c.r, c.g, c.b)
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
