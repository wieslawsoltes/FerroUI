use ferroui_base::utilities::CultureInfo;
use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::{BoxedValue, Thickness};
use std::cell::Cell;
use std::rc::Rc;

/// Converts a depth (a number of levels, or a thickness) to a margin by
/// multiplying it with an indent on the chosen sides.
#[derive(Default)]
pub struct MarginMultiplierConverter {
    indent: Cell<f64>,
    left: Cell<bool>,
    top: Cell<bool>,
    right: Cell<bool>,
    bottom: Cell<bool>,
}

impl MarginMultiplierConverter {
    pub fn new() -> Self {
        Self::default()
    }

    /// The indent per level.
    pub fn indent(&self) -> f64 {
        self.indent.get()
    }

    pub fn set_indent(&self, value: f64) {
        self.indent.set(value)
    }

    /// Whether the left side of the margin is indented.
    pub fn left(&self) -> bool {
        self.left.get()
    }

    pub fn set_left(&self, value: bool) {
        self.left.set(value)
    }

    /// Whether the top side of the margin is indented.
    pub fn top(&self) -> bool {
        self.top.get()
    }

    pub fn set_top(&self, value: bool) {
        self.top.set(value)
    }

    /// Whether the right side of the margin is indented.
    pub fn right(&self) -> bool {
        self.right.get()
    }

    pub fn set_right(&self, value: bool) {
        self.right.set(value)
    }

    /// Whether the bottom side of the margin is indented.
    pub fn bottom(&self) -> bool {
        self.bottom.get()
    }

    pub fn set_bottom(&self, value: bool) {
        self.bottom.set(value)
    }
}

impl IValueConverter for MarginMultiplierConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let indent = self.indent.get();

        let result = if let Some(scalar_depth) = cast_value::<i32>(value) {
            let scalar_depth = f64::from(scalar_depth);
            Thickness::new(
                if self.left.get() { indent * scalar_depth } else { 0.0 },
                if self.top.get() { indent * scalar_depth } else { 0.0 },
                if self.right.get() { indent * scalar_depth } else { 0.0 },
                if self.bottom.get() { indent * scalar_depth } else { 0.0 },
            )
        } else if let Some(thickness_depth) = cast_value::<Thickness>(value) {
            Thickness::new(
                if self.left.get() { indent * thickness_depth.left } else { 0.0 },
                if self.top.get() { indent * thickness_depth.top } else { 0.0 },
                if self.right.get() { indent * thickness_depth.right } else { 0.0 },
                if self.bottom.get() { indent * thickness_depth.bottom } else { 0.0 },
            )
        } else {
            Thickness::uniform(0.0)
        };

        Ok(Some(Rc::new(result)))
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
