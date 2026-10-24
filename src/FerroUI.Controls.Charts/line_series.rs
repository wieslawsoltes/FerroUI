use crate::CartesianSeries;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
};

/// A polyline series with optional sample markers and labels.
#[repr(C)]
pub struct LineSeries {
    base: CartesianSeries,
}

ferro_class!(LineSeries: CartesianSeries);
ferro_class_info!(LineSeries {
    new: LineSeries::new
});
ferro_impl_classes!(LineSeries: FerroObjectImpl);

impl LineSeries {
    pub fn construct() -> Self {
        Self {
            base: CartesianSeries::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
