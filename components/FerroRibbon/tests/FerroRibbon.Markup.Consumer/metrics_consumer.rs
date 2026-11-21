use ferroribbon::primitives::RibbonMetrics;
use ferroribbon_core::RibbonDensity;
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_base::{
    input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::sync::Arc;
#[repr(C)]
pub struct MetricsConsumer {
    base: Control,
}
ferro_class!(MetricsConsumer:Control);
ferro_impl_classes!(MetricsConsumer:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,LayoutableImpl,ControlImpl);
ferro_class_info!(MetricsConsumer {new:MetricsConsumer::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_properties! {impl MetricsConsumer {
    pub fn density_property()->StyledProperty<RibbonDensity>{FerroProperty::register::<Self,_>("Density",RibbonDensity::Comfortable)}
    pub fn optional_density_property()->StyledProperty<Option<RibbonDensity>>{FerroProperty::register::<Self,_>("OptionalDensity",None)}
    pub fn custom_metrics_property()->StyledProperty<Option<Arc<RibbonMetrics>>>{FerroProperty::register::<Self,_>("CustomMetrics",None)}
    pub fn metrics_property()->StyledProperty<Arc<RibbonMetrics>>{FerroProperty::register::<Self,_>("Metrics",RibbonMetrics::comfortable())}
}}
impl MetricsConsumer {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
        })
    }
    pub fn density(&self) -> RibbonDensity {
        self.get_value(Self::density_property())
    }
    pub fn set_density(&self, value: RibbonDensity) {
        self.set_value(Self::density_property(), value);
    }
    pub fn optional_density(&self) -> Option<RibbonDensity> {
        self.get_value(Self::optional_density_property())
    }
    pub fn set_optional_density(&self, value: Option<RibbonDensity>) {
        self.set_value(Self::optional_density_property(), value);
    }
    pub fn custom_metrics(&self) -> Option<Arc<RibbonMetrics>> {
        self.get_value(Self::custom_metrics_property())
    }
    pub fn set_custom_metrics(&self, value: Option<Arc<RibbonMetrics>>) {
        self.set_value(Self::custom_metrics_property(), value);
    }
    pub fn metrics(&self) -> Arc<RibbonMetrics> {
        self.get_value(Self::metrics_property())
    }
    pub fn set_metrics(&self, value: Arc<RibbonMetrics>) {
        self.set_value(Self::metrics_property(), value);
    }
}
