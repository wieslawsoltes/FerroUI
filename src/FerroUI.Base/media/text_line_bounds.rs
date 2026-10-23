//! How the box of a line of text is bounded above and below, the type of
//! `TextBlock.TextLineBounds` (a FerroUI addition after WinUI's `TextLineBounds`,
//! docs/porting/DEVIATIONS.md, Text block).
//!
//! The box of a line runs from the ascent of its font (room above the capitals for accents)
//! to its descent (room below the baseline for descenders), and centring a line centres that
//! box, not the letters. WinUI's `CCompositeFontFamily::GetTextLineBoundsMetrics` trims it to
//! the cap height above the baseline, to the baseline below, or both, so that a label centred
//! beside an icon has its capitals in the middle whatever its font.

use crate::ferro_markup_enum;

/// How the box of each line of text is bounded: by the ascent and the descent of its font
/// (the default), or trimmed to the cap height above the baseline, to the baseline below it,
/// or both.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TextLineBounds {
    /// The box of a line is the ascent, the descent and the line gap of its font.
    #[default]
    Full,
    /// The top of the box of a line is the cap height of its font above the baseline; the
    /// bottom is as `Full` has it.
    TrimToCapHeight,
    /// The bottom of the box of a line is its baseline; the top is as `Full` has it.
    TrimToBaseline,
    /// The box of a line runs from the cap height of its font above the baseline to the
    /// baseline.
    Tight,
}

ferro_markup_enum!(TextLineBounds { Full, TrimToCapHeight, TrimToBaseline, Tight }, { namespace: "FerroUI.Media" });

impl TextLineBounds {
    /// Whether the top of the box of a line is the cap height above the baseline.
    pub fn trims_top(self) -> bool {
        matches!(self, Self::TrimToCapHeight | Self::Tight)
    }

    /// Whether the bottom of the box of a line is the baseline.
    pub fn trims_bottom(self) -> bool {
        matches!(self, Self::TrimToBaseline | Self::Tight)
    }
}
