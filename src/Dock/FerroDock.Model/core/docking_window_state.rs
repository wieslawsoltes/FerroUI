// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use bitflags::bitflags;

bitflags! {
    /// Represents the logical docking state of a dockable.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct DockingWindowState: i32 {
        /// Docking state is not set.
        const NONE = 0;
        /// Dockable is part of normal dock/tool layout.
        const DOCKED = 1;
        /// Dockable is pinned in an auto-hide strip.
        const PINNED = 2;
        /// Dockable is hosted inside a document dock.
        const DOCUMENT = 4;
        /// Dockable is hosted in a floating window.
        const FLOATING = 8;
        /// Dockable is hidden.
        const HIDDEN = 16;
    }
}

impl DockingWindowState {
    /// Original declaration used by UI-owned source-generated metadata.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("docking_window_state.rs");
}

impl std::fmt::Display for DockingWindowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bits = self.bits();
        if bits == 0 {
            return f.write_str("None");
        }
        if bits & !Self::all().bits() != 0 {
            return write!(f, "{bits}");
        }
        let mut first = true;
        for (flag, name) in [
            (Self::DOCKED, "Docked"),
            (Self::PINNED, "Pinned"),
            (Self::DOCUMENT, "Document"),
            (Self::FLOATING, "Floating"),
            (Self::HIDDEN, "Hidden"),
        ] {
            if self.contains(flag) {
                if !first {
                    f.write_str(", ")?;
                }
                f.write_str(name)?;
                first = false;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_values_and_combinations_preserve_the_contract() {
        assert_eq!(DockingWindowState::NONE.bits(), 0);
        assert_eq!(DockingWindowState::DOCKED.bits(), 1);
        assert_eq!(DockingWindowState::PINNED.bits(), 2);
        assert_eq!(DockingWindowState::DOCUMENT.bits(), 4);
        assert_eq!(DockingWindowState::FLOATING.bits(), 8);
        assert_eq!(DockingWindowState::HIDDEN.bits(), 16);
        assert_eq!(DockingWindowState::default().bits(), 0);
        assert_eq!(DockingWindowState::from_bits_retain(-1).bits(), -1);
    }
    #[test]
    fn display_matches_actual_source_runtime_flags_and_unknown_values() {
        let mut count = 0;
        for line in include_str!("../../../../tests/Dock/data/docking-window-state-display.tsv")
            .lines()
            .filter(|line| !line.starts_with('#'))
        {
            let (bits, text) = line.split_once('\t').unwrap();
            let value = DockingWindowState::from_bits_retain(bits.parse().unwrap());
            assert_eq!(value.to_string(), text, "bits {bits}");
            count += 1;
        }
        assert_eq!(count, 98);
    }
}
