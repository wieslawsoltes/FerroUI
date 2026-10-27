//! Reusable wrap metrics, exclusion geometry and text flow across columns.
//!
//! All coordinates use the same units as the configured text measurer. This
//! crate has no rendering, windowing or native-platform dependency.

#![forbid(unsafe_code)]

mod column_flow_layout;
mod layout_primitives;
mod obstacle_layout_helper;
mod prepared_text_metrics;

pub use column_flow_layout::{ColumnFlowLayout, LayoutError};
pub use layout_primitives::{CircleObstacle, Interval, PositionedLine, RectObstacle, WrapMetrics};
pub use obstacle_layout_helper::ObstacleLayoutHelper;
pub use prepared_text_metrics::PreparedTextMetrics;

#[cfg(test)]
mod tests;
