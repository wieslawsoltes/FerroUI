//! The asymmetric extent of a coordinate's error bars.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Error {
    pub xi: f64,
    pub xj: f64,
    pub yi: f64,
    pub yj: f64,
    pub is_empty: bool,
}

impl Error {
    pub const EMPTY: Self = Self {
        xi: 0.0,
        xj: 0.0,
        yi: 0.0,
        yj: 0.0,
        is_empty: true,
    };

    pub const fn new(xi: f64, xj: f64, yi: f64, yj: f64) -> Self {
        Self {
            xi,
            xj,
            yi,
            yj,
            is_empty: false,
        }
    }

    pub const fn symmetric(x: f64, y: f64) -> Self {
        Self::new(x, x, y, y)
    }
}

impl Default for Error {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
}
