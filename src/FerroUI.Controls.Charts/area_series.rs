use crate::CartesianSeries;
use ferroui_base::media::IBrush;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::rc::Rc;

/// A filled area between sample values and the zero baseline.
#[repr(C)]
pub struct AreaSeries {
    base: CartesianSeries,
}

ferro_class!(AreaSeries: CartesianSeries);
ferro_class_info!(AreaSeries {
    new: AreaSeries::new
});
ferro_impl_classes!(AreaSeries: FerroObjectImpl);

ferro_properties! { impl AreaSeries {
    pub fn fill_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<AreaSeries, _>("Fill", None)
    }
} }

impl AreaSeries {
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
}
