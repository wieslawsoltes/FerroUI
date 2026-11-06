//! Native accessibility press action for a computed key; engine routing belongs to KeysView.
use super::KeyView;
use ferroui_base::{FerroObjectImpl, Ref, ferro_class, instantiate};
use ferroui_controls::automation::{
    ElementNotEnabledError,
    peers::{
        AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer,
        ControlAutomationPeerImpl,
    },
    provider::IInvokeProvider,
};
use std::rc::Rc;

#[repr(C)]
pub(super) struct KeyAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(KeyAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(KeyAutomationPeer {
    interfaces: [Rc<dyn IInvokeProvider> => KeyAutomationPeer::as_invoke_provider]
});
impl FerroObjectImpl for KeyAutomationPeer {}
impl ControlAutomationPeerImpl for KeyAutomationPeer {}
impl AutomationPeerImpl for KeyAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Button
    }
    fn is_enabled_core(this: &Self) -> bool {
        this.try_owner()
            .and_then(|owner| owner.cast::<KeyView>())
            .is_some_and(|owner| owner.is_effectively_enabled() && owner.key().is_enabled())
    }
}
struct KeyInvokeProvider(Ref<KeyAutomationPeer>);
impl IInvokeProvider for KeyInvokeProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledError> {
        let owner = self
            .0
            .try_owner()
            .and_then(|owner| owner.cast::<KeyView>())
            .ok_or_else(|| {
                ElementNotEnabledError::with_message("Keyboard key no longer exists.")
            })?;
        self.0.ensure_enabled()?;
        owner.invoke()
    }
}
impl KeyAutomationPeer {
    fn as_invoke_provider(peer: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(KeyInvokeProvider(peer))
    }
    pub(super) fn new(owner: &KeyView) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
}
