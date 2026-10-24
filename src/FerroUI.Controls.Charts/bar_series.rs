use crate::{BarOrientation, CartesianSeries};
use ferroui_base::media::IBrush;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::rc::Rc;

/// Bars displayed horizontally by default. X is always category/time data and Y values.
#[repr(C)]
pub struct BarSeries {
    base: CartesianSeries,
}

ferro_class!(BarSeries: CartesianSeries);
ferro_class_info!(BarSeries {
    new: BarSeries::new
});
ferro_impl_classes!(BarSeries: FerroObjectImpl);

ferro_properties! { impl BarSeries {
    pub fn fill_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<BarSeries, _>("Fill", None)
    }

    pub fn orientation_property() -> StyledProperty<BarOrientation> {
        FerroProperty::register::<BarSeries, _>("Orientation", BarOrientation::Horizontal)
    }
} }

impl BarSeries {
    pub fn construct() -> Self {
        Self {
            base: CartesianSeries::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn fill(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::fill_property())
    }
    pub fn set_fill(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::fill_property(), value);
    }
    pub fn orientation(&self) -> BarOrientation {
        self.get_value(Self::orientation_property())
    }
    pub fn set_orientation(&self, value: BarOrientation) {
        self.set_value(Self::orientation_property(), value);
    }
}
