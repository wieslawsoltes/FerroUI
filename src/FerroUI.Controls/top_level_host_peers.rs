//! The automation peers part of the host of a top-level.

use crate::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::top_level_host::TopLevelHost;
use crate::{ControlImpl, TopLevel};
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Rect, Ref, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

impl ControlImpl for TopLevelHost {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        TopLevelHostAutomationPeer::new(this).upcast()
    }
}

impl TopLevelHost {
    /// Gets the peer of the decoration layers of the host, creating it if
    /// it does not exist yet.
    pub fn get_or_create_decorations_overlays_peer(&self) -> Ref<AutomationPeer> {
        let existing = self.decorations_overlay_peer.borrow().clone();
        let peer = match existing {
            Some(peer) => peer,
            None => {
                let peer = DecorationsOverlaysAutomationPeer::new(self, self.top_level.borrow().clone());
                *self.decorations_overlay_peer.borrow_mut() = Some(peer.clone());
                peer
            }
        };
        peer.upcast()
    }
}

/// Automation peer that returns no children. The automation tree is managed
/// by the window automation peer, which directly includes decoration content.
/// Without this, connecting a peer would walk up through the host and
/// set the parent peer of the window to the peer of the host, breaking the root.
#[repr(C)]
struct TopLevelHostAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(TopLevelHostAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(TopLevelHostAutomationPeer {});

ferroui_base::ferro_impl_classes!(TopLevelHostAutomationPeer: FerroObjectImpl);
ferroui_base::ferro_impl_classes!(TopLevelHostAutomationPeer: AutomationPeerImpl);

impl ControlAutomationPeerImpl for TopLevelHostAutomationPeer {
    fn get_children_core(_this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        None
    }
}

impl TopLevelHostAutomationPeer {
    fn new(owner: &TopLevelHost) -> Ref<Self> {
        instantiate(Self { base: ControlAutomationPeer::construct(owner) })
    }
}

/// The peer of the decoration layers of the host of a top-level.
///
/// The host owns the peer, so the peer holds the host and the top-level
/// weakly.
#[repr(C)]
pub(crate) struct DecorationsOverlaysAutomationPeer {
    base: AutomationPeer,
    host: WeakRef<TopLevelHost>,
    top_level: WeakRef<TopLevel>,
    children: RefCell<Rc<Vec<Ref<AutomationPeer>>>>,
    children_valid: Cell<bool>,
}

ferro_class!(DecorationsOverlaysAutomationPeer: AutomationPeer);
ferroui_base::ferro_class_info!(DecorationsOverlaysAutomationPeer {});

ferroui_base::ferro_impl_classes!(DecorationsOverlaysAutomationPeer: FerroObjectImpl);

impl AutomationPeerImpl for DecorationsOverlaysAutomationPeer {
    fn bring_into_view_core(this: &Self) {
        this.top_level().get_or_create_automation_peer().bring_into_view()
    }

    fn get_accelerator_key_core(_this: &Self) -> Option<String> {
        None
    }

    fn get_access_key_core(_this: &Self) -> Option<String> {
        None
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Group
    }

    fn get_automation_id_core(_this: &Self) -> Option<String> {
        Some("FerroWindowChrome".to_owned())
    }

    fn get_bounding_rectangle_core(this: &Self) -> Rect {
        this.host().bounds()
    }

    fn get_or_create_children_core(this: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        if !this.children_valid.get() {
            let host = this.host();
            let layers = [
                host.fullscreen_popover.borrow().clone(),
                host.overlay.borrow().clone(),
                host.underlay.borrow().clone(),
            ];
            let new_children: Vec<Ref<AutomationPeer>> = layers
                .into_iter()
                .flatten()
                .filter(|c| c.is_visible())
                .map(|c| c.get_or_create_automation_peer())
                .collect();

            let children = this.children.borrow().clone();
            for peer in children.iter().filter(|peer| !new_children.contains(peer)) {
                peer.try_set_parent(None);
            }
            let this_peer: Ref<AutomationPeer> = this.to_ref().upcast();
            for peer in &new_children {
                peer.try_set_parent(Some(this_peer.clone()));
            }
            *this.children.borrow_mut() = Rc::new(new_children);
            this.children_valid.set(true);
        }

        this.children.borrow().clone()
    }

    fn get_class_name_core(_this: &Self) -> String {
        "WindowChrome".to_owned()
    }

    fn get_labeled_by_core(_this: &Self) -> Option<Ref<AutomationPeer>> {
        None
    }

    fn get_name_core(_this: &Self) -> Option<String> {
        Some("WindowChrome".to_owned())
    }

    fn get_parent_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        Some(this.top_level().get_or_create_automation_peer())
    }

    fn has_keyboard_focus_core(this: &Self) -> bool {
        let children = this.children.borrow().clone();
        children.iter().any(|x| x.has_keyboard_focus())
    }

    fn is_keyboard_focusable_core(_this: &Self) -> bool {
        false
    }

    fn is_content_element_core(_this: &Self) -> bool {
        false
    }

    fn is_control_element_core(_this: &Self) -> bool {
        true
    }

    fn is_enabled_core(_this: &Self) -> bool {
        true
    }

    fn set_focus_core(_this: &Self) {}

    fn show_context_menu_core(_this: &Self) -> bool {
        false
    }

    fn try_set_parent(_this: &Self, _parent: Option<Ref<AutomationPeer>>) -> bool {
        false
    }
}

impl DecorationsOverlaysAutomationPeer {
    fn new(host: &TopLevelHost, top_level: WeakRef<TopLevel>) -> Ref<Self> {
        instantiate(Self {
            base: AutomationPeer::construct(),
            host: host.to_ref().downgrade(),
            top_level,
            children: RefCell::new(Rc::new(Vec::new())),
            children_valid: Cell::new(false),
        })
    }

    fn host(&self) -> Ref<TopLevelHost> {
        self.host.upgrade().expect("The host of the automation peer no longer exists.")
    }

    fn top_level(&self) -> Ref<TopLevel> {
        self.top_level.upgrade().expect("The top-level of the automation peer no longer exists.")
    }

    /// Invalidates the children of the peer and causes them to be read
    /// again from the decoration layers.
    pub(crate) fn invalidate_children(&self) {
        if self.children_valid.get() {
            self.children_valid.set(false);
            self.raise_children_changed_event();
        }
    }
}
