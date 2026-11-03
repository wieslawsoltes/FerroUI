use super::{AutomationPeer, AutomationPeerImpl, ControlAutomationPeerImpl, NoneAutomationPeer};
use crate::Control;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// The editor's layout presenter is a raw automation leaf. The owning editor
/// exposes its document, hyperlinks and embedded controls through one Text
/// provider; enumerating those controls here would duplicate them and replace
/// their document parent (including an enclosing hyperlink).
#[repr(C)]
pub struct RichEditBoxTextPresenterAutomationPeer {
    base: NoneAutomationPeer,
}

ferro_class!(RichEditBoxTextPresenterAutomationPeer: NoneAutomationPeer);
ferroui_base::ferro_class_info!(RichEditBoxTextPresenterAutomationPeer {});

ferroui_base::ferro_impl_classes!(RichEditBoxTextPresenterAutomationPeer: FerroObjectImpl, AutomationPeerImpl);
ferroui_base::ferro_overrides! { impl ControlAutomationPeerImpl for RichEditBoxTextPresenterAutomationPeer {
    fn get_children_core(_this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        None
    }
} }

impl RichEditBoxTextPresenterAutomationPeer {
    pub fn construct(owner: &Control) -> Self {
        Self {
            base: NoneAutomationPeer::construct(owner),
        }
    }

    pub fn new(owner: &Control) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
