use super::Error;
use std::fmt;

/// A mapped chart datum. The primary dimension is normally Y, secondary X.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coordinate {
    pub primary_value: f64,
    pub secondary_value: f64,
    pub tertiary_value: f64,
    pub quaternary_value: f64,
    pub quinary_value: f64,
    pub senary_value: f64,
    pub point_error: Error,
    pub is_empty: bool,
}

impl Coordinate {
    pub const EMPTY: Self = Self {
        is_empty: true,
        ..Self::new(0.0, 0.0)
    };

    pub const fn from_values(
        primary: f64,
        secondary: f64,
        tertiary: f64,
        quaternary: f64,
        quinary: f64,
        senary: f64,
        error: Error,
    ) -> Self {
        Self {
            primary_value: primary,
            secondary_value: secondary,
            tertiary_value: tertiary,
            quaternary_value: quaternary,
            quinary_value: quinary,
            senary_value: senary,
            point_error: error,
            is_empty: false,
        }
    }

    pub const fn new(x: f64, y: f64) -> Self {
        Self::from_values(y, x, 0.0, 0.0, 0.0, 0.0, Error::EMPTY)
    }
    pub const fn weighted(x: f64, y: f64, weight: f64) -> Self {
        Self::from_values(y, x, weight, 0.0, 0.0, 0.0, Error::EMPTY)
    }
    pub const fn financial(x: f64, high: f64, open: f64, close: f64, low: f64) -> Self {
        Self::from_values(high, x, open, close, low, 0.0, Error::EMPTY)
    }
    pub const fn box_value(
        x: f64,
        maximum: f64,
        third_quartile: f64,
        first_quartile: f64,
        minimum: f64,
        median: f64,
    ) -> Self {
        Self::from_values(
            maximum,
            x,
            third_quartile,
            first_quartile,
            minimum,
            median,
            Error::EMPTY,
        )
    }
    pub const fn range(x: f64, high: f64, low: f64) -> Self {
        Self::from_values(high, x, low, 0.0, 0.0, 0.0, Error::EMPTY)
    }
    pub const fn with_error(mut self, error: Error) -> Self {
        self.point_error = error;
        self
    }
    pub fn is_finite(self) -> bool {
        !self.is_empty && self.primary_value.is_finite() && self.secondary_value.is_finite()
    }
}

impl Default for Coordinate {
    fn default() -> Self {
        Self::from_values(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, Error::default())
    }
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.secondary_value, self.primary_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_constructor_dimension_order_matches_source() {
        let c = Coordinate::new(10.0, 20.0);
        assert_eq!(
            (c.secondary_value, c.primary_value, c.is_empty),
            (10.0, 20.0, false)
        );
        let c = Coordinate::weighted(10.0, 20.0, 5.0);
        assert_eq!(
            (
                c.secondary_value,
                c.primary_value,
                c.tertiary_value,
                c.is_empty
            ),
            (10.0, 20.0, 5.0, false)
        );
        let c = Coordinate::financial(100.0, 150.0, 110.0, 140.0, 90.0);
        assert_eq!(
            (
                c.primary_value,
                c.secondary_value,
                c.tertiary_value,
                c.quaternary_value,
                c.quinary_value
            ),
            (150.0, 100.0, 110.0, 140.0, 90.0)
        );
        assert!(!c.is_empty);
        assert!(c.point_error.is_empty);
        assert_eq!(Coordinate::new(10.0, 20.0).to_string(), "(10, 20)");
        let c = Coordinate::box_value(0.0, 100.0, 75.0, 25.0, 0.0, 50.0);
        assert!(!c.is_empty);
        assert_eq!(
            (
                c.primary_value,
                c.tertiary_value,
                c.quaternary_value,
                c.quinary_value,
                c.senary_value
            ),
            (100.0, 75.0, 25.0, 0.0, 50.0)
        );
        let error = Error::new(1.0, 2.0, 3.0, 4.0);
        let c = Coordinate::from_values(10.0, 20.0, 30.0, 40.0, 50.0, 60.0, error);
        assert!(!c.is_empty);
        assert_eq!(
            (
                c.primary_value,
                c.secondary_value,
                c.tertiary_value,
                c.quaternary_value,
                c.quinary_value,
                c.senary_value
            ),
            (10.0, 20.0, 30.0, 40.0, 50.0, 60.0)
        );
        assert_eq!(
            (
                c.point_error.xi,
                c.point_error.xj,
                c.point_error.yi,
                c.point_error.yj
            ),
            (1.0, 2.0, 3.0, 4.0)
        );
    }

    #[test]
    fn empty_coordinate_and_zero_coordinate_are_distinct() {
        assert!(Coordinate::EMPTY.is_empty);
        assert_ne!(Coordinate::EMPTY, Coordinate::new(0.0, 0.0));
        assert!(!Error::symmetric(0.0, 0.0).is_empty);
        assert_eq!(Coordinate::range(2.0, 9.0, 3.0).tertiary_value, 3.0);
        assert!(!Coordinate::default().is_empty);
        assert!(!Error::default().is_empty);
    }
}
