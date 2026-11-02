//! Not from upstream: the page of the SankeyChart of the LiveCharts2 port
//! (docs/porting/livecharts2.md), the class of `Pages/SankeyChartPage.xaml`. The links of a Sankey
//! series refer to its node objects, so the page builds the graphs of its two series.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::media::Color;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;
use ferroui_controls_livecharts::{
    SankeyChart, SankeyLink, SankeyLinks, SankeyNode, SankeyNodes, SankeySeries,
};

#[repr(C)]
pub struct SankeyChartPage {
    base: ContentPage,
}

content_page_class!(SankeyChartPage);
ferro_class_info!(SankeyChartPage { new: SankeyChartPage::new });
xaml_class!(SankeyChartPage, "/Pages/SankeyChartPage.xaml");

/// Energy from four sources through electricity and heat to their uses: three columns.
const ENERGY_NODES: &[(&str, Option<Color>)] = &[
    ("Coal", None),
    ("Gas", None),
    ("Solar", None),
    ("Wind", None),
    ("Electricity", None),
    ("Heat", None),
    ("Homes", None),
    ("Industry", None),
    ("Losses", None),
];
const ENERGY_LINKS: &[(usize, usize, f64)] = &[
    (0, 4, 25.0),
    (1, 4, 20.0),
    (1, 5, 15.0),
    (2, 4, 10.0),
    (3, 4, 12.0),
    (4, 6, 30.0),
    (4, 7, 25.0),
    (4, 8, 12.0),
    (5, 6, 9.0),
    (5, 7, 6.0),
];

/// The categories and products of the upstream BipartiteArc snapshot, with its node colours.
const SALES_NODES: &[(&str, Option<Color>)] = &[
    ("Furniture", Some(Color::from_rgb(96, 138, 218))),
    ("Technology", Some(Color::from_rgb(52, 78, 144))),
    ("Office Supplies", Some(Color::from_rgb(150, 188, 232))),
    ("Chairs", Some(Color::from_rgb(96, 138, 218))),
    ("Phones", Some(Color::from_rgb(52, 78, 144))),
    ("Storage", Some(Color::from_rgb(120, 158, 220))),
    ("Tables", Some(Color::from_rgb(170, 60, 80))),
    ("Paper", Some(Color::from_rgb(232, 168, 130))),
];
const SALES_LINKS: &[(usize, usize, f64)] = &[
    (0, 3, 12.0),
    (0, 6, 6.0),
    (1, 4, 14.0),
    (1, 5, 8.0),
    (2, 5, 4.0),
    (2, 7, 9.0),
    (2, 3, 2.0),
];

impl SankeyChartPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        for (chart, nodes, links) in [
            ("EnergyChart", ENERGY_NODES, ENERGY_LINKS),
            ("SalesChart", SALES_NODES, SALES_LINKS),
        ] {
            if let Some(series) = this.series(chart) {
                build_graph(&series, nodes, links);
            }
        }
        this
    }

    fn series(&self, chart: &str) -> Option<Ref<SankeySeries>> {
        let chart = self.find_control::<SankeyChart>(chart)?;
        let series = chart.series().to_vec();
        series.first()?.cast::<SankeySeries>()
    }
}

fn build_graph(
    series: &SankeySeries,
    names: &[(&str, Option<Color>)],
    links: &[(usize, usize, f64)],
) {
    let nodes: Vec<_> = names
        .iter()
        .map(|&(name, color)| {
            let node = SankeyNode::with_name(name);
            node.set_color(color);
            node
        })
        .collect();
    series.set_links(SankeyLinks::from_items(links.iter().map(
        |&(source, target, weight)| SankeyLink::with_nodes(&nodes[source], &nodes[target], weight),
    )));
    series.set_nodes(SankeyNodes::from_items(nodes));
}
