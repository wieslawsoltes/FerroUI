//! WinUI's `ProgressRingAutomationPeer` (`controls/dev/ProgressRing/ProgressRingAutomationPeer.cpp`
//! and `.h` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT,
//! Copyright (c) Microsoft Corporation, see NOTICE.md).

use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer,
    ControlAutomationPeerImpl,
};
use crate::automation::provider::{IRangeValueProvider, ProviderAdapter};
use crate::automation::{AutomationElementIdentifiers, ElementNotEnabledException, RangeValuePatternIdentifiers};
use crate::ProgressRing;
use ferroui_base::data::core::ValueType;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Ref,
};
use std::rc::Rc;

// The texts WinUI takes from its localized resources (`controls/dev/ProgressRing/Strings/en-us/
// Resources.resw`), in English: FerroUI has no localized resources of its framework.
const PROGRESS_RING_NAME: &str = "ProgressRing";
const PROGRESS_RING_INDETERMINATE_STATUS: &str = "Busy";

/// The automation peer of a [`ProgressRing`]: a progress bar with the range value pattern while
/// the ring is determinate, as in WinUI. The name of an active indeterminate ring starts with
/// "Busy".
#[repr(C)]
pub struct ProgressRingAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(ProgressRingAutomationPeer: ControlAutomationPeer);
ferro_class_info!(ProgressRingAutomationPeer {
    interfaces: [Rc<dyn IRangeValueProvider> => ProviderAdapter::as_range_value_provider],
    markup: { namespace: "FerroUI.Automation.Peers" },
});

impl FerroObjectImpl for ProgressRingAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let _subscription = this.owner().property_changed(move |change| {
            if let Some(peer) = weak.upgrade() {
                peer.owner_property_changed(change);
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

    fn get_localized_control_type_core(_this: &Self) -> String {
        PROGRESS_RING_NAME.to_string()
    }

    /// ProgressRingAutomationPeer::GetNameCore: "Busy" before the name of an active indeterminate
    /// ring.
    fn get_name_core(this: &Self) -> Option<String> {
        let name = Self::parent_get_name_core(this);
        let owner = this.owner();
        if owner.is_active() && owner.is_indeterminate() {
            Some(match name.filter(|name| !name.is_empty()) {
                Some(name) => format!("{PROGRESS_RING_INDETERMINATE_STATUS} {name}"),
                None => PROGRESS_RING_INDETERMINATE_STATUS.to_string(),
            })
        } else {
            name
        }
    }

    /// ProgressRingAutomationPeer::GetPatternCore: no range value while indeterminate.
    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        if provider_type == ValueType::of::<Rc<dyn IRangeValueProvider>>() && this.owner().is_indeterminate() {
            return None;
        }
        Self::parent_get_provider_core(this, provider_type)
    }
}

impl ProgressRingAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &ProgressRing) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &ProgressRing) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning progress ring.
    pub fn owner(&self) -> Ref<ProgressRing> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a progress ring.")
    }

    /// Raises the changes of the range and of the name of the ring.
    fn owner_property_changed(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        let property = change.property();
        if property == ProgressRing::is_active_property().as_property()
            || property == ProgressRing::is_indeterminate_property().as_property()
        {
            self.raise_property_changed_event(
                AutomationElementIdentifiers::name_property(),
                None,
                Some(Rc::new(self.get_name()) as BoxedValue),
            );
            return;
        }
        let owner = self.owner();
        let (automation_property, new) = if property == ProgressRing::minimum_property().as_property() {
            (RangeValuePatternIdentifiers::minimum_property(), owner.minimum())
        } else if property == ProgressRing::maximum_property().as_property() {
            (RangeValuePatternIdentifiers::maximum_property(), owner.maximum())
        } else if property == ProgressRing::value_property().as_property() {
            (RangeValuePatternIdentifiers::value_property(), owner.value())
        } else {
            return;
        };
        // The effective value of the ring, after its coercion.
        self.raise_property_changed_event(
            automation_property,
            change.get_old_value::<f64>().map(|old| Rc::new(old) as BoxedValue),
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

    /// WinUI's provider is read-only and still sets the value (ProgressRingAutomationPeer::SetValue).
    fn set_value(&self, value: f64) -> Result<(), ElementNotEnabledException> {
        self.0.owner().set_range_value(value);
        Ok(())
    }
}
