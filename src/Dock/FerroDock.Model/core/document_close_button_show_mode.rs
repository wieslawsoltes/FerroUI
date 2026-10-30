// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Controls when document close buttons are displayed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DocumentCloseButtonShowMode {
    /// Always show close buttons.
    #[default]
    Always = 0,
    /// Show close buttons only for the active document.
    Active = 1,
    /// Never show close buttons.
    Never = 2,
}

impl TryFrom<i32> for DocumentCloseButtonShowMode {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Always),
            1 => Ok(Self::Active),
            2 => Ok(Self::Never),
            other => Err(other),
        }
    }
}

impl DocumentCloseButtonShowMode {
    /// Original declaration for native metadata generation without a Domain UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("document_close_button_show_mode.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DocumentCloseButtonShowMode::Always as i32, 0);
        assert_eq!(
            DocumentCloseButtonShowMode::try_from(0),
            Ok(DocumentCloseButtonShowMode::Always)
        );
        assert_eq!(DocumentCloseButtonShowMode::Active as i32, 1);
        assert_eq!(
            DocumentCloseButtonShowMode::try_from(1),
            Ok(DocumentCloseButtonShowMode::Active)
        );
        assert_eq!(DocumentCloseButtonShowMode::Never as i32, 2);
        assert_eq!(
            DocumentCloseButtonShowMode::try_from(2),
            Ok(DocumentCloseButtonShowMode::Never)
        );
        assert_eq!(DocumentCloseButtonShowMode::default() as i32, 0);
        assert_eq!(DocumentCloseButtonShowMode::try_from(999), Err(999));
    }
}
