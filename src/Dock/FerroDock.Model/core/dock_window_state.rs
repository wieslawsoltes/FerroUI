// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Represents the presentation state of a floating dock window.
/// All signed Int32 values are retained, including values without a declared name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DockWindowState(i32);

// Source enum member names are part of the public API.
#[allow(non_upper_case_globals)]
impl DockWindowState {
    /// Window is shown at its normal size and position.
    pub const Normal: Self = Self(0);
    /// Window is minimized.
    pub const Minimized: Self = Self(1);
    /// Window is maximized.
    pub const Maximized: Self = Self(2);
    /// Window is shown in full-screen mode.
    pub const FullScreen: Self = Self(3);

    /// Retains an original numeric enum cast, including undeclared values.
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    /// Gets the original signed underlying value.
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

impl TryFrom<i32> for DockWindowState {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Normal),
            1 => Ok(Self::Minimized),
            2 => Ok(Self::Maximized),
            3 => Ok(Self::FullScreen),
            other => Err(other),
        }
    }
}

impl DockWindowState {
    /// Original declaration text used by the UI-owned native enum metadata generator.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("dock_window_state.rs");
}

impl std::fmt::Display for DockWindowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Normal => f.write_str("Normal"),
            Self::Minimized => f.write_str("Minimized"),
            Self::Maximized => f.write_str("Maximized"),
            Self::FullScreen => f.write_str("FullScreen"),
            other => write!(f, "{}", other.to_raw()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_numeric_casts_retain_value_equality_hash_and_storage() {
        use std::hash::{Hash, Hasher};
        for raw in [i32::MIN, -1, 0, 1, 2, 3, 4, 5, 999, i32::MAX] {
            let value = DockWindowState::from_raw(raw);
            assert_eq!(value.to_raw(), raw);
            let mut actual = std::collections::hash_map::DefaultHasher::new();
            let mut expected = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut actual);
            raw.hash(&mut expected);
            assert_eq!(actual.finish(), expected.finish());
        }
        assert_eq!(
            std::mem::size_of::<DockWindowState>(),
            std::mem::size_of::<i32>()
        );
        assert_eq!(
            std::mem::align_of::<DockWindowState>(),
            std::mem::align_of::<i32>()
        );
    }
    #[test]
    fn numeric_and_named_display_matches_original_source_enum_values() {
        let mut count = 0;
        for row in
            include_str!("../../../../tests/Dock/data/serializer-json-audit.window.values.tsv").lines()
        {
            let fields: Vec<_> = row.split('\t').collect();
            if fields[0] != "STATE" {
                continue;
            }
            let value = DockWindowState::from_raw(fields[1].parse().unwrap());
            assert_eq!(value.to_string(), fields[2]);
            count += 1;
        }
        assert_eq!(count, 141);
    }
    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockWindowState::Normal.to_raw(), 0);
        assert_eq!(DockWindowState::try_from(0), Ok(DockWindowState::Normal));
        assert_eq!(DockWindowState::Minimized.to_raw(), 1);
        assert_eq!(DockWindowState::try_from(1), Ok(DockWindowState::Minimized));
        assert_eq!(DockWindowState::Maximized.to_raw(), 2);
        assert_eq!(DockWindowState::try_from(2), Ok(DockWindowState::Maximized));
        assert_eq!(DockWindowState::FullScreen.to_raw(), 3);
        assert_eq!(
            DockWindowState::try_from(3),
            Ok(DockWindowState::FullScreen)
        );
        assert_eq!(DockWindowState::default().to_raw(), 0);
        assert_eq!(DockWindowState::try_from(999), Err(999));
    }
}
