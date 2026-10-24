//! Port of `Diagnostics/Converters/BoolToOpacityConverter.cs`.

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::Cell;
use std::rc::Rc;

/// Converts `true` to an opacity of 1 and anything else to [`opacity`](Self::opacity).
#[derive(Default)]
pub struct BoolToOpacityConverter {
    opacity: Cell<f64>,
}

impl PartialEq for BoolToOpacityConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl BoolToOpacityConverter {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn opacity(&self) -> f64 {
        self.opacity.get()
    }

    pub fn set_opacity(&self, value: f64) {
        self.opacity.set(value);
    }
}

impl IValueConverter for BoolToOpacityConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if value.and_then(|value| value.downcast_ref::<bool>()) == Some(&true) {
            return Ok(Some(Rc::new(1f64)));
        }

        Ok(Some(Rc::new(self.opacity())))
    }

    /// Fails: the conversion has no inverse (`NotImplementedException`).
    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}

ferro_markup_type!(class BoolToOpacityConverter {
    handles: [Rc<BoolToOpacityConverter>, Option<Rc<BoolToOpacityConverter>>],
    this: Rc<BoolToOpacityConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => BoolToOpacityConverter::new],
    properties: [
        Opacity: f64 {
            get: |c: &Rc<BoolToOpacityConverter>| c.opacity(),
            set: |c: &Rc<BoolToOpacityConverter>, value: f64| c.set_opacity(value)
        },
    ],
});
