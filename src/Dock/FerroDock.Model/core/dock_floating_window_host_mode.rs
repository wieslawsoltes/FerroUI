// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines how floating dock windows are hosted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DockFloatingWindowHostMode(i32);

// Source enum member names are part of the public API.
#[allow(non_upper_case_globals)]
impl DockFloatingWindowHostMode {
    /// Uses the default host selection (falls back to DockSettings.UseManagedWindows).
    pub const Default: Self = Self(0);
    /// Use native OS windows for floating docks.
    pub const Native: Self = Self(1);
    /// Use managed in-app windows for floating docks.
    pub const Managed: Self = Self(2);
    /// Retains an original numeric enum cast, including undeclared values.
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    /// Gets the original signed underlying value.
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

impl TryFrom<i32> for DockFloatingWindowHostMode {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Default),
            1 => Ok(Self::Native),
            2 => Ok(Self::Managed),
            other => Err(other),
        }
    }
}

impl DockFloatingWindowHostMode {
    /// Original declaration for native metadata generation without a model UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("dock_floating_window_host_mode.rs");
}

impl std::fmt::Display for DockFloatingWindowHostMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Default => f.write_str("Default"),
            Self::Native => f.write_str("Native"),
            Self::Managed => f.write_str("Managed"),
            other => write!(f, "{}", other.to_raw()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_numeric_and_named_display_matches_source_values() {
        let mut count = 0;
        for row in
            include_str!("../../../../tests/Dock/data/serializer-json-audit.dock-enums.values.tsv").lines()
        {
            let fields: Vec<_> = row.split('\t').collect();
            if fields[0] != "HOST" {
                continue;
            }
            let value = DockFloatingWindowHostMode::from_raw(fields[1].parse().unwrap());
            assert_eq!(value.to_string(), fields[2]);
            count += 1;
        }
        assert_eq!(count, 141);
    }

    #[test]
    fn numeric_casts_keep_original_signed_values_and_storage() {
        use std::hash::{Hash, Hasher};
        for raw in [i32::MIN, -19, -1, 0, 1, 2, 3, 4, 5, 999, i32::MAX] {
            let value = DockFloatingWindowHostMode::from_raw(raw);
            assert_eq!(value.to_raw(), raw);
            let mut actual = std::collections::hash_map::DefaultHasher::new();
            let mut expected = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut actual);
            raw.hash(&mut expected);
            assert_eq!(actual.finish(), expected.finish());
        }
        assert_eq!(
            std::mem::size_of::<DockFloatingWindowHostMode>(),
            std::mem::size_of::<i32>()
        );
        assert_eq!(
            std::mem::align_of::<DockFloatingWindowHostMode>(),
            std::mem::align_of::<i32>()
        );
    }

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockFloatingWindowHostMode::Default.to_raw(), 0);
        assert_eq!(
            DockFloatingWindowHostMode::try_from(0),
            Ok(DockFloatingWindowHostMode::Default)
        );
        assert_eq!(DockFloatingWindowHostMode::Native.to_raw(), 1);
        assert_eq!(
            DockFloatingWindowHostMode::try_from(1),
            Ok(DockFloatingWindowHostMode::Native)
        );
        assert_eq!(DockFloatingWindowHostMode::Managed.to_raw(), 2);
        assert_eq!(
            DockFloatingWindowHostMode::try_from(2),
            Ok(DockFloatingWindowHostMode::Managed)
        );
        assert_eq!(DockFloatingWindowHostMode::default().to_raw(), 0);
        assert_eq!(DockFloatingWindowHostMode::try_from(999), Err(999));
    }
}
