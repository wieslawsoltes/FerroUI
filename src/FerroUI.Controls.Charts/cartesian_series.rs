use crate::{
    Axis, CategoryAxis, ChartError, DataLabelOverride, DataMarkerOverride, DateTimeAxis,
    LinearAxis, MarkerShape, Samples, StrokeDashStyle,
};
use ferroui_base::collections::FerroDictionary;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::IBrush;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, DirectProperty,
    FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Shared property and data contract for line, area, and bar series.
///
/// Series are chart-owned, but their samples and immutable override descriptors
/// are shareable. Axis references use validated direct properties because their
/// validity depends on chart ownership as well as the axis' concrete type.
#[repr(C)]
pub struct CartesianSeries {
    base: FerroObject,
    owner: Cell<Option<u64>>,
    x_axis: RefCell<Option<Ref<Axis>>>,
    y_axis: RefCell<Option<Ref<Axis>>>,
    data_label_overrides: FerroDictionary<u32, Rc<DataLabelOverride>>,
    data_marker_overrides: FerroDictionary<u32, Rc<DataMarkerOverride>>,
}

ferro_class!(CartesianSeries: FerroObject);
// This base is intentionally not constructible through XAML.
ferro_class_info!(CartesianSeries {});
ferro_impl_classes!(CartesianSeries: FerroObjectImpl);

ferro_properties! { impl CartesianSeries {
    pub fn title_property() -> StyledProperty<String> {
        FerroProperty::register::<CartesianSeries, _>("Title", String::new())
    }

    pub fn is_visible_property() -> StyledProperty<bool> {
        FerroProperty::register::<CartesianSeries, _>("IsVisible", true)
    }

    pub fn x_values_property() -> StyledProperty<Option<Ref<Samples>>> {
        FerroProperty::register::<CartesianSeries, _>("XValues", None)
    }

    pub fn y_values_property() -> StyledProperty<Option<Ref<Samples>>> {
        FerroProperty::register::<CartesianSeries, _>("YValues", None)
    }

    pub fn stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<CartesianSeries, _>("Stroke", None)
    }

    pub fn stroke_thickness_property() -> StyledProperty<f64> {
        FerroProperty::register::<CartesianSeries, _>("StrokeThickness", 1.0)
    }

    pub fn stroke_dash_style_property() -> StyledProperty<StrokeDashStyle> {
        FerroProperty::register::<CartesianSeries, _>("StrokeDashStyle", StrokeDashStyle::Solid)
    }

    pub fn show_data_labels_property() -> StyledProperty<bool> {
        FerroProperty::register::<CartesianSeries, _>("ShowDataLabels", false)
    }

    pub fn show_data_markers_property() -> StyledProperty<bool> {
        FerroProperty::register::<CartesianSeries, _>("ShowDataMarkers", false)
    }

    pub fn marker_shape_property() -> StyledProperty<MarkerShape> {
        FerroProperty::register::<CartesianSeries, _>("MarkerShape", MarkerShape::Circle)
    }

    pub fn data_label_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<CartesianSeries, _>("DataLabelBrush", None)
    }

    pub fn data_marker_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<CartesianSeries, _>("DataMarkerBrush", None)
    }

    /// Native direct-property notification for the source API's plain XAxis property.
    pub fn x_axis_property() -> DirectProperty<CartesianSeries, Option<Ref<Axis>>> {
        FerroProperty::register_direct::<CartesianSeries, _>(
            "XAxis", |this| this.x_axis(),
            Some(|this, value| {
                // Deviation (DEVIATIONS.md, WinUI Charts): upstream XAxis is a plain property; a
                // rejected write through the property system keeps the old axis and is logged.
                if let Err(error) = this.set_x_axis(value) {
                    log_rejected_axis(this, "XAxis", &error);
                }
            }), None,
        )
    }

    /// Native direct-property notification for the source API's plain YAxis property.
    pub fn y_axis_property() -> DirectProperty<CartesianSeries, Option<Ref<Axis>>> {
        FerroProperty::register_direct::<CartesianSeries, _>(
            "YAxis", |this| this.y_axis(),
            Some(|this, value| {
                // Deviation (DEVIATIONS.md, WinUI Charts): upstream YAxis is a plain property; a
                // rejected write through the property system keeps the old axis and is logged.
                if let Err(error) = this.set_y_axis(value) {
                    log_rejected_axis(this, "YAxis", &error);
                }
            }), None,
        )
    }
} }

/// A write of `XAxis` or `YAxis` through the property system (a binding, a style, an
/// inspector) that the series rejects: the old axis stays and the reason is logged.
fn log_rejected_axis(series: &CartesianSeries, property: &str, error: &ChartError) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::PROPERTY) {
        logger.log(Some(series as &dyn Any), &format!("{property} was not changed: {error}"));
    }
}

