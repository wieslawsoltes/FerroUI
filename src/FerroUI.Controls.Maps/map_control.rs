//! The `MapControl` control.

use crate::camera::{MapCamera, MAX_ZOOM};
use crate::map_elements::{view_path, MapElement, MapElementsLayer};
use crate::projection::{visible_tiles, BasicGeoposition, TILE_SIZE};
use crate::tile_cache::source_key;
use crate::tile_source::{RasterTileSource, SharedTileLoader};
use crate::tile_store::{TileStore, MEMORY_BUDGET};
use ferroui_base::input::gesture_recognizers::PinchGestureRecognizer;
use ferroui_base::input::{
    IPointer, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
    PinchEndedEventArgs, PinchEventArgs, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerType, PointerWheelEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{
    Color, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, MediaContext,
    StreamGeometry, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::utilities::Locale;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, ferro_property,
    instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs,
    Point, Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

/// A decoded tile of 256 by 256 pixels, for the budget of the memory cache.
const DECODED_TILE_BYTES: u64 = (TILE_SIZE as u64) * (TILE_SIZE as u64) * 4;
/// The pixels an arrow key moves the map.
const KEY_PAN: f64 = 100.0;
/// The zoom levels one notch of a wheel changes.
const WHEEL_ZOOM: f64 = 0.5;
/// How far back the speed of a released drag is measured, in milliseconds.
const FLICK_WINDOW_MS: u64 = 100;
const ICON_RADIUS: f64 = 7.0;
const ATTRIBUTION_SIZE: f64 = 11.0;

/// Whether an interaction is available to the user.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
#[repr(i32)]
pub enum MapInteractionMode {
    #[default]
    Auto = 0,
    Disabled = 1,
}
ferroui_base::ferro_markup_enum!(MapInteractionMode { Auto, Disabled }, { namespace: "FerroUI.Controls.Maps" });
ferroui_base::styled_value!(BasicGeoposition);

struct Drag {
    pointer: Weak<dyn IPointer>,
    last: Point,
    samples: VecDeque<(u64, Point)>,
}

/// A map of raster tiles with elements over them.
#[repr(C)]
pub struct MapControl {
    base: Control,
    camera: RefCell<MapCamera>,
    store: TileStore<Bitmap>,
    layers: RefCell<Vec<Rc<MapElementsLayer>>>,
    layer_listener: RefCell<Option<Rc<dyn Fn()>>>,
    tile_source: RefCell<Option<Rc<RasterTileSource>>>,
    /// The camera is writing its values to the properties.
    is_syncing: Cell<bool>,
    is_frame_requested: Cell<bool>,
    last_frame: Cell<Option<f64>>,
    drag: RefCell<Option<Drag>>,
    pinch_start: Cell<Option<f64>>,
}

ferro_class!(MapControl: Control);
ferro_class_info!(MapControl {
    new: MapControl::new
});
ferro_impl_classes!(MapControl: StyledElementImpl, InteractiveImpl);

ferro_properties! { impl MapControl {
    ferro_property!(
        /// Defines the `Center` property.
        pub fn center_property() -> StyledProperty<BasicGeoposition> {
            FerroProperty::register::<MapControl, _>("Center", BasicGeoposition::default())
        }
    );

    ferro_property!(
        /// Defines the `ZoomLevel` property.
        pub fn zoom_level_property() -> StyledProperty<f64> {
            FerroProperty::register::<MapControl, _>("ZoomLevel", 2.0)
        }
    );

    ferro_property!(
        /// Defines the `MapServiceToken` property. It has no meaning off Windows and no provider
        /// here reads it; it is kept for markup written for WinUI.
        pub fn map_service_token_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<MapControl, _>("MapServiceToken", None)
        }
    );

    ferro_property!(
        /// Defines the `PanInteractionMode` property.
        pub fn pan_interaction_mode_property() -> StyledProperty<MapInteractionMode> {
            FerroProperty::register::<MapControl, _>("PanInteractionMode", MapInteractionMode::Auto)
        }
    );

    ferro_property!(
        /// Defines the `ZoomInteractionMode` property.
        pub fn zoom_interaction_mode_property() -> StyledProperty<MapInteractionMode> {
            FerroProperty::register::<MapControl, _>("ZoomInteractionMode", MapInteractionMode::Auto)
        }
    );
}}

impl MapControl {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            camera: RefCell::new(MapCamera::default()),
            store: TileStore::new(MEMORY_BUDGET, |bytes| {
                let mut stream = std::io::Cursor::new(bytes);
                Bitmap::from_stream(&mut stream)
                    .ok()
                    .map(|bitmap| (bitmap, DECODED_TILE_BYTES))
            }),
            layers: RefCell::new(Vec::new()),
            layer_listener: RefCell::new(None),
            tile_source: RefCell::new(None),
            is_syncing: Cell::new(false),
            is_frame_requested: Cell::new(false),
            last_frame: Cell::new(None),
            drag: RefCell::new(None),
            pinch_start: Cell::new(None),
        }
    }

    /// Creates a map.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_focusable(true);
        this.set_clip_to_bounds(true);
        let weak = this.downgrade();
        this.store.set_on_loaded(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_visual();
            }
        });
        let weak = this.downgrade();
        *this.layer_listener.borrow_mut() = Some(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_visual();
            }
        }));
        this.gesture_recognizers()
            .add(PinchGestureRecognizer::new());
        let weak = this.downgrade();
        this.add_handler(
            InputElement::pinch_event(),
            move |_, args: &PinchEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_pinch(args);
                }
            },
        );
        let weak = this.downgrade();
        this.add_handler(
            InputElement::pinch_ended_event(),
            move |_, _: &PinchEndedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.pinch_start.set(None);
                }
            },
        );
        this
    }

    /// The place in the middle of the view.
    pub fn center(&self) -> BasicGeoposition {
        self.get_value(Self::center_property())
    }

    /// Goes to a place.
    pub fn set_center(&self, value: BasicGeoposition) {
        self.set_value(Self::center_property(), value);
    }

    /// The zoom level: the world is `256 * 2^level` pixels wide.
    pub fn zoom_level(&self) -> f64 {
        self.get_value(Self::zoom_level_property())
    }

    /// Sets the zoom level, limited to `0..=22`.
    pub fn set_zoom_level(&self, value: f64) {
        self.set_value(Self::zoom_level_property(), value);
    }

    /// The token of a map service; see [`Self::map_service_token_property`].
    pub fn map_service_token(&self) -> Option<String> {
        self.get_value(Self::map_service_token_property())
    }

    /// Sets the token of a map service.
    pub fn set_map_service_token(&self, value: Option<String>) {
        self.set_value(Self::map_service_token_property(), value);
    }

    /// Whether the user can move the map.
    pub fn pan_interaction_mode(&self) -> MapInteractionMode {
        self.get_value(Self::pan_interaction_mode_property())
    }

    /// Sets whether the user can move the map.
    pub fn set_pan_interaction_mode(&self, value: MapInteractionMode) {
        self.set_value(Self::pan_interaction_mode_property(), value);
    }

    /// Whether the user can zoom the map.
    pub fn zoom_interaction_mode(&self) -> MapInteractionMode {
        self.get_value(Self::zoom_interaction_mode_property())
    }

    /// Sets whether the user can zoom the map.
    pub fn set_zoom_interaction_mode(&self, value: MapInteractionMode) {
        self.set_value(Self::zoom_interaction_mode_property(), value);
    }

    /// The source of the tiles and what fetches them. No provider is built in.
    pub fn set_tile_source(
        &self,
        source: Option<Rc<RasterTileSource>>,
        loader: Option<SharedTileLoader>,
    ) {
        *self.tile_source.borrow_mut() = source.clone();
        self.store.set_source(source, loader);
        self.invalidate_visual();
    }

    /// The source of the tiles.
    pub fn tile_source(&self) -> Option<Rc<RasterTileSource>> {
        self.tile_source.borrow().clone()
    }

    /// Keeps the encoded tiles of a source in a folder, within a budget of bytes. `template`
    /// names the source in the folder. Not available in the browser.
    pub fn set_disk_cache(
        &self,
        folder: &std::path::Path,
        template: &str,
        budget: u64,
    ) -> std::io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let worker =
                crate::tile_store::disk::DiskWorker::open(folder, &source_key(template), budget)?;
            self.store.set_disk(Some(worker));
            Ok(())
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (folder, source_key(template), budget);
            Err(std::io::ErrorKind::Unsupported.into())
        }
    }

    /// The layers, in drawing order.
    pub fn layers(&self) -> Vec<Rc<MapElementsLayer>> {
        self.layers.borrow().clone()
    }

    /// Adds a layer on top of the others.
    pub fn add_layer(&self, layer: Rc<MapElementsLayer>) {
        if let Some(listener) = self.layer_listener.borrow().as_ref() {
            layer.add_listener(listener.clone());
        }
        self.layers.borrow_mut().push(layer);
        self.invalidate_visual();
    }

    /// Removes a layer. Returns whether the map had it.
    pub fn remove_layer(&self, layer: &Rc<MapElementsLayer>) -> bool {
        let removed = {
            let mut layers = self.layers.borrow_mut();
            let before = layers.len();
            layers.retain(|item| !Rc::ptr_eq(item, layer));
            layers.len() != before
        };
        if removed {
            if let Some(listener) = self.layer_listener.borrow().as_ref() {
                layer.remove_listener(listener);
            }
            self.invalidate_visual();
        }
        removed
    }

    /// Writes the camera to the properties and redraws.
    fn sync_from_camera(&self) {
        let (center, zoom) = {
            let camera = self.camera.borrow();
            (camera.center(), camera.zoom())
        };
        self.is_syncing.set(true);
        self.set_value(Self::center_property(), center);
        self.set_value(Self::zoom_level_property(), zoom);
        self.is_syncing.set(false);
        self.invalidate_visual();
    }

    fn request_frame(&self) {
        if self.is_frame_requested.get() || !self.camera.borrow().is_moving() {
            return;
        }
        self.is_frame_requested.set(true);
        let weak = self.to_ref().downgrade();
        MediaContext::instance().request_animation_frame(move |time| {
            if let Some(this) = weak.upgrade() {
                this.on_frame(time.total_seconds());
            }
        });
    }

    fn on_frame(&self, now: f64) {
        self.is_frame_requested.set(false);
        let seconds = self
            .last_frame
            .replace(Some(now))
            .map_or(1.0 / 60.0, |last| (now - last).clamp(0.0, 0.1));
        let changed = self.camera.borrow_mut().step(seconds);
        if changed {
            self.sync_from_camera();
        }
        if self.camera.borrow().is_moving() {
            self.request_frame();
        } else {
            self.last_frame.set(None);
        }
    }

    /// `point` of the control, from the middle of the view.
    fn anchor(&self, point: Point) -> (f64, f64) {
        let size = self.bounds().size();
        (point.x - size.width / 2.0, point.y - size.height / 2.0)
    }

    fn can_pan(&self) -> bool {
        self.pan_interaction_mode() != MapInteractionMode::Disabled
    }

    fn can_zoom(&self) -> bool {
        self.zoom_interaction_mode() != MapInteractionMode::Disabled
    }

    fn on_pinch(&self, args: &PinchEventArgs) {
        if !self.can_zoom() || !(args.scale() > 0.0) {
            return;
        }
        self.end_drag();
        let start = match self.pinch_start.get() {
            Some(start) => start,
            None => {
                let start = self.camera.borrow().zoom();
                self.pinch_start.set(Some(start));
                start
            }
        };
        let anchor = self.anchor(args.scale_origin());
        self.camera
            .borrow_mut()
            .zoom_at(start + args.scale().log2(), anchor);
        self.sync_from_camera();
        args.set_handled(true);
    }

    fn end_drag(&self) -> Option<Drag> {
        let drag = self.drag.borrow_mut().take()?;
        if let Some(pointer) = drag.pointer.upgrade() {
            pointer.capture(None);
        }
        Some(drag)
    }

    /// The speed of the end of a drag, in pixels per second.
    fn release_speed(samples: &VecDeque<(u64, Point)>) -> (f64, f64) {
        let Some(&(last_time, last)) = samples.back() else {
            return (0.0, 0.0);
        };
        let first = samples
            .iter()
            .find(|(time, _)| last_time.saturating_sub(*time) <= FLICK_WINDOW_MS);
        let Some(&(first_time, first)) = first else {
            return (0.0, 0.0);
        };
        if last_time <= first_time {
            return (0.0, 0.0);
        }
        let seconds = (last_time - first_time) as f64 / 1000.0;
        ((last.x - first.x) / seconds, (last.y - first.y) / seconds)
    }

    fn draw_elements(
        &self,
        context: &mut DrawingContext<'_>,
        center: BasicGeoposition,
        zoom: f64,
        size: Size,
    ) {
        let layers = self.layers.borrow().clone();
        for layer in layers.iter().filter(|layer| layer.is_visible()) {
            for element in layer.elements() {
                match element {
                    MapElement::Polygon(polygon) => {
                        let points =
                            view_path(&polygon.path, center, zoom, size.width, size.height);
                        let fill = brush(polygon.fill);
                        let stroke = pen(polygon.stroke, polygon.stroke_thickness);
                        draw_path(context, &points, true, Some(&fill), Some(&stroke));
                    }
                    MapElement::Polyline(line) => {
                        let points = view_path(&line.path, center, zoom, size.width, size.height);
                        let stroke = pen(line.stroke, line.stroke_thickness);
                        draw_path(context, &points, false, None, Some(&stroke));
                    }
                    MapElement::Icon(icon) => {
                        let point =
                            view_path(&[icon.location], center, zoom, size.width, size.height)[0];
                        let fill = brush(icon.color);
                        let outline = pen(Color::from_rgb(0xFF, 0xFF, 0xFF), 2.0);
                        let bounds = Rect::new(
                            point.0 - ICON_RADIUS,
                            point.1 - ICON_RADIUS,
                            ICON_RADIUS * 2.0,
                            ICON_RADIUS * 2.0,
                        );
                        context.draw_ellipse(Some(&fill), Some(&outline), bounds);
                        if let Some(title) = icon.title.as_deref() {
                            let text = label(title, Color::from_rgb(0x20, 0x20, 0x20));
                            let origin = Point::new(
                                point.0 + ICON_RADIUS + 3.0,
                                point.1 - text.height() / 2.0,
                            );
                            let back = Rect::new(
                                origin.x - 2.0,
                                origin.y,
                                text.width() + 4.0,
                                text.height(),
                            );
                            context.fill_rectangle(
                                &brush(Color::from_argb(0xC0, 0xFF, 0xFF, 0xFF)),
                                back,
                                0.0,
                            );
                            context.draw_text(&text, origin);
                        }
                    }
                }
            }
        }
    }

    fn draw_attribution(&self, context: &mut DrawingContext<'_>, size: Size) {
        let Some(source) = self.tile_source.borrow().clone() else {
            return;
        };
        if source.attribution().is_empty() {
            return;
        }
        let text = label(source.attribution(), Color::from_rgb(0x20, 0x20, 0x20));
        let (width, height) = (text.width() + 8.0, text.height() + 2.0);
        let back = Rect::new(
            (size.width - width).max(0.0),
            (size.height - height).max(0.0),
            width,
            height,
        );
        context.fill_rectangle(&brush(Color::from_argb(0xC0, 0xFF, 0xFF, 0xFF)), back, 0.0);
        context.draw_text(&text, Point::new(back.x + 4.0, back.y + 1.0));
    }
}

