// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::{converters::IMultiValueConverter, core::ValueType, BindingError},
    metadata::MarkupTyped,
    utilities::{
        span_helpers::{try_parse_double, NumberStyles},
        CultureInfo,
    },
    BoxedValue,
};
use ferroui_controls::primitives::ScrollBarVisibility;
use std::rc::Rc;
/// Original scroll-button visibility converter.
pub struct ScrollViewerScrollButtonVisibilityConverter;
impl PartialEq for ScrollViewerScrollButtonVisibilityConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ScrollViewerScrollButtonVisibilityConverter {
    /// Creates the source stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared instance on the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<ScrollViewerScrollButtonVisibilityConverter>=ScrollViewerScrollButtonVisibilityConverter::new();}
        INSTANCE.with(Rc::clone)
    }
    fn parse_scroll_bar_visibility(value: Option<&BoxedValue>) -> i32 {
        if let Some(visibility) = value.and_then(|v| v.downcast_ref::<ScrollBarVisibility>()) {
            return *visibility as i32;
        }
        if let Some(text) = value.and_then(|v| v.downcast_ref::<String>()) {
            let text = text.trim();
            if let Ok(value) = text.parse::<i32>() {
                return value;
            }
            let metadata = ScrollBarVisibility::MARKUP;
            let mut result = 0_i32;
            for name in text.split(',') {
                let Some(member) = metadata.find_enum_member(name.trim(), true) else {
                    return ScrollBarVisibility::Auto as i32;
                };
                result |= member.value as i32;
            }
            return result;
        }
        ScrollBarVisibility::Auto as i32
    }
    fn try_get_double(value: Option<&BoxedValue>) -> Option<f64> {
        let value = value?;
        let number = if let Some(value) = value.downcast_ref::<f64>() {
            *value
        } else if let Some(value) = value.downcast_ref::<f32>() {
            f64::from(*value)
        } else if let Some(value) = value.downcast_ref::<i32>() {
            f64::from(*value)
        } else {
            let value = value.downcast_ref::<String>()?;
            try_parse_double(value, NumberStyles::FLOAT)?
        };
        number.is_finite().then_some(number)
    }
    fn is_left(parameter: Option<&BoxedValue>) -> bool {
        let Some(parameter) = parameter else {
            return false;
        };
        if let Some(text) = parameter.downcast_ref::<String>() {
            if text.eq_ignore_ascii_case("left") {
                return true;
            }
            if text.eq_ignore_ascii_case("right") {
                return false;
            }
            if text.eq_ignore_ascii_case("up") {
                return true;
            }
            if text.eq_ignore_ascii_case("down") {
                return false;
            }
            if let Some(value) = try_parse_double(text, NumberStyles::FLOAT) {
                return value <= 0.0;
            }
        }
        if let Some(value) = parameter.downcast_ref::<i32>() {
            return *value <= 0;
        }
        if let Some(value) = parameter.downcast_ref::<f64>() {
            return *value <= 0.0;
        }
        false
    }
}
impl IMultiValueConverter for ScrollViewerScrollButtonVisibilityConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if values.len() < 4 {
            return Ok(Some(Rc::new(false)));
        }
        let visibility = Self::parse_scroll_bar_visibility(values[0].as_ref());
        if visibility == ScrollBarVisibility::Disabled as i32
            || visibility == ScrollBarVisibility::Hidden as i32
        {
            return Ok(Some(Rc::new(false)));
        }
        let Some((offset, extent, viewport)) = Self::try_get_double(values[1].as_ref())
            .zip(Self::try_get_double(values[2].as_ref()))
            .zip(Self::try_get_double(values[3].as_ref()))
            .map(|((o, e), v)| (o, e, v))
        else {
            return Ok(Some(Rc::new(false)));
        };
        if extent <= 0.0 || viewport <= 0.0 {
            return Ok(Some(Rc::new(false)));
        }
        let max_offset = extent - viewport;
        if max_offset <= 0.0 {
            return Ok(Some(Rc::new(false)));
        }
        Ok(Some(Rc::new(if Self::is_left(parameter) {
            offset > 0.0
        } else {
            offset < max_offset
        })))
    }
}
ferroui_base::ferro_markup_type!(class ScrollViewerScrollButtonVisibilityConverter {
    namespace:"FerroDock.Ferro.Converters",this:Rc<ScrollViewerScrollButtonVisibilityConverter>,handles:[Rc<ScrollViewerScrollButtonVisibilityConverter>,Option<Rc<ScrollViewerScrollButtonVisibilityConverter>>],interfaces:[Rc<dyn IMultiValueConverter>],constructors:[()=>ScrollViewerScrollButtonVisibilityConverter::new],static_properties:[Instance:Rc<ScrollViewerScrollButtonVisibilityConverter>{get:ScrollViewerScrollButtonVisibilityConverter::instance}],
});