impl CartesianSeries {
    /// Creates base data for a concrete series class.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            owner: Cell::new(None),
            x_axis: RefCell::new(None),
            y_axis: RefCell::new(None),
            data_label_overrides: FerroDictionary::new(),
            data_marker_overrides: FerroDictionary::new(),
        }
    }

    pub fn title(&self) -> String {
        self.get_value(Self::title_property())
    }
    pub fn set_title(&self, value: impl Into<String>) {
        self.set_value(Self::title_property(), value.into());
    }
    pub fn set_optional_title(&self, value: Option<&str>) {
        self.set_title(value.unwrap_or_default());
    }
    pub fn is_visible(&self) -> bool {
        self.get_value(Self::is_visible_property())
    }
    pub fn set_is_visible(&self, value: bool) {
        self.set_value(Self::is_visible_property(), value);
    }
    pub fn x_values(&self) -> Option<Ref<Samples>> {
        self.get_value(Self::x_values_property())
    }
    pub fn set_x_values(&self, value: Option<Ref<Samples>>) {
        self.set_value(Self::x_values_property(), value);
    }
    pub fn y_values(&self) -> Option<Ref<Samples>> {
        self.get_value(Self::y_values_property())
    }
    pub fn set_y_values(&self, value: Option<Ref<Samples>>) {
        self.set_value(Self::y_values_property(), value);
    }
    pub fn stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::stroke_property())
    }
    pub fn set_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::stroke_property(), value);
    }
    pub fn stroke_thickness(&self) -> f64 {
        self.get_value(Self::stroke_thickness_property())
    }
    pub fn set_stroke_thickness(&self, value: f64) {
        self.set_value(Self::stroke_thickness_property(), value);
    }
    pub fn stroke_dash_style(&self) -> StrokeDashStyle {
        self.get_value(Self::stroke_dash_style_property())
    }
    pub fn set_stroke_dash_style(&self, value: StrokeDashStyle) {
        self.set_value(Self::stroke_dash_style_property(), value);
    }
    pub fn show_data_labels(&self) -> bool {
        self.get_value(Self::show_data_labels_property())
    }
    pub fn set_show_data_labels(&self, value: bool) {
        self.set_value(Self::show_data_labels_property(), value);
    }
    pub fn show_data_markers(&self) -> bool {
        self.get_value(Self::show_data_markers_property())
    }
    pub fn set_show_data_markers(&self, value: bool) {
        self.set_value(Self::show_data_markers_property(), value);
    }
    pub fn marker_shape(&self) -> MarkerShape {
        self.get_value(Self::marker_shape_property())
    }
    pub fn set_marker_shape(&self, value: MarkerShape) {
        self.set_value(Self::marker_shape_property(), value);
    }
    pub fn data_label_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::data_label_brush_property())
    }
    pub fn set_data_label_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::data_label_brush_property(), value);
    }
    pub fn data_marker_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::data_marker_brush_property())
    }
    pub fn set_data_marker_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::data_marker_brush_property(), value);
    }

    pub fn x_axis(&self) -> Option<Ref<Axis>> {
        self.x_axis.borrow().clone()
    }
    pub fn y_axis(&self) -> Option<Ref<Axis>> {
        self.y_axis.borrow().clone()
    }

    /// Set the category/time data axis, preserving the old value on failure.
    pub fn set_x_axis(&self, value: Option<Ref<Axis>>) -> Result<(), ChartError> {
        if let Some(axis) = &value {
            if !axis.is::<CategoryAxis>() && !axis.is::<DateTimeAxis>() {
                return Err(ChartError::InvalidValue(
                    "XAxis must be CategoryAxis or DateTimeAxis.",
                ));
            }
            self.validate_axis_owner(axis)?;
        }
        self.set_and_raise(Self::x_axis_property(), &self.x_axis, value);
        Ok(())
    }

    /// Set the numeric value axis, preserving the old value on failure.
    pub fn set_y_axis(&self, value: Option<Ref<Axis>>) -> Result<(), ChartError> {
        if let Some(axis) = &value {
            if !axis.is::<LinearAxis>() {
                return Err(ChartError::InvalidValue("YAxis must be LinearAxis."));
            }
            self.validate_axis_owner(axis)?;
        }
        self.set_and_raise(Self::y_axis_property(), &self.y_axis, value);
        Ok(())
    }

    /// Per-source-index immutable label overrides; clones share map identity.
    pub fn data_label_overrides(&self) -> FerroDictionary<u32, Rc<DataLabelOverride>> {
        self.data_label_overrides.clone()
    }

    /// Per-source-index immutable marker overrides; clones share map identity.
    pub fn data_marker_overrides(&self) -> FerroDictionary<u32, Rc<DataMarkerOverride>> {
        self.data_marker_overrides.clone()
    }

    /// An invalid public thickness is retained, but never passed to a renderer.
    pub fn effective_stroke_thickness(&self) -> f64 {
        let value = self.stroke_thickness();
        if value.is_finite() && value >= 0.0 && value <= f32::MAX as f64 {
            value
        } else {
            0.0
        }
    }

    pub(crate) fn owner_id(&self) -> Option<u64> {
        self.owner.get()
    }
    pub(crate) fn set_owner_id(&self, value: Option<u64>) {
        self.owner.set(value);
    }

    fn validate_axis_owner(&self, axis: &Axis) -> Result<(), ChartError> {
        if self.owner_id().is_some() && axis.owner_id() != self.owner_id() {
            return Err(ChartError::AxisNotInChart);
        }
        Ok(())
    }
}
