//! The `MapControl` control.

use crate::camera::{MapCamera, MAX_ZOOM};
use crate::map_automation::MapControlAutomationPeer;
use crate::map_elements::{
    hit_test, neighbour, view_path, MapElement, MapElementClickEventArgs, MapElementsLayer,
    ICON_RADIUS,
};
use crate::projection::{shortest_turn, BasicGeoposition, MapView, TILE_SIZE};
use crate::tile_source::{RasterTileSource, SharedTileLoader};
use crate::tile_store::{TileStore, MEMORY_BUDGET};
use ferroui_base::input::gesture_recognizers::PinchGestureRecognizer;
use ferroui_base::input::{
    FocusChangedEventArgs, IPointer, InputElement, InputElementImpl, InputElementImplExt, Key,
    KeyEventArgs, KeyModifiers, PinchEndedEventArgs, PinchEventArgs, PointerCaptureLostEventArgs,
    PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
    PointerWheelEventArgs,
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
    Matrix, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

/// A decoded tile of 256 by 256 pixels, for the budget of the memory cache.
const DECODED_TILE_BYTES: u64 = (TILE_SIZE as u64) * (TILE_SIZE as u64) * 4;
/// The pixels an arrow key moves the map.
const KEY_PAN: f64 = 100.0;
/// The zoom levels one notch of a wheel changes.
const WHEEL_ZOOM: f64 = 0.5;
/// How far back the speed of a released drag is measured, in milliseconds.
const FLICK_WINDOW_MS: u64 = 100;
/// The degrees a key turns the map.
const KEY_TURN: f64 = 15.0;
/// How far a pointer moves, in pixels, before a press is a drag and no longer a click.
const CLICK_SLOP: f64 = 4.0;
/// Nearer the middle of the view than this, in pixels, the angle of a pointer means nothing.
const MIN_TURN_RADIUS: f64 = 8.0;
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

/// What a press of a pointer is doing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GestureKind {
    /// Moves the map, or clicks if the pointer stays put.
    Pan,
    /// Turns the map about the middle of the view.
    Rotate,
    /// Moving is disabled: the press can only click.
    Click,
}

struct Gesture {
    pointer: Weak<dyn IPointer>,
    kind: GestureKind,
    start: Point,
    last: Point,
    has_moved: bool,
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
    /// The folder and the budget in bytes of the disk cache.
    disk_cache: RefCell<Option<(PathBuf, u64)>>,
    /// The camera is writing its values to the properties.
    is_syncing: Cell<bool>,
    is_frame_requested: Cell<bool>,
    last_frame: Cell<Option<f64>>,
    gesture: RefCell<Option<Gesture>>,
    pinch_start: Cell<Option<f64>>,
    /// The bearing of the two fingers at the last pinch event.
    pinch_angle: Cell<Option<f64>>,
    /// The element with the keyboard focus, while the map has it.
    focused_element: RefCell<Option<(Weak<MapElementsLayer>, usize)>>,
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

    ferro_property!(
        /// Defines the `Heading` property: the compass direction at the top of the view, in
        /// degrees from 0 up to 360. At 90 the top of the view is east.
        pub fn heading_property() -> StyledProperty<f64> {
            FerroProperty::register::<MapControl, _>("Heading", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `RotateInteractionMode` property.
        pub fn rotate_interaction_mode_property() -> StyledProperty<MapInteractionMode> {
            FerroProperty::register::<MapControl, _>("RotateInteractionMode", MapInteractionMode::Auto)
        }
    );
}}

impl MapControl {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self::construct_with(|bytes| {
            let mut stream = std::io::Cursor::new(bytes);
            Bitmap::from_stream(&mut stream)
                .ok()
                .map(|bitmap| (bitmap, DECODED_TILE_BYTES))
        })
    }

    fn construct_with(decode: impl Fn(&[u8]) -> Option<(Bitmap, u64)> + 'static) -> Self {
        Self {
            base: Control::construct(),
            camera: RefCell::new(MapCamera::default()),
            store: TileStore::new(MEMORY_BUDGET, decode),
            layers: RefCell::new(Vec::new()),
            layer_listener: RefCell::new(None),
            tile_source: RefCell::new(None),
            disk_cache: RefCell::new(None),
            is_syncing: Cell::new(false),
            is_frame_requested: Cell::new(false),
            last_frame: Cell::new(None),
            gesture: RefCell::new(None),
            pinch_start: Cell::new(None),
            pinch_angle: Cell::new(None),
            focused_element: RefCell::new(None),
        }
    }

