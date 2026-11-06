//! Native chart lifecycle, data observation, input and retained-frame caching.

use crate::renderer::NativeScene;
use crate::scene_transition::SceneTransition;
use crate::{
    Axes, Axis, ChartKind, ChartSeries, ChartSeriesCollection, ChartVisualElement,
    ChartVisualElements, GeoMarkers, HeatSeries, MapInteractionMode, TooltipPosition,
    ZoomAndPanMode,
};
use ferroui_base::collections::FerroList;
use ferroui_base::input::{
    IPointer, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, MouseButton,
    PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerType, PointerWheelEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Color, DrawingContext};
use ferroui_base::reactive::{Disposable, IDisposable, Subscription};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_livecharts::geo_map_chart_engine::shortest_rotation;
use ferroui_livecharts::{
    AxisScale, CartesianChartEngine, CartesianOptions, ChartError, DrawnMap, EasingFunctions,
    FindingStrategy, GeoLand, GeoMapChartEngine, GeoMapMeasurement, GeoMapOptions,
    GeographicBounds, HitRegion, MapViewport, Maps, PieChartEngine, PieOptions, PointId,
    PolarChartEngine, PolarOptions, PolarScale, SankeyChartEngine, SankeyOptions, Scene,
    TreemapChartEngine, TreemapOptions, ZoomDirection,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

/// Immutable event data. Handlers may change collections or remove the chart.
#[derive(Clone, Debug, PartialEq)]
pub struct ChartPointerEventArgs {
    pub position: Point,
    pub points: Vec<HitRegion>,
}

struct Measurement {
    revision: u64,
    scene: Rc<Scene>,
    x_scales: Vec<AxisScale>,
    y_scales: Vec<AxisScale>,
    /// The centre and initial rotation of a polar chart, in control coordinates.
    polar_origin: Option<(Point, f64)>,
    /// The projector, viewport and lands of a geographic map (its scene is `scene`).
    geo: Option<GeoMapMeasurement>,
}

/// An animation of the map in progress: the rotation of a globe (`RotateTo`, quadratic in and
/// out) or the bounce back of the viewport after a release (`AnimateBounce`, cubic out).
#[derive(Clone, Copy, Debug, PartialEq)]
enum MapAnimation {
    Rotation {
        from: (f64, f64),
        to: (f64, f64),
        start: f64,
        duration: f64,
    },
    Bounce {
        from: MapViewport,
        to: MapViewport,
        start: f64,
    },
}

/// The 8 steps of 16 ms of the source's bounce back.
const BOUNCE_DURATION_MS: f64 = 128.0;
/// The squared distance a press may move and still be a click on a land (5 pixels).
const MAP_CLICK_DISTANCE_SQUARED: f64 = 25.0;

/// Shared host for all chart families. All application state stays on the UI thread.
#[repr(C)]
pub struct Chart {
    base: Control,
    revision: Cell<u64>,
    measurement: RefCell<Option<Measurement>>,
    native_scene: RefCell<Option<Rc<NativeScene>>>,
    source_subscriptions: RefCell<Vec<Subscription>>,
    fallback_x_axes: OnceCell<Axes>,
    fallback_y_axes: OnceCell<Axes>,
    pointer_down: Cell<Option<Point>>,
    pointer_capture: RefCell<Option<Weak<dyn IPointer>>>,
    pointer_button: Cell<MouseButton>,
    section_start: Cell<Option<Point>>,
    section_end: Cell<Option<Point>>,
    hover: RefCell<Option<ChartPointerEventArgs>>,
    hover_changed: HandlerList<dyn Fn(&ChartPointerEventArgs)>,
    data_pointer_down: HandlerList<dyn Fn(&ChartPointerEventArgs)>,
    update_finished: HandlerList<dyn Fn()>,
    last_error: RefCell<Option<String>>,
    attached: Cell<bool>,
    transition: RefCell<Option<SceneTransition>>,
    animation_timer: RefCell<Option<Rc<DispatcherTimer>>>,
    /// The pan and zoom of a geographic map (`ZoomLevel`, `PanOffset`).
    map_viewport: Cell<MapViewport>,
    /// Where a press on a map started while it can still be a click on a land.
    map_click: Cell<Option<Point>>,
    map_animation: RefCell<Vec<MapAnimation>>,
    map_timer: RefCell<Option<Rc<DispatcherTimer>>>,
    /// Set while a rotation writes the centre, which otherwise ends the rotation.
    rotating: Cell<bool>,
}

ferro_class!(Chart:Control);
ferro_class_info!(Chart {new:Chart::new,markup:{content:Series}});
ferro_impl_classes!(Chart:StyledElementImpl,InteractiveImpl);

impl FerroObjectImpl for Chart {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_series(ChartSeriesCollection::new());
        this.set_x_axes(Axes::new());
        this.set_y_axes(Axes::new());
        this.set_markers(GeoMarkers::new());
        this.set_visual_elements(ChartVisualElements::new());
        this.set_focusable(true);
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::series_property().as_property()
            || property == Self::x_axes_property().as_property()
            || property == Self::y_axes_property().as_property()
            || property == Self::markers_property().as_property()
            || property == Self::visual_elements_property().as_property()
        {
            this.rebind_sources();
        }
        // Setting the centre of a globe ends its rotation (`RemoveTransition`).
        if (property == Self::center_longitude_property().as_property()
            || property == Self::center_latitude_property().as_property())
            && !this.rotating.get()
        {
            this.map_animation
                .borrow_mut()
                .retain(|animation| !matches!(animation, MapAnimation::Rotation { .. }));
        }
        if property == Self::chart_kind_property().as_property()
            || property == Self::zoom_mode_property().as_property()
        {
            this.cancel_pointer_gesture();
        }
        this.invalidate_chart();
    }
}

impl ControlImpl for Chart {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        crate::ChartAutomationPeer::new(this).upcast()
    }
}

impl LayoutableImpl for Chart {
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
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        this.invalidate_chart();
        final_size
    }
}

