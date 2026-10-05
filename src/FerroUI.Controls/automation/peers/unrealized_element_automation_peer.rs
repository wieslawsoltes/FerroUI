use super::{AutomationPeer, AutomationPeerImpl};
use ferroui_base::{ferro_class, FerroObjectImpl, Rect, Ref};
use std::rc::Rc;

/// An automation peer which represents an unrealized element.
#[repr(C)]
pub struct UnrealizedElementAutomationPeer {
    base: AutomationPeer,
}

ferro_class!(UnrealizedElementAutomationPeer: AutomationPeer);
ferroui_base::ferro_class_info!(UnrealizedElementAutomationPeer {});

impl FerroObjectImpl for UnrealizedElementAutomationPeer {}

impl AutomationPeerImpl for UnrealizedElementAutomationPeer {
    fn bring_into_view_core(this: &Self) {
        if let Some(parent) = this.get_parent() {
            parent.bring_into_view();
        }
    }

    fn get_bounding_rectangle_core(this: &Self) -> Rect {
        this.get_parent().map(|parent| parent.get_bounding_rectangle()).unwrap_or_default()
    }

    fn get_or_create_children_core(_this: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        Rc::new(Vec::new())
    }

    fn has_keyboard_focus_core(_this: &Self) -> bool {
        false
    }

    fn is_content_element_core(_this: &Self) -> bool {
        false
    }

    fn is_control_element_core(_this: &Self) -> bool {
        false
    }

    fn is_enabled_core(_this: &Self) -> bool {
        true
    }

    fn is_keyboard_focusable_core(_this: &Self) -> bool {
        false
    }

    fn set_focus_core(_this: &Self) {}

    fn show_context_menu_core(_this: &Self) -> bool {
        false
    }

    fn try_set_parent(_this: &Self, _parent: Option<Ref<AutomationPeer>>) -> bool {
        false
    }
}

impl UnrealizedElementAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    /// The class is abstract: it is the base of the peers of elements that
    /// have no control.
    pub fn construct() -> Self {
        Self { base: AutomationPeer::construct() }
    }

    /// Sets the parent of the peer.
    pub fn set_parent(&self, parent: Option<Ref<AutomationPeer>>) {
        self.try_set_parent(parent);
    }
}
