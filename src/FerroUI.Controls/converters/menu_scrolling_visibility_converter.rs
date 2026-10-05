use crate::primitives::ScrollBarVisibility;
use ferroui_base::data::converters::{cast_value, IMultiValueConverter};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{BoxedValue, FerroProperty};
use std::rc::Rc;

/// Decides whether a scroll button of a scrolling menu is shown: the values
/// are the scroll bar visibility, the offset, the extent and the viewport;
/// the parameter is the percentage of the scroll range at which the button
/// is no longer needed.
#[derive(Default)]
pub struct MenuScrollingVisibilityConverter;

impl MenuScrollingVisibilityConverter {
    /// The shared instance of the converter.
    pub fn instance() -> Rc<MenuScrollingVisibilityConverter> {
        thread_local! {
            static INSTANCE: Rc<MenuScrollingVisibilityConverter> = Rc::new(MenuScrollingVisibilityConverter);
        }
        INSTANCE.with(Rc::clone)
    }
}

impl IMultiValueConverter for MenuScrollingVisibilityConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if parameter.cloned().and_then(ValueTypes::normalize).is_none() || values.len() != 4 {
            return Ok(Some(FerroProperty::unset_value()));
        }

        let (Some(visibility), Some(offset), Some(extent), Some(viewport)) = (
            cast_value::<ScrollBarVisibility>(values[0].as_ref()),
            cast_value::<f64>(values[1].as_ref()),
            cast_value::<f64>(values[2].as_ref()),
            cast_value::<f64>(values[3].as_ref()),
        ) else {
            return Ok(Some(FerroProperty::unset_value()));
        };

        if visibility == ScrollBarVisibility::Auto {
            if MathUtilities::are_close(extent, viewport) {
                return Ok(Some(Rc::new(false)));
            }

            let target = if let Some(d) = cast_value::<f64>(parameter) {
                d
            } else if let Some(s) = cast_value::<String>(parameter) {
                s.trim()
                    .parse::<f64>()
                    .map_err(|_| BindingError::message(format!("The input string '{s}' was not in a correct format.")))?
            } else {
                return Ok(Some(FerroProperty::unset_value()));
            };

            // Calculate the percent so that we can see if we are near the
            // edge of the range.
            let percent = MathUtilities::clamp(offset * 100.0 / (extent - viewport), 0.0, 100.0);

            if MathUtilities::are_close(percent, target) {
                // We are at the end of the range, so no need for this button
                // to be shown.
                return Ok(Some(Rc::new(false)));
            }

            return Ok(Some(Rc::new(true)));
        }

        if visibility == ScrollBarVisibility::Visible {
            return Ok(Some(Rc::new(true)));
        }

        Ok(Some(Rc::new(false)))
    }
}
