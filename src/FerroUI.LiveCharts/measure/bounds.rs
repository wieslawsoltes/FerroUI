/// Extrema and spacing metadata accumulated while measuring a series.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub is_empty: bool,
    pub min: f64,
    pub max: f64,
    pub padding_min: f64,
    pub padding_max: f64,
    pub requested_geometry_size: f64,
    pub min_delta: f64,
}

impl Default for Bounds {
    fn default() -> Self {
        Self {
            is_empty: true,
            min: f64::MAX,
            max: f64::MIN,
            padding_min: 0.0,
            padding_max: 0.0,
            requested_geometry_size: 0.0,
            min_delta: f64::MAX,
        }
    }
}

impl Bounds {
    /// Initializes the extrema; emptiness remains set until data is appended,
    /// matching the source constructor's distinction between limits and data.
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            ..Self::default()
        }
    }
    pub fn delta(&self) -> f64 {
        self.max - self.min
    }

    pub fn append_value(&mut self, value: f64) {
        if self.max <= value {
            self.max = value;
        }
        if self.min >= value {
            self.min = value;
        }
        self.is_empty = false;
    }

    pub fn append_bounds(&mut self, other: &Self) {
        if self.max < other.max {
            self.max = other.max;
        }
        if self.min > other.min {
            self.min = other.min;
        }
        if self.min_delta > other.min_delta {
            self.min_delta = other.min_delta;
        }
        if self.requested_geometry_size < other.requested_geometry_size {
            self.requested_geometry_size = other.requested_geometry_size;
        }
        if self.padding_min < other.padding_min {
            self.padding_min = other.padding_min;
        }
        if self.padding_max < other.padding_max {
            self.padding_max = other.padding_max;
        }
        self.is_empty = false;
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DimensionalBounds {
    pub primary_bounds: Bounds,
    pub secondary_bounds: Bounds,
    pub tertiary_bounds: Bounds,
    pub visible_primary_bounds: Bounds,
    pub visible_secondary_bounds: Bounds,
    pub visible_tertiary_bounds: Bounds,
    pub is_empty: bool,
}

impl DimensionalBounds {
    pub fn with_default_limits() -> Self {
        let mut axis = Bounds::default();
        axis.append_value(0.0);
        axis.append_value(10.0);
        let mut weight = Bounds::default();
        weight.append_value(1.0);
        Self {
            primary_bounds: axis,
            secondary_bounds: axis,
            tertiary_bounds: weight,
            visible_primary_bounds: axis,
            visible_secondary_bounds: axis,
            visible_tertiary_bounds: weight,
            is_empty: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_default_is_empty() {
        let b = Bounds::default();
        assert!(b.is_empty);
        assert_eq!(b.min, f64::MAX);
        assert_eq!(b.max, f64::MIN);
    }

    #[test]
    fn bounds_constructor_and_copy() {
        let b = Bounds::new(10.0, 100.0);
        assert_eq!((b.min, b.max), (10.0, 100.0));
        assert_eq!(b, b.clone());
        let mut original = Bounds::new(5.0, 50.0);
        original.append_value(10.0);
        original.append_value(20.0);
        let copied = original;
        assert_eq!(
            (copied.min, copied.max, copied.is_empty, copied.min_delta),
            (
                original.min,
                original.max,
                original.is_empty,
                original.min_delta
            )
        );
    }

    #[test]
    fn bounds_append_values_and_delta() {
        let mut b = Bounds::default();
        for x in [5.0, 15.0, 10.0] {
            b.append_value(x);
        }
        assert_eq!(
            (b.min, b.max, b.delta(), b.is_empty),
            (5.0, 15.0, 10.0, false)
        );
        b.append_value(-20.0);
        assert_eq!(b.min, -20.0);
        let mut delta = Bounds::default();
        delta.append_value(10.0);
        delta.append_value(30.0);
        assert_eq!(delta.delta(), 20.0);
        let mut negative = Bounds::default();
        for value in [-10.0, -5.0, -20.0] {
            negative.append_value(value);
        }
        assert_eq!((negative.min, negative.max), (-20.0, -5.0));
    }

    #[test]
    fn bounds_single_value_and_merge_metadata() {
        let mut b = Bounds::default();
        b.append_value(42.0);
        assert_eq!((b.min, b.max), (42.0, 42.0));
        assert!(!b.is_empty);
        let mut c = Bounds::new(3.0, 100.0);
        c.padding_min = 2.0;
        c.padding_max = 3.0;
        c.min_delta = 0.1;
        c.requested_geometry_size = 9.0;
        b.append_bounds(&c);
        assert_eq!(
            (
                b.min,
                b.max,
                b.padding_min,
                b.padding_max,
                b.min_delta,
                b.requested_geometry_size
            ),
            (3.0, 100.0, 2.0, 3.0, 0.1, 9.0)
        );
        let mut first = Bounds::default();
        first.append_value(5.0);
        first.append_value(15.0);
        let mut second = Bounds::default();
        second.append_value(3.0);
        second.append_value(20.0);
        first.append_bounds(&second);
        assert_eq!((first.min, first.max), (3.0, 20.0));
    }

    #[test]
    fn dimensional_bounds_defaults() {
        assert!(DimensionalBounds::default().primary_bounds.is_empty);
        let b = DimensionalBounds::with_default_limits();
        assert!(b.is_empty);
        assert_eq!(b.primary_bounds.delta(), 10.0);
        assert_eq!(b.tertiary_bounds.min, 1.0);
    }
}
