//! Live chart calculation and retained scenes for FerroUI: the core of the LiveCharts2 port
//! (axes, scales, bounds, stacking, motion, paints, scenes and visual states) and the Cartesian
//! engine. The other chart engines land with their upstream tests in the following parts
//! (docs/porting/livecharts2.md); see NOTICE.md for the source attribution.

pub mod axis;
pub mod cartesian_chart_engine;
pub mod easing;
pub mod easing_functions;
pub mod kernel;
pub mod measure;
pub mod motion;
pub mod paint;
pub mod scene;
mod scene_transform;
pub mod series;
pub mod visual_states;

pub use axis::{Axis, AxisPosition, AxisScale};
pub use cartesian_chart_engine::{
    CartesianChartEngine, CartesianMeasurement, CartesianOptions, CartesianSeriesBounds, Section,
};
pub use easing_functions::{EasingFunctions, KeyFrame};
pub use kernel::{Coordinate, Error, StackedValue, Stacker};
pub use motion::{Animatable, Animation, MotionProperty, Repeat};
pub use paint::BrushSpace;
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
mod cartesian_series_tests;
#[cfg(test)]
mod upstream_regression_tests;
