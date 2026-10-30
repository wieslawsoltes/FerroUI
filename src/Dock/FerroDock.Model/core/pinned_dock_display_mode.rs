// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Defines how pinned dock previews are displayed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PinnedDockDisplayMode(i32);

// Source enum member names are part of the public API.
#[allow(non_upper_case_globals)]
impl PinnedDockDisplayMode {
    /// Preview overlays the content when open.
    pub const Overlay: Self = Self(0);
    /// Preview takes layout space when open.
    pub const Inline: Self = Self(1);
    /// Retains an original numeric enum cast, including undeclared values.
    pub const fn from_raw(value: i32) -> Self {
        Self(value)
    }
    /// Gets the original signed underlying value.
    pub const fn to_raw(self) -> i32 {
        self.0
    }
}

impl PinnedDockDisplayMode {
    /// Original declaration used by UI-owned source-generated metadata.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("pinned_dock_display_mode.rs");
}

impl TryFrom<i32> for PinnedDockDisplayMode {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Overlay),
            1 => Ok(Self::Inline),
            other => Err(other),
        }
    }
}

impl std::fmt::Display for PinnedDockDisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Overlay => f.write_str("Overlay"),
            Self::Inline => f.write_str("Inline"),
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
            if fields[0] != "PINNED" {
                continue;
            }
            let value = PinnedDockDisplayMode::from_raw(fields[1].parse().unwrap());
            assert_eq!(value.to_string(), fields[2]);
            count += 1;
        }
        assert_eq!(count, 141);
    }

    #[test]
    fn numeric_casts_keep_original_signed_values_and_storage() {
        use std::hash::{Hash, Hasher};
        for raw in [i32::MIN, -19, -1, 0, 1, 2, 3, 4, 5, 999, i32::MAX] {
            let value = PinnedDockDisplayMode::from_raw(raw);
            assert_eq!(value.to_raw(), raw);
            let mut actual = std::collections::hash_map::DefaultHasher::new();
            let mut expected = std::collections::hash_map::DefaultHasher::new();
            value.hash(&mut actual);
            raw.hash(&mut expected);
            assert_eq!(actual.finish(), expected.finish());
        }
        assert_eq!(
            std::mem::size_of::<PinnedDockDisplayMode>(),
            std::mem::size_of::<i32>()
        );
        assert_eq!(
            std::mem::align_of::<PinnedDockDisplayMode>(),
            std::mem::align_of::<i32>()
        );
    }

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(PinnedDockDisplayMode::Overlay.to_raw(), 0);
        assert_eq!(
            PinnedDockDisplayMode::try_from(0),
            Ok(PinnedDockDisplayMode::Overlay)
        );
        assert_eq!(PinnedDockDisplayMode::Inline.to_raw(), 1);
        assert_eq!(
            PinnedDockDisplayMode::try_from(1),
            Ok(PinnedDockDisplayMode::Inline)
        );
        assert_eq!(PinnedDockDisplayMode::default().to_raw(), 0);
        assert_eq!(PinnedDockDisplayMode::try_from(999), Err(999));
    }
}
