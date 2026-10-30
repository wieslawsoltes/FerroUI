// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use bitflags::bitflags;

bitflags! {
    /// Defines the available drag actions.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DragAction: i32 {
        /// No action.
        const NONE = 0;
        /// Copy action.
        const COPY = 1;
        /// Move action.
        const MOVE = 2;
        /// Link action.
        const LINK = 4;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_values_and_combinations_preserve_the_contract() {
        assert_eq!(DragAction::NONE.bits(), 0);
        assert_eq!(DragAction::COPY.bits(), 1);
        assert_eq!(DragAction::MOVE.bits(), 2);
        assert_eq!(DragAction::LINK.bits(), 4);
        assert_eq!(DragAction::default().bits(), 0);
        assert_eq!(DragAction::from_bits_retain(-1).bits(), -1);
    }
}
