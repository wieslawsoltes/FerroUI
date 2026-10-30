use super::DocumentError;

/// The rich text object model's line-spacing rules. Distances are expressed in
/// FerroUI layout units; `Multiple` is a multiplier and `Percent` a percentage.
/// See the pinned Microsoft.UI.Text contract in docs/porting/winui-rich-text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum LineSpacingRule {
    /// No override: use the block's LineHeight and LineStackingStrategy.
    #[default]
    Undefined = 0,
    Single = 1,
    OneAndHalf = 2,
    Double = 3,
    AtLeast = 4,
    Exactly = 5,
    Multiple = 6,
    Percent = 7,
}

ferroui_base::ferro_markup_enum!(LineSpacingRule {
    Undefined, Single, OneAndHalf, Double, AtLeast, Exactly, Multiple, Percent
}, { namespace: "FerroUI.Controls.Documents" });

impl LineSpacingRule {
    pub(crate) fn validate(self, spacing: f64) -> Result<(), DocumentError> {
        // The pinned rich-edit contract exposes this enumeration value but
        // explicitly rejects it in SetLineSpacing.
        if self == Self::Percent {
            return Err(DocumentError::InvalidFormat(
                "Percent line spacing is not supported by RichEditBox",
            ));
        }
        if !spacing.is_finite() || spacing < 0.0 {
            return Err(DocumentError::InvalidFormat(
                "LineSpacing must be finite and nonnegative",
            ));
        }
        if matches!(self, Self::Exactly | Self::Multiple | Self::Percent) && spacing == 0.0 {
            return Err(DocumentError::InvalidFormat(
                "This LineSpacingRule requires positive LineSpacing",
            ));
        }
        Ok(())
    }

    pub(crate) fn advance(self, natural: f64, spacing: f64) -> f64 {
        match self {
            Self::Undefined | Self::Single => natural,
            Self::OneAndHalf => natural * 1.5,
            Self::Double => natural * 2.0,
            Self::AtLeast => natural.max(spacing),
            Self::Exactly => spacing,
            Self::Multiple => natural * spacing,
            Self::Percent => natural * spacing / 100.0,
        }
    }
}