    /// Creates a map.
    pub fn new() -> Ref<Self> {
        Self::set_up(instantiate(Self::construct()))
    }

    /// A map whose tiles are made by `decode`, for a renderer that cannot read images.
    #[cfg(test)]
    pub(crate) fn with_decoder(
        decode: impl Fn(&[u8]) -> Option<(Bitmap, u64)> + 'static,
    ) -> Ref<Self> {
        Self::set_up(instantiate(Self::construct_with(decode)))
    }

    fn set_up(this: Ref<Self>) -> Ref<Self> {
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
                this.on_layers_changed();
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
                    this.pinch_angle.set(None);
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

    /// The compass direction at the top of the view, in degrees from 0 up to 360.
    pub fn heading(&self) -> f64 {
        self.get_value(Self::heading_property())
    }

    /// Turns the map about the middle of the view.
    pub fn set_heading(&self, value: f64) {
        self.set_value(Self::heading_property(), value);
    }

    /// Whether the user can turn the map.
    pub fn rotate_interaction_mode(&self) -> MapInteractionMode {
        self.get_value(Self::rotate_interaction_mode_property())
    }

    /// Sets whether the user can turn the map.
    pub fn set_rotate_interaction_mode(&self, value: MapInteractionMode) {
        self.set_value(Self::rotate_interaction_mode_property(), value);
    }

    /// What the view shows now, and the math between its pixels and the globe.
    pub fn view(&self) -> MapView {
        let size = self.bounds().size();
        let camera = self.camera.borrow();
        MapView::new(
            camera.center(),
            camera.zoom(),
            camera.heading(),
            size.width,
            size.height,
        )
    }

    /// The source of the tiles and what fetches them. No provider is built in.
    pub fn set_tile_source(
        &self,
        source: Option<Rc<RasterTileSource>>,
        loader: Option<SharedTileLoader>,
    ) {
        *self.tile_source.borrow_mut() = source.clone();
        self.store.set_source(source, loader);
        // Without its disk cache the new source still works: its tiles are loaded each time.
        let _ = self.open_disk_cache();
        self.invalidate_visual();
    }

    /// The source of the tiles.
    pub fn tile_source(&self) -> Option<Rc<RasterTileSource>> {
        self.tile_source.borrow().clone()
    }

    /// Keeps the encoded tiles in a folder, within a budget of bytes for each source, or stops
    /// with `None`. Each source has its own subfolder. Not available in the browser.
    pub fn set_disk_cache(&self, folder: Option<&Path>, budget: u64) -> std::io::Result<()> {
        if cfg!(target_arch = "wasm32") && folder.is_some() {
            return Err(std::io::ErrorKind::Unsupported.into());
        }
        *self.disk_cache.borrow_mut() = folder.map(|folder| (folder.to_path_buf(), budget));
        self.open_disk_cache()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_disk_cache(&self) -> std::io::Result<()> {
        // First: if the new one cannot be opened, no tile goes to the folder of the old source.
        self.store.set_disk(None);
        let config = self.disk_cache.borrow().clone();
        let source = self.tile_source.borrow().clone();
        if let (Some((folder, budget)), Some(source)) = (config, source) {
            let worker =
                crate::tile_store::disk::DiskWorker::open(&folder, &source.cache_key(), budget)?;
            self.store.set_disk(Some(worker));
        }
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    fn open_disk_cache(&self) -> std::io::Result<()> {
        Ok(())
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
        self.on_layers_changed();
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
            self.on_layers_changed();
        }
        removed
    }

    /// Writes the camera to the properties and redraws.
    fn sync_from_camera(&self) {
        let (center, zoom, heading) = {
            let camera = self.camera.borrow();
            (camera.center(), camera.zoom(), camera.heading())
        };
        self.is_syncing.set(true);
        self.set_value(Self::center_property(), center);
        self.set_value(Self::zoom_level_property(), zoom);
        self.set_value(Self::heading_property(), heading);
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

    fn can_rotate(&self) -> bool {
        self.rotate_interaction_mode() != MapInteractionMode::Disabled
    }

    fn on_pinch(&self, args: &PinchEventArgs) {
        let (can_zoom, can_rotate) = (self.can_zoom(), self.can_rotate());
        if (!can_zoom && !can_rotate) || !(args.scale() > 0.0) {
            return;
        }
        // The recognizer holds the pointers of the pinch: the drag ends without releasing them.
        self.gesture.borrow_mut().take();
        let anchor = self.anchor(args.scale_origin());
        if can_zoom {
            let start = match self.pinch_start.get() {
                Some(start) => start,
                None => {
                    let start = self.camera.borrow().zoom();
                    self.pinch_start.set(Some(start));
                    start
                }
            };
            self.camera
                .borrow_mut()
                .zoom_at(start + args.scale().log2(), anchor);
        }
        if can_rotate {
            // The bearing of the fingers grows clockwise. The turn is taken from it and not from
            // `angle_delta`, which jumps when the bearing wraps from 360 to 0. The content follows
            // the fingers, so the heading goes the other way.
            let angle = args.angle();
            if let Some(previous) = self.pinch_angle.replace(Some(angle)) {
                self.camera
                    .borrow_mut()
                    .rotate_by(-shortest_turn(previous, angle), anchor);
            }
        }
        self.sync_from_camera();
        args.set_handled(true);
    }

    /// Ends the gesture of a pointer and gives it back, if that pointer has it.
    fn take_gesture(&self, pointer: &Rc<dyn IPointer>) -> Option<Gesture> {
        let is_same = self
            .gesture
            .borrow()
            .as_ref()
            .and_then(|gesture| gesture.pointer.upgrade())
            .is_some_and(|held| Rc::ptr_eq(&held, pointer));
        if !is_same {
            return None;
        }
        let gesture = self.gesture.borrow_mut().take();
        pointer.capture(None);
        gesture
    }

    /// Runs a smooth motion to its end, as frames would.
    #[cfg(test)]
    pub(crate) fn settle_for_test(&self) {
        let mut steps = 0;
        while self.camera.borrow().is_moving() {
            self.camera.borrow_mut().step(1.0 / 60.0);
            steps += 1;
            assert!(steps < 2000, "the motion does not end");
        }
        self.sync_from_camera();
    }

    /// The automation peer, if a client has asked for one. Never makes one.
    fn existing_peer(&self) -> Option<Ref<AutomationPeer>> {
        ControlAutomationPeer::from_element(self)
    }

    fn on_layers_changed(&self) {
        self.invalidate_visual();
        // A focused element that went away gives the focus back to the map.
        let held = self.focused_element.borrow().is_some();
        if held && self.focused_element().is_none() {
            self.focused_element.borrow_mut().take();
            self.raise_focused_element_changed();
        }
        if let Some(peer) = self
            .existing_peer()
            .and_then(|peer| peer.cast::<ControlAutomationPeer>())
        {
            peer.invalidate_children();
        }
    }

    fn raise_focused_element_changed(&self) {
        if let Some(peer) = self
            .existing_peer()
            .and_then(|peer| peer.cast::<MapControlAutomationPeer>())
        {
            peer.raise_focused_element_changed();
        }
    }

    /// The element with the keyboard focus, if it is still there and visible.
    pub fn focused_element(&self) -> Option<(Rc<MapElementsLayer>, usize)> {
        let focused = self.focused_element.borrow();
        let (layer, index) = focused.as_ref()?;
        let layer = layer.upgrade()?;
        let is_shown = self
            .layers
            .borrow()
            .iter()
            .any(|item| Rc::ptr_eq(item, &layer));
        (is_shown && layer.can_focus(*index)).then_some((layer, *index))
    }

    /// Moves the keyboard focus to an element of the map, or back to the map with `None`, and
    /// brings the element into view.
    pub fn set_focused_element(&self, element: Option<(Rc<MapElementsLayer>, usize)>) {
        let element = element.filter(|(layer, index)| {
            layer.can_focus(*index)
                && self
                    .layers
                    .borrow()
                    .iter()
                    .any(|item| Rc::ptr_eq(item, layer))
        });
        let is_same = match (&element, self.focused_element()) {
            (Some((new, new_index)), Some((old, old_index))) => {
                Rc::ptr_eq(new, &old) && *new_index == old_index
            }
            (None, None) => true,
            _ => false,
        };
        *self.focused_element.borrow_mut() = element
            .as_ref()
            .map(|(layer, index)| (Rc::downgrade(layer), *index));
        if let Some((layer, index)) = &element {
            self.reveal_element(layer, *index);
        }
        if !is_same {
            self.invalidate_visual();
            self.raise_focused_element_changed();
        }
    }

    /// Centers the map on an element that is out of the view.
    pub(crate) fn reveal_element(&self, layer: &Rc<MapElementsLayer>, index: usize) {
        let view = self.view();
        let Some(anchor) = layer
            .element(index)
            .and_then(|element| element.view_anchor(&view))
        else {
            return;
        };
        let is_in_view =
            anchor.0 >= 0.0 && anchor.0 <= view.width && anchor.1 >= 0.0 && anchor.1 <= view.height;
        if !is_in_view {
            let place = view.view_to_place(anchor.0, anchor.1);
            let zoom = self.camera.borrow().zoom();
            self.camera.borrow_mut().set_view(place, zoom);
            self.sync_from_camera();
        }
    }

    /// Raises the click of an element, as a keyboard or an automation client activates it.
    pub(crate) fn activate_element(&self, layer: &Rc<MapElementsLayer>, index: usize) {
        let view = self.view();
        let Some(element) = layer.element(index) else {
            return;
        };
        let Some(anchor) = element.view_anchor(&view) else {
            return;
        };
        let location = view.view_to_place(anchor.0, anchor.1);
        layer.raise_click(&MapElementClickEventArgs { element, location });
    }

    /// Moves the focus to the next or the previous element. Returns whether the key is used: past
    /// the last element it is not, so Tab goes on to the next control and never gets stuck.
    fn move_element_focus(&self, forward: bool) -> bool {
        let (target, had_focus) = {
            let layers = self.layers.borrow();
            let current = self.focused_element().and_then(|(layer, index)| {
                layers
                    .iter()
                    .position(|item| Rc::ptr_eq(item, &layer))
                    .map(|at| (at, index))
            });
            (
                neighbour(&layers, current, forward).map(|(at, index)| (layers[at].clone(), index)),
                current.is_some(),
            )
        };
        let moved = target.is_some();
        self.set_focused_element(target);
        // Back past the first element, the focus is on the map itself.
        moved || (!forward && had_focus)
    }

    fn draw_focus(&self, context: &mut DrawingContext<'_>, view: &MapView) {
        if !self.is_focused() {
            return;
        }
        let Some((layer, index)) = self.focused_element() else {
            return;
        };
        let Some(element) = layer.element(index) else {
            return;
        };
        // Two colors, so the ring shows on any tile.
        let outer = pen(Color::from_rgb(0x00, 0x00, 0x00), 2.0);
        let inner = pen(Color::from_rgb(0xFF, 0xFF, 0xFF), 1.0);
        match &element {
            MapElement::Icon(icon) => {
                let point = view_path(&[icon.location], view)[0];
                for (stroke, radius) in [(&outer, ICON_RADIUS + 4.0), (&inner, ICON_RADIUS + 2.5)] {
                    let bounds = Rect::new(
                        point.0 - radius,
                        point.1 - radius,
                        radius * 2.0,
                        radius * 2.0,
                    );
                    context.draw_ellipse(None, Some(stroke), bounds);
                }
            }
            MapElement::Polyline(line) => {
                let points = view_path(&line.path, view);
                let wide = pen(
                    Color::from_rgb(0x00, 0x00, 0x00),
                    line.stroke_thickness + 4.0,
                );
                let narrow = pen(
                    Color::from_rgb(0xFF, 0xFF, 0xFF),
                    line.stroke_thickness + 2.0,
                );
                draw_path(context, &points, false, None, Some(&wide));
                draw_path(context, &points, false, None, Some(&narrow));
                let stroke = pen(line.stroke, line.stroke_thickness);
                draw_path(context, &points, false, None, Some(&stroke));
            }
            MapElement::Polygon(polygon) => {
                let points = view_path(&polygon.path, view);
                let wide = pen(
                    Color::from_rgb(0x00, 0x00, 0x00),
                    polygon.stroke_thickness + 4.0,
                );
                let narrow = pen(
                    Color::from_rgb(0xFF, 0xFF, 0xFF),
                    polygon.stroke_thickness + 2.0,
                );
                draw_path(context, &points, true, None, Some(&wide));
                draw_path(context, &points, true, None, Some(&narrow));
            }
        }
    }

    /// Raises the click of the topmost element at a point of the view.
    fn click_at(&self, point: Point) -> bool {
        let view = self.view();
        let layers = self.layers.borrow().clone();
        let Some((layer, element)) = hit_test(&layers, &view, (point.x, point.y)) else {
            return false;
        };
        let location = view.view_to_place(point.x, point.y);
        layer.raise_click(&MapElementClickEventArgs { element, location });
        true
    }

    /// The speed of the end of a drag, in pixels per second.
    pub(crate) fn release_speed(samples: &VecDeque<(u64, Point)>) -> (f64, f64) {
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

    fn draw_elements(&self, context: &mut DrawingContext<'_>, view: &MapView) {
        let layers = self.layers.borrow().clone();
        for layer in layers.iter().filter(|layer| layer.is_visible()) {
            for element in layer.elements() {
                match element {
                    MapElement::Polygon(polygon) => {
                        let points = view_path(&polygon.path, view);
                        let fill = brush(polygon.fill);
                        let stroke = pen(polygon.stroke, polygon.stroke_thickness);
                        draw_path(context, &points, true, Some(&fill), Some(&stroke));
                    }
                    MapElement::Polyline(line) => {
                        let points = view_path(&line.path, view);
                        let stroke = pen(line.stroke, line.stroke_thickness);
                        draw_path(context, &points, false, None, Some(&stroke));
                    }
                    MapElement::Icon(icon) => {
                        let point = view_path(&[icon.location], view)[0];
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

/// Turns the north-up frame of the tiles about the middle of the view.
pub(crate) fn content_transform(view: &MapView) -> Matrix {
    Matrix::create_translation(-view.width / 2.0, -view.height / 2.0)
        * Matrix::create_rotation((-view.heading).to_radians())
        * Matrix::create_translation(view.width / 2.0, view.height / 2.0)
}

/// The clockwise turn, in degrees, of a pointer moving from `from` to `to` around `center`.
pub(crate) fn pointer_turn(center: Point, from: Point, to: Point) -> f64 {
    let (ax, ay) = (from.x - center.x, from.y - center.y);
    let (bx, by) = (to.x - center.x, to.y - center.y);
    if ax.hypot(ay) < MIN_TURN_RADIUS || bx.hypot(by) < MIN_TURN_RADIUS {
        return 0.0;
    }
    (ax * by - ay * bx).atan2(ax * bx + ay * by).to_degrees()
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
            || property == Self::heading_property().as_property()
        {
            let (center, zoom, heading) = (this.center(), this.zoom_level(), this.heading());
            let mut camera = this.camera.borrow_mut();
            camera.set_view(center, zoom);
            camera.set_heading(heading);
            drop(camera);
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
        let view = this.view();
        let source = this.tile_source.borrow().clone();
        if let Some(source) = source {
            let tiles = view.tiles(source.max_zoom());
            let usable = tiles
                .iter()
                .filter(|tile| tile.id.zoom >= source.min_zoom());
            this.store.request(usable.map(|tile| tile.id));
            let is_turned = view.heading != 0.0;
            // A turned edge is antialiased: neighbours overlap by a pixel so no seam shows.
            let overlap = if is_turned { 1.0 } else { 0.0 };
            let turn = is_turned.then(|| context.push_transform(content_transform(&view)));
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
                    Rect::new(x0, y0, x1 - x0 + overlap, y1 - y0 + overlap),
                );
            }
            if let Some(turn) = turn {
                context.pop(turn);
            }
        }
        // Elements are placed through the view and stay upright; so does the attribution.
        this.draw_elements(context, &view);
        this.draw_focus(context, &view);
        this.draw_attribution(context, size);
    }
}

impl InputElementImpl for MapControl {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        if this.gesture.borrow().is_some() {
            // A second finger: the gesture stays with the first, and a pinch takes over from it.
            return;
        }
        let point = e.get_current_point(Some(this));
        let is_primary =
            e.pointer().type_() != PointerType::Mouse || point.properties.is_left_button_pressed;
        let wants_turn = point.properties.is_right_button_pressed
            || point.properties.is_barrel_button_pressed
            || (is_primary && e.key_modifiers().contains(KeyModifiers::CONTROL));
        let kind = if wants_turn {
            if !this.can_rotate() {
                return;
            }
            GestureKind::Rotate
        } else if !is_primary {
            return;
        } else if this.can_pan() {
            GestureKind::Pan
        } else {
            GestureKind::Click
        };
        this.focus();
        // The focus ring belongs to the keyboard.
        this.set_focused_element(None);
        this.camera.borrow_mut().stop();
        let position = e.get_position(Some(this));
        let mut samples = VecDeque::new();
        samples.push_back((e.timestamp(), position));
        e.pointer().capture(Some(&this.to_ref().upcast()));
        *this.gesture.borrow_mut() = Some(Gesture {
            pointer: Rc::downgrade(e.pointer()),
            kind,
            start: position,
            last: position,
            has_moved: false,
            samples,
        });
        e.set_handled(true);
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        let position = e.get_position(Some(this));
        let (kind, last) = {
            let mut gesture = this.gesture.borrow_mut();
            let Some(gesture) = gesture.as_mut() else {
                return;
            };
            let is_same = gesture
                .pointer
                .upgrade()
                .is_some_and(|pointer| Rc::ptr_eq(&pointer, e.pointer()));
            if !is_same {
                return;
            }
            if !gesture.has_moved {
                let moved = (position.x - gesture.start.x).hypot(position.y - gesture.start.y);
                if moved <= CLICK_SLOP {
                    return;
                }
                gesture.has_moved = true;
            }
            let last = std::mem::replace(&mut gesture.last, position);
            gesture.samples.push_back((e.timestamp(), position));
            while gesture.samples.len() > 16 {
                gesture.samples.pop_front();
            }
            (gesture.kind, last)
        };
        match kind {
            GestureKind::Pan => {
                this.camera
                    .borrow_mut()
                    .drag(position.x - last.x, position.y - last.y);
            }
            GestureKind::Rotate => {
                let size = this.bounds().size();
                let center = Point::new(size.width / 2.0, size.height / 2.0);
                // The content follows the pointer, so the heading goes the other way.
                let turn = pointer_turn(center, last, position);
                this.camera.borrow_mut().rotate_by(-turn, (0.0, 0.0));
            }
            GestureKind::Click => return,
        }
        this.sync_from_camera();
        e.set_handled(true);
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        let Some(gesture) = this.take_gesture(e.pointer()) else {
            return;
        };
        match (gesture.kind, gesture.has_moved) {
            (GestureKind::Pan, true) => {
                let (vx, vy) = Self::release_speed(&gesture.samples);
                this.camera.borrow_mut().flick(vx, vy);
                this.request_frame();
                e.set_handled(true);
            }
            (GestureKind::Rotate, true) | (GestureKind::Click, true) => e.set_handled(true),
            (GestureKind::Pan | GestureKind::Click, false) => {
                this.click_at(e.get_position(Some(this)));
                e.set_handled(true);
            }
            // A right click that did not turn anything stays free for a context menu.
            (GestureKind::Rotate, false) => {}
        }
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        this.gesture.borrow_mut().take();
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

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);
        this.set_focused_element(None);
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        match e.key {
            Key::Tab => {
                let forward = !e.key_modifiers.contains(KeyModifiers::SHIFT);
                if this.move_element_focus(forward) {
                    e.set_handled(true);
                }
                return;
            }
            Key::Enter | Key::Space => {
                if let Some((layer, index)) = this.focused_element() {
                    this.activate_element(&layer, index);
                    e.set_handled(true);
                }
                return;
            }
            Key::Escape => {
                if this.focused_element().is_some() {
                    this.set_focused_element(None);
                    e.set_handled(true);
                }
                return;
            }
            _ => {}
        }
        let is_control = e.key_modifiers.contains(KeyModifiers::CONTROL);
        let turn = match e.key {
            Key::Left if is_control => Some(-KEY_TURN),
            Key::Right if is_control => Some(KEY_TURN),
            _ => None,
        };
        if turn.is_some() || e.key == Key::Home {
            if this.can_rotate() {
                {
                    let mut camera = this.camera.borrow_mut();
                    let target = turn.map_or(0.0, |turn| camera.target_heading() + turn);
                    camera.animate_heading(target);
                }
                this.sync_from_camera();
                this.request_frame();
                e.set_handled(true);
            }
            return;
        }
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