#[cfg(test)]
mod tests {
    use super::*;
    fn convert(
        visibility: Option<BoxedValue>,
        offset: Option<BoxedValue>,
        extent: Option<BoxedValue>,
        viewport: Option<BoxedValue>,
        parameter: Option<BoxedValue>,
    ) -> bool {
        *ScrollViewerScrollButtonVisibilityConverter::new()
            .convert(
                &[visibility, offset, extent, viewport],
                ValueType::of::<bool>(),
                parameter.as_ref(),
                &CultureInfo::get_culture_info("fr-FR"),
            )
            .unwrap()
            .unwrap()
            .downcast_ref::<bool>()
            .unwrap()
    }
    #[test]
    fn original_visibility_names_numeric_flags_unknown_values_and_fallbacks() {
        for (visibility, expected) in [
            (None, true),
            (Some(Rc::new(ScrollBarVisibility::Auto) as BoxedValue), true),
            (Some(Rc::new(ScrollBarVisibility::Disabled)), false),
            (Some(Rc::new(ScrollBarVisibility::Hidden)), false),
            (Some(Rc::new(ScrollBarVisibility::Visible)), true),
            (Some(Rc::new("hidden".to_owned())), false),
            (Some(Rc::new(" 0 ".to_owned())), false),
            (Some(Rc::new("Disabled, Hidden".to_owned())), false),
            (Some(Rc::new("Auto,Hidden".to_owned())), true),
            (Some(Rc::new("999".to_owned())), true),
            (Some(Rc::new("invalid".to_owned())), true),
            (Some(Rc::new(0_i32)), true),
        ] {
            assert_eq!(
                convert(
                    visibility,
                    Some(Rc::new(10.0)),
                    Some(Rc::new(100.0)),
                    Some(Rc::new(20.0)),
                    None
                ),
                expected
            );
        }
    }
    #[test]
    fn original_numeric_inputs_use_invariant_float_syntax_and_reject_nonfinite_and_wrong_types() {
        for (value, expected) in [
            (None, false),
            (Some(Rc::new(10.0_f64) as BoxedValue), true),
            (Some(Rc::new(10.0_f32)), true),
            (Some(Rc::new(10_i32)), true),
            (Some(Rc::new(" +1e1 ".to_owned())), true),
            (Some(Rc::new("10,0".to_owned())), false),
            (Some(Rc::new("1,000".to_owned())), false),
            (Some(Rc::new(10_i64)), false),
            (Some(Rc::new(f64::NAN)), false),
            (Some(Rc::new(f64::INFINITY)), false),
            (Some(Rc::new("1e999".to_owned())), false),
            (Some(Rc::new(true)), false),
        ] {
            assert_eq!(
                convert(None, value, Some(Rc::new(100.0)), Some(Rc::new(20.0)), None),
                expected
            );
        }
        for (extent, viewport) in [
            (0.0, 1.0),
            (100.0, 0.0),
            (20.0, 20.0),
            (10.0, 20.0),
            (-1.0, 10.0),
        ] {
            assert!(!convert(
                None,
                Some(Rc::new(0.0)),
                Some(Rc::new(extent)),
                Some(Rc::new(viewport)),
                None
            ));
        }
    }
    #[test]
    fn original_direction_uses_untrimmed_names_or_invariant_numeric_parameters_and_strict_endpoints(
    ) {
        for (parameter, left) in [
            (None, false),
            (Some(Rc::new("LeFt".to_owned()) as BoxedValue), true),
            (Some(Rc::new("UP".to_owned())), true),
            (Some(Rc::new("RIGHT".to_owned())), false),
            (Some(Rc::new("DOWN".to_owned())), false),
            (Some(Rc::new(" left ".to_owned())), false),
            (Some(Rc::new(" -0 ".to_owned())), true),
            (Some(Rc::new("-Infinity".to_owned())), true),
            (Some(Rc::new("NaN".to_owned())), false),
            (Some(Rc::new(-1_i32)), true),
            (Some(Rc::new(-0.0_f64)), true),
            (Some(Rc::new(-1.0_f32)), false),
        ] {
            assert_eq!(
                convert(
                    None,
                    Some(Rc::new(0.0)),
                    Some(Rc::new(100.0)),
                    Some(Rc::new(20.0)),
                    parameter.clone()
                ),
                !left
            );
            assert_eq!(
                convert(
                    None,
                    Some(Rc::new(80.0)),
                    Some(Rc::new(100.0)),
                    Some(Rc::new(20.0)),
                    parameter
                ),
                left
            );
        }
        assert!(
            ScrollViewerScrollButtonVisibilityConverter::new()
                .convert(
                    &[],
                    ValueType::of::<bool>(),
                    None,
                    &CultureInfo::invariant_culture()
                )
                .unwrap()
                .unwrap()
                .downcast_ref::<bool>()
                == Some(&false)
        );
    }
}
