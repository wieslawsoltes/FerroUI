//! Live chart calculation and retained scenes for FerroUI: the core of the LiveCharts2 port
//! (axes, scales, bounds, stacking, motion, paints, scenes, labelers and visual states) and the
//! Cartesian, pie, polar, Sankey, treemap and geographic engines (docs/porting/livecharts2.md);
//! see NOTICE.md for the source attribution and the bundled world map.

pub mod axis;
pub mod cartesian_chart_engine;
pub mod easing;
pub mod easing_functions;
pub mod geo;
pub mod geo_map_chart_engine;
pub mod heat_functions;
pub mod heat_legend;
pub mod kernel;
pub mod label_lines;
pub mod labelers;
pub mod measure;
pub mod motion;
pub mod paint;
pub mod pie_chart_engine;
pub mod polar_chart_engine;
pub mod sankey_chart_engine;
pub mod scene;
mod scene_transform;
pub mod series;
pub mod treemap_chart_engine;
pub mod visual_states;

pub use axis::{
    Axis, AxisPosition, AxisScale, CARTESIAN_LABELS_PADDING, POLAR_LABELS_BACKGROUND,
    POLAR_LABELS_PADDING,
};
pub use cartesian_chart_engine::{
    CartesianChartEngine, CartesianMeasurement, CartesianOptions, CartesianSeriesBounds,
    CartesianSeriesOptions, Section,
};
pub use easing_functions::{EasingFunctions, KeyFrame};
pub use geo::{
    DrawnMap, GeographicBounds, LandData, LandDefinition, MapLayer, MapProjection, MapProjector,
    Maps,
};
pub use geo_map_chart_engine::{
    GeoLand, GeoMapChartEngine, GeoMapMeasurement, GeoMapOptions, GeoMarker, GeoTooltipFormatter,
    GeoTooltipValue, HeatLand, HeatLandSeries, MapViewport, ZoomDirection,
};
pub use heat_functions::HeatFunctions;
pub use heat_legend::{HeatLegend, HeatLegendSource};
pub use kernel::{Coordinate, Error, StackedValue, Stacker};
pub use labelers::{as_date, as_time_span, ChartPointFormatter, Labeler, Labelers};
pub use motion::{Animatable, Animation, MotionProperty, Repeat};
pub use paint::BrushSpace;
pub use pie_chart_engine::{AngularTicks, Needle, PieChartEngine, PieOptions, PieVisual};
pub use polar_chart_engine::{
    PolarChartEngine, PolarMeasurement, PolarOptions, PolarScale, PolarToolTipLabelFormatters,
    COTANGENT_ANGLE, TANGENT_ANGLE,
};
pub use sankey_chart_engine::{
    SankeyChartEngine, SankeyLabelPosition, SankeyLayoutKind, SankeyLink, SankeyNode,
    SankeyOptions, SankeySeries, SankeySeriesOptions,
};
pub use scene::{
    ChartError, FindingStrategy, Geometry, HitRegion, HitShape, LegendItem, PaintStyle,
    PathCommand, PointId, Primitive, Scene, TextAnchor,
};
pub use series::{DataLabelsPosition, PolarLabelsPosition, RadialAlignment, Series, SeriesKind};
pub use treemap_chart_engine::{
    TreemapChartEngine, TreemapNode, TreemapOptions, TreemapSeries, TreemapSeriesOptions,
};
pub use visual_states::{
    DrawnPropertiesDictionary, DrawnPropertySetter, IStateBehavior, StateBehavior, StateOrigin,
    VisualStatesDictionary,
};
mod register_types;
pub use register_types::register_types;

#[cfg(test)]
mod cartesian_series_tests;
#[cfg(test)]
mod upstream_regression_tests;
#[cfg(test)]
mod pie_series_tests;