impl VisualImpl for Chart {
    fn render(this: &Self, context: &mut DrawingContext<'_>) {
        let size = this.bounds().size();
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        let scene = match this.scene(size) {
            Ok(scene) => scene,
            Err(error) => {
                *this.last_error.borrow_mut() = Some(error.to_string());
                let scene =
                    crate::decorations::error_scene(size, &error.to_string(), this.text_color());
                NativeScene::compile(&scene).render(context);
                return;
            }
        };
        let presentation = this.presentation_scene(Dispatcher::ui_thread().now() as f64, scene);
        let animating = this.transition.borrow().is_some();
        let native = if animating {
            Rc::new(NativeScene::compile(&presentation))
        } else {
            let cached = this.native_scene.borrow().clone();
            if let Some(cached) = cached {
                cached
            } else {
                let native = Rc::new(NativeScene::compile(&presentation));
                *this.native_scene.borrow_mut() = Some(native.clone());
                native
            }
        };
        let clip = context.push_clip(Rect::from_size(size));
        native.render(context);
        if let Some(selection) = this.section_selection() {
            let mut overlay = Scene::new(size, presentation.plot_bounds);
            overlay.primitives.push(
                ferroui_livecharts::Primitive::new(
                    ferroui_livecharts::Geometry::Rectangle {
                        rect: selection,
                        radius: 0.0,
                    },
                    ferroui_livecharts::PaintStyle {
                        fill: Some(Color::from_argb(40, 66, 99, 235)),
                        stroke: Some(Color::from_rgb(66, 99, 235)),
                        stroke_width: 1.0,
                        ..ferroui_livecharts::PaintStyle::default()
                    },
                )
                .clipped(presentation.plot_bounds),
            );
            NativeScene::compile(&overlay).render(context);
        }
        let hovered = this.hover.borrow().clone();
        if let Some(mut hovered) = hovered {
            // The tooltip of a land is anchored at its centre (`GeoTooltipPoint.LandCenter`).
            if this.chart_kind() == ChartKind::GeoMap {
                if let Some(land) = hovered.points.first() {
                    hovered.position = land.center;
                }
            }
            if this.tooltip_position() != TooltipPosition::Hidden && !hovered.points.is_empty() {
                let tooltip = crate::decorations::tooltip_scene(
                    size,
                    &hovered,
                    this.tooltip_position(),
                    this.text_color(),
                );
                NativeScene::compile(&tooltip).render(context);
            }
        }
        context.pop(clip);
    }
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.attached.set(true);
        this.rebind_sources();
        this.invalidate_chart();
    }
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        this.attached.set(false);
        this.stop_animation();
        this.stop_map_animations();
        this.map_click.set(None);
        this.cancel_pointer_gesture();
        this.clear_source_subscriptions();
        this.hover.borrow_mut().take();
        this.native_scene.borrow_mut().take();
        Self::parent_on_detached_from_visual_tree(this, e);
    }
}

impl InputElementImpl for Chart {
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        let position = e.get_position(Some(this));
        // A press that moves more than 5 pixels is a drag, not a click on a land.
        if let Some(start) = this.map_click.get() {
            let (dx, dy) = (position.x - start.x, position.y - start.y);
            if dx * dx + dy * dy > MAP_CLICK_DISTANCE_SQUARED {
                this.map_click.set(None);
            }
        }
        if this.section_start.get().is_some() && this.owns_pointer(e.pointer()) {
            this.section_end.set(Some(position));
            this.invalidate_visual();
            e.set_handled(true);
        } else if let Some(previous) = this
            .pointer_down
            .get()
            .filter(|_| this.owns_pointer(e.pointer()))
        {
            if this.pan_by(previous, position).unwrap_or(false) {
                this.pointer_down.set(Some(position));
                e.set_handled(true);
            }
        } else {
            this.move_pointer(position);
        }
    }
    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        this.clear_hover();
    }
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        let position = e.get_position(Some(this));
        let button = e
            .get_current_point(Some(this))
            .properties
            .pointer_update_kind
            .get_mouse_button();
        let action = pointer_action(
            this.zoom_mode(),
            e.pointer().type_(),
            button,
            e.click_count(),
        );
        if this
            .pointer_capture
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some()
        {
            return;
        }
        // A map pans with the primary button and reports a click on a land at its release
        // (`GeoMapChart.InvokePointerDown`, `InvokePointerUp`).
        if this.chart_kind() == ChartKind::GeoMap {
            this.map_click.set(Some(position));
            let primary = e.pointer().type_() != PointerType::Mouse || button == MouseButton::Left;
            if primary && this.pan_enabled() && this.ensure_measurement().is_ok() {
                this.pointer_down.set(Some(position));
                this.capture_pointer(e.pointer(), button);
                e.set_handled(true);
            }
            return;
        }
        if action == PointerAction::Section && this.chart_kind() == ChartKind::Cartesian {
            if !this
                .zoom_mode()
                .contains(ZoomAndPanMode::NO_ZOOM_BY_SECTION)
                && this
                    .zoom_mode()
                    .intersects(ZoomAndPanMode::ZOOM_X | ZoomAndPanMode::ZOOM_Y)
                && this.ensure_measurement().is_ok()
                && this.in_plot(position)
            {
                this.section_start.set(Some(position));
                this.section_end.set(Some(position));
                this.capture_pointer(e.pointer(), button);
                this.clear_hover();
                e.set_handled(true);
            }
            return;
        }
        let was_attached = this.attached.get();
        let args = ChartPointerEventArgs {
            position,
            points: this.find_points(position),
        };
        for (_, handler) in this.data_pointer_down.snapshot().iter() {
            handler(&args);
        }
        // Application handlers may remove the control or change its gesture mode.
        if was_attached && !this.attached.get() {
            return;
        }
        if action == PointerAction::Pan
            && this.pan_enabled()
            && this.ensure_measurement().is_ok()
            && this.in_plot(position)
        {
            this.pointer_down.set(Some(position));
            this.capture_pointer(e.pointer(), button);
            e.set_handled(true);
        }
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if this.chart_kind() == ChartKind::GeoMap {
            let click = this.map_click.take();
            if this.owns_pointer(e.pointer()) {
                this.cancel_pointer_gesture();
                e.set_handled(true);
            }
            this.bounce_back();
            if click.is_some() {
                let position = e.get_position(Some(this));
                let args = ChartPointerEventArgs {
                    position,
                    points: this.find_points(position),
                };
                // Only a click on a land is reported (`OnDataPointerDown` drops empty hits).
                if !args.points.is_empty() {
                    for (_, handler) in this.data_pointer_down.snapshot().iter() {
                        handler(&args);
                    }
                }
            }
            return;
        }
        if this.owns_pointer(e.pointer())
            && (e.pointer().type_() != PointerType::Mouse
                || e.initial_press_mouse_button() == this.pointer_button.get())
        {
            if let Some(start) = this.section_start.get() {
                let _ = this.zoom_to_section(start, e.get_position(Some(this)));
            }
            this.cancel_pointer_gesture();
            e.set_handled(true);
        }
    }
    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        if this.owns_pointer(e.pointer()) {
            this.pointer_capture.borrow_mut().take();
            this.pointer_down.set(None);
            this.section_start.set(None);
            this.section_end.set(None);
            this.pointer_button.set(MouseButton::None);
            this.invalidate_visual();
        }
    }
    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);
        let speed = this.zooming_speed();
        if !speed.is_finite() || speed <= 0.0 || speed >= 1.0 || !e.delta().y.is_finite() {
            return;
        }
        let factor = (1.0 - speed).powf(e.delta().y);
        if this
            .zoom_at(e.get_position(Some(this)), factor)
            .unwrap_or(false)
        {
            e.set_handled(true);
        }
    }
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if e.key == Key::Home {
            this.cancel_pointer_gesture();
            this.reset_zoom();
            e.set_handled(true);
        }
        if e.key == Key::Escape {
            if this.cancel_pointer_gesture() {
                e.set_handled(true);
            }
            this.clear_hover();
        }
    }
}

