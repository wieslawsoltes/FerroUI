use super::{
    AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
    ControlAutomationPeerImplExt, WindowBaseAutomationPeer,
};
use crate::Window;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::cell::RefCell;
use std::rc::Rc;

/// An automation peer which represents a [`Window`].
#[repr(C)]
pub struct WindowAutomationPeer {
    base: WindowBaseAutomationPeer,
    opened_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    closed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(WindowAutomationPeer: WindowBaseAutomationPeer);
ferroui_base::ferro_class_info!(WindowAutomationPeer {});

impl FerroObjectImpl for WindowAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let owner = this.owner();
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

impl ControlAutomationPeerImpl for WindowAutomationPeer {
    fn get_children_core(this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        let base_children = Self::parent_get_children_core(this);
        let overlay_peer = this.owner().top_level_host().get_or_create_decorations_overlays_peer();

        let mut rv = vec![overlay_peer];
        if let Some(base_children) = base_children {
            if !base_children.is_empty() {
                rv.extend(base_children);
            }
        }
        Some(rv)
    }
}

impl AutomationPeerImpl for WindowAutomationPeer {
    fn get_name_core(this: &Self) -> Option<String> {
        this.owner().title()
    }
}

impl WindowAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Window) -> Self {
        Self {
            base: WindowBaseAutomationPeer::construct(owner),
            opened_subscription: RefCell::new(None),
            closed_subscription: RefCell::new(None),
        }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Window) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning window.
    pub fn owner(&self) -> Ref<Window> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a window.")
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
