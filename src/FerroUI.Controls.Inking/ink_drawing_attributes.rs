use super::ink_point::InkError;
use ferroui_base::media::Color;
use ferroui_base::{Matrix, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PenTipShape {
    Circle = 0,
    Rectangle = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InkDrawingAttributesKind {
    Default = 0,
    Pencil = 1,
}

/// A value snapshot. Changing presenter defaults never mutates completed ink.
/// `size` describes the nib at full pressure, in device-independent pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InkDrawingAttributes {
    pub color: Color,
    pub size: Size,
    pub pen_tip: PenTipShape,
    pub pen_tip_transform: Matrix,
    pub draw_as_highlighter: bool,
    pub ignore_pressure: bool,
    pub fit_to_curve: bool,
    pub ignore_tilt: bool,
    pub kind: InkDrawingAttributesKind,
    pub pencil_opacity: f64,
}

impl Default for InkDrawingAttributes {
    fn default() -> Self {
        Self {
            color: Color::from_rgb(0, 0, 0),
            size: Size::new(2.0, 2.0),
            pen_tip: PenTipShape::Circle,
            pen_tip_transform: Matrix::IDENTITY,
            draw_as_highlighter: false,
            ignore_pressure: false,
            fit_to_curve: true,
            ignore_tilt: false,
            kind: InkDrawingAttributesKind::Default,
            pencil_opacity: 1.0,
        }
    }
}

impl InkDrawingAttributes {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn create_for_pencil() -> Self {
        Self {
            kind: InkDrawingAttributesKind::Pencil,
            pencil_opacity: 0.5,
            ..Self::default()
        }
    }

    pub fn validate(&self) -> Result<(), InkError> {
        if !self.size.width.is_finite()
            || !self.size.height.is_finite()
            || self.size.width <= 0.0
            || self.size.height <= 0.0
            || self.size.width > 100_000.0
            || self.size.height > 100_000.0
            || !self.pencil_opacity.is_finite()
            || !(0.0..=1.0).contains(&self.pencil_opacity)
        {
            return Err(InkError::InvalidDrawingAttributes);
        }
        validate_affine(self.pen_tip_transform)?;
        let m = self.pen_tip_transform;
        if m.m31 != 0.0 || m.m32 != 0.0 || (m.m11 * m.m22 - m.m12 * m.m21).abs() < 1.0e-12 {
            return Err(InkError::InvalidDrawingAttributes);
        }
        Ok(())
    }

    /// The factor of the nib size at `pressure`. The nominal pressure 0.5 (what devices without
    /// pressure report, and what `InkStrokeBuilder` gives its points) draws the configured
    /// size, full pressure 1.75 times it and the lightest a quarter of it. A sample without
    /// pressure (0, no contact) draws nothing: a stroke of such samples is invisible and has no
    /// bounds.
    // Deviation (DEVIATIONS.md, WinUI inking): DirectInk's pressure curve is not published; the
    // port uses the linear curve of WPF's ink, `0.25 + 1.5 * pressure`, so that a mouse or touch
    // stroke has the width the toolbar shows.
    pub(crate) fn pressure_scale(&self, pressure: f32) -> f64 {
        let pressure = f64::from(pressure);
        if self.ignore_pressure {
            1.0
        } else if !(pressure > 0.0) {
            0.0
        } else {
            0.25 + 1.5 * pressure.min(1.0)
        }
    }
}

pub(crate) fn validate_affine(m: Matrix) -> Result<(), InkError> {
    if [m.m11, m.m12, m.m21, m.m22, m.m31, m.m32]
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 1.0e12)
        || m.m13 != 0.0
        || m.m23 != 0.0
        || m.m33 != 1.0
    {
        return Err(InkError::InvalidTransform);
    }
    Ok(())
}

ferroui_base::ferro_markup_enum!(PenTipShape { Circle, Rectangle });
ferroui_base::ferro_markup_enum!(InkDrawingAttributesKind { Default, Pencil });

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_size_and_perspective() {
        let mut a = InkDrawingAttributes::default();
        a.size.width = 0.0;
        assert_eq!(a.validate(), Err(InkError::InvalidDrawingAttributes));
        a = InkDrawingAttributes::default();
        a.pen_tip_transform.m13 = 1.0;
        assert_eq!(a.validate(), Err(InkError::InvalidTransform));
    }
    #[test]
    fn pencil_defaults_are_a_value_snapshot() {
        let a = InkDrawingAttributes::create_for_pencil();
        let mut b = a;
        b.pencil_opacity = 0.8;
        assert_eq!(a.pencil_opacity, 0.5);
        assert_eq!(a.kind, InkDrawingAttributesKind::Pencil);
        assert!(a.validate().is_ok());
    }
}
