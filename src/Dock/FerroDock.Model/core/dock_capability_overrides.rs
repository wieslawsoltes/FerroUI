// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{DockCapabilityPolicy, DockCapabilityPolicyImpl, IDockCapabilityPolicy};
use std::ops::Deref;
use std::rc::Rc;

/// Per-dockable capability overrides with the highest policy precedence.
/// Unset values inherit dock/root policy resolution.
#[derive(Debug, Default)]
pub struct DockCapabilityOverrides {
    base: DockCapabilityPolicy,
}

impl DockCapabilityOverrides {
    /// Creates a shared set of unset overrides.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Returns whether at least one override value has been configured.
    pub fn has_any_override(&self) -> bool {
        self.can_close().is_some()
            || self.can_pin().is_some()
            || self.can_float().is_some()
            || self.can_drag().is_some()
            || self.can_drop().is_some()
            || self.can_dock_as_document().is_some()
    }
}

impl Deref for DockCapabilityOverrides {
    type Target = DockCapabilityPolicy;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

macro_rules! delegate_properties {
    ($($getter:ident, $setter:ident);+ $(;)?) => {
        impl IDockCapabilityPolicy for DockCapabilityOverrides {
            $(
                fn $getter(&self) -> Option<bool> { self.base.$getter() }
                fn $setter(&self, value: Option<bool>) { self.base.$setter(value); }
            )+
        }
    };
}

delegate_properties! {
    can_close, set_can_close;
    can_pin, set_can_pin;
    can_float, set_can_float;
    can_drag, set_can_drag;
    can_drop, set_can_drop;
    can_dock_as_document, set_can_dock_as_document;
}

impl DockCapabilityPolicyImpl for DockCapabilityOverrides {}

impl PartialEq for DockCapabilityOverrides {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::DockCapability;

    #[test]
    fn has_any_override_is_false_when_all_values_are_null() {
        assert!(!DockCapabilityOverrides::new().has_any_override());
    }

    macro_rules! override_test {
        ($test:ident, $setter:ident, $capability:ident) => {
            #[test]
            fn $test() {
                let overrides = DockCapabilityOverrides::new();
                let policy: Rc<dyn DockCapabilityPolicyImpl> = overrides.clone();
                for value in [Some(false), Some(true)] {
                    policy.$setter(value);
                    assert!(overrides.has_any_override());
                    assert_eq!(policy.get(DockCapability::$capability), value);
                }
                policy.$setter(None);
                assert!(!overrides.has_any_override());
            }
        };
    }
    override_test!(
        close_override_is_present_even_when_false,
        set_can_close,
        Close
    );
    override_test!(pin_override_is_present_even_when_false, set_can_pin, Pin);
    override_test!(
        float_override_is_present_even_when_false,
        set_can_float,
        Float
    );
    override_test!(drag_override_is_present_even_when_false, set_can_drag, Drag);
    override_test!(drop_override_is_present_even_when_false, set_can_drop, Drop);
    override_test!(
        dock_as_document_override_is_present_even_when_false,
        set_can_dock_as_document,
        DockAsDocument
    );

    #[test]
    fn overrides_are_reference_objects() {
        let overrides = DockCapabilityOverrides::new();
        assert_eq!(overrides, overrides.clone());
        assert_ne!(overrides, DockCapabilityOverrides::new());
    }
}
