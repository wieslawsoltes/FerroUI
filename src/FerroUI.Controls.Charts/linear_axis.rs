use crate::{CartesianAxis, ChartError};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty, StyledPropertyOptions,
};

/// A finite numeric axis with independently automatic bounds and tick spacing.
#[repr(C)]
pub struct LinearAxis {
    base: CartesianAxis,
}

ferro_class!(LinearAxis: CartesianAxis);
ferro_class_info!(LinearAxis {
    new: LinearAxis::new
});
ferro_impl_classes!(LinearAxis: FerroObjectImpl);

ferro_properties! { impl LinearAxis {
    pub fn minimum_property() -> StyledProperty<Option<f64>> {
        FerroProperty::register_with::<LinearAxis, _>("Minimum",
            StyledPropertyOptions::new(None).coerce(Self::coerce_minimum))
    }
    pub fn maximum_property() -> StyledProperty<Option<f64>> {
        FerroProperty::register_with::<LinearAxis, _>("Maximum",
            StyledPropertyOptions::new(None).coerce(Self::coerce_maximum))
    }
    pub fn spacing_property() -> StyledProperty<Option<f64>> {
        FerroProperty::register_with::<LinearAxis, _>("Spacing",
            StyledPropertyOptions::new(None).coerce(Self::coerce_spacing))
    }
} }

impl LinearAxis {
    pub fn construct() -> Self {
        Self {
            base: CartesianAxis::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn minimum(&self) -> Option<f64> {
        self.get_value(Self::minimum_property())
    }

    /// Sets a finite lower bound, or `None` for automatic range calculation.
    /// A rejected change preserves every previous property value.
    pub fn set_minimum(&self, value: Option<f64>) -> Result<(), ChartError> {
        validate_range(value, self.maximum(), self.spacing())?;
        self.set_value(Self::minimum_property(), value);
        Ok(())
    }

    pub fn maximum(&self) -> Option<f64> {
        self.get_value(Self::maximum_property())
    }

    pub fn set_maximum(&self, value: Option<f64>) -> Result<(), ChartError> {
        validate_range(self.minimum(), value, self.spacing())?;
        self.set_value(Self::maximum_property(), value);
        Ok(())
    }

    pub fn spacing(&self) -> Option<f64> {
        self.get_value(Self::spacing_property())
    }

    pub fn set_spacing(&self, value: Option<f64>) -> Result<(), ChartError> {
        validate_range(self.minimum(), self.maximum(), value)?;
        self.set_value(Self::spacing_property(), value);
        Ok(())
    }

    // Deviation (DEVIATIONS.md, WinUI Charts): upstream rejects an invalid bound with an
    // exception; a write through the property system keeps the old value instead
    // (winui-charts.md section 4, item 2). The typed setters return `ChartError`.
    fn coerce_minimum(sender: &FerroObject, value: Option<f64>) -> Option<f64> {
        if validate_range(
            value,
            sender.get_value(Self::maximum_property()),
            sender.get_value(Self::spacing_property()),
        )
        .is_ok()
        {
            value
        } else {
            sender.get_value(Self::minimum_property())
        }
    }

    fn coerce_maximum(sender: &FerroObject, value: Option<f64>) -> Option<f64> {
        if validate_range(
            sender.get_value(Self::minimum_property()),
            value,
            sender.get_value(Self::spacing_property()),
        )
        .is_ok()
        {
            value
        } else {
            sender.get_value(Self::maximum_property())
        }
    }

    fn coerce_spacing(sender: &FerroObject, value: Option<f64>) -> Option<f64> {
        if validate_range(
            sender.get_value(Self::minimum_property()),
            sender.get_value(Self::maximum_property()),
            value,
        )
        .is_ok()
        {
            value
        } else {
            sender.get_value(Self::spacing_property())
        }
    }

    /// Resolves automatic bounds from the finite samples, ignoring missing
    /// (`NaN`) and infinite values. Does not mutate the public properties.
    pub fn resolve_range(&self, values: &[f64], target_tick_count: usize) -> NumericRange {
        NumericRange::from_values(
            values,
            self.minimum(),
            self.maximum(),
            self.spacing(),
            target_tick_count,
        )
    }
}

fn validate_range(
    minimum: Option<f64>,
    maximum: Option<f64>,
    spacing: Option<f64>,
) -> Result<(), ChartError> {
    if minimum.is_some_and(|value| !value.is_finite()) {
        return Err(ChartError::InvalidValue("Minimum must be finite."));
    }
    if maximum.is_some_and(|value| !value.is_finite()) {
        return Err(ChartError::InvalidValue("Maximum must be finite."));
    }
    if let (Some(minimum), Some(maximum)) = (minimum, maximum) {
        if minimum >= maximum {
            return Err(ChartError::InvalidValue(
                "Minimum must be less than Maximum.",
            ));
        }
        if spacing.is_some_and(|spacing| spacing > maximum - minimum) {
            return Err(ChartError::InvalidValue(
                "Spacing must not exceed the axis range.",
            ));
        }
    }
    if spacing.is_some_and(|value| !value.is_finite() || value <= 0.0) {
        return Err(ChartError::InvalidValue(
            "Spacing must be finite and greater than zero.",
        ));
    }
    Ok(())
}

/// Resolved layout data of a numeric axis. This is a value snapshot, separate
/// from the notifying axis object and safe to use during scene construction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericRange {
    pub minimum: f64,
    pub maximum: f64,
    pub spacing: f64,
}

impl NumericRange {
    /// The maximum number of ticks emitted by [`Self::ticks`]. Explicit
    /// subpixel spacing is sampled at an integer multiple of its interval.
    pub const MAX_TICKS: usize = 4096;

