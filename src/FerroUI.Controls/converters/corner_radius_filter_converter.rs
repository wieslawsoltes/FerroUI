use ferroui_base::utilities::CultureInfo;
use super::Corners;
use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::{BoxedValue, CornerRadius};
use std::cell::Cell;
use std::rc::Rc;

/// Converts a corner radius by filtering parts of it, and/or scaling it.
pub struct CornerRadiusFilterConverter {
    filter: Cell<Corners>,
    scale: Cell<f64>,
}

impl Default for CornerRadiusFilterConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl CornerRadiusFilterConverter {
    pub fn new() -> Self {
        Self { filter: Cell::new(Corners::NONE), scale: Cell::new(1.0) }
    }

    /// Gets the corners to filter by. Only the specified corners will be
    /// included in the converted corner radius.
    pub fn filter(&self) -> Corners {
        self.filter.get()
    }

    /// Sets the corners to filter by. Only the specified corners will be
    /// included in the converted corner radius.
    pub fn set_filter(&self, value: Corners) {
        self.filter.set(value)
    }

    /// Gets the scale multiplier applied uniformly to each corner.
    pub fn scale(&self) -> f64 {
        self.scale.get()
    }

    /// Sets the scale multiplier applied uniformly to each corner.
    pub fn set_scale(&self, value: f64) {
        self.scale.set(value)
    }
}

impl IValueConverter for CornerRadiusFilterConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(radius) = cast_value::<CornerRadius>(value) else {
            return Ok(value.cloned());
        };

        let filter = self.filter.get();
        let scale = self.scale.get();

        Ok(Some(Rc::new(CornerRadius::new(
            if filter.contains(Corners::TOP_LEFT) { radius.top_left * scale } else { 0.0 },
            if filter.contains(Corners::TOP_RIGHT) { radius.top_right * scale } else { 0.0 },
            if filter.contains(Corners::BOTTOM_RIGHT) { radius.bottom_right * scale } else { 0.0 },
            if filter.contains(Corners::BOTTOM_LEFT) { radius.bottom_left * scale } else { 0.0 },
        ))))
    }

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
