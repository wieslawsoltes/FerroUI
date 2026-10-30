// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines the available docking modes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DockMode(i32);

// Source enum member names are part of the public API.
#[allow(non_upper_case_globals)]
impl DockMode {
    /// Dock center.
    pub const Center: Self = Self(0);
    /// Dock left.
    pub const Left: Self = Self(1);
    /// Dock bottom.
    pub const Bottom: Self = Self(2);
    /// Dock right.
    pub const Right: Self = Self(3);
    /// Dock top.
    pub const Top: Self = Self(4);
    /// Retains an original numeric enum cast, including undeclared values.
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    /// Gets the original signed underlying value.
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

impl TryFrom<i32> for DockMode {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Center),
            1 => Ok(Self::Left),
            2 => Ok(Self::Bottom),
            3 => Ok(Self::Right),
            4 => Ok(Self::Top),
            other => Err(other),
        }
    }
}

impl DockMode {
    /// Original declaration for UI metadata generation without a Domain UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("dock_mode.rs");
}

impl std::fmt::Display for DockMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Center => f.write_str("Center"),
            Self::Left => f.write_str("Left"),
            Self::Bottom => f.write_str("Bottom"),
            Self::Right => f.write_str("Right"),
            Self::Top => f.write_str("Top"),
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
            if fields[0] != "DOCK" {
                continue;
            }
            let value = DockMode::from_raw(fields[1].parse().unwrap());
            assert_eq!(value.to_string(), fields[2]);
            count += 1;
        }
        assert_eq!(count, 141);
    }

    #[test]
    fn numeric_casts_keep_original_signed_values_and_storage() {
        use std::hash::{Hash, Hasher};
        for raw in [i32::MIN, -19, -1, 0, 1, 2, 3, 4, 5, 999, i32::MAX] {
            let value = DockMode::from_raw(raw);
            assert_eq!(value.to_raw(), raw);
            let mut actual = std::collections::hash_map::DefaultHasher::new();
            let mut expected = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut actual);
            raw.hash(&mut expected);
            assert_eq!(actual.finish(), expected.finish());
        }
        assert_eq!(std::mem::size_of::<DockMode>(), std::mem::size_of::<i32>());
        assert_eq!(
            std::mem::align_of::<DockMode>(),
            std::mem::align_of::<i32>()
        );
    }

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockMode::Center.to_raw(), 0);
        assert_eq!(DockMode::try_from(0), Ok(DockMode::Center));
        assert_eq!(DockMode::Left.to_raw(), 1);
        assert_eq!(DockMode::try_from(1), Ok(DockMode::Left));
        assert_eq!(DockMode::Bottom.to_raw(), 2);
        assert_eq!(DockMode::try_from(2), Ok(DockMode::Bottom));
        assert_eq!(DockMode::Right.to_raw(), 3);
        assert_eq!(DockMode::try_from(3), Ok(DockMode::Right));
        assert_eq!(DockMode::Top.to_raw(), 4);
        assert_eq!(DockMode::try_from(4), Ok(DockMode::Top));
        assert_eq!(DockMode::default().to_raw(), 0);
        assert_eq!(DockMode::try_from(999), Err(999));
    }
}
