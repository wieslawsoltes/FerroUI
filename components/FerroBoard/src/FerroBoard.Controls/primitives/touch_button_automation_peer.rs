//! Native accessibility for the source's focus-neutral Grid button.
//! Uses FerroUI's existing peer/provider contracts; the control remains nonfocusable.
use super::TouchButton;
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
pub(super) struct TouchButtonAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(TouchButtonAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TouchButtonAutomationPeer {
    interfaces: [Rc<dyn IInvokeProvider> => TouchButtonAutomationPeer::as_invoke_provider]
});
impl FerroObjectImpl for TouchButtonAutomationPeer {}
// Preserve the framework's child peers and invalidation, including custom SetContent controls.
impl ControlAutomationPeerImpl for TouchButtonAutomationPeer {}
impl AutomationPeerImpl for TouchButtonAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Button
    }
}
struct TouchButtonInvokeProvider(Ref<TouchButtonAutomationPeer>);
impl IInvokeProvider for TouchButtonInvokeProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledError> {
        // ControlAutomationPeer holds its owner weakly. Check expiration before inherited
        // owner-dependent queries so a retained platform provider cannot revive/panic on it.
        let owner = self
            .0
            .try_owner()
            .and_then(|owner| owner.cast::<TouchButton>())
            .ok_or_else(|| ElementNotEnabledError::with_message("Button no longer exists."))?;
        self.0.ensure_enabled()?;
        owner.invoke_click();
        Ok(())
    }
}
impl TouchButtonAutomationPeer {
    fn as_invoke_provider(peer: Ref<Self>) -> Rc<dyn IInvokeProvider> {
        Rc::new(TouchButtonInvokeProvider(peer))
    }
    fn construct(owner: &TouchButton) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
        }
    }
    pub(super) fn new(owner: &TouchButton) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
