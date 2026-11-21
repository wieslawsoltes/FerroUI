//! Explicit adapter between source Visibility enum properties and native IsVisible.
use crate::compat::Visibility;
use ferroui_base::{
    BoxedValue,
    data::{BindingError, converters::IValueConverter, core::ValueType},
    utilities::CultureInfo,
};
use std::rc::Rc;
pub struct RibbonInputTemplateValues;
struct VisibilityConverter;
impl IValueConverter for VisibilityConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let v = value
            .and_then(|v| v.downcast_ref::<Visibility>())
            .ok_or_else(|| BindingError::message("Expected source Visibility"))?;
        Ok(Some(Rc::new(*v == Visibility::Visible)))
    }
    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let v = value
            .and_then(|v| v.downcast_ref::<bool>())
            .ok_or_else(|| BindingError::message("Expected native IsVisible bool"))?;
        Ok(Some(Rc::new(if *v {
            Visibility::Visible
        } else {
            Visibility::Collapsed
        })))
    }
}
impl RibbonInputTemplateValues {
    pub fn visibility_to_boolean() -> Rc<dyn IValueConverter> {
        ferroui_base::ferro_thread_local! {static CONVERTER:Rc<dyn IValueConverter>=Rc::new(VisibilityConverter);}
        CONVERTER.with(Clone::clone)
    }
}
ferroui_base::ferro_markup_type!(static RibbonInputTemplateValues{namespace:"FerroRibbon.Internal",static_properties:[VisibilityToBoolean:Rc<dyn IValueConverter>{get:RibbonInputTemplateValues::visibility_to_boolean}]});
