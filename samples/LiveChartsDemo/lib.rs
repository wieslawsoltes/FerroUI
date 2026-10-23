//! A native LiveCharts gallery, compiled from XAML by the Rust build pipeline.
//!
//! The view contains all six chart families. Its button changes observable
//! points, links, hierarchy leaves and map overlays in place. The optional
//! desktop and CPU-render entry points load this same compiled view.

extern crate self as livecharts_demo;

use ferroui_base::media::{Color, FontManagerOptions};
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::{Ref, Size};
use ferroui_controls::{AppBuilder, Application, Button, Control, Grid, TextBlock, Window};
use ferroui_controls_livecharts::{Chart, ChartKind};
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_livecharts::geo::MapProjection;
use ferroui_livecharts::geo_map_chart::{GeoData, GeoMapOptions};
use ferroui_livecharts::sankey_chart_engine::SankeyOptions;
use ferroui_livecharts::treemap_chart_engine::TreemapOptions;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::XamlLoadException;
use std::cell::Cell;
use std::rc::Rc;

ferroui_markup_xaml::include_compiled_xaml!();

pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "LiveChartsDemo",
    crate_name: "livecharts_demo",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

pub const VIEW_SIZE: Size = Size::new(1440.0, 1040.0);
pub const CHART_NAMES: [&str; 6] = [
    "CartesianDemo",
    "PieDemo",
    "PolarDemo",
    "SankeyDemo",
    "TreemapDemo",
    "GeoDemo",
];

/// Registers the framework services that both compiled and runtime markup name.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_controls_livecharts::register_types();
        ferroui_themes_simple::register_types();
        MarkupAssembly::register(&ASSEMBLY);
        compiled_markup::register();
    });
}

/// The common application setup, before the entry point selects its platform.
pub fn app_builder() -> AppBuilder {
    AppBuilder::configure::<Application>()
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".into()),
            ..FontManagerOptions::default()
        }))
        .with_inter_font()
        .after_setup(|builder| {
            register_types();
            builder
                .instance()
                .expect("the application has been initialized")
                .styles()
                .add(ferroui_themes_simple::SimpleTheme::new().as_style());
        })
}

/// The compiled visual tree and its deterministic live-data controller.
pub struct DemoView {
    pub root: Ref<Grid>,
    step: Rc<Cell<u32>>,
}

impl DemoView {
    pub fn new() -> Result<Self, XamlLoadException> {
        register_types();
        let root = compiled_xaml::build_mainview_xaml(Some(
            XamlIlRuntimeHelpers::create_root_service_provider_v3(None),
        ))?;
        Ok(Self::from_root(root))
    }

    /// Populates an already-loaded view; tests use this for the runtime loader.
    pub fn from_root(root: Ref<Grid>) -> Self {
        populate(&root);
        let step = Rc::new(Cell::new(0_u32));
        let weak = root.downgrade();
        let action_step = step.clone();
        root.get_control::<Button>("UpdateButton")
            .click(move |_, _| {
                if let Some(root) = weak.upgrade() {
                    let next = action_step.get().wrapping_add(1);
                    action_step.set(next);
                    update_values(&root, next);
                }
            });
        Self { root, step }
    }

    pub fn chart(&self, index: usize) -> Ref<Chart> {
        self.root.get_control(CHART_NAMES[index])
    }

    pub fn step(&self) -> u32 {
        self.step.get()
    }

    /// Hosts the view in the same native window for desktop and CPU captures.
    /// Attaching it applies the application theme, including the button template.
    pub fn create_window(&self) -> Ref<Window> {
        let window = Window::new();
        window.set_title(Some("FerroUI · LiveCharts".into()));
        window.set_width(VIEW_SIZE.width);
        window.set_height(VIEW_SIZE.height);
        window.set_min_width(1100.);
        window.set_min_height(800.);
        window.set_content(Some(Control::boxed(self.root.clone().upcast::<Control>())));
        window
    }

    /// The same action as the button; reproducible in tests and render captures.
    pub fn advance(&self) {
        let next = self.step.get().wrapping_add(1);
        self.step.set(next);
        update_values(&self.root, next);
    }

    /// Measures and arranges the actual native visual tree.
    pub fn layout(&self, size: Size) {
        self.root.measure(size);
        self.root.arrange(ferroui_base::Rect::from_size(size));
    }

