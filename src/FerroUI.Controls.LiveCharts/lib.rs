//! Live chart controls using FerroUI's property, input and drawing services: the Cartesian and
//! pie charts (L2) and the polar chart (L4) of the LiveCharts2 port (docs/porting/livecharts2.md);
//! see NOTICE.md for the source attribution.

pub use ferroui_livecharts as core;

mod axis;
mod chart;
mod chart_automation_peer;
mod chart_classes;
mod chart_point;
mod chart_properties;
mod chart_series;
mod collections;
mod decorations;
mod enums;
mod identity;
mod register_types;
pub mod renderer;
pub mod scene_transition;
mod series_classes;

pub use axis::Axis;
pub use chart::{Chart, ChartPointerEventArgs};
pub use chart_automation_peer::ChartAutomationPeer;
pub use chart_classes::*;
pub use chart_point::ChartPoint;
pub use chart_series::ChartSeries;
pub use collections::{Axes, ChartPoints, ChartSeriesCollection};
pub use enums::{ChartKind, LegendPosition, TooltipPosition, ZoomAndPanMode};
pub use ferroui_livecharts::{
    AxisPosition, BrushSpace, FindingStrategy, PolarLabelsPosition, PolarScale, RadialAlignment,
    SeriesKind,
};
pub use register_types::{register_types, ASSEMBLY};
pub use series_classes::*;
