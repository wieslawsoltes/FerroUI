// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::{DockCapability, DockCapabilityValueSource};
use std::rc::Rc;

/// An immutable resolved capability value with its policy provenance and diagnostics.
#[derive(Debug)]
pub struct DockCapabilityEvaluation {
    capability: DockCapability,
    base_value: bool,
    root_policy_value: Option<bool>,
    dock_policy_value: Option<bool>,
    dockable_override_value: Option<bool>,
    effective_value: bool,
    effective_source: DockCapabilityValueSource,
    diagnostic_message: String,
}

impl DockCapabilityEvaluation {
    /// Stores a capability evaluation exactly as supplied, without re-resolving it.
    #[allow(clippy::too_many_arguments)] // Preserve the source constructor's complete contract.
    pub fn new(
        capability: DockCapability,
        base_value: bool,
        root_policy_value: Option<bool>,
        dock_policy_value: Option<bool>,
        dockable_override_value: Option<bool>,
        effective_value: bool,
        effective_source: DockCapabilityValueSource,
        diagnostic_message: String,
    ) -> Rc<Self> {
        Rc::new(Self {
            capability,
            base_value,
            root_policy_value,
            dock_policy_value,
            dockable_override_value,
            effective_value,
            effective_source,
            diagnostic_message,
        })
    }

    /// Gets the evaluated capability.
    pub fn capability(&self) -> DockCapability {
        self.capability
    }
    /// Gets the dockable's base capability flag.
    pub fn base_value(&self) -> bool {
        self.base_value
    }
    /// Gets the root policy value, if configured.
    pub fn root_policy_value(&self) -> Option<bool> {
        self.root_policy_value
    }
    /// Gets the dock policy value, if configured.
    pub fn dock_policy_value(&self) -> Option<bool> {
        self.dock_policy_value
    }
    /// Gets the dockable override value, if configured.
    pub fn dockable_override_value(&self) -> Option<bool> {
        self.dockable_override_value
    }
    /// Gets the resolved effective capability flag.
    pub fn effective_value(&self) -> bool {
        self.effective_value
    }
    /// Gets the source of the effective flag.
    pub fn effective_source(&self) -> DockCapabilityValueSource {
        self.effective_source
    }
    /// Gets the human-readable diagnostics supplied by the resolver.
    pub fn diagnostic_message(&self) -> &str {
        &self.diagnostic_message
    }
}

impl PartialEq for DockCapabilityEvaluation {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_retains_every_optional_value_and_the_supplied_diagnostics() {
        for raw in 0..6 {
            let capability = DockCapability::try_from(raw).unwrap();
            for root in [None, Some(false), Some(true)] {
                for dock in [None, Some(false), Some(true)] {
                    for override_value in [None, Some(false), Some(true)] {
                        let evaluation = DockCapabilityEvaluation::new(
                            capability,
                            true,
                            root,
                            dock,
                            override_value,
                            false,
                            DockCapabilityValueSource::DockableOverride,
                            "diagnostics".into(),
                        );
                        assert_eq!(evaluation.capability(), capability);
                        assert!(evaluation.base_value());
                        assert_eq!(evaluation.root_policy_value(), root);
                        assert_eq!(evaluation.dock_policy_value(), dock);
                        assert_eq!(evaluation.dockable_override_value(), override_value);
                        assert!(!evaluation.effective_value());
                        assert_eq!(
                            evaluation.effective_source(),
                            DockCapabilityValueSource::DockableOverride
                        );
                        assert_eq!(evaluation.diagnostic_message(), "diagnostics");
                    }
                }
            }
        }
    }

    #[test]
    fn equal_contents_do_not_change_reference_identity() {
        let create = || {
            DockCapabilityEvaluation::new(
                DockCapability::Close,
                false,
                None,
                None,
                None,
                true,
                DockCapabilityValueSource::Dockable,
                String::new(),
            )
        };
        let first = create();
        assert_eq!(first, first.clone());
        assert_ne!(first, create());
        assert_eq!(first.diagnostic_message(), "");
        assert!(first.effective_value());
        assert!(!first.base_value());
    }
}