impl Chart {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            revision: Cell::new(0),
            measurement: RefCell::new(None),
            native_scene: RefCell::new(None),
            source_subscriptions: RefCell::new(Vec::new()),
            fallback_x_axes: OnceCell::new(),
            fallback_y_axes: OnceCell::new(),
            pointer_down: Cell::new(None),
            pointer_capture: RefCell::new(None),
            pointer_button: Cell::new(MouseButton::None),
            section_start: Cell::new(None),
            section_end: Cell::new(None),
            hover: RefCell::new(None),
            hover_changed: HandlerList::new(),
            data_pointer_down: HandlerList::new(),
            update_finished: HandlerList::new(),
            last_error: RefCell::new(None),
            attached: Cell::new(false),
            transition: RefCell::new(None),
            animation_timer: RefCell::new(None),
            map_viewport: Cell::new(MapViewport::default()),
            map_click: Cell::new(None),
            map_animation: RefCell::new(Vec::new()),
            map_timer: RefCell::new(None),
            rotating: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn revision(&self) -> u64 {
        self.revision.get()
    }
    pub fn last_error(&self) -> Option<String> {
        self.last_error.borrow().clone()
    }

    pub fn invalidate_chart(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
        self.native_scene.borrow_mut().take();
        self.invalidate_visual();
    }

    // Upstream views expose empty, independently owned collections. The engine
    // supplies virtual axes only while a collection is empty. Keeping those axes
    // outside the declared collection also lets XAML append the first real axis
    // without accidentally retaining an extra default axis.
    fn effective_x_axes(&self) -> Axes {
        let declared = self.x_axes();
        if declared.is_empty() {
            self.fallback_x_axes
                .get_or_init(|| Axes::from_items([Axis::new()]))
                .clone()
        } else {
            declared
        }
    }

    fn effective_y_axes(&self) -> Axes {
        let declared = self.y_axes();
        if declared.is_empty() {
            self.fallback_y_axes
                .get_or_init(|| Axes::from_items([Axis::new()]))
                .clone()
        } else {
            declared
        }
    }

