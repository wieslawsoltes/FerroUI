//! The control that draws a scene node (docs/osprey-native-host.md, section 5): every primitive
//! with FerroUI's drawing context, which the renderer of the platform draws on the GPU, and the
//! pointer, the keys and the size of the region sent back through the scene's binding, in scene
//! units.
//!
//! The scene is drawn from the control's top left at its `zoom` (window pixels per unit), which is
//! an upper bound: it shrinks uniformly when the region would clip the scene; a zoom of 0 draws it
//! as large as fits. A scene of zero width and height is in window pixels: no scale and no clip.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ferroui_base::input::InputElement;
use ferroui_base::input::{
    InputElementImpl, Key, KeyEventArgs, KeyModifiers, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    Color, DashStyle, DrawingContext, FlowDirection, FontFamily, FontStretch, FontStyle,
    FontWeight, FormattedText, GradientStop, GradientStops, IBrush, IPen, LinearGradientBrush, Pen,
    PenLineCap, PenLineJoin, SolidColorBrush, StreamGeometry, SweepDirection, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::utilities::Locale;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroProperty, Matrix, Point, Rect, Ref, RelativePoint, RelativeUnit, Size,
    StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_view::ViewClass;
use serde_json::{json, Value};

use crate::scene::{Outline, Paint, Primitive, Scene, Step, Tint};

/// A decoded scene; equal when it is the same decoding, so a region that was not patched is not
/// drawn again.
#[derive(Clone, Default)]
pub struct SceneData(pub Rc<Scene>);

impl PartialEq for SceneData {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Where the scene's input goes: the scene's binding in its region's current revision.
#[derive(Clone)]
pub struct SceneLink(pub Rc<dyn Fn(Value)>);

impl PartialEq for SceneLink {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Default for SceneLink {
    fn default() -> Self {
        Self(Rc::new(|_| {}))
    }
}

/// Draws one scene node and sends its input.
#[repr(C)]
pub struct SceneSurface {
    base: Control,
    /// The region's size in scene units the application was last told.
    told: Cell<Option<(f64, f64)>>,
    pressed: Cell<bool>,
    /// The last pointer position sent while not pressed, so a hover sends once per movement.
    hovered: RefCell<Option<(f64, f64)>>,
}

ferro_class!(SceneSurface: Control);
ferro_impl_classes!(SceneSurface: FerroObjectImpl, StyledElementImpl, InteractiveImpl);
ferro_class_info!(SceneSurface {
    new: SceneSurface::new
});

ferro_properties! {
    impl SceneSurface {
        /// The scene drawn.
        pub fn scene_property() -> StyledProperty<SceneData> {
            FerroProperty::register::<SceneSurface, _>("Scene", SceneData::default())
        }

        /// Where input goes.
        pub fn link_property() -> StyledProperty<SceneLink> {
            FerroProperty::register::<SceneSurface, _>("Link", SceneLink::default())
        }
    }
}

impl ViewClass for SceneSurface {
    fn create() -> Ref<Self> {
        Self::new()
    }
}

/// The scale and origin of a scene in a region of `size`, and the region in scene units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub scale: f64,
    pub viewed: (f64, f64),
}

impl Placement {
    pub fn of(scene: &Scene, size: Size) -> Self {
        if scene.in_window_pixels() {
            return Self {
                scale: 1.0,
                viewed: (size.width, size.height),
            };
        }
        let whole = (size.width / scene.width).min(size.height / scene.height);
        let scale = if scene.zoom == 0.0 {
            whole
        } else {
            scene.zoom.min(whole)
        };
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        Self {
            scale,
            viewed: (size.width / scale, size.height / scale),
        }
    }
}

impl SceneSurface {
    fn static_constructor() {
        Visual::affects_render::<SceneSurface>(&[Self::scene_property().as_property()]);
    }

    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            told: Cell::new(None),
            pressed: Cell::new(false),
            hovered: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_focusable(true);
        this
    }

    pub fn scene(&self) -> Rc<Scene> {
        self.get_value(Self::scene_property()).0
    }

    fn send(&self, value: Value) {
        let link = self.get_value(Self::link_property());
        (link.0)(value);
    }

    fn placement(&self) -> Placement {
        Placement::of(&self.scene(), self.bounds().size())
    }

    /// Tells the application the region's size in scene units when it changed (a sized scene).
    fn tell_size(&self) {
        let scene = self.scene();
        let size = self.bounds().size();
        if scene.in_window_pixels() || size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        let viewed = Placement::of(&scene, size).viewed;
        if self.told.get() != Some(viewed) {
            self.told.set(Some(viewed));
            self.send(json!({"width": viewed.0, "height": viewed.1}));
        }
    }

    /// Where the scene's origin is in the control: the window's top left for a scene in window
    /// pixels, the control's top left otherwise.
    fn origin(&self) -> Point {
        if !self.scene().in_window_pixels() {
            return Point::new(0.0, 0.0);
        }
        let root = self.visual_root();
        let at = root.and_then(|root| self.translate_point(Point::new(0.0, 0.0), &root));
        at.map_or(Point::new(0.0, 0.0), |at| Point::new(-at.x, -at.y))
    }

    fn pointer(&self, position: Point, down: bool, modifiers: KeyModifiers) {
        let scale = self.placement().scale;
        let origin = self.origin();
        let (x, y) = (
            (position.x - origin.x) / scale,
            (position.y - origin.y) / scale,
        );
        let shift = modifiers.contains(KeyModifiers::SHIFT);
        self.send(json!({"x": x, "y": y, "down": down, "shift": shift}));
    }
}

impl ControlImpl for SceneSurface {}

impl LayoutableImpl for SceneSurface {
    fn measure_override(this: &Self, available: Size) -> Size {
        let scene = this.scene();
        if scene.in_window_pixels() {
            // A canvas takes the space its region gives it; it draws from the window's origin.
            let finite = |length: f64| if length.is_finite() { length } else { 0.0 };
            return Size::new(finite(available.width), finite(available.height));
        }
        // A sized scene asks for its size at its zoom where the region is unbounded, and takes the
        // region otherwise: it is drawn to fit.
        let zoom = if scene.zoom > 0.0 { scene.zoom } else { 1.0 };
        let width = if available.width.is_finite() {
            available.width
        } else {
            scene.width * zoom
        };
        let height = if available.height.is_finite() {
            available.height
        } else {
            scene.height * zoom
        };
        Size::new(
            width.min(scene.width * zoom.max(1.0) * 4.0).max(0.0),
            height.max(0.0),
        )
    }
}

impl InputElementImpl for SceneSurface {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        e.set_handled(true);
        this.focus();
        let element: Ref<InputElement> = this.to_ref().upcast();
        e.pointer().capture(Some(&element));
        this.pressed.set(true);
        this.pointer(e.get_position(Some(this)), true, e.key_modifiers());
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        let position = e.get_position(Some(this));
        if !this.pressed.get() {
            let at = Some((position.x, position.y));
            if *this.hovered.borrow() == at {
                return;
            }
            *this.hovered.borrow_mut() = at;
        }
        this.pointer(position, this.pressed.get(), e.key_modifiers());
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        if this.pressed.replace(false) {
            e.set_handled(true);
            e.pointer().capture(None);
            this.pointer(e.get_position(Some(this)), false, e.key_modifiers());
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        if let Some(name) = key_name(e) {
            e.set_handled(true);
            this.send(json!({"key": name, "down": true}));
        }
    }

    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        if let Some(name) = key_name(e) {
            e.set_handled(true);
            this.send(json!({"key": name, "down": false}));
        }
    }
}

