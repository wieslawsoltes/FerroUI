// Copyright (c) 2026 Wiesław Šoltés. MIT. Native port of CadDrawingCanvas.cs.
use super::cad_plan::{Entity, Plan};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, Point, Ref, RelativePoint, Size,
    StyledElementImpl, Thickness, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    input::{
        Cursor, IPointer, InputElement, InputElementImpl, InputElementImplExt, PointerEventArgs,
        PointerPressedEventArgs, PointerReleasedEventArgs, PointerWheelEventArgs,
        StandardCursorType,
    },
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{
        ArcSegment, Color, FontFamily, LineSegment, MediaCollection, PathFigure, PathGeometry,
        RotateTransform, SolidColorBrush, SweepDirection,
    },
    metadata::MarkupDelegate,
    styling::ThemeVariant,
    utilities::math_utilities::{max, min},
};
use ferroui_controls::{
    Canvas, Control, ControlImpl, ControlImplExt, Grid, PanelImpl, SizeChangedEventArgs, TextBlock,
    shapes::{Ellipse, Line, Path, Rectangle},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};
type EntityGroup<'a> = ((&'a str, bool, bool), Vec<&'a Entity>);
#[repr(C)]
pub struct CadDrawingCanvas {
    base: Grid,
    pub(super) grid_layer: Ref<Canvas>,
    pub(super) drawing: Ref<Canvas>,
    pub(super) overlay: Ref<Canvas>,
    pub(super) cross_h: Ref<Line>,
    pub(super) cross_v: Ref<Line>,
    pub(super) pick_box: Ref<Rectangle>,
    pub(super) plan: Plan,
    pub(super) center: Cell<Point>,
    scale: Cell<f64>,
    pub(super) fitted: Cell<bool>,
    pub(super) pan_start: Cell<Option<Point>>,
    pub(super) cursor_position: Cell<Point>,
    dark: Cell<bool>,
    view_insets: Cell<Thickness>,
    pan_mode: Cell<bool>,
    lineweights: Cell<bool>,
    hidden: Rc<RefCell<HashSet<String>>>,
    cursor_changed: RefCell<Option<MarkupDelegate>>,
    captures: RefCell<Vec<Rc<dyn IPointer>>>,
}
ferro_class!(CadDrawingCanvas:Grid);
ferro_class_info!(CadDrawingCanvas{new:CadDrawingCanvas::new,markup:{namespace:"FerroRibbon.Sample.Cad",
 properties:[ViewInsets:Thickness{get:CadDrawingCanvas::view_insets,set:CadDrawingCanvas::set_view_insets},IsPanMode:bool{get:CadDrawingCanvas::is_pan_mode,set:CadDrawingCanvas::set_is_pan_mode},IsGridVisible:bool{get:CadDrawingCanvas::is_grid_visible,set:CadDrawingCanvas::set_is_grid_visible},ShowLineweights:bool{get:CadDrawingCanvas::show_lineweights,set:CadDrawingCanvas::set_show_lineweights},CanvasColor:Color{get:CadDrawingCanvas::canvas_color},ViewScale:f64{get:CadDrawingCanvas::view_scale}],
 methods:[fn SetCursor(Point)=>CadDrawingCanvas::set_crosshair_cursor,fn ZoomExtents()=>CadDrawingCanvas::zoom_extents,fn Zoom(f64)=>|this:&CadDrawingCanvas,factor|this.zoom(factor,None),fn Zoom(f64,Option<Point>)=>CadDrawingCanvas::zoom,fn Redraw()=>CadDrawingCanvas::redraw,fn ModelToScreen(f64,f64)->Point=>CadDrawingCanvas::model_to_screen],
 events:[CursorMoved(Option<BoxedValue>,Point){add:CadDrawingCanvas::add_cursor_moved,remove:CadDrawingCanvas::remove_cursor_moved}]
}});
ferro_impl_classes!(CadDrawingCanvas:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,PanelImpl);
// Only the source filled wall path relies on the framework default thickness.
// Override metadata on this private native subtype; local/style/binding priority remains native.
#[repr(C)]
pub(super) struct DrawingPath {
    base: Path,
}
ferro_class!(DrawingPath:Path);
ferro_class_info!(DrawingPath {
    new: DrawingPath::new
});
ferro_impl_classes!(DrawingPath:FerroObjectImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,ferroui_controls::shapes::ShapeImpl);
impl StyledElementImpl for DrawingPath {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        Path::TYPE
    }
}
impl DrawingPath {
    fn static_constructor() {
        ferroui_controls::shapes::Shape::stroke_thickness_property()
            .override_default_value::<Self>(1.);
    }
    pub(super) fn new() -> Ref<Self> {
        instantiate(Self {
            base: Path::construct(),
        })
    }
}
impl CadDrawingCanvas {
    fn static_constructor() {
        ferroui_base::Visual::clip_to_bounds_property().override_default_value::<Self>(true);
    }

