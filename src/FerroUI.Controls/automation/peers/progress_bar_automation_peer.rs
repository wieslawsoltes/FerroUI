use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeerImpl, RangeBaseAutomationPeer,
    RangeBaseAutomationPeerImpl,
};
use crate::automation::provider::{IRangeValueProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledException;
use crate::primitives::RangeBase;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// An automation peer which represents a progress bar.
#[repr(C)]
pub struct ProgressBarAutomationPeer {
    base: RangeBaseAutomationPeer,
}

ferro_class!(ProgressBarAutomationPeer: RangeBaseAutomationPeer);
ferroui_base::ferro_class_info!(ProgressBarAutomationPeer {
    interfaces: [Rc<dyn IRangeValueProvider> => ProviderAdapter::as_range_value_provider]
});

impl FerroObjectImpl for ProgressBarAutomationPeer {}
impl ControlAutomationPeerImpl for ProgressBarAutomationPeer {}
impl RangeBaseAutomationPeerImpl for ProgressBarAutomationPeer {}

impl AutomationPeerImpl for ProgressBarAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "ProgressBar".to_owned()
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::ProgressBar
    }
}

/// The class implements the members `set_value`, `is_read_only`,
/// `large_change` and `small_change` of the contract again; the others are
/// those of the base class.
impl IRangeValueProvider for ProviderAdapter<ProgressBarAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    /// Request to set the value that this UI element is representing.
    fn set_value(&self, _val: f64) -> Result<(), ElementNotEnabledException> {
        panic!("ProgressBar is ReadOnly, value can't be set.");
    }

    /// Indicates that the value can only be read, not modified.
    fn is_read_only(&self) -> bool {
        true
    }

    /// Value of a large change.
    fn large_change(&self) -> f64 {
        f64::NAN
    }

    /// Value of a small change.
    fn small_change(&self) -> f64 {
        f64::NAN
    }

    fn minimum(&self) -> f64 {
        self.0.minimum()
    }

    fn maximum(&self) -> f64 {
        self.0.maximum()
    }

    fn value(&self) -> f64 {
        self.0.value()
    }
}

impl ProgressBarAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &RangeBase) -> Self {
        Self { base: RangeBaseAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &RangeBase) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
