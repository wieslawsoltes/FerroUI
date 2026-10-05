use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::utilities::ByteSizeHelper;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::rc::Rc;

/// Converts a size in bytes to text with a unit (`1.5KB`); a size that is
/// not positive converts to empty text.
#[derive(Default)]
pub struct FileSizeStringConverter;

impl PartialEq for FileSizeStringConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl FileSizeStringConverter {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
}

impl IValueConverter for FileSizeStringConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(&size) = value.and_then(|value| value.downcast_ref::<i64>()) {
            if size > 0 {
                return Ok(Some(Rc::new(ByteSizeHelper::to_string(size as u64, true))));
            }
        }

        Ok(Some(Rc::new(String::new())))
    }

    /// Fails: the conversion has no inverse.
    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}

ferro_markup_type!(class FileSizeStringConverter {
    this: Rc<FileSizeStringConverter>,
    handles: [FileSizeStringConverter, Rc<FileSizeStringConverter>, Option<Rc<FileSizeStringConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => FileSizeStringConverter::new],
});

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project has no tests.
    use super::*;

    fn convert(value: BoxedValue) -> String {
        let converted = FileSizeStringConverter::new().convert(Some(&value), ValueType::of::<String>(), None).unwrap();
        converted.unwrap().downcast_ref::<String>().cloned().unwrap()
    }

    #[test]
    fn converts_positive_sizes_and_nothing_else() {
        assert_eq!("1.5KB", convert(Rc::new(1500i64)));
        assert_eq!("", convert(Rc::new(0i64)));
        assert_eq!("", convert(Rc::new(-5i64)));
        // Only a 64-bit size is a size.
        assert_eq!("", convert(Rc::new(1500i32)));
    }
}
