use super::{AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::primitives::Popup;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref};

/// An automation peer which represents a [`Popup`].
#[repr(C)]
pub struct PopupAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(PopupAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(PopupAutomationPeer {});

impl FerroObjectImpl for PopupAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        // The popup holds the handlers and the peer; the handlers hold the
        // peer weakly.
        let owner = this.popup();
        let weak = this.to_ref().downgrade();
        owner.opened(move || {
            if let Some(this) = weak.upgrade() {
                this.popup_opened_closed();
            }
        });
        let weak = this.to_ref().downgrade();
        owner.closed(move || {
            if let Some(this) = weak.upgrade() {
                this.popup_opened_closed();
            }
        });
    }
}

impl ControlAutomationPeerImpl for PopupAutomationPeer {
    fn get_children_core(this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        let host = this.popup();
        host.host().map(|popup_host| vec![this.get_or_create(&popup_host.as_control())])
    }
}

impl AutomationPeerImpl for PopupAutomationPeer {
    fn is_content_element_core(_this: &Self) -> bool {
        false
    }

    fn is_control_element_core(_this: &Self) -> bool {
        false
    }
}

impl PopupAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Popup) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Popup) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    fn popup(&self) -> Ref<Popup> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a popup.")
    }

    fn popup_opened_closed(&self) {
        // This is golden. We're following WPF's automation peer API here where the
        // parent of a peer is set when another peer returns it as a child. We want to
        // add the popup root as a child of the popup, so we need to return it as a
        // child right? Yeah except invalidating children doesn't automatically cause
        // UIA to re-read the children meaning that the parent doesn't get set. So the
        // MAIN MECHANISM FOR PARENTING CONTROLS IS BROKEN WITH THE ONLY AUTOMATION API
        // IT WAS WRITTEN FOR. Luckily WPF provides an escape-hatch by exposing the
        // TrySetParent API internally to work around this. We're exposing it publicly
        // to shame whoever came up with this abomination of an API.
        if let Some(popup_root) = self.get_popup_root() {
            popup_root.try_set_parent(Some(self.to_ref().upcast()));
        }
        self.invalidate_children();
    }

    fn get_popup_root(&self) -> Option<Ref<AutomationPeer>> {
        let popup_root = self.popup().host().map(|popup_host| popup_host.as_control());
        popup_root.map(|popup_root| self.get_or_create(&popup_root))
    }
}