fn brush(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

fn pen(color: Color, thickness: f64) -> Rc<dyn IPen> {
    Rc::new(ImmutablePen::with_brush(
        Some(Rc::new(ImmutableSolidColorBrush::new(color))),
        thickness,
    ))
}

fn label(text: &str, color: Color) -> FormattedText {
    FormattedText::new(
        text,
        Locale::UNKNOWN,
        FlowDirection::LeftToRight,
        Typeface::default_typeface(),
        ATTRIBUTION_SIZE,
        Some(brush(color)),
    )
}

fn draw_path(
    context: &mut DrawingContext<'_>,
    points: &[(f64, f64)],
    is_closed: bool,
    fill: Option<&Rc<dyn IBrush>>,
    stroke: Option<&Rc<dyn IPen>>,
) {
    let Some((&first, rest)) = points.split_first() else {
        return;
    };
    if rest.is_empty() {
        return;
    }
    let geometry = StreamGeometry::new();
    {
        let mut figure = geometry.open();
        figure.begin_figure(Point::new(first.0, first.1), fill.is_some());
        for point in rest {
            figure.line_to(Point::new(point.0, point.1), true);
        }
        figure.end_figure(is_closed);
    }
    context.draw_geometry(fill, stroke, &geometry.upcast());
}

impl FerroObjectImpl for MapControl {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if this.is_syncing.get() {
            return;
        }
        let property = change.property();
        if property == Self::center_property().as_property()
            || property == Self::zoom_level_property().as_property()
        {
            let (center, zoom) = (this.center(), this.zoom_level());
            this.camera.borrow_mut().set_view(center, zoom);
            // The camera limits the values; the properties show what it kept.
            this.sync_from_camera();
        }
    }
}

