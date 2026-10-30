// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Specifies the direction in which a grid can be resized.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum GridResizeDirection {
    /// Resize grid columns.
    #[default]
    Columns = 0,
    /// Resize grid rows.
    Rows = 1,
}

impl TryFrom<i32> for GridResizeDirection {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Columns),
            1 => Ok(Self::Rows),
            other => Err(other),
        }
    }
}

impl GridResizeDirection {
    /// Original declaration for UI metadata generation without a Domain UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("grid_resize_direction.rs");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(GridResizeDirection::Columns as i32, 0);
        assert_eq!(
            GridResizeDirection::try_from(0),
            Ok(GridResizeDirection::Columns)
        );
        assert_eq!(GridResizeDirection::Rows as i32, 1);
        assert_eq!(
            GridResizeDirection::try_from(1),
            Ok(GridResizeDirection::Rows)
        );
        assert_eq!(GridResizeDirection::default() as i32, 0);
        assert_eq!(GridResizeDirection::try_from(999), Err(999));
    }
}
