// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../../NOTICE.md.
// Adapted from the pinned ProgressRingAutomationPeer.cpp/.h contracts.

use crate::ProgressRing;
use crate::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
    ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IRangeValueProvider, ProviderAdapter};
use crate::automation::{
    AutomationElementIdentifiers, ElementNotEnabledException, RangeValuePatternIdentifiers,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::ValueType;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

#[repr(C)]
pub struct ProgressRingAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(ProgressRingAutomationPeer: ControlAutomationPeer);
ferro_class_info!(ProgressRingAutomationPeer {
    interfaces: [Rc<dyn IRangeValueProvider> => ProviderAdapter::as_range_value_provider],
});

impl FerroObjectImpl for ProgressRingAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        this.owner().property_changed(move |change| {
            if let Some(peer) = weak.upgrade() {
                peer.owner_changed(change);
            }
        });
    }
}
impl ControlAutomationPeerImpl for ProgressRingAutomationPeer {}
impl AutomationPeerImpl for ProgressRingAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::ProgressBar
    }
    fn get_class_name_core(_this: &Self) -> String {
        "ProgressRing".to_string()
    }
    fn get_localized_control_type_core(this: &Self) -> String {
        this.resource_string("ProgressRingName")
            .unwrap_or_else(|| "progress ring".to_string())
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let name = Self::parent_get_name_core(this);
        let owner = this.owner();
        if owner.is_active() && owner.is_indeterminate() {
            let status = this
                .resource_string("ProgressRingIndeterminateStatus")
                .unwrap_or_else(|| "Busy".to_string());
            Some(match name.filter(|name| !name.is_empty()) {
                Some(name) => format!("{status} {name}"),
                None => status,
            })
        } else {
            name
        }
    }
    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        if provider_type == ValueType::of::<Rc<dyn IRangeValueProvider>>()
            && this.owner().is_indeterminate()
        {
            return None;
        }
        Self::parent_get_provider_core(this, provider_type)
    }
}

impl ProgressRingAutomationPeer {
    pub fn new(owner: &ProgressRing) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    pub fn owner(&self) -> Ref<ProgressRing> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("ProgressRingAutomationPeer owns a ProgressRing")
    }
    fn resource_string(&self, key: &str) -> Option<String> {
        let owner = self.owner();
        owner
            .try_find_resource(
                &ResourceKey::from(key),
                owner.actual_theme_variant().as_ref(),
            )
            .flatten()?
            .downcast_ref::<String>()
            .cloned()
    }
    fn owner_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        let property = change.property();
        let automation_property = if property == ProgressRing::minimum_property().as_property() {
            RangeValuePatternIdentifiers::minimum_property()
        } else if property == ProgressRing::maximum_property().as_property() {
            RangeValuePatternIdentifiers::maximum_property()
        } else if property == ProgressRing::value_property().as_property() {
            RangeValuePatternIdentifiers::value_property()
        } else if property == ProgressRing::is_active_property().as_property()
            || property == ProgressRing::is_indeterminate_property().as_property()
        {
            self.raise_property_changed_event(
                AutomationElementIdentifiers::name_property(),
                None,
                Some(Rc::new(self.get_name()) as BoxedValue),
            );
            return;
        } else {
            return;
        };
        // Observer callbacks run after the owner's coercion. Use the effective
        // owner value so platform clients never receive an out-of-range payload.
        let owner = self.owner();
        let new = if property == ProgressRing::minimum_property().as_property() {
            owner.minimum()
        } else if property == ProgressRing::maximum_property().as_property() {
            owner.maximum()
        } else {
            owner.value()
        };
        self.raise_property_changed_event(
            automation_property,
            change
                .get_old_value::<f64>()
                .map(|v| Rc::new(v) as BoxedValue),
            Some(Rc::new(new) as BoxedValue),
        );
    }
}

impl IRangeValueProvider for ProviderAdapter<ProgressRingAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        true
    }
    fn minimum(&self) -> f64 {
        self.0.owner().minimum()
    }
    fn maximum(&self) -> f64 {
        self.0.owner().maximum()
    }
    fn value(&self) -> f64 {
        self.0.owner().value()
    }
    fn small_change(&self) -> f64 {
        f64::NAN
    }
    fn large_change(&self) -> f64 {
        f64::NAN
    }
    fn set_value(&self, value: f64) -> Result<(), ElementNotEnabledException> {
        // The pinned native provider advertises read-only but forwards SetValue.
        self.0.owner().set_range_value(value);
        Ok(())
    }
}
