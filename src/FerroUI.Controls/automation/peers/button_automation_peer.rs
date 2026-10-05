use super::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ContentControlAutomationPeer,
    ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IInvokeProvider, ProviderAdapter};
use crate::automation::ElementNotEnabledException;
use crate::automation::peers::AutomationPeer;
use crate::Button;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// An automation peer which represents a [`Button`].
#[repr(C)]
pub struct ButtonAutomationPeer {
    base: ContentControlAutomationPeer,
}

ferro_class!(ButtonAutomationPeer: ContentControlAutomationPeer);
ferroui_base::ferro_class_info!(ButtonAutomationPeer { interfaces: [Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider] });

impl FerroObjectImpl for ButtonAutomationPeer {}
impl ControlAutomationPeerImpl for ButtonAutomationPeer {}

impl AutomationPeerImpl for ButtonAutomationPeer {
    fn get_accelerator_key_core(this: &Self) -> Option<String> {
        let mut result = Self::parent_get_accelerator_key_core(this);

        if result.as_deref().is_none_or(|result| result.chars().all(char::is_whitespace)) {
            result = this.owner().hot_key().map(|hot_key| hot_key.to_string());
        }

        result
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Button
    }

    fn is_content_element_core(_this: &Self) -> bool {
        true
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }
}

impl IInvokeProvider for ProviderAdapter<ButtonAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.0.invoke()
    }
}

impl ButtonAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Button) -> Self {
        Self { base: ContentControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Button) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning button.
    pub fn owner(&self) -> Ref<Button> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a button.")
    }

    /// Sends a request to activate the button.
    pub fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;
        self.owner().perform_click();
        Ok(())
    }
}
