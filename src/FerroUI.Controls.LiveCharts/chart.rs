//! Native chart lifecycle, data observation, input and retained-frame caching.

use crate::renderer::NativeScene;
use crate::scene_transition::SceneTransition;
use crate::{
    Axes, Axis, ChartKind, ChartSeriesCollection, GeoMarkers, HeatLands, MapInteractionMode,
    SankeyLinks, SankeyNodes, TooltipPosition, TreemapNodes, ZoomAndPanMode,
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
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Point, Rect, Ref, Size, StyledElementImpl,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_livecharts::geo::{DrawnMap, GeoCoordinate, MapProjector, Projector};
use ferroui_livecharts::geo_map_chart::{GeoData, GeoMapChartEngine, MapViewportTransform};
use ferroui_livecharts::polar_chart_engine::PolarScale;
use ferroui_livecharts::sankey_chart_engine::SankeyChartEngine;
use ferroui_livecharts::treemap_chart_engine::TreemapChartEngine;
use ferroui_livecharts::{
    AxisScale, CartesianChartEngine, CartesianOptions, ChartError, HitRegion, PieChartEngine,
    PieOptions, PolarChartEngine, PolarOptions, Scene,
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
    polar_origin: Option<(Point, f64)>,
    map_transform: Option<MapTransform>,
}

struct MapTransform {
    projector: Projector,
    viewport: MapViewportTransform,
    origin: Point,
}

/// Shared host for all chart families. All application state stays on the UI thread.
#[repr(C)]
pub struct Chart {
    base: Control,
    revision: Cell<u64>,
    measurement: RefCell<Option<Measurement>>,
    native_scene: RefCell<Option<Rc<NativeScene>>>,
    source_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
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
    parsed_map: RefCell<Option<(String, GeoData)>>,
    attached: Cell<bool>,
    transition: RefCell<Option<SceneTransition>>,
    animation_timer: RefCell<Option<Rc<DispatcherTimer>>>,
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
        // Native collections are per-instance mutable objects; property defaults
        // must never let two charts share the same observable collection.
        this.set_sankey_nodes(SankeyNodes::new());
        this.set_sankey_links(SankeyLinks::new());
        this.set_treemap_roots(TreemapNodes::new());
        this.set_heat_lands(HeatLands::new());
        this.set_geo_markers(GeoMarkers::new());
        this.set_focusable(true);
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::series_property().as_property()
            || property == Self::x_axes_property().as_property()
            || property == Self::y_axes_property().as_property()
            || property == Self::sankey_nodes_property().as_property()
            || property == Self::sankey_links_property().as_property()
            || property == Self::treemap_roots_property().as_property()
            || property == Self::heat_lands_property().as_property()
            || property == Self::geo_markers_property().as_property()
        {
            this.rebind_sources();
        }
        if property == Self::chart_kind_property().as_property()
            || property == Self::zoom_mode_property().as_property()
            || property == Self::map_interaction_mode_property().as_property()
        {
            this.cancel_pointer_gesture();
        }
        if property == Self::geo_json_property().as_property() {
            this.parsed_map.borrow_mut().take();
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
        if let Some(hovered) = hovered {
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
        let action = if this.chart_kind() == ChartKind::GeoMap {
            if e.pointer().type_() != PointerType::Mouse || button == MouseButton::Left {
                PointerAction::Pan
            } else {
                PointerAction::None
            }
        } else {
            pointer_action(
                this.zoom_mode(),
                e.pointer().type_(),
                button,
                e.click_count(),
            )
        };
        if this
            .pointer_capture
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some()
        {
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
        if this.owns_pointer(e.pointer())
            && (e.pointer().type_() != PointerType::Mouse
                || e.initial_press_mouse_button() == this.pointer_button.get())
        {
            if let Some(start) = this.section_start.get() {
                let _ = this.zoom_to_section(start, e.get_position(Some(this)));
            }
            if this.pointer_down.get().is_some() && this.chart_kind() == ChartKind::GeoMap {
                let _ = this.constrain_map_viewport();
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
            parsed_map: RefCell::new(None),
            attached: Cell::new(false),
            transition: RefCell::new(None),
            animation_timer: RefCell::new(None),
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
        // Advanced collections own deep observers, including changed link endpoints
        // and replaced subtree collections. We only retain their disposal handles.
        let weak = self.to_ref().downgrade();
        subscriptions.push(self.sankey_nodes().changed(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_chart();
            }
        }));
        let weak = self.to_ref().downgrade();
        subscriptions.push(self.sankey_links().changed(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_chart();
            }
        }));
        let weak = self.to_ref().downgrade();
        subscriptions.push(self.treemap_roots().changed(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_chart();
            }
        }));
        let weak = self.to_ref().downgrade();
        subscriptions.push(self.heat_lands().changed(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_chart();
            }
        }));
        let weak = self.to_ref().downgrade();
        subscriptions.push(self.geo_markers().changed(move || {
            if let Some(this) = weak.upgrade() {
                this.invalidate_chart();
            }
        }));
        let weak = self.to_ref().downgrade();
        subscriptions.push(observe_list(&self.series(), move || {
            if let Some(this) = weak.upgrade() {
                this.rebind_sources();
                this.invalidate_chart();
            }
        }));
        for axes in [self.x_axes(), self.y_axes()] {
            let weak = self.to_ref().downgrade();
            subscriptions.push(observe_list(&axes, move || {
                if let Some(this) = weak.upgrade() {
                    this.rebind_sources();
                    this.invalidate_chart();
                }
            }));
        }
        for axes in [self.effective_x_axes(), self.effective_y_axes()] {
            for axis in axes.to_vec() {
                let weak = self.to_ref().downgrade();
                subscriptions.push(axis.property_changed(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.invalidate_chart();
                    }
                }));
            }
        }
        for series in self.series().to_vec() {
            let weak = self.to_ref().downgrade();
            subscriptions.push(series.changed(move || {
                if let Some(this) = weak.upgrade() {
                    this.invalidate_chart();
                }
            }));
        }
        *self.source_subscriptions.borrow_mut() = subscriptions;
    }

    fn clear_source_subscriptions(&self) {
        let old = std::mem::take(&mut *self.source_subscriptions.borrow_mut());
        for subscription in old {
            subscription.dispose();
        }
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
        let series: Vec<_> = models.iter().map(|s| s.snapshot()).collect();
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
        let mut map_transform = None;
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
                    },
                    crate::decorations::text_size,
                )?,
                Vec::new(),
                Vec::new(),
            ),
            ChartKind::Polar => {
                // Automatic title/legend space is external to polar axis-label
                // negotiation. Explicit drawing margins retain control coordinates.
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
            ChartKind::Sankey => {
                let mut scene = SankeyChartEngine::measure_with_labels(
                    &self.sankey_nodes().snapshots(),
                    &self.sankey_links().snapshots(),
                    viewport.size(),
                    &self.sankey_options(),
                    crate::decorations::text_size,
                )?;
                scene.translate(Point::new(viewport.x, viewport.y), size)?;
                (scene, Vec::new(), Vec::new())
            }
            ChartKind::Treemap => {
                let mut scene = TreemapChartEngine::measure_with_labels(
                    &self.treemap_roots().snapshots()?,
                    viewport.size(),
                    &self.treemap_options(),
                    crate::decorations::text_size,
                )?;
                scene.translate(Point::new(viewport.x, viewport.y), size)?;
                (scene, Vec::new(), Vec::new())
            }
            ChartKind::GeoMap => {
                let mut data = if let Some(data) = self.geo_data() {
                    data
                } else {
                    let source = self.geo_json();
                    let cached = self
                        .parsed_map
                        .borrow()
                        .as_ref()
                        .filter(|(key, _)| *key == source)
                        .map(|(_, data)| data.clone());
                    if let Some(data) = cached {
                        data
                    } else {
                        let map = if source.trim().is_empty() {
                            DrawnMap::world()?
                        } else {
                            DrawnMap::from_geojson(&source)?
                        };
                        let data = GeoData {
                            map,
                            heat: Vec::new(),
                            markers: Vec::new(),
                        };
                        *self.parsed_map.borrow_mut() = Some((source, data.clone()));
                        data
                    }
                };
                data.heat.extend(self.heat_lands().snapshots());
                data.markers.extend(self.geo_markers().snapshots());
                let mut measured =
                    GeoMapChartEngine::layout(&data, viewport.size(), &self.geo_options())?;
                let origin = Point::new(viewport.x, viewport.y);
                measured.scene.translate(origin, size)?;
                map_transform = Some(MapTransform {
                    projector: measured.projector,
                    viewport: measured.viewport,
                    origin,
                });
                (measured.scene, Vec::new(), Vec::new())
            }
        };
        crate::decorations::decorate(
            &mut scene,
            &self.title(),
            self.title_size(),
            self.text_color(),
            self.legend_position(),
        );
        if matches!(
            self.chart_kind(),
            ChartKind::Cartesian | ChartKind::Pie | ChartKind::Polar
        ) {
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
            map_transform,
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
        if self.chart_kind() == ChartKind::GeoMap {
            if !self
                .map_interaction_mode()
                .contains(MapInteractionMode::ZOOM)
            {
                return Ok(false);
            }
            if !factor.is_finite()
                || factor <= 0.0
                || ![position.x, position.y].iter().all(|v| v.is_finite())
            {
                return Err(ChartError::InvalidData(
                    "zoom factor and position must be finite; factor must be positive".into(),
                ));
            }
            self.ensure_measurement()?;
            if !self.in_plot(position) {
                return Ok(false);
            }
            let plot = self
                .measurement
                .borrow()
                .as_ref()
                .unwrap()
                .scene
                .plot_bounds;
            let mut options = self.geo_options();
            let previous = options.clone();
            options.zoom_at(
                Point::new(position.x - plot.x, position.y - plot.y),
                1.0 / factor,
                plot.size(),
            )?;
            if options == previous {
                return Ok(false);
            }
            self.set_geo_options(options);
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
            self.ensure_measurement()?;
            if self.pointer_down.get().is_none() && !self.in_plot(from) {
                return Ok(false);
            }
            let mut options = self.geo_options();
            let previous = options.clone();
            options.pan_by(Point::new(to.x - from.x, to.y - from.y))?;
            if options == previous {
                return Ok(false);
            }
            self.set_geo_options(options);
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
            let mut options = self.geo_options();
            options.reset_viewport();
            self.set_geo_options(options);
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

    /// A resolved polar axis pair. The angular scale is in degrees; the radial
    /// scale is in logical pixels. Its center includes native title/legend layout.
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

    /// Converts chart values (X/Y, or angle/radius) into control-local pixels.
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

    /// Projects a geographic coordinate through the active projection and viewport.
    /// Points on the hidden side of an orthographic globe return `None`.
    pub fn project_coordinate(
        &self,
        coordinate: GeoCoordinate,
    ) -> Result<Option<Point>, ChartError> {
        if !coordinate.is_finite() {
            return Ok(None);
        }
        self.ensure_measurement()?;
        let measured = self.measurement.borrow();
        let Some(transform) = measured.as_ref().and_then(|m| m.map_transform.as_ref()) else {
            return Ok(None);
        };
        if !transform
            .projector
            .is_visible(coordinate.longitude, coordinate.latitude)
        {
            return Ok(None);
        }
        let mut point = transform.viewport.transform(
            transform
                .projector
                .to_map(coordinate.longitude, coordinate.latitude),
        );
        point.x += transform.origin.x;
        point.y += transform.origin.y;
        Ok((point.x.is_finite() && point.y.is_finite()).then_some(point))
    }

    pub fn unproject_coordinate(&self, point: Point) -> Result<Option<GeoCoordinate>, ChartError> {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Ok(None);
        }
        self.ensure_measurement()?;
        let measured = self.measurement.borrow();
        let Some(transform) = measured.as_ref().and_then(|m| m.map_transform.as_ref()) else {
            return Ok(None);
        };
        let point = Point::new(point.x - transform.origin.x, point.y - transform.origin.y);
        Ok(transform
            .viewport
            .inverse(point)
            .and_then(|p| transform.projector.to_coordinates(p)))
    }

    /// Settles a geographic viewport inside its configured zoom and pan limits.
    pub fn constrain_map_viewport(&self) -> Result<bool, ChartError> {
        if self.chart_kind() != ChartKind::GeoMap {
            return Ok(false);
        }
        self.ensure_measurement()?;
        let size = self
            .measurement
            .borrow()
            .as_ref()
            .unwrap()
            .scene
            .plot_bounds
            .size();
        let mut options = self.geo_options();
        let previous = options.clone();
        options.constrain_viewport(size)?;
        if options == previous {
            return Ok(false);
        }
        self.set_geo_options(options);
        Ok(true)
    }

    fn pan_enabled(&self) -> bool {
        match self.chart_kind() {
            ChartKind::Cartesian => self
                .zoom_mode()
                .intersects(ZoomAndPanMode::PAN_X | ZoomAndPanMode::PAN_Y),
            ChartKind::GeoMap => self
                .map_interaction_mode()
                .contains(MapInteractionMode::PAN),
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
        if !self.attached.get() || !duration.is_finite() || duration <= 0.0 {
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

impl Drop for Chart {
    fn drop(&mut self) {
        for subscription in self.source_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        if let Some(timer) = self.animation_timer.get_mut().take() {
            timer.stop();
        }
    }
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
