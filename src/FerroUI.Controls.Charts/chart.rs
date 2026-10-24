use crate::chart_collection::CollectionOwner;
use crate::rendering::{ChartLayout, ChartScene};
use crate::{
    ChartAutomationPeer, ChartAxisCollection, ChartDataCollection, ChartSeriesCollection, Samples,
};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{subscribe_invalidated, DrawingContext, IBrush};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, Size,
    StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::primitives::{TemplatedControl, TemplatedControlImpl};
use ferroui_controls::ControlImpl;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Displays cartesian line, area and bar series with shared, observable data.
/// Drawing goes through the framework's retained drawing context on every backend.
#[repr(C)]
pub struct Chart {
    base: TemplatedControl,
    owner: Rc<CollectionOwner>,
    series: ChartSeriesCollection,
    axes: ChartAxisCollection,
    data: ChartDataCollection,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    refreshing_subscriptions: Cell<bool>,
    subscriptions_dirty: Cell<bool>,
    resource_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    scene: RefCell<Option<Rc<ChartScene>>>,
    revision: Cell<u64>,
    scene_build_count: Cell<u64>,
}

ferro_class!(Chart: TemplatedControl);
ferro_class_info!(Chart {
    new: Chart::new,
    markup: {
        content: Series,
        properties: [
            Series: ChartSeriesCollection { get: Chart::series },
            Axes: ChartAxisCollection { get: Chart::axes },
            Data: ChartDataCollection { get: Chart::data },
        ],
    },
});
ferro_impl_classes!(Chart: StyledElementImpl, InteractiveImpl, InputElementImpl, TemplatedControlImpl);

ferro_properties! { impl Chart {
    pub fn show_legend_property() -> StyledProperty<bool> {
        FerroProperty::register::<Chart, _>("ShowLegend", false)
    }
    pub fn legend_title_property() -> StyledProperty<String> {
        FerroProperty::register::<Chart, _>("LegendTitle", String::new())
    }
} }

impl FerroObjectImpl for Chart {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        *this.owner.changed.borrow_mut() = Some(Rc::new(move || {
            if let Some(chart) = weak.upgrade() {
                chart.refresh_subscriptions();
                chart.invalidate_scene();
            }
        }));
        let weak = this.to_ref().downgrade();
        *this.owner.axis_in_use.borrow_mut() = Some(Rc::new(move |axis| {
            weak.upgrade().is_some_and(|chart| {
                chart.series.snapshot().iter().any(|series| {
                    [series.x_axis(), series.y_axis()]
                        .into_iter()
                        .flatten()
                        .any(|used| used == axis.to_ref())
                })
            })
        }));
        let weak = this.to_ref().downgrade();
        let resources = this.resources_changed(move |_| {
            if let Some(chart) = weak.upgrade() {
                chart.refresh_subscriptions();
                chart.invalidate_scene();
            }
        });
        let weak = this.to_ref().downgrade();
        let theme = this.actual_theme_variant_changed(move || {
            if let Some(chart) = weak.upgrade() {
                chart.refresh_subscriptions();
                chart.invalidate_scene();
            }
        });
        this.resource_subscriptions
            .borrow_mut()
            .extend([resources, theme]);
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs) {
        Self::parent_on_property_changed(this, change);
        if change.property() == TemplatedControl::foreground_property().as_property()
            || change.property() == TemplatedControl::background_property().as_property()
        {
            this.refresh_subscriptions();
        }
        // Only the properties the scene is built from rebuild it: pointer-over, focus,
        // the data context and the like do not, and a new size is caught by `scene_for`.
        if affects_scene(change.property()) {
            this.invalidate_scene();
        }
    }
}

/// Whether a change of the chart's own property `property` changes what the scene draws.
fn affects_scene(property: &FerroProperty) -> bool {
    let affecting: [&FerroProperty; 13] = [
        Chart::show_legend_property().as_property(),
        Chart::legend_title_property().as_property(),
        TemplatedControl::padding_property().as_property(),
        TemplatedControl::foreground_property().as_property(),
        TemplatedControl::background_property().as_property(),
        TemplatedControl::font_family_property().as_property(),
        TemplatedControl::font_size_property().as_property(),
        TemplatedControl::font_style_property().as_property(),
        TemplatedControl::font_weight_property().as_property(),
        TemplatedControl::font_stretch_property().as_property(),
        TemplatedControl::font_features_property().as_property(),
        TemplatedControl::letter_spacing_property().as_property(),
        Visual::flow_direction_property().as_property(),
    ];
    affecting.contains(&property)
}

