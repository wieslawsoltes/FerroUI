use ferroui_base::controls::{ResourceDictionary, ResourceKey, ResourceProviderImpl};
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref};
use std::rc::Rc;

/// A resource dictionary that converts a resource key to the resource it
/// holds under that key.
#[repr(C)]
pub struct ResourceSelectorConverter {
    base: ResourceDictionary,
}

ferro_class!(ResourceSelectorConverter: ResourceDictionary);
ferro_class_info!(ResourceSelectorConverter {
    new: ResourceSelectorConverter::new,
    interfaces: [Rc<dyn IValueConverter> => ResourceSelectorConverter::as_value_converter],
});

impl FerroObjectImpl for ResourceSelectorConverter {}

impl ResourceProviderImpl for ResourceSelectorConverter {}

impl ResourceSelectorConverter {
    pub fn construct() -> Self {
        Self { base: ResourceDictionary::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The converter as its contract (the class implements it in the
    /// original).
    pub fn as_value_converter(this: Ref<Self>) -> Rc<dyn IValueConverter> {
        Rc::new(ResourceSelectorConverterHandle(this))
    }

    /// `Convert`: the resource of the dictionary whose key is the text
    /// `key`. Fails when `key` is not text.
    pub fn convert(&self, key: Option<&BoxedValue>) -> Result<Option<BoxedValue>, BindingError> {
        let Some(key) = key.and_then(|key| key.downcast_ref::<String>()) else {
            return Err(BindingError::message("The resource key of the converter must be a string."));
        };

        Ok(self.try_get_resource(&ResourceKey::from(key.as_str()), None).flatten())
    }

    /// `ConvertBack`: fails, the conversion has no inverse.
    pub fn convert_back(&self, _value: Option<&BoxedValue>) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}

/// The value converter contract of a [`ResourceSelectorConverter`].
struct ResourceSelectorConverterHandle(Ref<ResourceSelectorConverter>);

impl IValueConverter for ResourceSelectorConverterHandle {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        self.0.convert(value)
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        self.0.convert_back(value)
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project has no tests.
    use super::*;

    #[test]
    fn converts_a_key_to_the_resource_of_the_dictionary() {
        let converter = ResourceSelectorConverter::new();
        converter.add("Icon_File", Some(Rc::new(42i32) as BoxedValue));
        let contract = ResourceSelectorConverter::as_value_converter(converter);

        let found = contract.convert(Some(&(Rc::new("Icon_File".to_string()) as BoxedValue)), ValueType::of::<i32>(), None);
        assert_eq!(Some(&42), found.unwrap().unwrap().downcast_ref::<i32>());

        let missing = contract.convert(Some(&(Rc::new("Icon_Folder".to_string()) as BoxedValue)), ValueType::of::<i32>(), None);
        assert!(missing.unwrap().is_none());

        assert!(contract.convert(Some(&(Rc::new(1i32) as BoxedValue)), ValueType::of::<i32>(), None).is_err());
        assert!(contract.convert(None, ValueType::of::<i32>(), None).is_err());
    }
}
