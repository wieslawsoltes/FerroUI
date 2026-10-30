//! Backend-independent native charts using FerroUI properties, layout, drawing,
//! text, observable collections, resources, and automation.
//!
//! The compatibility contract and its source boundary are documented in
//! `docs/porting/winui-charts.md` in the repository.

mod area_series;
mod axis;
mod bar_series;
mod cartesian_axis;
mod cartesian_series;
mod category_axis;
mod chart;
mod chart_automation_peer;
mod chart_collection;
mod chart_error;
mod data_label_override;
mod data_marker_override;
mod date_time_axis;
mod enums;
mod line_series;
mod linear_axis;
mod register_types;
mod rendering;
mod rust_paths;
mod samples;
mod xaml_charts_resources;

pub use area_series::*;
pub use axis::*;
pub use bar_series::*;
pub use cartesian_axis::*;
pub use cartesian_series::*;
pub use category_axis::*;
pub use chart::*;
pub use chart_automation_peer::*;
pub use chart_collection::*;
pub use chart_error::*;
pub use data_label_override::*;
pub use data_marker_override::*;
pub use date_time_axis::*;
pub use enums::*;
pub use line_series::*;
pub use linear_axis::*;
pub use register_types::{register_types, CHARTS_XML_NAMESPACE};
pub use rendering::{ChartLayout, ChartPoint};
pub use samples::*;
pub use xaml_charts_resources::*;

#[cfg(test)]
mod axis_tests;
#[cfg(test)]
mod chart_tests;
#[cfg(test)]
mod samples_tests;
#[cfg(test)]
mod series_tests;
#[cfg(test)]
mod xaml_tests;
