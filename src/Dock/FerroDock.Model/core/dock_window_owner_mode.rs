// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines how a dock window resolves its host owner.
/// All signed Int32 values are retained, including values without a declared name.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DockWindowOwnerMode(i32);

// Source enum member names are part of the public API.
#[allow(non_upper_case_globals)]
impl DockWindowOwnerMode {
    /// Uses the related dock contract when set; otherwise falls back to platform defaults.
    pub const Default: Self = Self(0);
    /// Do not assign an owner.
    pub const None: Self = Self(1);
    /// Use the related dock contract as the owner when available.
    pub const ParentWindow: Self = Self(2);
    /// Use the root dock window as the owner when available.
    pub const RootWindow: Self = Self(3);
    /// Use the window that currently hosts the dockable being floated.
    pub const DockableWindow: Self = Self(4);

    /// Retains an original numeric enum cast, including undeclared values.
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    /// Gets the original signed underlying value.
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

impl TryFrom<i32> for DockWindowOwnerMode {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Default),
            1 => Ok(Self::None),
            2 => Ok(Self::ParentWindow),
            3 => Ok(Self::RootWindow),
            4 => Ok(Self::DockableWindow),
            other => Err(other),
        }
    }
}

impl DockWindowOwnerMode {
    /// Original declaration for native metadata generation without a model UI dependency.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("dock_window_owner_mode.rs");
}

impl std::fmt::Display for DockWindowOwnerMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Default => f.write_str("Default"),
            Self::None => f.write_str("None"),
            Self::ParentWindow => f.write_str("ParentWindow"),
            Self::RootWindow => f.write_str("RootWindow"),
            Self::DockableWindow => f.write_str("DockableWindow"),
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
            let value = DockWindowOwnerMode::from_raw(raw);
            assert_eq!(value.to_raw(), raw);
            let mut actual = std::collections::hash_map::DefaultHasher::new();
            let mut expected = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut actual);
            raw.hash(&mut expected);
            assert_eq!(actual.finish(), expected.finish());
        }
        assert_eq!(
            std::mem::size_of::<DockWindowOwnerMode>(),
            std::mem::size_of::<i32>()
        );
        assert_eq!(
            std::mem::align_of::<DockWindowOwnerMode>(),
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
            if fields[0] != "OWNER" {
                continue;
            }
            let value = DockWindowOwnerMode::from_raw(fields[1].parse().unwrap());
            assert_eq!(value.to_string(), fields[2]);
            count += 1;
        }
        assert_eq!(count, 141);
    }
    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockWindowOwnerMode::Default.to_raw(), 0);
        assert_eq!(
            DockWindowOwnerMode::try_from(0),
            Ok(DockWindowOwnerMode::Default)
        );
        assert_eq!(DockWindowOwnerMode::None.to_raw(), 1);
        assert_eq!(
            DockWindowOwnerMode::try_from(1),
            Ok(DockWindowOwnerMode::None)
        );
        assert_eq!(DockWindowOwnerMode::ParentWindow.to_raw(), 2);
        assert_eq!(
            DockWindowOwnerMode::try_from(2),
            Ok(DockWindowOwnerMode::ParentWindow)
        );
        assert_eq!(DockWindowOwnerMode::RootWindow.to_raw(), 3);
        assert_eq!(
            DockWindowOwnerMode::try_from(3),
            Ok(DockWindowOwnerMode::RootWindow)
        );
        assert_eq!(DockWindowOwnerMode::DockableWindow.to_raw(), 4);
        assert_eq!(
            DockWindowOwnerMode::try_from(4),
            Ok(DockWindowOwnerMode::DockableWindow)
        );
        assert_eq!(DockWindowOwnerMode::default().to_raw(), 0);
        assert_eq!(DockWindowOwnerMode::try_from(999), Err(999));
    }
}