impl LayoutableImpl for Chart {
    fn measure_override(this: &Self, available: Size) -> Size {
        // Deviation (DEVIATIONS.md, WinUI Charts): the size of the published measure contract
        // (the available size, or 160 x 96 when it is infinite) rather than the unpublished
        // measure of WinUI, which is not available (winui-charts.md section 2.3).
        // Native templates can add adornments; retain their child lifecycle
        // while the chart keeps its published finite/infinite size contract.
        Self::parent_measure_override(this, available);
        Size::new(
            if available.width.is_finite() {
                available.width.max(0.0)
            } else {
                160.0
            },
            if available.height.is_finite() {
                available.height.max(0.0)
            } else {
                96.0
            },
        )
    }
    fn arrange_override(this: &Self, size: Size) -> Size {
        Self::parent_arrange_override(this, size)
    }
}

impl VisualImpl for Chart {
    fn render(this: &Self, context: &mut DrawingContext) {
        let bounds = this.bounds();
        this.scene_for(Size::new(bounds.width, bounds.height))
            .draw(context);
    }
}

impl ControlImpl for Chart {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ChartAutomationPeer::new(this).upcast()
    }
}

impl Chart {
    pub fn construct() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let owner = CollectionOwner::new(NEXT_ID.fetch_add(1, Ordering::Relaxed));
        Self {
            base: TemplatedControl::construct(),
            series: ChartSeriesCollection::new(owner.clone()),
            axes: ChartAxisCollection::new(owner.clone()),
            data: ChartDataCollection::new(owner.clone()),
            owner,
            subscriptions: RefCell::new(Vec::new()),
            refreshing_subscriptions: Cell::new(false),
            subscriptions_dirty: Cell::new(false),
            resource_subscriptions: RefCell::new(Vec::new()),
            scene: RefCell::new(None),
            revision: Cell::new(0),
            scene_build_count: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn show_legend(&self) -> bool {
        self.get_value(Self::show_legend_property())
    }
    pub fn set_show_legend(&self, value: bool) {
        self.set_value(Self::show_legend_property(), value);
    }
    pub fn legend_title(&self) -> String {
        self.get_value(Self::legend_title_property())
    }
    pub fn set_legend_title(&self, value: impl Into<String>) {
        self.set_value(Self::legend_title_property(), value.into());
    }
    pub fn series(&self) -> ChartSeriesCollection {
        self.series.clone()
    }
    pub fn axes(&self) -> ChartAxisCollection {
        self.axes.clone()
    }
    pub fn data(&self) -> ChartDataCollection {
        self.data.clone()
    }
    /// Increases on a data, collection, style or layout-related property change.
    pub fn revision(&self) -> u64 {
        self.revision.get()
    }
    /// Number of retained scene rebuilds. Unchanged paints reuse the same scene.
    pub fn scene_build_count(&self) -> u64 {
        self.scene_build_count.get()
    }
    /// Returns the calculated plot and point positions without drawing. A normal
    /// framework text service is required, as for measuring any text control.
    pub fn layout_snapshot(&self, size: Size) -> ChartLayout {
        self.scene_for(size).layout.clone()
    }

    pub(crate) fn invalidate_scene(&self) {
        self.revision.set(self.revision.get().wrapping_add(1));
        self.scene.borrow_mut().take();
        self.invalidate_visual();
    }

    fn scene_for(&self, size: Size) -> Rc<ChartScene> {
        let scene = self.scene.borrow().clone();
        if let Some(scene) = scene {
            if scene.layout.size == size {
                return scene;
            }
        }
        let revision = self.revision.get();
        let scene = Rc::new(ChartScene::build(self, size));
        self.scene_build_count
            .set(self.scene_build_count.get().wrapping_add(1));
        // Sources and resource providers may invoke application code while a
        // snapshot is built. Never cache a scene invalidated by that code.
        if self.revision.get() == revision {
            *self.scene.borrow_mut() = Some(scene.clone());
        }
        scene
    }

    fn refresh_subscriptions(&self) {
        self.subscriptions_dirty.set(true);
        if self.refreshing_subscriptions.replace(true) {
            return;
        }
        struct RefreshGuard<'a>(&'a Cell<bool>);
        impl Drop for RefreshGuard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = RefreshGuard(&self.refreshing_subscriptions);
        // Deferred resource providers can mutate this chart synchronously.
        // Queue another complete pass instead of installing an inner set of
        // handlers that the outer pass would overwrite without disposing.
        while self.subscriptions_dirty.replace(false) {
            self.rebuild_subscriptions();
        }
    }