    fn rebind_sources(&self) {
        self.clear_source_subscriptions();
        let mut subscriptions = Vec::new();
        let weak = self.to_ref().downgrade();
        subscriptions.push(Subscription::new(observe_list(&self.series(), move || {
            if let Some(this) = weak.upgrade() {
                this.rebind_sources();
                this.invalidate_chart();
            }
        })));
        for axes in [self.x_axes(), self.y_axes()] {
            let weak = self.to_ref().downgrade();
            subscriptions.push(Subscription::new(observe_list(&axes, move || {
                if let Some(this) = weak.upgrade() {
                    this.rebind_sources();
                    this.invalidate_chart();
                }
            })));
        }
        for axes in [self.effective_x_axes(), self.effective_y_axes()] {
            for axis in axes.to_vec() {
                let weak = self.to_ref().downgrade();
                subscriptions.push(Subscription::new(axis.property_changed(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.invalidate_chart();
                    }
                })));
            }
        }
        for series in self.series().to_vec() {
            // A Sankey or treemap series also observes its nodes and links; replacing one of its
            // collections, or changing a list or a tree, makes the subscriptions again.
            let holds_nodes = series.is::<crate::SankeySeries>()
                || series.is::<crate::TreemapSeries>()
                || series.is::<crate::HeatLandSeries>();
            let weak = self.to_ref().downgrade();
            subscriptions.push(Subscription::new(series.changed(move || {
                if let Some(this) = weak.upgrade() {
                    if holds_nodes {
                        this.rebind_sources();
                    }
                    this.invalidate_chart();
                }
            })));
            if holds_nodes {
                let changed = self.to_ref().downgrade();
                let restructured = self.to_ref().downgrade();
                let changed: Rc<dyn Fn()> = Rc::new(move || {
                    if let Some(this) = changed.upgrade() {
                        this.invalidate_chart();
                    }
                });
                let restructured: Rc<dyn Fn()> = Rc::new(move || {
                    if let Some(this) = restructured.upgrade() {
                        this.rebind_sources();
                        this.invalidate_chart();
                    }
                });
                subscriptions.extend(crate::node_models::observe_nodes(
                    &series,
                    changed.clone(),
                    restructured.clone(),
                ));
                subscriptions.extend(crate::geo_models::observe_lands(
                    &series,
                    changed,
                    restructured,
                ));
            }
        }
        // The markers of a map, as the visual elements of the source.
        let changed = self.to_ref().downgrade();
        let restructured = self.to_ref().downgrade();
        subscriptions.extend(crate::geo_models::observe_markers(
            &self.markers(),
            Rc::new(move || {
                if let Some(this) = changed.upgrade() {
                    this.invalidate_chart();
                }
            }),
            Rc::new(move || {
                if let Some(this) = restructured.upgrade() {
                    this.rebind_sources();
                    this.invalidate_chart();
                }
            }),
        ));
        // The visual elements of a pie chart (the ticks and needles of an angular gauge).
        let visuals = self.visual_elements();
        let weak = self.to_ref().downgrade();
        subscriptions.push(Subscription::new(observe_list(&visuals, move || {
            if let Some(this) = weak.upgrade() {
                this.rebind_sources();
                this.invalidate_chart();
            }
        })));
        for visual in visuals.to_vec() {
            let weak = self.to_ref().downgrade();
            subscriptions.push(Subscription::new(visual.property_changed(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.invalidate_chart();
                }
            })));
        }
        *self.source_subscriptions.borrow_mut() = subscriptions;
    }

    fn clear_source_subscriptions(&self) {
        drop(self.source_subscriptions.take());
    }

    /// Measures once per data revision and size. The returned scene is an immutable snapshot.
    pub fn scene(&self, size: Size) -> Result<Rc<Scene>, ChartError> {
        let revision = self.revision.get();
        if let Some(measurement) = self.measurement.borrow().as_ref() {
            if measurement.revision == revision && measurement.scene.size == size {
                return Ok(measurement.scene.clone());
            }
        }
        let models = self.series().to_vec();
        let series: Vec<_> = models.iter().map(series_snapshot).collect();
        let xs: Vec<_> = self
            .effective_x_axes()
            .to_vec()
            .iter()
            .map(|a| a.snapshot())
            .collect();
        let ys: Vec<_> = self
            .effective_y_axes()
            .to_vec()
            .iter()
            .map(|a| a.snapshot())
            .collect();
        let margin = self.draw_margin().or_else(|| {
            crate::decorations::draw_margin(
                size,
                self.legend_position(),
                !self.title().is_empty(),
                self.chart_kind(),
            )
        });
        let viewport = margin.unwrap_or(Rect::from_size(size));
        if ![viewport.x, viewport.y, viewport.width, viewport.height]
            .iter()
            .all(|v| v.is_finite())
            || viewport.width < 0.0
            || viewport.height < 0.0
        {
            return Err(ChartError::InvalidData(
                "drawing margin must be a finite nonnegative rectangle".into(),
            ));
        }
        let mut polar_origin = None;
        let mut geo = None;
        let (mut scene, x_scales, y_scales) = match self.chart_kind() {
            ChartKind::Cartesian => {
                let measured = CartesianChartEngine::measure_with_axes_and_labels(
                    size,
                    &series,
                    &xs,
                    &ys,
                    &CartesianOptions {
                        draw_margin: margin,
                        sections: self.sections(),
                    },
                    crate::decorations::text_size,
                )?;
                (measured.scene, measured.x_scales, measured.y_scales)
            }
            ChartKind::Pie => (
                PieChartEngine::measure_with_labels(
                    size,
                    &series,
                    &PieOptions {
                        draw_margin: margin,
                        initial_rotation: self.initial_rotation(),
                        max_angle: self.max_angle(),
                        min_value: self.min_value(),
                        max_value: self.max_value(),
                        clockwise: self.clockwise(),
                        visual_elements: self
                            .visual_elements()
                            .to_vec()
                            .iter()
                            .filter_map(ChartVisualElement::snapshot)
                            .collect(),
                    },
                    crate::decorations::text_size,
                )?,
                Vec::new(),
                Vec::new(),
            ),
            ChartKind::Polar => {
                // The angle axes are the X axes and the radius axes the Y axes. Automatic
                // title and legend space is outside the polar label negotiation, so the
                // engine measures the viewport and the scene moves into it; an explicit
                // drawing margin keeps control coordinates.
                let local = self.draw_margin().is_none();
                let mut measured = PolarChartEngine::measure_with_axes_and_labels(
                    if local { viewport.size() } else { size },
                    &series,
                    &xs,
                    &ys,
                    &PolarOptions {
                        draw_margin: if local { None } else { margin },
                        inner_radius: self.inner_radius(),
                        initial_rotation: self.initial_rotation(),
                        total_angle: self.max_angle(),
                        fit_to_bounds: self.fit_to_bounds(),
                    },
                    crate::decorations::text_size,
                )?;
                if local {
                    let offset = Point::new(viewport.x, viewport.y);
                    measured.scene.translate(offset, size)?;
                    measured.center.x += offset.x;
                    measured.center.y += offset.y;
                }
                polar_origin = Some((measured.center, measured.initial_rotation));
                (
                    measured.scene,
                    measured.angle_scales,
                    measured.radius_scales,
                )
            }
            // The Sankey and treemap charts have no axes; their series hold nodes and links.
            ChartKind::Sankey => {
                let sankey = models
                    .iter()
                    .zip(&series)
                    .map(|(model, s)| crate::node_models::sankey_series(model, s.clone()))
                    .collect::<Result<Vec<_>, _>>()?;
                (
                    SankeyChartEngine::measure_with_labels(
                        size,
                        &sankey,
                        &SankeyOptions {
                            draw_margin: margin,
                        },
                        crate::decorations::text_size,
                    )?,
                    Vec::new(),
                    Vec::new(),
                )
            }
            ChartKind::Treemap => {
                let treemap = models
                    .iter()
                    .zip(&series)
                    .map(|(model, s)| crate::node_models::treemap_series(model, s.clone()))
                    .collect::<Result<Vec<_>, _>>()?;
                (
                    TreemapChartEngine::measure_with_labels(
                        size,
                        &treemap,
                        &TreemapOptions {
                            draw_margin: margin,
                        },
                        crate::decorations::text_size,
                    )?,
                    Vec::new(),
                    Vec::new(),
                )
            }
            // A map has no axes; its series are heat land series over the active map (the
            // bundled world map without one).
            ChartKind::GeoMap => {
                let heat = models
                    .iter()
                    .zip(&series)
                    .map(|(model, s)| crate::geo_models::heat_land_series(model, s.clone()))
                    .collect::<Result<Vec<_>, _>>()?;
                let markers: Vec<_> = self
                    .markers()
                    .to_vec()
                    .iter()
                    .map(|marker| marker.snapshot())
                    .collect();
                let active = self.active_map();
                let map: &DrawnMap = active.as_deref().unwrap_or_else(|| Maps::world_map());
                let mut measured = GeoMapChartEngine::measure(
                    size,
                    map,
                    &heat,
                    &markers,
                    &self.geo_options(margin),
                )?;
                let scene = std::mem::replace(&mut measured.scene, Scene::new(size, viewport));
                geo = Some(measured);
                (scene, Vec::new(), Vec::new())
            }
        };
        crate::decorations::decorate(
            &mut scene,
            &self.title(),
            self.title_size(),
            self.text_color(),
            self.legend_position(),
        );
        {
            for (series_index, model) in models.iter().enumerate() {
                scene.entity_keys.insert(
                    ferroui_livecharts::PointId {
                        series: series_index,
                        point: usize::MAX,
                    },
                    (model.entity_id(), 0),
                );
                for (point_index, point) in model.values().to_vec().iter().enumerate() {
                    scene.entity_keys.insert(
                        ferroui_livecharts::PointId {
                            series: series_index,
                            point: point_index,
                        },
                        (model.entity_id(), point.entity_id()),
                    );
                }
                crate::node_models::insert_entity_keys(
                    series_index,
                    model,
                    &mut scene.entity_keys,
                );
            }
            for primitive in &mut scene.primitives {
                primitive.entity_key = primitive
                    .id
                    .and_then(|id| scene.entity_keys.get(&id).copied());
            }
        }
        let scene = Rc::new(scene);
        let previous = self.measurement.borrow().as_ref().map(|m| m.scene.clone());
        self.start_transition(previous, scene.clone());
        *self.measurement.borrow_mut() = Some(Measurement {
            revision,
            scene: scene.clone(),
            x_scales,
            y_scales,
            polar_origin,
            geo,
        });
        self.last_error.borrow_mut().take();
        self.native_scene.borrow_mut().take();
        for (_, handler) in self.update_finished.snapshot().iter() {
            handler();
        }
        Ok(scene)
    }

    pub fn find_points(&self, position: Point) -> Vec<HitRegion> {
        let size = self.bounds().size();
        let cached_size = self.measurement.borrow().as_ref().map(|m| m.scene.size);
        let size = if size.width > 0.0 && size.height > 0.0 {
            size
        } else {
            cached_size.unwrap_or(size)
        };
        match self.scene(size) {
            Ok(scene) => self
                .presentation_scene(Dispatcher::ui_thread().now() as f64, scene)
                .hit_test(
                    position,
                    self.tooltip_finding_strategy(),
                    self.tooltip_tolerance(),
                )
                .into_iter()
                .cloned()
                .collect(),
            Err(_) => Vec::new(),
        }
    }
    pub fn move_pointer(&self, position: Point) {
        let next = ChartPointerEventArgs {
            position,
            points: self.find_points(position),
        };
        let changed = self.hover.borrow().as_ref() != Some(&next);
        if changed {
            *self.hover.borrow_mut() = Some(next.clone());
            self.invalidate_visual();
            for (_, handler) in self.hover_changed.snapshot().iter() {
                handler(&next);
            }
        }
    }
    pub fn clear_hover(&self) {
        let old = self.hover.borrow_mut().take();
        if let Some(old) = old {
            self.invalidate_visual();
            let event = ChartPointerEventArgs {
                position: old.position,
                points: Vec::new(),
            };
            for (_, handler) in self.hover_changed.snapshot().iter() {
                handler(&event);
            }
        }
    }
    pub fn hover_changed(
        &self,
        handler: impl Fn(&ChartPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.hover_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.hover_changed.remove(token);
            }
        })
    }
    pub fn data_pointer_down(
        &self,
        handler: impl Fn(&ChartPointerEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.data_pointer_down.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.data_pointer_down.remove(token);
            }
        })
    }
    pub fn update_finished(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.update_finished.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.update_finished.remove(token);
            }
        })
    }

    /// Zooms about a control-local point. A disabled gesture returns false.
    pub fn zoom_at(&self, position: Point, factor: f64) -> Result<bool, ChartError> {
        // A map zooms by steps (`InvokePointerWheel`): a factor below 1 is a step in.
        if self.chart_kind() == ChartKind::GeoMap {
            if !self.interaction_mode().contains(MapInteractionMode::ZOOM) {
                return Ok(false);
            }
            if !factor.is_finite()
                || factor <= 0.0
                || !position.x.is_finite()
                || !position.y.is_finite()
            {
                return Err(ChartError::InvalidData(
                    "zoom factor and position must be finite; factor must be positive".into(),
                ));
            }
            let direction = if factor < 1.0 {
                ZoomDirection::ZoomIn
            } else if factor > 1.0 {
                ZoomDirection::ZoomOut
            } else {
                return Ok(false);
            };
            self.zoom_map(position, direction);
            return Ok(true);
        }
        if self.chart_kind() != ChartKind::Cartesian
            || !self
                .zoom_mode()
                .intersects(ZoomAndPanMode::ZOOM_X | ZoomAndPanMode::ZOOM_Y)
        {
            return Ok(false);
        }
        if !factor.is_finite()
            || factor <= 0.0
            || !position.x.is_finite()
            || !position.y.is_finite()
        {
            return Err(ChartError::InvalidData(
                "zoom factor and position must be finite; factor must be positive".into(),
            ));
        }
        self.ensure_measurement()?;
        if !self.in_plot(position) {
            return Ok(false);
        }
        let (xs, ys) = self.axis_scales();
        let mut changed = false;
        if self.zoom_mode().contains(ZoomAndPanMode::ZOOM_X) {
            changed |= zoom_axes(&self.effective_x_axes(), &xs, position.x, factor);
        }
        if self.zoom_mode().contains(ZoomAndPanMode::ZOOM_Y) {
            changed |= zoom_axes(&self.effective_y_axes(), &ys, position.y, factor);
        }
        Ok(changed)
    }
    pub fn pan_by(&self, from: Point, to: Point) -> Result<bool, ChartError> {
        if self.chart_kind() == ChartKind::GeoMap {
            if !self.pan_enabled() {
                return Ok(false);
            }
            if ![from.x, from.y, to.x, to.y].iter().all(|v| v.is_finite()) {
                return Err(ChartError::InvalidData(
                    "pan coordinates must be finite".into(),
                ));
            }
            self.pan_map(Point::new(to.x - from.x, to.y - from.y));
            return Ok(true);
        }
        if self.chart_kind() != ChartKind::Cartesian
            || !self
                .zoom_mode()
                .intersects(ZoomAndPanMode::PAN_X | ZoomAndPanMode::PAN_Y)
        {
            return Ok(false);
        }
        if ![from.x, from.y, to.x, to.y].iter().all(|v| v.is_finite()) {
            return Err(ChartError::InvalidData(
                "pan coordinates must be finite".into(),
            ));
        }
        self.ensure_measurement()?;
        if self.pointer_down.get().is_none() && !self.in_plot(from) {
            return Ok(false);
        }
        let (xs, ys) = self.axis_scales();
        let mut changed = false;
        if self.zoom_mode().contains(ZoomAndPanMode::PAN_X) {
            changed |= pan_axes(&self.effective_x_axes(), &xs, from.x, to.x);
        }
        if self.zoom_mode().contains(ZoomAndPanMode::PAN_Y) {
            changed |= pan_axes(&self.effective_y_axes(), &ys, from.y, to.y);
        }
        Ok(changed)
    }
    /// Zooms to the part of the plot selected by a secondary pointer drag.
    /// Reversed selections and inverted/logarithmic axes use the same inverse scales.
    pub fn zoom_to_section(&self, from: Point, to: Point) -> Result<bool, ChartError> {
        let mode = self.zoom_mode();
        if self.chart_kind() != ChartKind::Cartesian
            || mode.contains(ZoomAndPanMode::NO_ZOOM_BY_SECTION)
            || !mode.intersects(ZoomAndPanMode::ZOOM_X | ZoomAndPanMode::ZOOM_Y)
        {
            return Ok(false);
        }
        if ![from.x, from.y, to.x, to.y].iter().all(|v| v.is_finite()) {
            return Err(ChartError::InvalidData(
                "section coordinates must be finite".into(),
            ));
        }
        self.ensure_measurement()?;
        if !self.in_plot(from) {
            return Ok(false);
        }
        let plot = self
            .measurement
            .borrow()
            .as_ref()
            .map(|m| m.scene.plot_bounds)
            .unwrap();
        let to = clamp_to_plot(to, plot);
        let (xs, ys) = self.axis_scales();
        let mut changed = false;
        if mode.contains(ZoomAndPanMode::ZOOM_X) && (from.x - to.x).abs() >= 4.0 {
            changed |= section_axes(&self.effective_x_axes(), &xs, from.x, to.x);
        }
        if mode.contains(ZoomAndPanMode::ZOOM_Y) && (from.y - to.y).abs() >= 4.0 {
            changed |= section_axes(&self.effective_y_axes(), &ys, from.y, to.y);
        }
        Ok(changed)
    }
    pub fn reset_zoom(&self) {
        if self.chart_kind() == ChartKind::GeoMap {
            self.reset_viewport();
            return;
        }
        for axes in [self.effective_x_axes(), self.effective_y_axes()] {
            for axis in axes.to_vec() {
                axis.set_min_limit(None);
                axis.set_max_limit(None);
            }
        }
    }
    fn ensure_measurement(&self) -> Result<(), ChartError> {
        let bounds_size = self.bounds().size();
        let size = if bounds_size.width > 0.0 && bounds_size.height > 0.0 {
            bounds_size
        } else {
            self.measurement
                .borrow()
                .as_ref()
                .map(|m| m.scene.size)
                .unwrap_or(bounds_size)
        };
        self.scene(size).map(|_| ())
    }
    pub fn axis_scales(&self) -> (Vec<AxisScale>, Vec<AxisScale>) {
        self.measurement
            .borrow()
            .as_ref()
            .map(|m| (m.x_scales.clone(), m.y_scales.clone()))
            .unwrap_or_default()
    }

    /// A resolved polar axis pair (`PolarScaler`): the angle scale maps to degrees, the radius
    /// scale to logical pixels from the centre, which includes the title and legend layout.
    /// `None` for another chart family or a missing axis.
    pub fn polar_scale(
        &self,
        angle_axis: usize,
        radius_axis: usize,
    ) -> Result<Option<PolarScale>, ChartError> {
        self.ensure_measurement()?;
        let measured = self.measurement.borrow();
        let Some(measured) = measured.as_ref() else {
            return Ok(None);
        };
        let Some((center, initial_rotation)) = measured.polar_origin else {
            return Ok(None);
        };
        let (Some(angle), Some(radius)) = (
            measured.x_scales.get(angle_axis),
            measured.y_scales.get(radius_axis),
        ) else {
            return Ok(None);
        };
        Ok(Some(PolarScale {
            center,
            angle: *angle,
            radius: *radius,
            initial_rotation,
        }))
    }

    /// Converts chart values (X/Y, or angle/radius of a polar chart) into control-local pixels.
    /// A missing axis or a chart family without axes returns `None`.
    pub fn scale_data_to_pixels(
        &self,
        data: Point,
        x_axis: usize,
        y_axis: usize,
    ) -> Result<Option<Point>, ChartError> {
        if !data.x.is_finite() || !data.y.is_finite() {
            return Ok(None);
        }
        if self.chart_kind() == ChartKind::Polar {
            return Ok(self
                .polar_scale(x_axis, y_axis)?
                .map(|scale| scale.to_pixels(data.x, data.y))
                .filter(|point| point.x.is_finite() && point.y.is_finite()));
        }
        self.ensure_measurement()?;
        let measured = self.measurement.borrow();
        let Some(measured) = measured.as_ref() else {
            return Ok(None);
        };
        let (Some(x), Some(y)) = (measured.x_scales.get(x_axis), measured.y_scales.get(y_axis))
        else {
            return Ok(None);
        };
        let point = Point::new(x.to_pixels(data.x), y.to_pixels(data.y));
        Ok((point.x.is_finite() && point.y.is_finite()).then_some(point))
    }

    /// Inverse of `scale_data_to_pixels`, using the current layout and axis limits.
    pub fn scale_pixels_to_data(
        &self,
        point: Point,
        x_axis: usize,
        y_axis: usize,
    ) -> Result<Option<Point>, ChartError> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Ok(None);
        }
        if self.chart_kind() == ChartKind::Polar {
            return Ok(self
                .polar_scale(x_axis, y_axis)?
                .map(|scale| scale.to_chart_values(point))
                .filter(|data| data.x.is_finite() && data.y.is_finite()));
        }
        self.ensure_measurement()?;
        let measured = self.measurement.borrow();
        let Some(measured) = measured.as_ref() else {
            return Ok(None);
        };
        let (Some(x), Some(y)) = (measured.x_scales.get(x_axis), measured.y_scales.get(y_axis))
        else {
            return Ok(None);
        };
        let data = Point::new(x.to_chart_values(point.x), y.to_chart_values(point.y));
        Ok((data.x.is_finite() && data.y.is_finite()).then_some(data))
    }

    fn pan_enabled(&self) -> bool {
        match self.chart_kind() {
            ChartKind::Cartesian => self
                .zoom_mode()
                .intersects(ZoomAndPanMode::PAN_X | ZoomAndPanMode::PAN_Y),
            ChartKind::GeoMap => self.interaction_mode().contains(MapInteractionMode::PAN),
            _ => false,
        }
    }

    fn in_plot(&self, point: Point) -> bool {
        self.measurement
            .borrow()
            .as_ref()
            .is_some_and(|m| contains_point(m.scene.plot_bounds, point))
    }
    fn owns_pointer(&self, pointer: &Rc<dyn IPointer>) -> bool {
        self.pointer_capture
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|p| p.id() == pointer.id())
    }
    fn capture_pointer(&self, pointer: &Rc<dyn IPointer>, button: MouseButton) {
        self.pointer_button.set(button);
        *self.pointer_capture.borrow_mut() = Some(Rc::downgrade(pointer));
        pointer.capture(Some(&self.to_ref().upcast()));
    }
    fn cancel_pointer_gesture(&self) -> bool {
        let pointer = self
            .pointer_capture
            .borrow_mut()
            .take()
            .and_then(|p| p.upgrade());
        let changed = self.pointer_down.take().is_some() | self.section_start.take().is_some();
        self.section_end.set(None);
        self.pointer_button.set(MouseButton::None);
        if let Some(pointer) = pointer {
            if pointer.captured().as_ref().is_some_and(|captured| {
                *captured == self.to_ref().upcast::<ferroui_base::input::InputElement>()
            }) {
                pointer.capture(None);
            }
        }
        if changed {
            self.invalidate_visual();
        }
        changed
    }
    fn section_selection(&self) -> Option<Rect> {
        let start = self.section_start.get()?;
        let end = self.section_end.get()?;
        let plot = self.measurement.borrow().as_ref()?.scene.plot_bounds;
        let a = clamp_to_plot(start, plot);
        let b = clamp_to_plot(end, plot);
        let mode = self.zoom_mode();
        Some(Rect::new(
            if mode.contains(ZoomAndPanMode::ZOOM_X) {
                a.x.min(b.x)
            } else {
                plot.x
            },
            if mode.contains(ZoomAndPanMode::ZOOM_Y) {
                a.y.min(b.y)
            } else {
                plot.y
            },
            if mode.contains(ZoomAndPanMode::ZOOM_X) {
                (a.x - b.x).abs()
            } else {
                plot.width
            },
            if mode.contains(ZoomAndPanMode::ZOOM_Y) {
                (a.y - b.y).abs()
            } else {
                plot.height
            },
        ))
    }

    fn start_transition(&self, previous: Option<Rc<Scene>>, next: Rc<Scene>) {
        let duration = self.animations_speed();
        // The lands of a map take their colours at once (GeoMapAnimationTests): a map has no
        // scene transition.
        if !self.attached.get()
            || !duration.is_finite()
            || duration <= 0.0
            || self.chart_kind() == ChartKind::GeoMap
        {
            self.stop_animation();
            return;
        }
        let now = Dispatcher::ui_thread().now() as f64;
        let from = self
            .transition
            .borrow()
            .as_ref()
            .map(|transition| Rc::new(transition.sample(now)))
            .or(previous);
        if let Some(from) = from {
            *self.transition.borrow_mut() = Some(SceneTransition::new(from, next, now, duration));
            let timer = self.animation_timer.borrow().clone();
            if let Some(timer) = timer {
                timer.start();
            } else {
                let weak = self.to_ref().downgrade();
                let timer = DispatcherTimer::with_callback(
                    Duration::from_millis(16),
                    DispatcherPriority::RENDER,
                    move |_| {
                        if let Some(this) = weak.upgrade() {
                            this.invalidate_visual();
                        }
                    },
                );
                timer.start();
                *self.animation_timer.borrow_mut() = Some(timer);
            }
        }
    }
    pub fn presentation_scene(&self, now_ms: f64, target: Rc<Scene>) -> Rc<Scene> {
        let completed = self
            .transition
            .borrow()
            .as_ref()
            .is_some_and(|transition| transition.is_completed(now_ms));
        if completed {
            self.stop_animation();
        }
        let result = self
            .transition
            .borrow()
            .as_ref()
            .map(|transition| Rc::new(transition.sample(now_ms)));
        result.unwrap_or(target)
    }
    fn stop_animation(&self) {
        self.transition.borrow_mut().take();
        let timer = self.animation_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.stop();
        }
    }
}

