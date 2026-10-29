// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines document tab layout placement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DocumentTabLayout {
    /// Tabs placed on the top.
    #[default]
    Top = 0,
    /// Tabs placed on the left.
    Left = 1,
    /// Tabs placed on the right.
    Right = 2,
}

impl TryFrom<i32> for DocumentTabLayout {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Top),
            1 => Ok(Self::Left),
            2 => Ok(Self::Right),
            other => Err(other),
        }
    }
}

impl DocumentTabLayout {
    /// Original declaration for native metadata generation without a Domain UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("document_tab_layout.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DocumentTabLayout::Top as i32, 0);
        assert_eq!(DocumentTabLayout::try_from(0), Ok(DocumentTabLayout::Top));
        assert_eq!(DocumentTabLayout::Left as i32, 1);
        assert_eq!(DocumentTabLayout::try_from(1), Ok(DocumentTabLayout::Left));
        assert_eq!(DocumentTabLayout::Right as i32, 2);
        assert_eq!(DocumentTabLayout::try_from(2), Ok(DocumentTabLayout::Right));
        assert_eq!(DocumentTabLayout::default() as i32, 0);
        assert_eq!(DocumentTabLayout::try_from(999), Err(999));
    }
}
