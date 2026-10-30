// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use bitflags::bitflags;

bitflags! {
    /// Flags describing allowed docking operations.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DockOperationMask: i32 {
        /// No docking operations are allowed.
        const NONE = 0;
        /// Allow docking as a fill operation (tab/center).
        const FILL = 1;
        /// Allow docking to the left.
        const LEFT = 2;
        /// Allow docking to the right.
        const RIGHT = 4;
        /// Allow docking to the top.
        const TOP = 8;
        /// Allow docking to the bottom.
        const BOTTOM = 16;
        /// Allow docking into a new window.
        const WINDOW = 32;
        /// Allow all docking operations.
        const ALL = 63;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_values_and_combinations_preserve_the_contract() {
        assert_eq!(DockOperationMask::NONE.bits(), 0);
        assert_eq!(DockOperationMask::FILL.bits(), 1);
        assert_eq!(DockOperationMask::LEFT.bits(), 2);
        assert_eq!(DockOperationMask::RIGHT.bits(), 4);
        assert_eq!(DockOperationMask::TOP.bits(), 8);
        assert_eq!(DockOperationMask::BOTTOM.bits(), 16);
        assert_eq!(DockOperationMask::WINDOW.bits(), 32);
        assert_eq!(DockOperationMask::ALL.bits(), 63);
        assert_eq!(DockOperationMask::default().bits(), 0);
        assert_eq!(DockOperationMask::from_bits_retain(-1).bits(), -1);
    }
}
