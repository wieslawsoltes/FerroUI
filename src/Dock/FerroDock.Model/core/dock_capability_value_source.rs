// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Describes which layer resolved an effective capability value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DockCapabilityValueSource {
    /// The value came from dockable base flags (`Can*` properties).
    #[default]
    Dockable = 0,
    /// The value came from the root policy.
    RootPolicy = 1,
    /// The value came from the dock policy.
    DockPolicy = 2,
    /// The value came from dockable overrides.
    DockableOverride = 3,
}

impl DockCapabilityValueSource {
    /// Original declaration read by native metadata generation without copying members.
    #[doc(hidden)]
    pub const DECLARATION_SOURCE: &'static str = include_str!("dock_capability_value_source.rs");
}

impl TryFrom<i32> for DockCapabilityValueSource {
    type Error = i32;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Dockable),
            1 => Ok(Self::RootPolicy),
            2 => Ok(Self::DockPolicy),
            3 => Ok(Self::DockableOverride),
            other => Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_values_preserve_the_contract() {
        assert_eq!(DockCapabilityValueSource::Dockable as i32, 0);
        assert_eq!(
            DockCapabilityValueSource::try_from(0),
            Ok(DockCapabilityValueSource::Dockable)
        );
        assert_eq!(DockCapabilityValueSource::RootPolicy as i32, 1);
        assert_eq!(
            DockCapabilityValueSource::try_from(1),
            Ok(DockCapabilityValueSource::RootPolicy)
        );
        assert_eq!(DockCapabilityValueSource::DockPolicy as i32, 2);
        assert_eq!(
            DockCapabilityValueSource::try_from(2),
            Ok(DockCapabilityValueSource::DockPolicy)
        );
        assert_eq!(DockCapabilityValueSource::DockableOverride as i32, 3);
        assert_eq!(
            DockCapabilityValueSource::try_from(3),
            Ok(DockCapabilityValueSource::DockableOverride)
        );
        assert_eq!(DockCapabilityValueSource::default() as i32, 0);
        assert_eq!(DockCapabilityValueSource::try_from(999), Err(999));
    }
}