/// The geographic map (`GeoMapChart`): its viewport, rotation, projection and lands.
impl Chart {
    fn geo_options(&self, margin: Option<Rect>) -> GeoMapOptions {
        GeoMapOptions {
            draw_margin: margin,
            projection: self.map_projection(),
            bounds: GeographicBounds {
                min_latitude: self.min_latitude(),
                max_latitude: self.max_latitude(),
                min_longitude: self.min_longitude(),
                max_longitude: self.max_longitude(),
            },
            center_longitude: self.center_longitude(),
            center_latitude: self.center_latitude(),
            viewport: self.map_viewport.get(),
            stroke: self.stroke(),
            fill: self.fill(),
            stroke_width: 1.0,
        }
    }

    /// The size of the control, or of the last measurement before a layout.
    fn control_size(&self) -> Size {
        let size = self.bounds().size();
        if size.width > 0.0 && size.height > 0.0 {
            size
        } else {
            self.measurement
                .borrow()
                .as_ref()
                .map_or(size, |m| m.scene.size)
        }
    }

    /// The zoom of a map (`ZoomLevel`), 1 by default.
    pub fn zoom_level(&self) -> f64 {
        self.map_viewport.get().zoom
    }
    /// The pan of a map (`PanOffset`), in pixels.
    pub fn pan_offset(&self) -> Point {
        self.map_viewport.get().pan
    }
    /// The pan and zoom of a map.
    pub fn map_viewport(&self) -> MapViewport {
        self.map_viewport.get()
    }
    /// Sets the zoom and pan of a map (`MapFactory.SetViewport`).
    pub fn set_viewport(&self, zoom: f64, pan: Point) {
        if !zoom.is_finite() || zoom <= 0.0 || !pan.x.is_finite() || !pan.y.is_finite() {
            return;
        }
        let viewport = MapViewport::new(zoom, pan);
        if self.map_viewport.get() != viewport {
            self.map_viewport.set(viewport);
            self.invalidate_chart();
        }
    }
    /// Moves a map by a screen distance (`GeoMapChart.Pan`), whatever its interaction mode.
    pub fn pan_map(&self, delta: Point) {
        if delta.x.is_finite() && delta.y.is_finite() {
            let next = self.map_viewport.get().panned(delta);
            self.set_viewport(next.zoom, next.pan);
        }
    }
    /// One zoom step of a map about a control-local position, which stays in place
    /// (`GeoMapChart.Zoom`), with the chart's `ZoomingSpeed`, `MinZoomLevel` and
    /// `MaxZoomLevel`, whatever its interaction mode.
    pub fn zoom_map(&self, pivot: Point, direction: ZoomDirection) {
        if !pivot.x.is_finite() || !pivot.y.is_finite() {
            return;
        }
        let size = self.control_size();
        let center = Point::new(size.width * 0.5, size.height * 0.5);
        let next = self.map_viewport.get().zoomed(
            pivot,
            direction,
            self.zooming_speed(),
            self.min_zoom_level(),
            self.max_zoom_level(),
            center,
        );
        self.set_viewport(next.zoom, next.pan);
    }
    /// The zoom and pan of a map back to none (`ResetViewport`).
    pub fn reset_viewport(&self) {
        self.map_animation
            .borrow_mut()
            .retain(|animation| !matches!(animation, MapAnimation::Bounce { .. }));
        self.set_viewport(1.0, Point::new(0.0, 0.0));
    }
    /// Turns a globe to face a longitude and latitude, the short way round, over the given time
    /// (`RotateTo`, 800 ms in the source, quadratic in and out); a duration of 0, or a chart not
    /// in a visual tree, turns it at once. Setting `CenterLongitude` or `CenterLatitude` ends
    /// the rotation.
    pub fn rotate_to(&self, longitude: f64, latitude: f64, duration_ms: f64) {
        if !longitude.is_finite() || !latitude.is_finite() {
            return;
        }
        let from = (self.center_longitude(), self.center_latitude());
        let to = (shortest_rotation(from.0, longitude), latitude);
        self.map_animation
            .borrow_mut()
            .retain(|animation| !matches!(animation, MapAnimation::Rotation { .. }));
        if !(duration_ms > 0.0) || !self.attached.get() {
            self.set_center(to);
            return;
        }
        self.map_animation.borrow_mut().push(MapAnimation::Rotation {
            from,
            to,
            start: Dispatcher::ui_thread().now() as f64,
            duration: duration_ms,
        });
        self.ensure_map_timer();
    }
    fn set_center(&self, (longitude, latitude): (f64, f64)) {
        self.rotating.set(true);
        self.set_center_longitude(longitude);
        self.set_center_latitude(latitude);
        self.rotating.set(false);
    }
    /// Settles the viewport of a map after a release (`BounceBack`): back to the minimum zoom
    /// and over the whole control, in eight steps of 16 ms.
    fn bounce_back(&self) {
        let bouncing = self
            .map_animation
            .borrow()
            .iter()
            .any(|animation| matches!(animation, MapAnimation::Bounce { .. }));
        if bouncing {
            return;
        }
        let from = self.map_viewport.get();
        let Some(to) = from.bounce_back_target(self.control_size(), self.min_zoom_level()) else {
            return;
        };
        if !self.attached.get() {
            self.set_viewport(to.zoom, to.pan);
            return;
        }
        self.map_animation.borrow_mut().push(MapAnimation::Bounce {
            from,
            to,
            start: Dispatcher::ui_thread().now() as f64,
        });
        self.ensure_map_timer();
    }
    fn ensure_map_timer(&self) {
        let timer = self.map_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.start();
            return;
        }
        let weak = self.to_ref().downgrade();
        let timer = DispatcherTimer::with_callback(
            Duration::from_millis(16),
            DispatcherPriority::RENDER,
            move |_| {
                if let Some(this) = weak.upgrade() {
                    this.advance_map_animations();
                }
            },
        );
        timer.start();
        *self.map_timer.borrow_mut() = Some(timer);
    }
    fn advance_map_animations(&self) {
        let now = Dispatcher::ui_thread().now() as f64;
        let animations = self.map_animation.borrow().clone();
        let mut finished = Vec::new();
        for animation in animations {
            let t = match animation {
                MapAnimation::Rotation {
                    from,
                    to,
                    start,
                    duration,
                } => {
                    let t = ((now - start) / duration).clamp(0.0, 1.0);
                    let eased = EasingFunctions::quadratic_in_out(t);
                    self.set_center((
                        from.0 + (to.0 - from.0) * eased,
                        from.1 + (to.1 - from.1) * eased,
                    ));
                    t
                }
                MapAnimation::Bounce { from, to, start } => {
                    let t = ((now - start) / BOUNCE_DURATION_MS).clamp(0.0, 1.0);
                    let eased = 1.0 - (1.0 - t).powi(3);
                    let next = from.interpolate(&to, eased);
                    self.set_viewport(next.zoom, next.pan);
                    t
                }
            };
            if t >= 1.0 {
                finished.push(animation);
            }
        }
        self.map_animation
            .borrow_mut()
            .retain(|animation| !finished.contains(animation));
        if self.map_animation.borrow().is_empty() {
            self.stop_map_timer();
        }
    }
    fn stop_map_timer(&self) {
        let timer = self.map_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.stop();
        }
    }
    fn stop_map_animations(&self) {
        self.map_animation.borrow_mut().clear();
        self.stop_map_timer();
    }

    /// The control-local position of a longitude and latitude on a map (`GeoMapChart.Project`),
    /// through its projection, rotation and viewport; `None` on the far side of a globe or for
    /// another chart family.
    pub fn project(&self, longitude: f64, latitude: f64) -> Result<Option<Point>, ChartError> {
        self.ensure_measurement()?;
        Ok(self
            .measurement
            .borrow()
            .as_ref()
            .and_then(|m| m.geo.as_ref())
            .and_then(|geo| geo.project(longitude, latitude)))
    }
    /// The longitude and latitude of a control-local position on a map
    /// (`GeoMapChart.Unproject`); `None` outside the disc of a globe or for another chart family.
    pub fn unproject(&self, point: Point) -> Result<Option<(f64, f64)>, ChartError> {
        self.ensure_measurement()?;
        Ok(self
            .measurement
            .borrow()
            .as_ref()
            .and_then(|m| m.geo.as_ref())
            .and_then(|geo| geo.unproject(point)))
    }
    /// The land of a map whose shape contains the position (`FindLandAt`): its names, the values
    /// of the series that have one and the anchor of its tooltip.
    pub fn find_land_at(&self, position: Point) -> Option<GeoLand> {
        self.ensure_measurement().ok()?;
        let measured = self.measurement.borrow();
        let measured = measured.as_ref()?;
        let geo = measured.geo.as_ref()?;
        let hit = measured
            .scene
            .hit_test(position, FindingStrategy::ExactMatch, 0.0)
            .into_iter()
            .next()?;
        geo.land(hit.id).cloned()
    }
    /// The land of a point of a map event (`ChartPoint.Context.DataSource`, a `LandDefinition`
    /// in the source).
    pub fn map_land(&self, id: PointId) -> Option<GeoLand> {
        self.measurement.borrow().as_ref()?.geo.as_ref()?.land(id).cloned()
    }
}

