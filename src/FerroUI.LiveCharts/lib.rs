//! Live chart calculation and retained scenes for FerroUI.
//! See the port record and NOTICE.md for source attribution and parity status.

pub mod axis;
pub mod cartesian_chart_engine;
pub mod easing;
pub mod easing_functions;
pub mod geo;
pub mod geo_map_chart;
pub mod heat_functions;
pub mod kernel;
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

pub use axis::{Axis, AxisPosition, AxisScale};
pub use cartesian_chart_engine::{
    CartesianChartEngine, CartesianMeasurement, CartesianOptions, CartesianSeriesBounds, Section,
};
pub use easing_functions::{EasingFunctions, KeyFrame};
pub use geo_map_chart::{
    GeoData, GeoMapChartEngine, GeoMapLayout, GeoMapOptions, GeoMarker, HeatLand,
    MapViewportTransform,
};
pub use heat_functions::HeatFunctions;
pub use kernel::{Coordinate, Error, StackedValue, Stacker};
pub use motion::{Animatable, Animation, MotionProperty, Repeat};
pub use paint::BrushSpace;
pub use pie_chart_engine::{PieChartEngine, PieOptions};
pub use polar_chart_engine::{
    PolarChartEngine, PolarMeasurement, PolarOptions, PolarScale, COTANGENT_ANGLE, TANGENT_ANGLE,
};
pub use scene::{
    ChartError, FindingStrategy, Geometry, HitRegion, HitShape, LegendItem, PaintStyle,
    PathCommand, PointId, Primitive, Scene, TextAnchor,
};
pub use series::{PolarLabelsPosition, RadialAlignment, Series, SeriesKind};
pub use visual_states::{
    DrawnPropertiesDictionary, DrawnPropertySetter, IStateBehavior, StateBehavior, StateOrigin,
    VisualStatesDictionary,
};
mod register_types;
pub use register_types::register_types;

#[cfg(test)]
mod upstream_regression_tests;
