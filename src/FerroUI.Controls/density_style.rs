//! How much space the controls of a theme take: one type for every theme, so that the name is one
//! type in the default XML namespace.

use ferroui_base::ferro_markup_enum;

/// How much space the controls of a theme take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DensityStyle {
    /// The sizes of the theme.
    #[default]
    Normal,
    /// The compact sizes of the theme.
    Compact,
}

ferro_markup_enum!(DensityStyle { Normal, Compact }, { namespace: "FerroUI.Controls" });