    fn rebuild_subscriptions(&self) {
        // Dispose outside the borrow: subscriptions are allowed to call user code.
        let previous = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in previous {
            subscription.dispose();
        }
        let mut subscriptions = Vec::new();
        let mut samples = Vec::new();
        for sample in self.data.snapshot().iter() {
            if !samples.contains(sample) {
                samples.push(sample.clone());
            }
        }
        let mut brushes = vec![self.foreground(), self.background()];
        for key in [
            "ChartsControlForegroundBrush",
            "ChartsControlBackgroundBrush",
            "ChartsGridLineMajorBrush",
            "ChartsGridLineMinorBrush",
            "ChartsTickBrush",
            "ChartsTickLabelBrush",
            "ChartsAxisLineBrush",
        ] {
            brushes.push(crate::rendering::resource(self, key));
        }
        for axis in self.axes.snapshot().iter() {
            let weak = self.to_ref().downgrade();
            subscriptions.push(axis.property_changed(move |_| {
                if let Some(chart) = weak.upgrade() {
                    chart.refresh_subscriptions();
                    chart.invalidate_scene();
                }
            }));
            if let Some(axis) = axis.cast::<crate::CartesianAxis>() {
                brushes.extend([
                    axis.axis_line_brush(),
                    axis.grid_line_major_brush(),
                    axis.grid_line_minor_brush(),
                    axis.tick_brush(),
                    axis.tick_label_brush(),
                ]);
            }
        }
        for series in self.series.snapshot().iter() {
            let weak = self.to_ref().downgrade();
            subscriptions.push(series.property_changed(move |_| {
                if let Some(chart) = weak.upgrade() {
                    chart.refresh_subscriptions();
                    chart.invalidate_scene();
                }
            }));
            for sample in [series.x_values(), series.y_values()].into_iter().flatten() {
                if !samples.contains(&sample) {
                    samples.push(sample);
                }
            }
            brushes.extend([
                series.stroke(),
                series.data_label_brush(),
                series.data_marker_brush(),
            ]);
            if let Some(series) = series.cast::<crate::AreaSeries>() {
                brushes.push(series.fill());
            }
            if let Some(series) = series.cast::<crate::BarSeries>() {
                brushes.push(series.fill());
            }
            let labels = series.data_label_overrides();
            brushes.extend(labels.values().into_iter().map(|label| label.brush()));
            let weak = self.to_ref().downgrade();
            let token = labels.add_collection_changed(Rc::new(move |_| {
                if let Some(chart) = weak.upgrade() {
                    chart.refresh_subscriptions();
                    chart.invalidate_scene();
                }
            }));
            subscriptions.push(Disposable::create(move || {
                labels.remove_collection_changed(token);
            }));
            let markers = series.data_marker_overrides();
            brushes.extend(markers.values().into_iter().map(|marker| marker.brush()));
            let weak = self.to_ref().downgrade();
            let token = markers.add_collection_changed(Rc::new(move |_| {
                if let Some(chart) = weak.upgrade() {
                    chart.refresh_subscriptions();
                    chart.invalidate_scene();
                }
            }));
            subscriptions.push(Disposable::create(move || {
                markers.remove_collection_changed(token);
            }));
        }
        for sample in samples {
            subscriptions.push(self.observe_samples(sample));
        }
        let mut watched: Vec<Rc<dyn IBrush>> = Vec::new();
        for brush in brushes.into_iter().flatten() {
            if watched.iter().any(|old| old == &brush) {
                continue;
            }
            watched.push(brush.clone());
            let weak = self.to_ref().downgrade();
            if let Some(subscription) = subscribe_invalidated(
                &Some(brush),
                Rc::new(move || {
                    if let Some(chart) = weak.upgrade() {
                        chart.invalidate_scene();
                    }
                }),
            ) {
                subscriptions.push(subscription);
            }
        }
        *self.subscriptions.borrow_mut() = subscriptions;
    }

    fn observe_samples(&self, samples: Ref<Samples>) -> Rc<dyn IDisposable> {
        let weak = self.to_ref().downgrade();
        let token = samples.add_changed(Rc::new(move || {
            if let Some(chart) = weak.upgrade() {
                chart.invalidate_scene();
            }
        }));
        Disposable::create(move || {
            samples.remove_changed(token);
        })
    }
}

impl Drop for Chart {
    fn drop(&mut self) {
        self.owner.changed.borrow_mut().take();
        self.owner.axis_in_use.borrow_mut().take();
        self.series.release_ownership();
        self.axes.release_ownership();
        self.owner.id.set(None);
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        for subscription in self.resource_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}
