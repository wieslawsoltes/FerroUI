use crate::media::immutable::ImmutableTransform;
use crate::media::{
    Color, IBrush, IImmutableBrush, IImmutableSolidColorBrush, ISolidColorBrush, ITransform, TransformExtensions,
};
use crate::RelativePoint;
use std::any::Any;
use std::fmt;
use std::rc::Rc;

/// Fills an area with a solid color.
#[derive(Clone, Debug)]
pub struct ImmutableSolidColorBrush {
    color: Color,
    opacity: f64,
    transform: Option<Rc<dyn ITransform>>,
    transform_origin: RelativePoint,
    relative_transform: Option<Rc<dyn ITransform>>,
}

impl ImmutableSolidColorBrush {
    /// Creates a fully opaque brush with the given color.
    pub fn new(color: Color) -> Self {
        Self::with_transforms(color, 1.0, None, None)
    }

    /// Creates a brush with the given color and opacity.
    pub fn with_opacity(color: Color, opacity: f64) -> Self {
        Self::with_transforms(color, opacity, None, None)
    }

    /// Creates a brush with the given color, opacity and transforms.
    pub fn with_transforms(
        color: Color,
        opacity: f64,
        transform: Option<Rc<ImmutableTransform>>,
        relative_transform: Option<Rc<ImmutableTransform>>,
    ) -> Self {
        Self {
            color,
            opacity,
            transform: transform.map(|t| t as Rc<dyn ITransform>),
            transform_origin: RelativePoint::default(),
            relative_transform: relative_transform.map(|t| t as Rc<dyn ITransform>),
        }
    }

    /// Creates a brush with the color given as an `0xAARRGGBB` value.
    pub fn from_uint32(color: u32) -> Self {
        Self::new(Color::from_uint32(color))
    }

    /// Creates an immutable copy of `source`.
    pub fn from_brush(source: &dyn ISolidColorBrush) -> Self {
        Self::with_transforms(
            source.color(),
            source.opacity(),
            source.transform().map(|t| Rc::new(TransformExtensions::to_immutable(&*t))),
            source.relative_transform().map(|t| Rc::new(TransformExtensions::to_immutable(&*t))),
        )
    }

    /// The color of the brush.
    #[inline]
    pub fn color(&self) -> Color {
        self.color
    }

    /// The opacity of the brush.
    #[inline]
    pub fn opacity(&self) -> f64 {
        self.opacity
    }
}

/// Brushes are equal when their color and opacity are equal and they share
/// the same transform instances.
impl PartialEq for ImmutableSolidColorBrush {
    fn eq(&self, other: &Self) -> bool {
        self.color == other.color
            && self.opacity == other.opacity
            && self.transform == other.transform
            && self.relative_transform == other.relative_transform
    }
}

/// Writes the brush's color.
impl fmt::Display for ImmutableSolidColorBrush {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.color, f)
    }
}

impl IBrush for ImmutableSolidColorBrush {
    #[inline]
    fn opacity(&self) -> f64 {
        self.opacity
    }

    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        self.transform.clone()
    }

    #[inline]
    fn transform_origin(&self) -> RelativePoint {
        self.transform_origin
    }

    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.relative_transform.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_solid_color_brush(&self) -> Option<&dyn ISolidColorBrush> {
        Some(self)
    }

    fn into_immutable_brush(self: Rc<Self>) -> Option<Rc<dyn IImmutableBrush>> {
        Some(self)
    }

    fn equals(&self, other: &dyn IBrush) -> bool {
        match other.as_any().downcast_ref::<ImmutableSolidColorBrush>() {
            Some(other) => self == other,
            None => false,
        }
    }
}

impl ISolidColorBrush for ImmutableSolidColorBrush {
    #[inline]
    fn color(&self) -> Color {
        self.color
    }
}

impl IImmutableBrush for ImmutableSolidColorBrush {}

impl IImmutableSolidColorBrush for ImmutableSolidColorBrush {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Colors;
    use crate::Matrix;

    #[test]
    fn equality_is_structural_with_transform_identity() {
        let a = ImmutableSolidColorBrush::with_opacity(Colors::RED, 0.5);
        let b = ImmutableSolidColorBrush::with_opacity(Colors::RED, 0.5);
        assert_eq!(a, b);
        assert_ne!(a, ImmutableSolidColorBrush::with_opacity(Colors::RED, 0.6));
        assert_ne!(a, ImmutableSolidColorBrush::with_opacity(Colors::BLUE, 0.5));

        let transform = Rc::new(ImmutableTransform::new(Matrix::IDENTITY));
        let c = ImmutableSolidColorBrush::with_transforms(Colors::RED, 0.5, Some(transform.clone()), None);
        let d = ImmutableSolidColorBrush::with_transforms(Colors::RED, 0.5, Some(transform), None);
        let e = ImmutableSolidColorBrush::with_transforms(
            Colors::RED,
            0.5,
            Some(Rc::new(ImmutableTransform::new(Matrix::IDENTITY))),
            None,
        );
        assert_eq!(c, d);
        assert_ne!(c, e);
        assert_ne!(a, c);
    }

    #[test]
    fn from_uint32_and_display() {
        let brush = ImmutableSolidColorBrush::from_uint32(0xffff0000);
        assert_eq!(Colors::RED, brush.color());
        assert_eq!(1.0, brush.opacity());
        assert_eq!("Red", brush.to_string());
    }
}