impl Drop for Chart {
    fn drop(&mut self) {
        drop(std::mem::take(self.source_subscriptions.get_mut()));
        if let Some(timer) = self.animation_timer.get_mut().take() {
            timer.stop();
        }
        if let Some(timer) = self.map_timer.get_mut().take() {
            timer.stop();
        }
    }
}

/// The engine snapshot of a series, with the properties of its family that the shared
/// snapshot leaves at their defaults: the `ColorStops`, `PointPadding`, `MinValue` and `MaxValue`
/// of a heat series.
fn series_snapshot(model: &Ref<ChartSeries>) -> ferroui_livecharts::Series {
    let mut series = model.snapshot();
    if let Some(heat) = model.cast::<HeatSeries>() {
        series.color_stops = heat.color_stops();
        series.point_padding = heat.point_padding();
        series.min_value = heat.min_value();
        series.max_value = heat.max_value();
    }
    series
}

fn observe_list<T: Clone + 'static>(
    list: &FerroList<T>,
    action: impl Fn() + 'static,
) -> Rc<dyn IDisposable> {
    let token = list.add_collection_changed(Rc::new(move |_| action()));
    let list = list.clone();
    Disposable::create(move || {
        list.remove_collection_changed(token);
    })
}
fn set_limits(axis: &Axis, min: f64, max: f64) -> bool {
    if !min.is_finite()
        || !max.is_finite()
        || min >= max
        || axis.logarithmic_base().is_some() && min <= 0.0
    {
        return false;
    }
    if axis.min_limit() == Some(min) && axis.max_limit() == Some(max) {
        return false;
    }
    // Keep the intermediate pair ordered for synchronous property observers.
    if axis.max_limit().is_some_and(|old| max > old && min >= old) {
        axis.set_max_limit(Some(max));
        axis.set_min_limit(Some(min));
    } else {
        axis.set_min_limit(Some(min));
        axis.set_max_limit(Some(max));
    }
    true
}
fn zoom_axes(axes: &Axes, scales: &[AxisScale], pivot: f64, factor: f64) -> bool {
    let mut changed = false;
    for (axis, scale) in axes.to_vec().iter().zip(scales) {
        let anchor = scale.to_chart_values(pivot);
        let (min, max) = if let Some(base) = scale.logarithmic_base {
            let a = anchor.log(base);
            (
                base.powf(a + (scale.min.log(base) - a) * factor),
                base.powf(a + (scale.max.log(base) - a) * factor),
            )
        } else {
            (
                anchor + (scale.min - anchor) * factor,
                anchor + (scale.max - anchor) * factor,
            )
        };
        changed |= set_limits(axis, min, max);
    }
    changed
}
fn pan_axes(axes: &Axes, scales: &[AxisScale], from: f64, to: f64) -> bool {
    let mut changed = false;
    for (axis, scale) in axes.to_vec().iter().zip(scales) {
        let (a, b) = (scale.to_chart_values(from), scale.to_chart_values(to));
        let (min, max) = if scale.logarithmic_base.is_some() {
            let factor = a / b;
            (scale.min * factor, scale.max * factor)
        } else {
            let delta = a - b;
            (scale.min + delta, scale.max + delta)
        };
        changed |= set_limits(axis, min, max);
    }
    changed
}
fn section_axes(axes: &Axes, scales: &[AxisScale], from: f64, to: f64) -> bool {
    let mut changed = false;
    for (axis, scale) in axes.to_vec().iter().zip(scales) {
        let (a, b) = (scale.to_chart_values(from), scale.to_chart_values(to));
        changed |= set_limits(axis, a.min(b), a.max(b));
    }
    changed
}
fn contains_point(rect: Rect, point: Point) -> bool {
    rect.width > 0.0
        && rect.height > 0.0
        && point.x.is_finite()
        && point.y.is_finite()
        && point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}
fn clamp_to_plot(point: Point, plot: Rect) -> Point {
    Point::new(
        point.x.clamp(plot.x, plot.x + plot.width),
        point.y.clamp(plot.y, plot.y + plot.height),
    )
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PointerAction {
    None,
    Pan,
    Section,
}
fn pointer_action(
    mode: ZoomAndPanMode,
    kind: PointerType,
    button: MouseButton,
    click_count: i32,
) -> PointerAction {
    let secondary = match kind {
        PointerType::Touch => click_count >= 2,
        PointerType::Mouse | PointerType::Pen => match button {
            MouseButton::Left => false,
            MouseButton::Right => true,
            _ => return PointerAction::None,
        },
    };
    if secondary ^ mode.contains(ZoomAndPanMode::INVERT_PANNING_POINTER_TRIGGER) {
        PointerAction::Section
    } else {
        PointerAction::Pan
    }
}

#[cfg(test)]
#[path = "chart_input_tests.rs"]
mod input_tests;
