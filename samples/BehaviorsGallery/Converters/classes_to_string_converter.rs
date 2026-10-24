//! Port of `Converters/ClassesToStringConverter.cs`.

use ferroui_base::controls::Classes;
use ferroui_base::data::converters::IMultiValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_type, ferro_thread_local, BoxedValue, FerroProperty};
use std::rc::Rc;

/// Converts the number of the classes of an element and the classes to the classes as text,
/// separated by spaces: a multi-binding of the count (which changes when the classes change) and
/// the classes.
pub struct ClassesToStringConverter;

impl PartialEq for ClassesToStringConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_thread_local! {
    static INSTANCE: Rc<ClassesToStringConverter> = Rc::new(ClassesToStringConverter);
}

impl ClassesToStringConverter {
    /// `Instance`.
    pub fn instance() -> Rc<ClassesToStringConverter> {
        INSTANCE.with(Rc::clone)
    }

    /// `ProvideValue(IServiceProvider)`: the converter is a markup extension that gives its one
    /// instance.
    pub fn provide_value(
        &self,
        _service_provider: &Rc<dyn IServiceProvider>,
    ) -> Option<BoxedValue> {
        let converter: Rc<dyn IMultiValueConverter> = Self::instance();
        Some(Rc::new(converter) as BoxedValue)
    }
}

impl IMultiValueConverter for ClassesToStringConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let [Some(count), Some(classes)] = values {
            if let (Some(_), Some(classes)) = (
                count.downcast_ref::<i32>(),
                classes.downcast_ref::<Classes>(),
            ) {
                return Ok(Some(Rc::new(classes.snapshot().join(" ")) as BoxedValue));
            }
        }

        Ok(Some(FerroProperty::unset_value()))
    }
}

ferro_markup_type!(class ClassesToStringConverter {
    this: Rc<ClassesToStringConverter>,
    handles: [ClassesToStringConverter, Rc<ClassesToStringConverter>, Option<Rc<ClassesToStringConverter>>],
    interfaces: [Rc<dyn IMultiValueConverter>],
    constructors: [() => ClassesToStringConverter::instance],
    fields: [Instance: Rc<ClassesToStringConverter> => ClassesToStringConverter::instance],
    methods: [
        fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
            |this: &Rc<ClassesToStringConverter>, service_provider: Rc<dyn IServiceProvider>| this.provide_value(&service_provider),
    ],
});
