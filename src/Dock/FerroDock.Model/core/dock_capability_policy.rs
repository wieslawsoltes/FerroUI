// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{DockCapability, IDockCapabilityPolicy};
use std::cell::Cell;
use std::rc::Rc;

/// Default root/dock capability policy. Unset values inherit lower-precedence values.
#[derive(Debug, Default)]
pub struct DockCapabilityPolicy {
    can_close: Cell<Option<bool>>,
    can_pin: Cell<Option<bool>>,
    can_float: Cell<Option<bool>>,
    can_drag: Cell<Option<bool>>,
    can_drop: Cell<Option<bool>>,
    can_dock_as_document: Cell<Option<bool>>,
}

/// The virtual capability query of a policy, separate from its property contract.
///
/// Custom policies compose a [`DockCapabilityPolicy`], delegate its properties,
/// and override `get`. Policy references use `Rc<dyn DockCapabilityPolicyImpl>`
/// to preserve virtual dispatch.
pub trait DockCapabilityPolicyImpl: IDockCapabilityPolicy {
    /// Gets the configured value for a capability, or `None` when unset.
    fn get(&self, capability: DockCapability) -> Option<bool> {
        match capability {
            DockCapability::Close => self.can_close(),
            DockCapability::Pin => self.can_pin(),
            DockCapability::Float => self.can_float(),
            DockCapability::Drag => self.can_drag(),
            DockCapability::Drop => self.can_drop(),
            DockCapability::DockAsDocument => self.can_dock_as_document(),
        }
    }

    /// Queries a numeric capability value, returning `None` for an unknown value.
    ///
    /// This is the checked boundary for callers that receive numeric values.
    fn get_raw(&self, capability: i32) -> Option<bool> {
        DockCapability::try_from(capability)
            .ok()
            .and_then(|value| self.get(value))
    }
}

impl DockCapabilityPolicyImpl for DockCapabilityPolicy {}

impl PartialEq for dyn DockCapabilityPolicyImpl {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

macro_rules! policy_properties {
    ($($name:ident, $setter:ident, $doc:literal);+ $(;)?) => {
        impl DockCapabilityPolicy {
            $(
                #[doc = concat!("Gets the optional ", $doc, " capability.")]
                pub fn $name(&self) -> Option<bool> { self.$name.get() }
                #[doc = concat!("Sets the optional ", $doc, " capability.")]
                pub fn $setter(&self, value: Option<bool>) { self.$name.set(value); }
            )+
        }
        impl IDockCapabilityPolicy for DockCapabilityPolicy {
            $(
                fn $name(&self) -> Option<bool> { self.$name() }
                fn $setter(&self, value: Option<bool>) { self.$setter(value); }
            )+
        }
    };
}

policy_properties! {
    can_close, set_can_close, "close";
    can_pin, set_can_pin, "pin";
    can_float, set_can_float, "float";
    can_drag, set_can_drag, "drag";
    can_drop, set_can_drop, "drop";
    can_dock_as_document, set_can_dock_as_document, "dock-as-document";
}

impl DockCapabilityPolicy {
    /// Creates a shared policy with all values unset.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Gets the configured capability value.
    pub fn get(&self, capability: DockCapability) -> Option<bool> {
        DockCapabilityPolicyImpl::get(self, capability)
    }

    /// Gets the configured value for a numeric capability, or `None` if unknown.
    pub fn get_raw(&self, capability: i32) -> Option<bool> {
        DockCapabilityPolicyImpl::get_raw(self, capability)
    }
}

impl PartialEq for DockCapabilityPolicy {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! configured_value_test {
        ($test:ident, $capability:ident, $getter:ident, $setter:ident) => {
            #[test]
            fn $test() {
                let policy = DockCapabilityPolicy::new();
                let properties: Rc<dyn IDockCapabilityPolicy> = policy.clone();
                assert_eq!(properties.$getter(), None);
                for value in [Some(true), Some(false), None] {
                    properties.$setter(value);
                    assert_eq!(policy.$getter(), value);
                    assert_eq!(policy.get(DockCapability::$capability), value);
                }
                for raw in 0..6 {
                    if raw != DockCapability::$capability as i32 {
                        assert_eq!(policy.get_raw(raw), None);
                    }
                }
            }
        };
    }
    configured_value_test!(
        close_returns_configured_value,
        Close,
        can_close,
        set_can_close
    );
    configured_value_test!(pin_returns_configured_value, Pin, can_pin, set_can_pin);
    configured_value_test!(
        float_returns_configured_value,
        Float,
        can_float,
        set_can_float
    );
    configured_value_test!(drag_returns_configured_value, Drag, can_drag, set_can_drag);
    configured_value_test!(drop_returns_configured_value, Drop, can_drop, set_can_drop);
    configured_value_test!(
        dock_as_document_returns_configured_value,
        DockAsDocument,
        can_dock_as_document,
        set_can_dock_as_document
    );

    #[test]
    fn get_returns_null_for_unknown_capability() {
        let policy = DockCapabilityPolicy::new();
        policy.set_can_close(Some(true));
        assert_eq!(policy.get_raw(999), None);
        assert_eq!(policy.get_raw(-1), None);
    }

    #[test]
    fn reference_equality_and_shared_mutation_match_a_reference_class() {
        let policy = DockCapabilityPolicy::new();
        let handle = policy.clone();
        assert_eq!(policy, handle);
        assert_ne!(policy, DockCapabilityPolicy::new());
        handle.set_can_close(Some(false));
        assert_eq!(policy.can_close(), Some(false));
    }

    #[test]
    fn a_composed_policy_can_override_the_virtual_query() {
        struct CustomPolicy(DockCapabilityPolicy);
        macro_rules! delegate {
            ($($getter:ident, $setter:ident);+ $(;)?) => {
                impl IDockCapabilityPolicy for CustomPolicy {
                    $(
                        fn $getter(&self) -> Option<bool> { self.0.$getter() }
                        fn $setter(&self, value: Option<bool>) { self.0.$setter(value); }
                    )+
                }
            };
        }
        delegate! {
            can_close, set_can_close;
            can_pin, set_can_pin;
            can_float, set_can_float;
            can_drag, set_can_drag;
            can_drop, set_can_drop;
            can_dock_as_document, set_can_dock_as_document;
        }
        impl DockCapabilityPolicyImpl for CustomPolicy {
            fn get(&self, capability: DockCapability) -> Option<bool> {
                if capability == DockCapability::Close {
                    Some(false)
                } else {
                    self.0.get(capability)
                }
            }
        }
        let policy: Rc<dyn DockCapabilityPolicyImpl> =
            Rc::new(CustomPolicy(DockCapabilityPolicy::default()));
        policy.set_can_close(Some(true));
        assert_eq!(policy.can_close(), Some(true));
        assert_eq!(policy.get(DockCapability::Close), Some(false));
        assert_eq!(policy.get_raw(0), Some(false));
        policy.set_can_pin(Some(true));
        assert_eq!(policy.get(DockCapability::Pin), Some(true));
        assert_eq!(policy.get_raw(999), None);
    }
}
