use ferroui_base::{media::Color, Point, Rect, Size, Thickness};

/// Typed interpolation replaces the generated hierarchy of motion properties.
pub trait Interpolate: Clone {
    fn can_interpolate(from: &Self, to: &Self) -> bool {
        let _ = (from, to);
        true
    }
    fn interpolate(from: &Self, to: &Self, progress: f64) -> Self;
}

impl Interpolate for f64 {
    fn interpolate(from: &Self, to: &Self, p: f64) -> Self {
        *from + p * (*to - *from)
    }
}
impl Interpolate for f32 {
    fn interpolate(from: &Self, to: &Self, p: f64) -> Self {
        *from + p as f32 * (*to - *from)
    }
}
impl Interpolate for Point {
    fn interpolate(from: &Self, to: &Self, p: f64) -> Self {
        Point::new(
            f64::interpolate(&from.x, &to.x, p),
            f64::interpolate(&from.y, &to.y, p),
        )
    }
}
impl Interpolate for Size {
    fn interpolate(from: &Self, to: &Self, p: f64) -> Self {
        Size::new(
            f64::interpolate(&from.width, &to.width, p),
            f64::interpolate(&from.height, &to.height, p),
        )
    }
}
impl Interpolate for Rect {
    fn interpolate(from: &Self, to: &Self, p: f64) -> Self {
        Rect::new(
            f64::interpolate(&from.x, &to.x, p),
            f64::interpolate(&from.y, &to.y, p),
            f64::interpolate(&from.width, &to.width, p),
            f64::interpolate(&from.height, &to.height, p),
        )
    }
}
impl Interpolate for Thickness {
    fn interpolate(from: &Self, to: &Self, p: f64) -> Self {
        Thickness::new(
            f64::interpolate(&from.left, &to.left, p),
            f64::interpolate(&from.top, &to.top, p),
            f64::interpolate(&from.right, &to.right, p),
            f64::interpolate(&from.bottom, &to.bottom, p),
        )
    }
}
impl Interpolate for Color {
    fn interpolate(from: &Self, to: &Self, p: f64) -> Self {
        let channel = |a: u8, b: u8| (f64::from(a) + p * (f64::from(b) - f64::from(a))) as u8;
        Color::from_argb(
            channel(from.a, to.a),
            channel(from.r, to.r),
            channel(from.g, to.g),
            channel(from.b, to.b),
        )
    }
}
impl<T: Interpolate> Interpolate for Option<T> {
    fn can_interpolate(from: &Self, to: &Self) -> bool {
        match (from, to) {
            (Some(a), Some(b)) => T::can_interpolate(a, b),
            _ => false,
        }
    }
    fn interpolate(from: &Self, to: &Self, p: f64) -> Self {
        match (from, to) {
            (Some(a), Some(b)) => Some(T::interpolate(a, b, p)),
            _ => to.clone(),
        }
    }
}
impl<T: Interpolate> Interpolate for Vec<T> {
    fn can_interpolate(from: &Self, to: &Self) -> bool {
        from.len() == to.len() && from.iter().zip(to).all(|(a, b)| T::can_interpolate(a, b))
    }
    fn interpolate(from: &Self, to: &Self, p: f64) -> Self {
        if from.len() != to.len() {
            return to.clone();
        }
        from.iter()
            .zip(to)
            .map(|(a, b)| T::interpolate(a, b, p))
            .collect()
    }
}

/// A reference-valued property has no transition between its two identities.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Discrete<T: Clone>(pub T);
impl<T: Clone> Interpolate for Discrete<T> {
    fn can_interpolate(_from: &Self, _to: &Self) -> bool {
        false
    }
    fn interpolate(_from: &Self, to: &Self, _progress: f64) -> Self {
        to.clone()
    }
}
