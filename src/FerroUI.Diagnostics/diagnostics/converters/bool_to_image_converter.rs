//! Port of `Diagnostics/Converters/BoolToImageConverter.cs`.

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::{BindingError, BindingOperations};
use ferroui_base::media::IImage;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Converts `true` to [`true_image`](Self::true_image), `false` to
/// [`false_image`](Self::false_image) and anything else to null.
#[derive(Default)]
pub struct BoolToImageConverter {
    true_image: RefCell<Option<Rc<dyn IImage>>>,
    false_image: RefCell<Option<Rc<dyn IImage>>>,
}

impl PartialEq for BoolToImageConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl BoolToImageConverter {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn true_image(&self) -> Option<Rc<dyn IImage>> {
        self.true_image.borrow().clone()
    }

    pub fn set_true_image(&self, value: Option<Rc<dyn IImage>>) {
        *self.true_image.borrow_mut() = value;
    }

    pub fn false_image(&self) -> Option<Rc<dyn IImage>> {
        self.false_image.borrow().clone()
    }

    pub fn set_false_image(&self, value: Option<Rc<dyn IImage>>) {
        *self.false_image.borrow_mut() = value;
    }
}

impl IValueConverter for BoolToImageConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let image = match value.and_then(|value| value.downcast_ref::<bool>()) {
            Some(true) => self.true_image(),
            Some(false) => self.false_image(),
            None => None,
        };

        Ok(image.map(|image| Rc::new(image) as BoxedValue))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(BindingOperations::do_nothing()))
    }
}

ferro_markup_type!(class BoolToImageConverter {
    handles: [Rc<BoolToImageConverter>, Option<Rc<BoolToImageConverter>>],
    this: Rc<BoolToImageConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => BoolToImageConverter::new],
    properties: [
        TrueImage: Option<Rc<dyn IImage>> {
            get: |c: &Rc<BoolToImageConverter>| c.true_image(),
            set: |c: &Rc<BoolToImageConverter>, value: Option<Rc<dyn IImage>>| c.set_true_image(value)
        },
        FalseImage: Option<Rc<dyn IImage>> {
            get: |c: &Rc<BoolToImageConverter>| c.false_image(),
            set: |c: &Rc<BoolToImageConverter>, value: Option<Rc<dyn IImage>>| c.set_false_image(value)
        },
    ],
});