impl LayoutableImpl for MapControl {
    fn measure_override(_this: &Self, available: Size) -> Size {
        Size::new(
            if available.width.is_finite() {
                available.width.max(0.0)
            } else {
                480.0
            },
            if available.height.is_finite() {
                available.height.max(0.0)
            } else {
                320.0
            },
        )
    }
}

impl VisualImpl for MapControl {
    fn render(this: &Self, context: &mut DrawingContext<'_>) {
        let size = this.bounds().size();
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        context.fill_rectangle(
            &brush(Color::from_rgb(0xE5, 0xE8, 0xEB)),
            Rect::from_size(size),
            0.0,
        );
        let (center, zoom) = {
            let camera = this.camera.borrow();
            (camera.center(), camera.zoom())
        };
        let source = this.tile_source.borrow().clone();
        if let Some(source) = source {
            let tiles = visible_tiles(center, zoom, size.width, size.height, source.max_zoom());
            let usable = tiles
                .iter()
                .filter(|tile| tile.id.zoom >= source.min_zoom());
            this.store.request(usable.map(|tile| tile.id));
            for tile in &tiles {
                let Some((bitmap, (left, top, side))) = this.store.best(tile.id) else {
                    continue;
                };
                // Whole pixels on both edges: neighbours meet without a seam.
                let (x0, y0) = (tile.left.round(), tile.top.round());
                let (x1, y1) = (
                    (tile.left + tile.size).round(),
                    (tile.top + tile.size).round(),
                );
                let bitmap_size = bitmap.size();
                let source_rect = Rect::new(
                    left * bitmap_size.width,
                    top * bitmap_size.height,
                    side * bitmap_size.width,
                    side * bitmap_size.height,
                );
                context.draw_image_with_rects(
                    &*bitmap,
                    source_rect,
                    Rect::new(x0, y0, x1 - x0, y1 - y0),
                );
            }
        }
        this.draw_elements(context, center, zoom, size);
        this.draw_attribution(context, size);
    }
}

