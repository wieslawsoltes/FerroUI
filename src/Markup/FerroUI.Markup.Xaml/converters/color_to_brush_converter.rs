//! Port of `Converters/ColorToBrushConverter.cs`.

use ferroui_base::utilities::CultureInfo;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::rc::Rc;

/// Converts a color to a brush when a brush is asked for: what lets a
/// color resource be used where a brush is expected.
pub struct ColorToBrushConverter;

fn is_brush_type(target_type: ValueType) -> bool {
    target_type.is::<Rc<dyn IBrush>>() || target_type.is::<Option<Rc<dyn IBrush>>>()
}

fn is_color_type(target_type: ValueType) -> bool {
    target_type.is::<Color>() || target_type.is::<Option<Color>>()
}

impl ColorToBrushConverter {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }

    /// If `target_type` is the brush type and `value` is a color: a solid
    /// color brush of that color. Otherwise the value unchanged.
    pub fn convert_to(value: Option<BoxedValue>, target_type: Option<ValueType>) -> Option<BoxedValue> {
        if target_type.is_some_and(is_brush_type) {
            if let Some(c) = value.as_ref().and_then(|v| v.downcast_ref::<Color>()) {
                let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(*c));
                return Some(Rc::new(brush));
            }
        }

        value
    }

    /// If `target_type` is the color type and `value` is a solid color
    /// brush: the color of the brush. Otherwise the value unchanged.
    pub fn convert_back_to(value: Option<BoxedValue>, target_type: Option<ValueType>) -> Option<BoxedValue> {
        if target_type.is_some_and(is_color_type) {
            if let Some(brush) = from_markup_value::<Rc<dyn IBrush>>(&value) {
                if let Some(brush) = brush.as_solid_color_brush() {
                    return Some(Rc::new(brush.color()));
                }
            }
        }

        value
    }
}

impl IValueConverter for ColorToBrushConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Self::convert_to(value.cloned(), Some(target_type)))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Self::convert_back_to(value.cloned(), Some(target_type)))
    }
}

crate::identity_eq!(ColorToBrushConverter);

ferro_markup_type!(class ColorToBrushConverter {
    this: Rc<ColorToBrushConverter>,
    handles: [ColorToBrushConverter, Rc<ColorToBrushConverter>, Option<Rc<ColorToBrushConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => ColorToBrushConverter::new],
    methods: [
        static fn Convert(Option<BoxedValue>, Option<ValueType>) -> Option<BoxedValue> => ColorToBrushConverter::convert_to,
        static fn ConvertBack(Option<BoxedValue>, Option<ValueType>) -> Option<BoxedValue> =>
            ColorToBrushConverter::convert_back_to,
    ],
});
