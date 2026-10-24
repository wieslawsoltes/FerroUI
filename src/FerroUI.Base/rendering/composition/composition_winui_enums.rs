//! The enumerations of the WinUI visual and brush members of phase 2 (not
//! from upstream): the 3D modes of a visual, which a 2D renderer stores,
//! and the mapping mode of the gradient brushes. (The colour space of the
//! gradient brushes is the `CompositionColorSpace` of the animations.)

use super::server::ServerPropertyValue;
use super::transport::{BatchValue, BatchValueReader};

/// How the edges of a visual are antialiased (WinUI `CompositionBorderMode`).
///
/// Deviation (winui-composition.md): stored only; the drawing contexts
/// antialias every edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionBorderMode {
    /// The mode of the parent visual (the WinUI default).
    #[default]
    Inherit = 0,
    /// Antialiased edges.
    Soft = 1,
    /// Aliased edges.
    Hard = 2,
}

/// Whether the back of a visual rotated in 3D is shown (WinUI
/// `CompositionBackfaceVisibility`).
///
/// Deviation (winui-composition.md): stored only; a 2D renderer has no back
/// face.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionBackfaceVisibility {
    /// The visibility of the parent visual (the WinUI default).
    #[default]
    Inherit = 0,
    /// The back face is shown.
    Visible = 1,
    /// The back face is hidden.
    Hidden = 2,
}

/// How a visual is blended with what is behind it (WinUI
/// `CompositionCompositeMode`).
///
/// Deviation (winui-composition.md): stored only; the drawing contexts
/// blend whole subtrees with source-over only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionCompositeMode {
    /// The mode of the parent visual (the WinUI default).
    #[default]
    Inherit = 0,
    /// The visual is drawn over what is behind it.
    SourceOver = 1,
    /// What is behind the visual is inverted where the visual is.
    DestinationInvert = 2,
    /// The minimum of the visual and what is behind it.
    MinBlend = 3,
}

/// How the points of a gradient brush are measured (WinUI
/// `CompositionMappingMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CompositionMappingMode {
    /// In pixels of the area the brush paints.
    Absolute = 0,
    /// As fractions of the size of the area the brush paints (the WinUI
    /// default).
    #[default]
    Relative = 1,
}

/// The enumerations of the schema cross a batch as their numeric value and
/// compare as values on the server.
macro_rules! winui_schema_enum {
    ($ty:ident { $($variant:ident),* $(,)? }) => {
        impl BatchValue for $ty {
            fn write_to(&self, out: &mut Vec<u8>) {
                (*self as i32).write_to(out);
            }

            fn read_from(input: &mut BatchValueReader<'_>) -> Self {
                let value = i32::read_from(input);
                $(if value == $ty::$variant as i32 {
                    return $ty::$variant;
                })*
                panic!("the batch value stream holds {value}, which is not a {}", stringify!($ty));
            }
        }

        impl ServerPropertyValue for $ty {
            fn differs(&self, other: &Self) -> bool {
                self != other
            }
        }
    };
}

winui_schema_enum!(CompositionBorderMode { Inherit, Soft, Hard });
winui_schema_enum!(CompositionBackfaceVisibility { Inherit, Visible, Hidden });
winui_schema_enum!(CompositionCompositeMode { Inherit, SourceOver, DestinationInvert, MinBlend });

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winui_values() {
        assert_eq!(CompositionBorderMode::Hard as i32, 2);
        assert_eq!(CompositionBackfaceVisibility::Hidden as i32, 2);
        assert_eq!(CompositionCompositeMode::MinBlend as i32, 3);
        assert_eq!(CompositionMappingMode::default(), CompositionMappingMode::Relative);
    }

    #[test]
    fn the_visual_modes_cross_a_batch() {
        let mut out = Vec::new();
        CompositionBorderMode::Soft.write_to(&mut out);
        CompositionBackfaceVisibility::Visible.write_to(&mut out);
        CompositionCompositeMode::DestinationInvert.write_to(&mut out);
        let mut offset = 0;
        let mut input = BatchValueReader::new(&out, &mut offset);
        assert_eq!(CompositionBorderMode::Soft, CompositionBorderMode::read_from(&mut input));
        assert_eq!(CompositionBackfaceVisibility::Visible, CompositionBackfaceVisibility::read_from(&mut input));
        assert_eq!(CompositionCompositeMode::DestinationInvert, CompositionCompositeMode::read_from(&mut input));
    }
}