impl InputElementImpl for MapControl {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        let point = e.get_current_point(Some(this));
        let is_primary =
            e.pointer().type_() != PointerType::Mouse || point.properties.is_left_button_pressed;
        if !is_primary {
            return;
        }
        this.focus();
        if !this.can_pan() {
            return;
        }
        this.camera.borrow_mut().stop();
        let position = e.get_position(Some(this));
        let mut samples = VecDeque::new();
        samples.push_back((e.timestamp(), position));
        e.pointer().capture(Some(&this.to_ref().upcast()));
        *this.drag.borrow_mut() = Some(Drag {
            pointer: Rc::downgrade(e.pointer()),
            last: position,
            samples,
        });
        e.set_handled(true);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        let position = e.get_position(Some(this));
        let delta = {
            let mut drag = this.drag.borrow_mut();
            let Some(drag) = drag.as_mut() else { return };
            let is_same = drag
                .pointer
                .upgrade()
                .is_some_and(|pointer| Rc::ptr_eq(&pointer, e.pointer()));
            if !is_same {
                return;
            }
            let delta = (position.x - drag.last.x, position.y - drag.last.y);
            drag.last = position;
            drag.samples.push_back((e.timestamp(), position));
            while drag.samples.len() > 16 {
                drag.samples.pop_front();
            }
            delta
        };
        this.camera.borrow_mut().drag(delta.0, delta.1);
        this.sync_from_camera();
        e.set_handled(true);
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        let is_same = this
            .drag
            .borrow()
            .as_ref()
            .and_then(|drag| drag.pointer.upgrade())
            .is_some_and(|pointer| Rc::ptr_eq(&pointer, e.pointer()));
        if !is_same {
            return;
        }
        if let Some(drag) = this.end_drag() {
            let (vx, vy) = Self::release_speed(&drag.samples);
            this.camera.borrow_mut().flick(vx, vy);
            this.request_frame();
        }
        e.set_handled(true);
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        this.drag.borrow_mut().take();
    }

    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);
        let delta = e.delta().y;
        if !this.can_zoom() || !delta.is_finite() || delta == 0.0 {
            return;
        }
        let anchor = this.anchor(e.get_position(Some(this)));
        {
            let mut camera = this.camera.borrow_mut();
            let target = camera.target_zoom() + delta * WHEEL_ZOOM;
            camera.animate_zoom(target.clamp(0.0, MAX_ZOOM), anchor);
        }
        this.request_frame();
        e.set_handled(true);
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        let pan = match e.key {
            Key::Left => Some((KEY_PAN, 0.0)),
            Key::Right => Some((-KEY_PAN, 0.0)),
            Key::Up => Some((0.0, KEY_PAN)),
            Key::Down => Some((0.0, -KEY_PAN)),
            _ => None,
        };
        if let Some((dx, dy)) = pan {
            if this.can_pan() {
                this.camera.borrow_mut().drag(dx, dy);
                this.sync_from_camera();
                e.set_handled(true);
            }
            return;
        }
        let steps = match e.key {
            Key::Add | Key::OemPlus => 1.0,
            Key::Subtract | Key::OemMinus => -1.0,
            _ => return,
        };
        if this.can_zoom() {
            {
                let mut camera = this.camera.borrow_mut();
                let target = camera.target_zoom() + steps;
                camera.animate_zoom(target, (0.0, 0.0));
            }
            this.request_frame();
            e.set_handled(true);
        }
    }
}

impl ControlImpl for MapControl {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        MapControlAutomationPeer::new(this).upcast()
    }
}

/// The automation peer of a map: a pane named after what the map shows.
#[repr(C)]
pub struct MapControlAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(MapControlAutomationPeer: ControlAutomationPeer);
ferro_class_info!(MapControlAutomationPeer {});
ferro_impl_classes!(MapControlAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);

impl AutomationPeerImpl for MapControlAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }

    fn get_class_name_core(_: &Self) -> String {
        "MapControl".into()
    }
}

impl MapControlAutomationPeer {
    pub fn new(owner: &MapControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
}
