//! Port of `Converter/GdpToBrushConverter.cs`.

use ferroui_base::utilities::CultureInfo;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Colors, IBrush};
use ferroui_base::{ferro_markup_type, BoxedValue, FerroProperty};
use std::rc::{Rc, Weak};

pub struct GdpToBrushConverter {
    this: Weak<GdpToBrushConverter>,
    orange_brush: Rc<dyn IBrush>,
    yellow_brush: Rc<dyn IBrush>,
    green_brush: Rc<dyn IBrush>,
}

impl PartialEq for GdpToBrushConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl GdpToBrushConverter {
    pub fn new() -> Rc<GdpToBrushConverter> {
        Rc::new_cyclic(|this| Self { this: this.clone(),
            orange_brush: Rc::new(ImmutableSolidColorBrush::with_opacity(Colors::ORANGE, 0.6)),
            yellow_brush: Rc::new(ImmutableSolidColorBrush::with_opacity(Colors::YELLOW, 0.6)),
            green_brush: Rc::new(ImmutableSolidColorBrush::with_opacity(Colors::LIGHT_GREEN, 0.6)),
        })
    }

    /// The converter as the value converter contract.
    pub fn as_value_converter(&self) -> Rc<dyn IValueConverter> {
        self.this.upgrade().expect("the converter is alive while it is used")
    }
}

impl IValueConverter for GdpToBrushConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(gdp) = value.and_then(|value| value.downcast_ref::<i32>()).copied() else {
            return Ok(Some(FerroProperty::unset_value()));
        };

        let brush = if gdp <= 5000 {
            &self.orange_brush
        } else if gdp <= 10000 {
            &self.yellow_brush
        } else {
            &self.green_brush
        };
        Ok(Some(Rc::new(brush.clone())))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("Specified method is not supported."))
    }
}

ferro_markup_type!(class GdpToBrushConverter {
    this: Rc<GdpToBrushConverter>,
    handles: [GdpToBrushConverter, Rc<GdpToBrushConverter>, Option<Rc<GdpToBrushConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => GdpToBrushConverter::new],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
        Rc::new(value)
    }

    fn brush(converter: &GdpToBrushConverter, gdp: i32) -> Rc<dyn IBrush> {
        let value = converter.convert(Some(&boxed(gdp)), ValueType::of::<Rc<dyn IBrush>>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture());
        value.ok().flatten().expect("a value").downcast_ref::<Rc<dyn IBrush>>().expect("a brush").clone()
    }

    #[test]
    fn the_brush_follows_the_thresholds() {
        let converter = GdpToBrushConverter::new();
        assert!(Rc::ptr_eq(&converter.orange_brush, &brush(&converter, 5000)));
        assert!(Rc::ptr_eq(&converter.yellow_brush, &brush(&converter, 5001)));
        assert!(Rc::ptr_eq(&converter.yellow_brush, &brush(&converter, 10000)));
        assert!(Rc::ptr_eq(&converter.green_brush, &brush(&converter, 10001)));
        assert_eq!(0.6, brush(&converter, 1).opacity());
    }

    #[test]
    fn a_value_that_is_not_an_integer_is_unset_and_the_way_back_is_not_supported() {
        let converter = GdpToBrushConverter::new();
        let target = ValueType::of::<Rc<dyn IBrush>>();
        let value = converter.convert(Some(&boxed(1.0_f64)), target, None, &ferroui_base::utilities::CultureInfo::invariant_culture()).ok().flatten().expect("a value");
        assert!(Rc::ptr_eq(&value, &FerroProperty::unset_value()));
        assert!(converter.convert_back(Some(&boxed(1_i32)), target, None, &ferroui_base::utilities::CultureInfo::invariant_culture()).is_err());
    }
}
