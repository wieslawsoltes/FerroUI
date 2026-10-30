// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines document layout mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DocumentLayoutMode {
    /// Documents are displayed as tabs.
    #[default]
    Tabbed = 0,
    /// Documents are displayed as MDI windows.
    Mdi = 1,
}

impl TryFrom<i32> for DocumentLayoutMode {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Tabbed),
            1 => Ok(Self::Mdi),
            other => Err(other),
        }
    }
}

impl DocumentLayoutMode {
    /// Original declaration for native metadata generation without a Domain UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("document_layout_mode.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DocumentLayoutMode::Tabbed as i32, 0);
        assert_eq!(
            DocumentLayoutMode::try_from(0),
            Ok(DocumentLayoutMode::Tabbed)
        );
        assert_eq!(DocumentLayoutMode::Mdi as i32, 1);
        assert_eq!(DocumentLayoutMode::try_from(1), Ok(DocumentLayoutMode::Mdi));
        assert_eq!(DocumentLayoutMode::default() as i32, 0);
        assert_eq!(DocumentLayoutMode::try_from(999), Err(999));
    }
}
