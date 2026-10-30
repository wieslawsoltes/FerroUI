// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{DockOperation, DockOperationMask};

impl DockOperationMask {
    /// Returns whether this mask allows the supplied docking operation.
    pub fn allows(self, operation: DockOperation) -> bool {
        let required = match operation {
            DockOperation::Fill => Self::FILL,
            DockOperation::Left | DockOperation::OuterLeft => Self::LEFT,
            DockOperation::Right | DockOperation::OuterRight => Self::RIGHT,
            DockOperation::Top | DockOperation::OuterTop => Self::TOP,
            DockOperation::Bottom | DockOperation::OuterBottom => Self::BOTTOM,
            DockOperation::Window => Self::WINDOW,
            DockOperation::None => return false,
        };
        self.contains(required)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mask_matches_the_allowed_operations_including_outer_splits() {
        let operations = [
            (DockOperation::None, 0),
            (DockOperation::Fill, 1),
            (DockOperation::Left, 2),
            (DockOperation::Bottom, 16),
            (DockOperation::Right, 4),
            (DockOperation::Top, 8),
            (DockOperation::Window, 32),
            (DockOperation::OuterLeft, 2),
            (DockOperation::OuterTop, 8),
            (DockOperation::OuterRight, 4),
            (DockOperation::OuterBottom, 16),
        ];
        for bits in 0..64 {
            let mask = DockOperationMask::from_bits_retain(bits);
            for (operation, required) in operations {
                assert_eq!(
                    mask.allows(operation),
                    required != 0 && bits & required != 0
                );
            }
        }
    }

    #[test]
    fn unknown_mask_bits_never_allow_an_operation() {
        let mask = DockOperationMask::from_bits_retain(1 << 20);
        for raw in 0..=10 {
            assert!(!mask.allows(DockOperation::try_from(raw).unwrap()));
        }
    }
}