    /// Counts the measured data hits in every family; used by the render smoke.
    pub fn data_counts(&self) -> Result<Vec<(ChartKind, usize)>, ferroui_livecharts::ChartError> {
        (0..CHART_NAMES.len())
            .map(|index| {
                let chart = self.chart(index);
                let scene = chart.scene(chart.bounds().size())?;
                Ok((chart.chart_kind(), scene.hits.len()))
            })
            .collect()
    }
}

fn chart(root: &Grid, name: &str) -> Ref<Chart> {
    root.get_control(name)
}

fn populate(root: &Grid) {
    let cartesian = chart(root, "CartesianDemo");
    cartesian
        .series()
        .get(0)
        .set_numeric_values([24., 31., 28., 43., 38., 52.]);
    cartesian
        .series()
        .get(1)
        .set_numeric_values([28., 32., 35., 39., 43., 48.]);
    cartesian.x_axes().get(0).set_labels(
        ["May", "Jun", "Jul", "Aug", "Sep", "Oct"]
            .map(String::from)
            .to_vec(),
    );

    let pie = chart(root, "PieDemo");
    for (series, value) in pie.series().to_vec().iter().zip([42., 28., 18., 12.]) {
        series.set_numeric_values([value]);
    }

    let polar = chart(root, "PolarDemo");
    polar
        .series()
        .get(0)
        .set_numeric_values([8., 6., 9., 7., 5., 8.]);
    polar
        .series()
        .get(1)
        .set_numeric_values([9., 8., 9., 9., 8., 9.]);
    polar.x_axes().get(0).set_labels(
        [
            "Speed",
            "Quality",
            "Reach",
            "Reliability",
            "Access",
            "Support",
        ]
        .map(String::from)
        .to_vec(),
    );

    chart(root, "SankeyDemo").set_sankey_options(SankeyOptions {
        node_width: 16.,
        node_padding: 24.,
        node_corner_radius: 3.,
        show_labels: true,
        label_size: 12.,
        link_opacity: 0.4,
        ..SankeyOptions::default()
    });
    chart(root, "TreemapDemo").set_treemap_options(TreemapOptions {
        padding: 3.,
        corner_radius: 3.,
        show_labels: true,
        label_size: 13.,
        ..TreemapOptions::default()
    });
    let geo = chart(root, "GeoDemo");
    geo.set_geo_data(Some(
        GeoData::world().expect("the pinned world asset is valid GeoJSON"),
    ));
    geo.set_geo_options(GeoMapOptions {
        projection: MapProjection::Mercator,
        land_fill: Color::from_rgb(229, 235, 242),
        land_stroke: Color::from_rgb(255, 255, 255),
        stroke_width: 0.7,
        min_heat: Some(0.),
        max_heat: Some(100.),
        heat_colors: vec![
            Color::from_rgb(216, 239, 243),
            Color::from_rgb(38, 133, 163),
        ],
        ..GeoMapOptions::default()
    });
}

fn update_values(root: &Grid, step: u32) {
    let phase = (step % 5) as f64;
    chart(root, "CartesianDemo")
        .series()
        .get(0)
        .values()
        .get(5)
        .set_y(52. + phase * 6.);
    chart(root, "PieDemo")
        .series()
        .get(0)
        .values()
        .get(0)
        .set_y(42. + phase * 5.);
    chart(root, "PolarDemo")
        .series()
        .get(0)
        .values()
        .get(4)
        .set_y(5. + phase);
    let links = chart(root, "SankeyDemo").sankey_links();
    links.get(0).set_weight(38. + phase * 6.);
    links.get(3).set_weight(56. + phase * 6.);
    chart(root, "TreemapDemo")
        .treemap_roots()
        .get(0)
        .children()
        .expect("the Applications root has children")
        .get(0)
        .set_numeric_value(45. + phase * 12.);
    chart(root, "GeoDemo")
        .heat_lands()
        .get(0)
        .set_numeric_value(80. - phase * 12.);
    chart(root, "GeoDemo")
        .geo_markers()
        .get(0)
        .set_radius(4. + phase);
    root.get_control::<TextBlock>("StatusText")
        .set_text(Some(&format!(
        "Update {step} · points, flow weights, hierarchy leaves and map overlays changed in place"
    )));
}

#[cfg(test)]
mod tests;
