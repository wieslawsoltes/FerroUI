use super::Corners;
use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::{BoxedValue, CornerRadius, FerroProperty};
use std::cell::Cell;
use std::rc::Rc;

/// Converts one corner of a corner radius to a double.
#[derive(Default)]
pub struct CornerRadiusToDoubleConverter {
    corner: Cell<Corners>,
}

impl CornerRadiusToDoubleConverter {
    pub fn new() -> Self {
        Self { corner: Cell::new(Corners::NONE) }
    }

    /// Gets the specific corner of the corner radius to convert to a double.
    pub fn corner(&self) -> Corners {
        self.corner.get()
    }

    /// Sets the specific corner of the corner radius to convert to a double.
    pub fn set_corner(&self, value: Corners) {
        self.corner.set(value)
    }
}

impl IValueConverter for CornerRadiusToDoubleConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(corner_radius) = cast_value::<CornerRadius>(value) else {
            return Ok(Some(FerroProperty::unset_value()));
        };

        let corner = self.corner.get();
        let result = if corner == Corners::TOP_LEFT {
            corner_radius.top_left
        } else if corner == Corners::TOP_RIGHT {
            corner_radius.top_right
        } else if corner == Corners::BOTTOM_RIGHT {
            corner_radius.bottom_right
        } else if corner == Corners::BOTTOM_LEFT {
            corner_radius.bottom_left
        } else {
            0.0
        };

        Ok(Some(Rc::new(result)))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}