/// The host-neutral name of a key: arrows, space, enter, escape, tab, backspace, and a single
/// lowercase character for letters and digits. Any other key is not sent.
pub fn key_name(e: &KeyEventArgs) -> Option<String> {
    let named = match e.key {
        Key::Left => "left",
        Key::Right => "right",
        Key::Up => "up",
        Key::Down => "down",
        Key::Space => "space",
        Key::Return => "enter",
        Key::Escape => "escape",
        Key::Tab => "tab",
        Key::Back => "backspace",
        _ => {
            let symbol = e.key_symbol.as_deref()?;
            let mut chars = symbol.chars();
            let first = chars.next()?.to_ascii_lowercase();
            return (chars.next().is_none() && first.is_ascii_alphanumeric())
                .then(|| first.to_string());
        }
    };
    Some(named.to_owned())
}

impl VisualImpl for SceneSurface {
    fn render(this: &Self, context: &mut DrawingContext) {
        this.tell_size();
        let scene = this.scene();
        let placement = this.placement();
        let origin = this.origin();
        // Pushed states are popped explicitly, in order (`DrawingContext::pop`).
        let placed = context.push_transform(
            Matrix::create_scale(placement.scale, placement.scale)
                * Matrix::create_translation(origin.x, origin.y),
        );
        for shape in &scene.shapes {
            draw(context, &shape.primitive);
        }
        context.pop(placed);
    }
}

