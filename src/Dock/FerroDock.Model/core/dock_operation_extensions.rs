// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{Alignment, DockOperation};

impl DockOperation {
    #[allow(dead_code)] // Used by factory splitting once that source file is ported.
    pub(crate) fn to_alignment(self) -> Alignment {
        match self {
            Self::Left => Alignment::Left,
            Self::Bottom => Alignment::Bottom,
            Self::Right => Alignment::Right,
            Self::Top => Alignment::Top,
            _ => Alignment::Unset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_inner_edge_operations_convert_to_an_alignment() {
        let expected = [-1, -1, 0, 1, 2, 3, -1, -1, -1, -1, -1];
        for (raw, alignment) in expected.into_iter().enumerate() {
            assert_eq!(
                DockOperation::try_from(raw as i32).unwrap().to_alignment() as i32,
                alignment
            );
        }
    }
}
