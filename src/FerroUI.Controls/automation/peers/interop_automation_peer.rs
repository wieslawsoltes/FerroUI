use super::{AutomationControlType, AutomationPeer, AutomationPeerImpl};
use crate::platform::IPlatformHandle;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Rect, Ref, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

/// Represents the root of a native control automation tree hosted by a
/// [`NativeControlHost`](crate::NativeControlHost).
///
/// This peer should be special-cased in the platform backend, as it represents a native control
/// and hence none of the standard automation peer methods are applicable.
///
/// Internal in the reference: for the framework and the platform backends.
#[repr(C)]
pub struct InteropAutomationPeer {
    base: AutomationPeer,
    parent: RefCell<Option<WeakRef<AutomationPeer>>>,
    native_control_handle: Rc<dyn IPlatformHandle>,
}

ferro_class!(InteropAutomationPeer: AutomationPeer);
ferroui_base::ferro_class_info!(InteropAutomationPeer {});

fn not_implemented() -> ! {
    panic!("The method or operation is not implemented.")
}

impl FerroObjectImpl for InteropAutomationPeer {}

impl AutomationPeerImpl for InteropAutomationPeer {
    fn bring_into_view_core(_this: &Self) {
        not_implemented()
    }

    fn get_accelerator_key_core(_this: &Self) -> Option<String> {
        not_implemented()
    }

    fn get_access_key_core(_this: &Self) -> Option<String> {
        not_implemented()
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        not_implemented()
    }

    fn get_automation_id_core(_this: &Self) -> Option<String> {
        not_implemented()
    }

    fn get_bounding_rectangle_core(_this: &Self) -> Rect {
        not_implemented()
    }

    fn get_class_name_core(_this: &Self) -> String {
        not_implemented()
    }

    fn get_labeled_by_core(_this: &Self) -> Option<Ref<AutomationPeer>> {
        not_implemented()
    }

    fn get_name_core(_this: &Self) -> Option<String> {
        not_implemented()
    }

    fn get_help_text_core(_this: &Self) -> Option<String> {
        not_implemented()
    }

    fn get_placeholder_text_core(_this: &Self) -> Option<String> {
        not_implemented()
    }

    fn get_or_create_children_core(_this: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        not_implemented()
    }

    fn get_parent_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.parent.borrow().as_ref().and_then(|parent| parent.upgrade())
    }

    fn has_keyboard_focus_core(_this: &Self) -> bool {
        not_implemented()
    }

    fn is_content_element_core(_this: &Self) -> bool {
        not_implemented()
    }

    fn is_control_element_core(_this: &Self) -> bool {
        not_implemented()
    }

    fn is_enabled_core(_this: &Self) -> bool {
        not_implemented()
    }

    fn is_keyboard_focusable_core(_this: &Self) -> bool {
        not_implemented()
    }

    fn set_focus_core(_this: &Self) {
        not_implemented()
    }

    fn show_context_menu_core(_this: &Self) -> bool {
        not_implemented()
    }

    fn try_set_parent(this: &Self, parent: Option<Ref<AutomationPeer>>) -> bool {
        *this.parent.borrow_mut() = parent.map(|parent| parent.downgrade());
        true
    }
}

impl InteropAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(native_control_handle: Rc<dyn IPlatformHandle>) -> Self {
        Self { base: AutomationPeer::construct(), parent: RefCell::new(None), native_control_handle }
    }

    /// Initializes a new peer of the native control with the handle
    /// `native_control_handle`.
    pub fn new(native_control_handle: Rc<dyn IPlatformHandle>) -> Ref<Self> {
        instantiate(Self::construct(native_control_handle))
    }

    /// Gets the handle of the native control.
    pub fn native_control_handle(&self) -> Rc<dyn IPlatformHandle> {
        self.native_control_handle.clone()
    }
}