    pub fn construct() -> Self {
        let grid = Canvas::new();
        grid.set_is_hit_test_visible(false);
        let drawing = Canvas::new();
        drawing.set_is_hit_test_visible(false);
        let overlay = Canvas::new();
        overlay.set_is_hit_test_visible(false);
        let pick = Rectangle::new();
        pick.set_width(9.);
        pick.set_height(9.);
        pick.set_stroke_thickness(1.);
        Self {
            base: Grid::construct(),
            grid_layer: grid,
            drawing,
            overlay,
            cross_h: Line::new(),
            cross_v: Line::new(),
            pick_box: pick,
            plan: Plan::new(),
            center: Cell::new(Point::new(6000., 3600.)),
            scale: Cell::new(0.06),
            fitted: Cell::new(false),
            pan_start: Cell::new(None),
            cursor_position: Cell::new(Point::new(f64::NAN, f64::NAN)),
            dark: Cell::new(true),
            view_insets: Cell::new(Thickness::new(40., 30., 150., 130.)),
            pan_mode: Cell::new(false),
            lineweights: Cell::new(false),
            hidden: Rc::new(RefCell::new(HashSet::new())),
            cursor_changed: RefCell::new(None),
            captures: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn view_insets(&self) -> Thickness {
        self.view_insets.get()
    }
    pub fn set_view_insets(&self, value: Thickness) {
        self.view_insets.set(value);
    }
    pub fn is_pan_mode(&self) -> bool {
        self.pan_mode.get()
    }
    pub fn set_is_pan_mode(&self, value: bool) {
        self.pan_mode.set(value);
    }
    pub fn is_grid_visible(&self) -> bool {
        self.grid_layer.is_visible()
    }
    pub fn set_is_grid_visible(&self, value: bool) {
        self.grid_layer.set_is_visible(value);
    }
    pub fn show_lineweights(&self) -> bool {
        self.lineweights.get()
    }
    pub fn set_show_lineweights(&self, value: bool) {
        self.lineweights.set(value);
    }
    pub fn hidden_layers(&self) -> Rc<RefCell<HashSet<String>>> {
        self.hidden.clone()
    }
    pub fn canvas_color(&self) -> Color {
        if self.dark.get() {
            Color::from_rgb(0x21, 0x28, 0x30)
        } else {
            Color::from_rgb(0xFA, 0xFB, 0xFC)
        }
    }
    pub fn set_crosshair_cursor(&self, position: Point) {
        self.cursor_position.set(position);
        self.update_crosshair();
    }
    pub fn zoom_extents(&self) {
        let size = self.bounds().size();
        if size.width <= 0. || size.height <= 0. {
            return;
        }
        let insets = self.view_insets();
        let w = max(100., size.width - insets.left - insets.right);
        let h = max(100., size.height - insets.top - insets.bottom);
        let scale = min(w / 15000., h / 11400.);
        self.scale.set(scale);
        let screen = Point::new(insets.left + (w / 2.), insets.top + (h / 2.));
        self.center.set(Point::new(
            5350. - ((screen.x - (size.width / 2.)) / scale),
            3150. + ((screen.y - (size.height / 2.)) / scale),
        ));
        self.render();
    }
    pub fn zoom(&self, factor: f64, at: Option<Point>) {
        let size = self.bounds().size();
        let anchor = at.unwrap_or(Point::new(size.width / 2., size.height / 2.));
        let model = self.to_model(anchor);
        self.scale.set((self.scale.get() * factor).clamp(0.005, 2.));
        let after = self.to_model(anchor);
        let center = self.center.get();
        self.center.set(Point::new(
            center.x + (model.x - after.x),
            center.y + (model.y - after.y),
        ));
        self.render();
    }
    pub fn redraw(&self) {
        self.render();
    }
    pub fn model_to_screen(&self, x: f64, y: f64) -> Point {
        let size = self.bounds().size();
        let center = self.center.get();
        Point::new(
            ((x - center.x) * self.scale.get()) + (size.width / 2.),
            (size.height / 2.) - ((y - center.y) * self.scale.get()),
        )
    }
    pub fn view_scale(&self) -> f64 {
        self.scale.get()
    }
    fn to_model(&self, p: Point) -> Point {
        let size = self.bounds().size();
        let center = self.center.get();
        Point::new(
            ((p.x - (size.width / 2.)) / self.scale.get()) + center.x,
            center.y - ((p.y - (size.height / 2.)) / self.scale.get()),
        )
    }
    pub fn add_cursor_moved(&self, handler: Option<MarkupDelegate>) {
        let result =
            MarkupDelegate::combine(self.cursor_changed.borrow().as_ref(), handler.as_ref());
        self.cursor_changed.replace(result);
    }
    pub fn remove_cursor_moved(&self, handler: Option<MarkupDelegate>) {
        let result =
            MarkupDelegate::remove(self.cursor_changed.borrow().as_ref(), handler.as_ref());
        self.cursor_changed.replace(result);
    }
    fn moved(&self, p: Point) {
        if let Some(start) = self.pan_start.get() {
            let center = self.center.get();
            self.center.set(Point::new(
                center.x - ((p.x - start.x) / self.scale.get()),
                center.y + ((p.y - start.y) / self.scale.get()),
            ));
            self.pan_start.set(Some(p));
            self.render();
        }
        self.set_crosshair_cursor(p);
        let handlers = self.cursor_changed.borrow().clone();
        if let Some(h) = handlers {
            h.try_invoke(&[
                Some(Control::boxed(self.to_ref())),
                Some(Rc::new(self.to_model(p))),
            ])
            .expect("CAD CursorMoved handler");
        }
    }
    fn apply_theme(&self) {
        self.dark.set(
            ThemeVariant::to_platform_theme_variant(self.actual_theme_variant().as_ref())
                != Some(ferroui_base::platform::PlatformThemeVariant::Light),
        );
        self.set_background(Some(
            SolidColorBrush::with_color(self.canvas_color()).into(),
        ));
        let color = if self.dark.get() {
            Color::from_rgb(0xE6, 0xEA, 0xEF)
        } else {
            Color::from_rgb(0x30, 0x36, 0x3E)
        };
        let brush = SolidColorBrush::with_color(color);
        self.cross_h.set_stroke(Some(brush.clone().into()));
        self.cross_v.set_stroke(Some(brush.clone().into()));
        self.pick_box.set_stroke(Some(brush.into()));
        self.render();
    }
    fn update_crosshair(&self) {
        let cursor = self.cursor_position.get();
        let size = self.bounds().size();
        let visible = !cursor.x.is_nan() && size.width > 0.;
        self.cross_h.set_is_visible(visible);
        self.cross_v.set_is_visible(visible);
        self.pick_box.set_is_visible(visible);
        if !visible {
            return;
        }
        let y = cursor.y.round_ties_even() + 0.5;
        let x = cursor.x.round_ties_even() + 0.5;
        self.cross_h.set_start_point(Point::new(0., y));
        self.cross_h.set_end_point(Point::new(size.width, y));
        self.cross_v.set_start_point(Point::new(x, 0.));
        self.cross_v.set_end_point(Point::new(x, size.height));
        Canvas::set_left(&self.pick_box, cursor.x.round_ties_even() - 4.);
        Canvas::set_top(&self.pick_box, cursor.y.round_ties_even() - 4.);
    }
    fn layer_color(&self, layer: &str) -> Color {
        let rgb = match (layer, self.dark.get()) {
            ("A-WALL", true) => (0xC9, 0xCF, 0xD6),
            ("A-WALL", false) => (0x3A, 0x41, 0x4A),
            ("A-DOOR", true) => (0xFF, 0xD5, 0x4F),
            ("A-DOOR", false) => (0xB0, 0x7A, 0x00),
            ("A-GLAZ", true) => (0x4D, 0xD0, 0xE1),
            ("A-GLAZ", false) => (0x00, 0x83, 0x8F),
            ("A-FURN", true) => (0x9C, 0xCC, 0x65),
            ("A-FURN", false) => (0x4E, 0x7D, 0x2A),
            ("A-ANNO-DIMS", true) => (0xEF, 0x6B, 0x68),
            ("A-ANNO-DIMS", false) => (0xC6, 0x28, 0x28),
            ("A-ANNO-TEXT", true) => (0xE8, 0xEC, 0xF0),
            ("A-ANNO-TEXT", false) => (0x1F, 0x24, 0x2B),
            ("SELECT", _) => (0x3D, 0xA9, 0xF5),
            ("CENTER", true) => (0xB3, 0x88, 0xFF),
            ("CENTER", false) => (0x6A, 0x3D, 0xB8),
            (_, true) => (0xE8, 0xEC, 0xF0),
            (_, false) => (0x1F, 0x24, 0x2B),
        };
        Color::from_rgb(rgb.0, rgb.1, rgb.2)
    }
    fn render(&self) {
        let size = self.bounds().size();
        if size.width <= 0. || size.height <= 0. {
            return;
        }
        self.render_grid();
        self.drawing.children().clear();
        // GroupBy preserves first key occurrence and source order within each group.
        let mut groups: Vec<EntityGroup<'_>> = Vec::new();
        for entity in &self.plan.entities {
            let key = entity.key();
            if let Some((_, entities)) = groups.iter_mut().find(|(k, _)| *k == key) {
                entities.push(entity);
            } else {
                groups.push((key, vec![entity]));
            }
        }
        for ((layer, fill, dashed), entities) in groups {
            if self.hidden.borrow().contains(layer) {
                continue;
            }
            let geometry = PathGeometry::new();
            for entity in entities {
                self.add_entity(&geometry, entity);
            }
            let path: Ref<Path> = if fill {
                DrawingPath::new().upcast()
            } else {
                Path::new()
            };
            path.set_data(Some(geometry.upcast()));
            let brush = SolidColorBrush::with_color(self.layer_color(layer));
            if fill {
                path.set_fill(Some(brush.into()));
            } else {
                path.set_stroke(Some(brush.into()));
                path.set_stroke_thickness(if self.show_lineweights() && layer == "A-WALL" {
                    2.
                } else {
                    1.
                });
                if dashed {
                    path.set_stroke_dash_array(Some(MediaCollection::from_items([6., 4.])));
                }
            }
            self.drawing.children().add(path);
        }
        for text in &self.plan.texts {
            if self.hidden.borrow().contains(text.layer) {
                continue;
            }
            let size = text.height * self.scale.get();
            if size < 4. {
                continue;
            }
            let block = TextBlock::new();
            block.set_text(Some(&text.text));
            block.set_font_size(size);
            block.set_foreground(Some(
                SolidColorBrush::with_color(self.layer_color(text.layer)).into(),
            ));
            block.set_font_family(FontFamily::new("Arial"));
            block.measure(Size::new(f64::INFINITY, f64::INFINITY));
            let screen = self.model_to_screen(text.point.0, text.point.1);
            let desired = block.desired_size();
            Canvas::set_left(&block, screen.x - (desired.width / 2.));
            Canvas::set_top(&block, screen.y - (desired.height / 2.));
            if text.angle != 0. {
                block.set_render_transform_origin(RelativePoint::TOP_LEFT);
                block.set_render_transform(Some(
                    RotateTransform::with_angle_and_center(
                        -text.angle,
                        desired.width / 2.,
                        desired.height / 2.,
                    )
                    .into(),
                ));
            }
            self.drawing.children().add(block);
        }
        self.render_selection();
        self.update_crosshair();
    }
    fn render_grid(&self) {
        self.grid_layer.children().clear();
        let size = self.bounds().size();
        let mut minor = 500.;
        while minor * self.scale.get() < 8. {
            minor *= 5.;
        }
        let major = minor * 5.;
        let top = self.to_model(Point::new(0., 0.));
        let bottom = self.to_model(Point::new(size.width, size.height));
        let minor_geometry = PathGeometry::new();
        let major_geometry = PathGeometry::new();
        let mut x = (top.x / minor).floor() * minor;
        while x <= bottom.x {
            let sx = self.model_to_screen(x, 0.).x.round_ties_even() + 0.5;
            add_line(
                if (x % major).abs() < 0.5 {
                    &major_geometry
                } else {
                    &minor_geometry
                },
                Point::new(sx, 0.),
                Point::new(sx, size.height),
            );
            x += minor;
        }
        let mut y = (bottom.y / minor).floor() * minor;
        while y <= top.y {
            let sy = self.model_to_screen(0., y).y.round_ties_even() + 0.5;
            add_line(
                if (y % major).abs() < 0.5 {
                    &major_geometry
                } else {
                    &minor_geometry
                },
                Point::new(0., sy),
                Point::new(size.width, sy),
            );
            y += minor;
        }
        let colors = if self.dark.get() {
            [(0x29, 0x31, 0x3B), (0x33, 0x3D, 0x49)]
        } else {
            [(0xEC, 0xEE, 0xF1), (0xDC, 0xE0, 0xE5)]
        };
        for (geometry, color) in [(minor_geometry, colors[0]), (major_geometry, colors[1])] {
            let path = Path::new();
            path.set_data(Some(geometry.upcast()));
            path.set_stroke(Some(
                SolidColorBrush::with_color(Color::from_rgb(color.0, color.1, color.2)).into(),
            ));
            path.set_stroke_thickness(1.);
            self.grid_layer.children().add(path);
        }
        let origin = self.model_to_screen(0., 0.);
        let vertical = Line::new();
        let x = origin.x.round_ties_even() + 0.5;
        vertical.set_start_point(Point::new(x, 0.));
        vertical.set_end_point(Point::new(x, size.height));
        vertical.set_stroke(Some(
            SolidColorBrush::with_color(Color::from_argb(0x70, 0x5C, 0xB8, 0x5C)).into(),
        ));
        vertical.set_stroke_thickness(1.);
        self.grid_layer.children().add(vertical);
        let horizontal = Line::new();
        let y = origin.y.round_ties_even() + 0.5;
        horizontal.set_start_point(Point::new(0., y));
        horizontal.set_end_point(Point::new(size.width, y));
        horizontal.set_stroke(Some(
            SolidColorBrush::with_color(Color::from_argb(0x70, 0xE0, 0x5A, 0x5A)).into(),
        ));
        horizontal.set_stroke_thickness(1.);
        self.grid_layer.children().add(horizontal);
    }
    fn render_selection(&self) {
        if self.hidden.borrow().contains("A-FURN") {
            return;
        }
        let center = self.model_to_screen(8600., 2500.);
        let r = 600. * self.scale.get();
        let highlight = Ellipse::new();
        highlight.set_width(r * 2.);
        highlight.set_height(r * 2.);
        highlight.set_stroke(Some(
            SolidColorBrush::with_color(self.layer_color("SELECT")).into(),
        ));
        highlight.set_stroke_thickness(2.);
        highlight.set_stroke_dash_array(Some(MediaCollection::from_items([3., 2.])));
        Canvas::set_left(&highlight, center.x - r);
        Canvas::set_top(&highlight, center.y - r);
        self.drawing.children().add(highlight);
        for (dx, dy) in [(0., 0.), (r, 0.), (-r, 0.), (0., r), (0., -r)] {
            let grip = Rectangle::new();
            grip.set_width(8.);
            grip.set_height(8.);
            grip.set_fill(Some(
                SolidColorBrush::with_color(Color::from_rgb(0x2E, 0x7F, 0xE0)).into(),
            ));
            grip.set_stroke(Some(
                SolidColorBrush::with_color(Color::from_rgb(0xBF, 0xDF, 0xFF)).into(),
            ));
            grip.set_stroke_thickness(1.);
            Canvas::set_left(&grip, center.x + dx - 4.);
            Canvas::set_top(&grip, center.y + dy - 4.);
            self.drawing.children().add(grip);
        }
    }
    fn add_entity(&self, geometry: &PathGeometry, entity: &Entity) {
        match entity {
            Entity::Poly {
                points,
                closed,
                fill,
                ..
            } => {
                let first = points[0];
                let figure = PathFigure::new();
                figure.set_start_point(self.model_to_screen(first.0, first.1));
                figure.set_is_closed(*closed);
                figure.set_is_filled(*fill);
                for point in &points[1..] {
                    let segment = LineSegment::new();
                    segment.set_point(self.model_to_screen(point.0, point.1));
                    figure.segments().unwrap().add(segment.upcast());
                }
                geometry.figures().unwrap().add(figure);
            }
            Entity::Arc {
                center,
                radius,
                start,
                sweep,
                ..
            } => {
                let at = |deg: f64| {
                    self.model_to_screen(
                        center.0 + (radius * (deg * std::f64::consts::PI / 180.).cos()),
                        center.1 + (radius * (deg * std::f64::consts::PI / 180.).sin()),
                    )
                };
                let r = radius * self.scale.get();
                if sweep.abs() >= 360. {
                    let figure = PathFigure::new();
                    figure.set_start_point(at(0.));
                    figure.set_is_closed(true);
                    figure.set_is_filled(false);
                    for deg in [180., 360.] {
                        add_arc(&figure, at(deg), r, false, SweepDirection::CounterClockwise);
                    }
                    geometry.figures().unwrap().add(figure);
                } else {
                    let figure = PathFigure::new();
                    figure.set_start_point(at(*start));
                    figure.set_is_closed(false);
                    figure.set_is_filled(false);
                    add_arc(
                        &figure,
                        at(start + sweep),
                        r,
                        sweep.abs() > 180.,
                        if *sweep > 0. {
                            SweepDirection::Clockwise
                        } else {
                            SweepDirection::CounterClockwise
                        },
                    );
                    geometry.figures().unwrap().add(figure);
                }
            }
        }
    }
}
impl FerroObjectImpl for CadDrawingCanvas {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_background(Some(
            SolidColorBrush::with_color(Color::from_argb(0, 0, 0, 0)).into(),
        ));
        this.children().add(this.grid_layer.clone());
        this.children().add(this.drawing.clone());
        this.children().add(this.overlay.clone());
        this.cross_h.set_stroke_thickness(1.);
        this.cross_v.set_stroke_thickness(1.);
        this.overlay.children().add(this.cross_h.clone());
        this.overlay.children().add(this.cross_v.clone());
        this.overlay.children().add(this.pick_box.clone());
        this.set_cursor(Some(Cursor::new(StandardCursorType::Cross)));
        let weak = this.to_ref().downgrade();
        this.actual_theme_variant_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.apply_theme();
            }
        })
        .detach();
    }
}
impl ControlImpl for CadDrawingCanvas {
    fn on_size_changed(this: &Self, e: &SizeChangedEventArgs) {
        Self::parent_on_size_changed(this, e);
        if !this.fitted.get() && this.bounds().width > 0. {
            this.zoom_extents();
            this.fitted.set(true);
        } else {
            this.render();
        }
    }
    fn on_loaded(this: &Self, e: &ferroui_base::interactivity::RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.apply_theme();
    }
}
impl InputElementImpl for CadDrawingCanvas {
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        this.moved(e.get_current_point(Some(this)).position);
    }
    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        this.set_crosshair_cursor(Point::new(f64::NAN, f64::NAN));
    }
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        let point = e.get_current_point(Some(this));
        if point.properties.is_middle_button_pressed
            || (this.is_pan_mode() && point.properties.is_left_button_pressed)
        {
            this.pan_start.set(Some(point.position));
            e.pointer()
                .capture(Some(&this.to_ref().upcast::<InputElement>()));
            if !this
                .captures
                .borrow()
                .iter()
                .any(|p| Rc::ptr_eq(p, e.pointer()))
            {
                this.captures.borrow_mut().push(e.pointer().clone());
            }
            e.set_handled(true);
        }
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if this.pan_start.take().is_some() {
            let captures = std::mem::take(&mut *this.captures.borrow_mut());
            for pointer in captures {
                if pointer.captured().is_some_and(|v| v.ptr_eq(&this.to_ref())) {
                    pointer.capture(None);
                }
            }
        }
    }
    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);
        let point = e.get_current_point(Some(this));
        let delta = e.delta();
        let delta = if delta.y != 0. { delta.y } else { delta.x };
        this.zoom(
            if delta > 0. { 1.15 } else { 1. / 1.15 },
            Some(point.position),
        );
        e.set_handled(true);
    }
}
fn add_line(geometry: &PathGeometry, a: Point, b: Point) {
    let figure = PathFigure::new();
    figure.set_start_point(a);
    figure.set_is_closed(false);
    figure.set_is_filled(false);
    let segment = LineSegment::new();
    segment.set_point(b);
    figure.segments().unwrap().add(segment.upcast());
    geometry.figures().unwrap().add(figure);
}
fn add_arc(figure: &PathFigure, point: Point, r: f64, large: bool, sweep: SweepDirection) {
    let segment = ArcSegment::new();
    segment.set_point(point);
    segment.set_size(Size::new(r, r));
    segment.set_is_large_arc(large);
    segment.set_sweep_direction(sweep);
    figure.segments().unwrap().add(segment.upcast());
}
