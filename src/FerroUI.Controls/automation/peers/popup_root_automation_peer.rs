use super::{
    AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl,
    WindowBaseAutomationPeer,
};
use crate::primitives::PopupRoot;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::cell::RefCell;
use std::rc::Rc;

/// An automation peer which represents a [`PopupRoot`].
#[repr(C)]
pub struct PopupRootAutomationPeer {
    base: WindowBaseAutomationPeer,
    opened_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    closed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(PopupRootAutomationPeer: WindowBaseAutomationPeer);
ferroui_base::ferro_class_info!(PopupRootAutomationPeer {});

impl FerroObjectImpl for PopupRootAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let owner = this.popup_root();
        if owner.is_visible() {
            this.start_tracking_focus();
        } else {
            let weak = this.to_ref().downgrade();
            let subscription = owner.opened(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_opened();
                }
            });
            *this.opened_subscription.borrow_mut() = Some(subscription);
        }

        let weak = this.to_ref().downgrade();
        let subscription = owner.closed(move || {
            if let Some(this) = weak.upgrade() {
                this.on_closed();
            }
        });
        *this.closed_subscription.borrow_mut() = Some(subscription);
    }
}

impl ControlAutomationPeerImpl for PopupRootAutomationPeer {}

impl AutomationPeerImpl for PopupRootAutomationPeer {
    fn is_content_element_core(_this: &Self) -> bool {
        false
    }

    fn is_control_element_core(_this: &Self) -> bool {
        false
    }

    fn get_parent_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        let parent = Self::parent_get_parent_core(this);
        parent
    }
}

impl PopupRootAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &PopupRoot) -> Self {
        Self {
            base: WindowBaseAutomationPeer::construct(owner),
            opened_subscription: RefCell::new(None),
            closed_subscription: RefCell::new(None),
        }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &PopupRoot) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    fn popup_root(&self) -> Ref<PopupRoot> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a popup root.")
    }

    fn on_opened(&self) {
        let subscription = self.opened_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        self.start_tracking_focus();
    }

    fn on_closed(&self) {
        let subscription = self.closed_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        self.stop_tracking_focus();
    }
}
