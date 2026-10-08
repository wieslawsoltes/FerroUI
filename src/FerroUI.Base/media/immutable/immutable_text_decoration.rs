use crate::media::immutable::ImmutablePen;
use crate::media::{TextDecorationLocation, TextDecorationUnit};

/// An immutable representation of a text decoration.
#[derive(Clone, Debug)]
pub struct ImmutableTextDecoration {
    location: TextDecorationLocation,
    pen: ImmutablePen,
    pen_thickness_unit: TextDecorationUnit,
    pen_offset: f64,
    pen_offset_unit: TextDecorationUnit,
}

impl ImmutableTextDecoration {
    /// Creates a text decoration.
    pub fn new(
        location: TextDecorationLocation,
        pen: ImmutablePen,
        pen_thickness_unit: TextDecorationUnit,
        pen_offset: f64,
        pen_offset_unit: TextDecorationUnit,
    ) -> ImmutableTextDecoration {
        ImmutableTextDecoration { location, pen, pen_thickness_unit, pen_offset, pen_offset_unit }
    }

    /// Gets the location.
    pub fn location(&self) -> TextDecorationLocation {
        self.location
    }

    /// Gets the pen.
    pub fn pen(&self) -> &ImmutablePen {
        &self.pen
    }

    /// Gets the units in which the thickness of the pen is expressed.
    pub fn pen_thickness_unit(&self) -> TextDecorationUnit {
        self.pen_thickness_unit
    }

    /// Gets the pen offset.
    pub fn pen_offset(&self) -> f64 {
        self.pen_offset
    }

    /// Gets the units in which the pen offset is expressed.
    pub fn pen_offset_unit(&self) -> TextDecorationUnit {
        self.pen_offset_unit
    }
}

// Not an upstream test: the class has none.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constructor_keeps_its_arguments() {
        let decoration = ImmutableTextDecoration::new(
            TextDecorationLocation::Strikethrough,
            ImmutablePen::from_uint32(0xFF00_0000, 2.0),
            TextDecorationUnit::FontRecommended,
            3.0,
            TextDecorationUnit::Pixel,
        );

        assert_eq!(TextDecorationLocation::Strikethrough, decoration.location());
        assert_eq!(2.0, decoration.pen().thickness());
        assert_eq!(TextDecorationUnit::FontRecommended, decoration.pen_thickness_unit());
        assert_eq!(3.0, decoration.pen_offset());
        assert_eq!(TextDecorationUnit::Pixel, decoration.pen_offset_unit());
    }
}
