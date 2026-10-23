//! Typed, immutable series snapshots consumed by the chart measurement engines.

use crate::kernel::Coordinate;
use crate::paint::{representative_color, snapshot_brush, BrushSpace};
use crate::scene::{palette_color, PaintStyle};
use ferroui_base::media::{Color, IBrush, PenLineCap, PenLineJoin};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum SeriesKind {
    #[default]
    Line,
    StepLine,
    Area,
    StackedArea,
    StackedStepArea,
    Column,
    Row,
    StackedColumn,
    StackedRow,
    Scatter,
    Heat,
    RangeColumn,
    RangeRow,
    RangeLine,
    Financial,
    Box,
    Pie,
    PolarLine,
}
ferroui_base::ferro_markup_enum!(SeriesKind {
    Line,
    StepLine,
    Area,
    StackedArea,
    StackedStepArea,
    Column,
    Row,
    StackedColumn,
    StackedRow,
    Scatter,
    Heat,
    RangeColumn,
    RangeRow,
    RangeLine,
    Financial,
    Box,
    Pie,
    PolarLine
});
impl SeriesKind {
    pub fn is_stacked(self) -> bool {
        matches!(
            self,
            Self::StackedArea | Self::StackedStepArea | Self::StackedColumn | Self::StackedRow
        )
    }
    pub fn is_row(self) -> bool {
        matches!(self, Self::Row | Self::StackedRow | Self::RangeRow)
    }
    pub fn is_bar(self) -> bool {
        matches!(
            self,
            Self::Column
                | Self::Row
                | Self::StackedColumn
                | Self::StackedRow
                | Self::RangeColumn
                | Self::RangeRow
                | Self::Financial
                | Self::Box
        )
    }
    pub fn is_area(self) -> bool {
        matches!(
            self,
            Self::Area | Self::StackedArea | Self::StackedStepArea | Self::RangeLine
        )
    }
}

/// Which edge of a radial slot retains its position when its width is limited.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RadialAlignment {
    #[default]
    Outer,
    Center,
    Inner,
}
ferroui_base::ferro_markup_enum!(RadialAlignment {
    Outer,
    Center,
    Inner
});

/// Pie label positions corresponding to the upstream PolarLabelsPosition enum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PolarLabelsPosition {
    ChartCenter,
    End,
    Start,
    #[default]
    Middle,
    Outer,
}
ferroui_base::ferro_markup_enum!(PolarLabelsPosition {
    ChartCenter,
    End,
    Start,
    Middle,
    Outer
});

#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub name: String,
    pub kind: SeriesKind,
    pub values: Vec<Coordinate>,
    pub visible: bool,
    pub x_axis: usize,
    pub y_axis: usize,
    pub stack_group: u32,
    pub stack_100_percent: bool,
    pub pivot: f64,
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub fill_brush: Option<Rc<dyn IBrush>>,
    pub stroke_brush: Option<Rc<dyn IBrush>>,
    pub brush_space: BrushSpace,
    pub stroke_width: f64,
    pub stroke_cap: PenLineCap,
    pub stroke_join: PenLineJoin,
    pub stroke_miter: f64,
    pub dashes: Vec<f64>,
    pub dash_offset: f64,
    pub opacity: f64,
    pub geometry_size: f64,
    pub min_geometry_size: f64,
    pub line_smoothness: f64,
    pub enable_null_splitting: bool,
    pub max_bar_width: f64,
    pub padding: f64,
    pub corner_radius: f64,
    pub ignores_bar_position: bool,
    pub data_labels: bool,
    pub data_labels_size: f64,
    pub data_labels_color: Option<Color>,
    pub data_labels_rotation: f64,
    pub heat_map: Vec<Color>,
    pub inner_radius: f64,
    pub outer_radius_offset: f64,
    pub push_out: f64,
    pub hover_push_out: f64,
    pub relative_inner_radius: f64,
    pub relative_outer_radius: f64,
    pub max_radial_column_width: f64,
    pub radial_align: RadialAlignment,
    pub inverted_corner_radius: bool,
    pub is_gauge: bool,
    pub is_fill_series: bool,
    pub is_relative_to_min_value: bool,
    pub polar_labels_position: PolarLabelsPosition,
    pub polar_closed: bool,
}
impl Default for Series {
    fn default() -> Self {
        Self::new(SeriesKind::Line)
    }
}
impl Series {
    pub fn new(kind: SeriesKind) -> Self {
        Self {
            name: String::new(),
            kind,
            values: Vec::new(),
            visible: true,
            x_axis: 0,
            y_axis: 0,
            stack_group: 0,
            stack_100_percent: false,
            pivot: 0.0,
            fill: None,
            stroke: None,
            fill_brush: None,
            stroke_brush: None,
            brush_space: BrushSpace::Clip,
            stroke_width: 2.0,
            stroke_cap: PenLineCap::Flat,
            stroke_join: PenLineJoin::Miter,
            stroke_miter: 0.0,
            dashes: Vec::new(),
            dash_offset: 0.0,
            opacity: 1.0,
            geometry_size: 8.0,
            min_geometry_size: 4.0,
            line_smoothness: 0.65,
            enable_null_splitting: true,
            max_bar_width: 50.0,
            padding: 4.0,
            corner_radius: 0.0,
            ignores_bar_position: false,
            data_labels: false,
            data_labels_size: 12.0,
            data_labels_color: None,
            data_labels_rotation: 0.0,
            heat_map: vec![Color::from_rgb(235, 244, 255), Color::from_rgb(30, 64, 175)],
            inner_radius: 0.0,
            outer_radius_offset: 0.0,
            push_out: 0.0,
            hover_push_out: 20.0,
            relative_inner_radius: 0.0,
            relative_outer_radius: 0.0,
            max_radial_column_width: f64::MAX,
            radial_align: RadialAlignment::Outer,
            inverted_corner_radius: false,
            is_gauge: false,
            is_fill_series: false,
            is_relative_to_min_value: false,
            polar_labels_position: PolarLabelsPosition::Middle,
            polar_closed: true,
        }
    }
    pub fn from_values(kind: SeriesKind, values: impl IntoIterator<Item = f64>) -> Self {
        let mut result = Self::new(kind);
        result.values = values
            .into_iter()
            .enumerate()
            .map(|(i, y)| Coordinate::new(i as f64, y))
            .collect();
        result
    }
    pub fn color(&self, index: usize) -> Color {
        self.stroke_brush
            .as_ref()
            .and_then(|brush| representative_color(brush.as_ref()))
            .or(self.stroke)
            .or_else(|| {
                self.fill_brush
                    .as_ref()
                    .and_then(|brush| representative_color(brush.as_ref()))
            })
            .or(self.fill)
            .unwrap_or_else(|| palette_color(index))
    }
    pub fn style(&self, index: usize, filled: bool) -> PaintStyle {
        let color = self.color(index);
        PaintStyle {
            fill: if filled {
                Some(self.fill.unwrap_or(color))
            } else {
                self.fill
            },
            stroke: Some(self.stroke.unwrap_or(color)),
            fill_brush: self.fill_brush.as_ref().map(snapshot_brush),
            stroke_brush: self.stroke_brush.as_ref().map(snapshot_brush),
            brush_space: self.brush_space,
            stroke_width: self.stroke_width,
            stroke_cap: self.stroke_cap,
            stroke_join: self.stroke_join,
            stroke_miter: self.stroke_miter,
            opacity: self.opacity,
            dashes: self.dashes.clone(),
            dash_offset: self.dash_offset,
        }
    }
}
