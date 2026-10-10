//! Port of `GenericValueConverter.cs`.

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::media::{IBrush, SolidColorBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_type, BoxedValue, ObjectType, Ref};
use std::fmt::Display;
use std::marker::PhantomData;
use std::rc::{Rc, Weak};

/// A value converter with a type argument, which markup states with `x:TypeArguments`.
///
/// The managed original is an open generic class the markup compiler instantiates for the
/// type arguments a document names. Markup knows the instantiations a crate declares
/// (docs/porting/DEVIATIONS.md, "Run-time type system of markup"): the one the documents of
/// the sample name is declared below ([`GenericValueConverterOfSolidColorBrush`]). The type
/// argument is a class of the object model here (`value is T` is a cast of an object).
pub struct GenericValueConverter<T> {
    this: Weak<GenericValueConverter<T>>,
    _marker: PhantomData<fn(&T)>,
}

impl<T> PartialEq for GenericValueConverter<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl<T: ObjectType + Display> GenericValueConverter<T> {
    pub fn new() -> Rc<GenericValueConverter<T>> {
        Rc::new_cyclic(|this| Self { this: this.clone(), _marker: PhantomData })
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }

    /// `value as T`.
    fn instance(value: &BoxedValue) -> Option<Ref<T>> {
        from_markup_value::<Ref<T>>(&Some(value.clone())).or_else(|| {
            // A brush a binding read from a property of the contract type: the object behind
            // the handle of the contract.
            let brush = value.downcast_ref::<Rc<dyn IBrush>>()?;
            brush.as_object()?.to_ref().cast::<T>()
        })
    }
}

impl<T: ObjectType + Display> IValueConverter for GenericValueConverter<T> {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(value) = value.and_then(Self::instance) {
            return Ok(Some(Rc::new(format!("{}: {}", T::TYPE.name(), &*value)) as BoxedValue));
        }

        Ok(None)
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // `throw new NotSupportedException()`.
        Err(BindingError::message("Specified method is not supported."))
    }
}

/// Declares an instantiation of [`GenericValueConverter`] to markup: `$carrier` carries the
/// markup metadata of `GenericValueConverter<$class>`.
macro_rules! generic_value_converter {
    ($carrier:ident: $class:ty) => {
        /// Carries the markup metadata of an instantiation of the generic value converter.
        pub struct $carrier;

        ferro_markup_type!(class $carrier as "GenericValueConverter`1" {
            this: Rc<GenericValueConverter<$class>>,
            handles: [
                GenericValueConverter<$class>,
                Rc<GenericValueConverter<$class>>,
                Option<Rc<GenericValueConverter<$class>>>
            ],
            interfaces: [Rc<dyn IValueConverter>],
            generic: "GenericValueConverter`1" [Ref<$class>],
            constructors: [() => GenericValueConverter::<$class>::new],
        });
    };
}

// `<local:GenericValueConverter x:Key="BrushConverter" x:TypeArguments="SolidColorBrush"/>` of
// `MainWindow.xaml`.
generic_value_converter!(GenericValueConverterOfSolidColorBrush: SolidColorBrush);

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use ferroui_base::media::{Colors, LinearGradientBrush};

    fn convert(value: Option<BoxedValue>) -> Option<String> {
        let converter = GenericValueConverter::<SolidColorBrush>::new();
        let converted = converter.convert(value.as_ref(), ValueType::of::<String>(), None, &CultureInfo::invariant_culture());
        converted.ok().flatten().and_then(|text| text.downcast_ref::<String>().cloned())
    }

    #[test]
    fn a_value_of_the_type_argument_converts_to_its_type_and_text() {
        crate::register_types();
        let brush = SolidColorBrush::new();
        brush.set_color(Colors::YELLOW);
        assert_eq!(Some(String::from("SolidColorBrush: Yellow")), convert(Some(Rc::new(brush.clone()) as BoxedValue)));

        // As a binding reads it from a property of the brush contract.
        let contract: Rc<dyn IBrush> = brush.into();
        assert_eq!(Some(String::from("SolidColorBrush: Yellow")), convert(Some(Rc::new(contract) as BoxedValue)));
    }

    #[test]
    fn any_other_value_converts_to_null() {
        crate::register_types();
        assert_eq!(None, convert(None));
        assert_eq!(None, convert(Some(Rc::new(String::from("Yellow")) as BoxedValue)));
        assert_eq!(None, convert(Some(Rc::new(LinearGradientBrush::new()) as BoxedValue)));
    }

    #[test]
    fn the_converter_does_not_convert_back() {
        let converter = GenericValueConverter::<SolidColorBrush>::new();
        let value: BoxedValue = Rc::new(String::from("SolidColorBrush: Yellow"));
        let converted = converter.convert_back(Some(&value), ValueType::of::<String>(), None, &CultureInfo::invariant_culture());
        assert!(converted.is_err());
    }
}