    /// Resolves bounds without requiring a property-system object.
    pub fn from_values(
        values: &[f64],
        minimum: Option<f64>,
        maximum: Option<f64>,
        spacing: Option<f64>,
        target_tick_count: usize,
    ) -> Self {
        let minimum = minimum.filter(|value| value.is_finite());
        let maximum = maximum.filter(|value| value.is_finite());
        let spacing = spacing.filter(|value| value.is_finite() && *value > 0.0);
        let mut data_min = f64::INFINITY;
        let mut data_max = f64::NEG_INFINITY;
        for &value in values.iter().filter(|value| value.is_finite()) {
            data_min = data_min.min(value);
            data_max = data_max.max(value);
        }
        if !data_min.is_finite() {
            data_min = 0.0;
            data_max = 1.0;
        }
        let mut low = minimum.unwrap_or(data_min);
        let mut high = maximum.unwrap_or(data_max);
        if low >= high {
            let anchor = minimum.or(maximum).unwrap_or(low);
            let delta = (anchor.abs() * 0.05).max(1.0);
            if minimum.is_none() {
                low = (anchor - delta).max(-f64::MAX);
            }
            if maximum.is_none() {
                high = (anchor + delta).min(f64::MAX);
            }
            // Values at a floating-point extremum have no finite neighbour
            // on one side. Use the nearest representable interval.
            if low >= high {
                if low == f64::MAX {
                    low = f64::MAX.next_down();
                    high = f64::MAX;
                } else if high == -f64::MAX {
                    low = -f64::MAX;
                    high = (-f64::MAX).next_up();
                } else {
                    high = low.next_up();
                }
            }
        }
        let target = target_tick_count.clamp(2, Self::MAX_TICKS) as f64;
        let span = high - low;
        let raw_step = if span.is_finite() {
            // Preserve subnormal ranges whose division would underflow, and
            // subtract before dividing to avoid cancellation near large bounds.
            let divided = span / target;
            if divided > 0.0 {
                divided
            } else {
                span
            }
        } else {
            // Dividing first handles a range from -MAX to MAX.
            high / target - low / target
        };
        let step = spacing.unwrap_or_else(|| nice_step(raw_step));
        if minimum.is_none() {
            let rounded = (low / step).floor() * step;
            if rounded.is_finite() {
                low = rounded;
            }
        }
        if maximum.is_none() {
            let rounded = (high / step).ceil() * step;
            if rounded.is_finite() {
                high = rounded;
            }
        }
        Self {
            minimum: low,
            maximum: high,
            spacing: step,
        }
    }

    /// Maps a value to axis coordinates without clipping to the axis domain.
    /// Scaled subtraction avoids overflow at opposite floating-point extrema.
    /// A finite value whose coordinate exceeds the representable range
    /// saturates to `-f64::MAX` or `f64::MAX`, retaining its side of the domain
    /// for line clipping. Non-finite sample values remain non-finite.
    pub fn normalize(&self, value: f64) -> f64 {
        let span = self.maximum - self.minimum;
        if span == 0.0 {
            return 0.5;
        }
        let normalized = if span.is_finite() {
            let difference = value - self.minimum;
            if difference.is_finite() {
                difference / span
            } else {
                // A finite span can still have an overflowing numerator when
                // the sample is very far outside the explicit axis interval.
                (value * 0.5 - self.minimum * 0.5) / (span * 0.5)
            }
        } else {
            (value * 0.5 - self.minimum * 0.5) / (self.maximum * 0.5 - self.minimum * 0.5)
        };
        if normalized.is_infinite()
            && value.is_finite()
            && self.minimum.is_finite()
            && self.maximum.is_finite()
        {
            normalized.signum() * f64::MAX
        } else {
            normalized
        }
    }

    /// Finite, strictly increasing major ticks, aligned to multiples of the
    /// chosen spacing. Work and allocation remain bounded for tiny spacing.
    pub fn ticks(&self) -> Vec<f64> {
        if !self.minimum.is_finite()
            || !self.maximum.is_finite()
            || self.minimum >= self.maximum
            || !self.spacing.is_finite()
            || self.spacing <= 0.0
        {
            return Vec::new();
        }
        let first = (self.minimum / self.spacing).ceil();
        let last = (self.maximum / self.spacing).floor();
        if !first.is_finite() || !last.is_finite() {
            return vec![self.minimum, self.maximum];
        }
        if first > last {
            return Vec::new();
        }
        let distance = last - first;
        let limit = (Self::MAX_TICKS - 1) as f64;
        let stride = if distance.is_finite() {
            distance / limit
        } else {
            last / limit - first / limit
        }
        .ceil()
        .max(1.0);
        let mut ticks = Vec::new();
        for index in 0..Self::MAX_TICKS {
            // Fused evaluation avoids overflowing an intermediate product
            // when the tick indices span both floating-point extrema.
            let number = stride.mul_add(index as f64, first);
            if number > last {
                break;
            }
            let tick = number * self.spacing;
            if tick.is_finite()
                && tick >= self.minimum
                && tick <= self.maximum
                && ticks.last().is_none_or(|&previous| tick > previous)
            {
                ticks.push(if tick == 0.0 { 0.0 } else { tick });
            }
        }
        ticks
    }
}

fn nice_step(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return f64::MIN_POSITIVE;
    }
    let magnitude = 10.0_f64.powf(raw.log10().floor());
    if magnitude == 0.0 {
        return raw;
    }
    let fraction = raw / magnitude;
    let multiple = if fraction <= 1.0 {
        1.0
    } else if fraction <= 2.0 {
        2.0
    } else if fraction <= 5.0 {
        5.0
    } else {
        10.0
    };
    let result = multiple * magnitude;
    if result.is_finite() && result > 0.0 {
        result
    } else {
        raw
    }
}
