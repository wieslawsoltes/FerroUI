use crate::{Axis, GridLines};
use ferroui_base::media::IBrush;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::rc::Rc;

/// Presentation properties shared by category, numeric, and date axes.
#[repr(C)]
pub struct CartesianAxis {
    base: Axis,
}

ferro_class!(CartesianAxis: Axis);
ferro_class_info!(CartesianAxis {
    new: CartesianAxis::new
});
ferro_impl_classes!(CartesianAxis: FerroObjectImpl);

ferro_properties! { impl CartesianAxis {
    pub fn show_tick_labels_property() -> StyledProperty<bool> {
        FerroProperty::register::<CartesianAxis, _>("ShowTickLabels", true)
    }
    pub fn show_tick_marks_property() -> StyledProperty<bool> {
        FerroProperty::register::<CartesianAxis, _>("ShowTickMarks", false)
    }
    pub fn grid_lines_property() -> StyledProperty<GridLines> {
        FerroProperty::register::<CartesianAxis, _>("GridLines", GridLines::None)
    }
    pub fn grid_line_major_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<CartesianAxis, _>("GridLineMajorBrush", None)
    }
    pub fn grid_line_minor_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<CartesianAxis, _>("GridLineMinorBrush", None)
    }
    pub fn tick_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<CartesianAxis, _>("TickBrush", None)
    }
    pub fn tick_label_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<CartesianAxis, _>("TickLabelBrush", None)
    }
    pub fn axis_line_brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<CartesianAxis, _>("AxisLineBrush", None)
    }
} }

impl CartesianAxis {
    pub fn construct() -> Self {
        Self {
            base: Axis::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn show_tick_labels(&self) -> bool {
        self.get_value(Self::show_tick_labels_property())
    }
    pub fn set_show_tick_labels(&self, value: bool) {
        self.set_value(Self::show_tick_labels_property(), value);
    }
    pub fn show_tick_marks(&self) -> bool {
        self.get_value(Self::show_tick_marks_property())
    }
    pub fn set_show_tick_marks(&self, value: bool) {
        self.set_value(Self::show_tick_marks_property(), value);
    }
    pub fn grid_lines(&self) -> GridLines {
        self.get_value(Self::grid_lines_property())
    }
    pub fn set_grid_lines(&self, value: GridLines) {
        self.set_value(Self::grid_lines_property(), value);
    }
    pub fn grid_line_major_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::grid_line_major_brush_property())
    }
    pub fn set_grid_line_major_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::grid_line_major_brush_property(), value);
    }
    pub fn grid_line_minor_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::grid_line_minor_brush_property())
    }
    pub fn set_grid_line_minor_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::grid_line_minor_brush_property(), value);
    }
    pub fn tick_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::tick_brush_property())
    }
    pub fn set_tick_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::tick_brush_property(), value);
    }
    pub fn tick_label_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::tick_label_brush_property())
    }
    pub fn set_tick_label_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::tick_label_brush_property(), value);
    }
    pub fn axis_line_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::axis_line_brush_property())
    }
    pub fn set_axis_line_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::axis_line_brush_property(), value);
    }
}