/// A brush of a packed `0xRRGGBBAA` colour.
fn packed(colour: i64) -> Rc<dyn IBrush> {
    tinted(Tint::packed(colour))
}

fn colour(tint: Tint) -> Color {
    let byte = |channel: f64| (channel.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::from_argb(byte(tint.a), byte(tint.r), byte(tint.g), byte(tint.b))
}

fn tinted(tint: Tint) -> Rc<dyn IBrush> {
    SolidColorBrush::with_color(colour(tint)).into()
}

/// Draws one primitive in scene units.
fn draw(context: &mut DrawingContext, primitive: &Primitive) {
    match primitive {
        Primitive::Rect {
            x,
            y,
            w,
            h,
            angle,
            colour,
        } => {
            let brush = packed(*colour);
            let rect = Rect::new(-w / 2.0, -h / 2.0, *w, *h);
            let placed = Matrix::create_rotation(*angle) * Matrix::create_translation(*x, *y);
            let placed = context.push_transform(placed);
            context.fill_rectangle(&brush, rect, 0.0);
            context.pop(placed);
        }
        Primitive::Disc {
            x,
            y,
            radius,
            colour,
        } => {
            context.draw_ellipse_at(
                Some(&packed(*colour)),
                None,
                Point::new(*x, *y),
                *radius,
                *radius,
            );
        }
        Primitive::Strip {
            points,
            colour,
            stroke,
        } => {
            if points.len() < 2 {
                return;
            }
            let geometry = StreamGeometry::new();
            {
                let mut path = geometry.open();
                path.begin_figure(Point::new(points[0].0, points[0].1), false);
                for (x, y) in &points[1..] {
                    path.line_to(Point::new(*x, *y), true);
                }
                path.end_figure(false);
                path.dispose();
            }
            let pen: Rc<dyn IPen> = Pen::with_all(
                Some(packed(*colour)),
                *stroke,
                None,
                PenLineCap::Round,
                PenLineJoin::Round,
                10.0,
            )
            .into();
            context.draw_geometry(None, Some(&pen), &geometry.upcast());
        }
        // Sprites name images of the application's atlas, which this host does not load yet: a
        // sprite is drawn as its tinted quad, as the GPUI host draws one without its image.
        Primitive::Sprite {
            index,
            x,
            y,
            w,
            h,
            angle,
            colour,
        } => {
            let tint = if *colour as u32 != 0xFFFF_FFFF {
                Tint::packed(*colour)
            } else {
                Tint::packed(match index {
                    0 => 0x87CE_EBFF,
                    1 => 0xFFA5_00FF,
                    2 => 0x55AA_55FF,
                    _ => 0x8888_88FF,
                })
            };
            let placed = Matrix::create_rotation(*angle) * Matrix::create_translation(*x, *y);
            let placed = context.push_transform(placed);
            context.fill_rectangle(&tinted(tint), Rect::new(-w / 2.0, -h / 2.0, *w, *h), 0.0);
            context.pop(placed);
        }
        Primitive::Glyphs {
            text,
            x,
            y,
            size,
            colour,
        } => {
            let formatted = line(text, *size, *colour, FontWeight::Normal);
            context.draw_text(&formatted, Point::new(*x, *y));
        }
        Primitive::Lettering {
            text,
            x,
            y,
            size,
            colour,
            anchor,
            weight,
        } => {
            let formatted = line(text, *size, *colour, FontWeight(weight.round() as i32));
            let shift = formatted.width() * (*anchor as f64) / 2.0;
            context.draw_text(&formatted, Point::new(x - shift, *y));
        }
        Primitive::Path {
            steps,
            outline,
            paint,
            transform,
        } => {
            let geometry = path(steps, matches!(outline, Outline::Fill));
            // The path's own transform: a translation, then a scale.
            let moved = transform.map(|(x, y, scale)| {
                context.push_transform(
                    Matrix::create_translation(x, y) * Matrix::create_scale(scale, scale),
                )
            });
            let brush = painted(paint);
            match outline {
                Outline::Fill => context.draw_geometry(Some(&brush), None, &geometry.upcast()),
                Outline::Stroke {
                    width,
                    bevel,
                    dashes,
                } => {
                    let dash = (!dashes.is_empty()).then(|| {
                        // A dash style counts in pen widths; the scene's dashes are in units.
                        let relative: Vec<f64> = dashes
                            .iter()
                            .map(|dash| dash / width.max(f64::EPSILON))
                            .collect();
                        DashStyle::with_dashes(Some(&relative), 0.0).into()
                    });
                    let join = if *bevel {
                        PenLineJoin::Bevel
                    } else {
                        PenLineJoin::Miter
                    };
                    let pen: Rc<dyn IPen> =
                        Pen::with_all(Some(brush), *width, dash, PenLineCap::Flat, join, 10.0)
                            .into();
                    context.draw_geometry(None, Some(&pen), &geometry.upcast());
                }
            }
            if let Some(moved) = moved {
                context.pop(moved);
            }
        }
    }
}

/// One line of text, the top of its line at the origin.
fn line(text: &str, size: f64, colour: i64, weight: FontWeight) -> FormattedText {
    let typeface = Typeface::with_style(
        FontFamily::default(),
        FontStyle::Normal,
        weight,
        FontStretch::Normal,
    );
    FormattedText::new(
        text,
        Locale::UNKNOWN,
        FlowDirection::LeftToRight,
        typeface,
        size.max(0.1),
        Some(packed(colour)),
    )
}

/// The geometry of a path's steps, replayed in order.
fn path(steps: &[Step], filled: bool) -> Ref<StreamGeometry> {
    let geometry = StreamGeometry::new();
    {
        let mut path = geometry.open();
        let mut open = false;
        let mut at = Point::new(0.0, 0.0);
        let start = |path: &mut ferroui_base::media::StreamGeometryContext,
                     open: &mut bool,
                     from: Point| {
            if !*open {
                path.begin_figure(from, filled);
                *open = true;
            }
        };
        for step in steps {
            match step {
                Step::Move(x, y) => {
                    if open {
                        path.end_figure(false);
                        open = false;
                    }
                    at = Point::new(*x, *y);
                    start(&mut path, &mut open, at);
                }
                Step::Line(x, y) => {
                    start(&mut path, &mut open, at);
                    at = Point::new(*x, *y);
                    path.line_to(at, true);
                }
                Step::Quad(x, y, cx, cy) => {
                    start(&mut path, &mut open, at);
                    at = Point::new(*x, *y);
                    path.quadratic_bezier_to(Point::new(*cx, *cy), at, true);
                }
                Step::Cubic(x, y, ax, ay, bx, by) => {
                    start(&mut path, &mut open, at);
                    at = Point::new(*x, *y);
                    path.cubic_bezier_to(Point::new(*ax, *ay), Point::new(*bx, *by), at, true);
                }
                Step::Arc {
                    rx,
                    ry,
                    rotation,
                    large,
                    sweep,
                    x,
                    y,
                } => {
                    start(&mut path, &mut open, at);
                    at = Point::new(*x, *y);
                    let direction = if *sweep {
                        SweepDirection::Clockwise
                    } else {
                        SweepDirection::CounterClockwise
                    };
                    path.arc_to(at, Size::new(*rx, *ry), *rotation, *large, direction, true);
                }
                Step::Polygon { closed, corners } => {
                    let Some(first) = corners.first() else {
                        continue;
                    };
                    if open {
                        path.end_figure(false);
                    }
                    path.begin_figure(Point::new(first.0, first.1), filled);
                    for (x, y) in &corners[1..] {
                        path.line_to(Point::new(*x, *y), true);
                    }
                    path.end_figure(*closed);
                    open = false;
                    at = Point::new(first.0, first.1);
                }
                Step::Close => {
                    if open {
                        path.end_figure(true);
                        open = false;
                    }
                }
            }
        }
        if open {
            path.end_figure(false);
        }
        path.dispose();
    }
    geometry
}

/// A path's paint: a colour, or a two-stop linear gradient at an angle in degrees (0 points up,
/// 90 to the right, as CSS and the GPUI host draw it).
fn painted(paint: &Paint) -> Rc<dyn IBrush> {
    match paint {
        Paint::Solid(tint) => tinted(*tint),
        Paint::Linear {
            angle,
            from,
            from_at,
            to,
            to_at,
            ..
        } => {
            let radians = angle.to_radians();
            let (dx, dy) = (radians.sin() / 2.0, -radians.cos() / 2.0);
            let brush = LinearGradientBrush::new();
            brush.set_start_point(RelativePoint::new(
                0.5 - dx,
                0.5 - dy,
                RelativeUnit::Relative,
            ));
            brush.set_end_point(RelativePoint::new(
                0.5 + dx,
                0.5 + dy,
                RelativeUnit::Relative,
            ));
            brush.set_gradient_stops(GradientStops::from_items([
                GradientStop::with_color_and_offset(colour(*from), *from_at),
                GradientStop::with_color_and_offset(colour(*to), *to_at),
            ]));
            brush.into()
        }
    }
}
