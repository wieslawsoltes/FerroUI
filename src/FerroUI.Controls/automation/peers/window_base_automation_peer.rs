use super::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{IEmbeddedRootProvider, IRootProvider, ProviderAdapter};
use crate::platform::ITopLevelImpl;
use crate::{Control, TopLevel, WindowBase};
use ferroui_base::input::{InputElement, KeyboardDevice};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Point, Ref, Visual, WeakRef};
use std::cell::RefCell;
use std::rc::Rc;

/// An automation peer which represents a [`WindowBase`]: the root of an
/// automation tree.
#[repr(C)]
pub struct WindowBaseAutomationPeer {
    base: ControlAutomationPeer,
    // The focused control is a descendant of the owner; it is held weakly
    // like the owner.
    focus: RefCell<Option<WeakRef<Control>>>,
    focus_changed: HandlerList<dyn Fn()>,
    keyboard_device_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(WindowBaseAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(WindowBaseAutomationPeer { interfaces: [Rc<dyn IRootProvider> => ProviderAdapter::as_root_provider] });

impl FerroObjectImpl for WindowBaseAutomationPeer {}

impl ControlAutomationPeerImpl for WindowBaseAutomationPeer {
    fn get_visual_parent(_this: &Self) -> Option<Ref<Visual>> {
        None
    }
}

impl AutomationPeerImpl for WindowBaseAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Window
    }
}

impl IRootProvider for ProviderAdapter<WindowBaseAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn platform_impl(&self) -> Option<Rc<dyn ITopLevelImpl>> {
        self.0.platform_impl()
    }

    fn get_focus(&self) -> Option<Ref<AutomationPeer>> {
        self.0.get_focus()
    }

    fn get_peer_from_point(&self, p: Point) -> Option<Ref<AutomationPeer>> {
        self.0.get_peer_from_point(p)
    }

    fn focus_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.focus_changed(move || handler())
    }
}

impl WindowBaseAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &WindowBase) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
            focus: RefCell::new(None),
            focus_changed: HandlerList::new(),
            keyboard_device_subscription: RefCell::new(None),
        }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &WindowBase) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning window.
    pub fn owner(&self) -> Ref<WindowBase> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a window base.")
    }

    /// Gets the platform implementation of the owner (the root provider
    /// contract).
    pub fn platform_impl(&self) -> Option<Rc<dyn ITopLevelImpl>> {
        let owner = self.owner();
        TopLevel::platform_impl(&owner)
    }

    /// Raised when the focus changes (the root provider contract).
    pub fn focus_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.focus_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.focus_changed.remove(token);
            }
        })
    }

    /// Gets the peer of the currently focused element (the root provider
    /// contract).
    pub fn get_focus(&self) -> Option<Ref<AutomationPeer>> {
        self.focus().map(|focus| self.get_or_create(&focus))
    }

    /// Gets the peer of the element at the specified point, expressed in
    /// top-level coordinates (the root provider contract).
    pub fn get_peer_from_point(&self, p: Point) -> Option<Ref<AutomationPeer>> {
        let hit = self.owner().get_visual_at(p).and_then(|visual| visual.find_ancestor_of_type::<Control>(true))?;

        let mut peer = self.get_or_create(&hit);
        let this_peer: Ref<AutomationPeer> = self.to_ref().upcast();

        while peer != this_peer {
            let Some(embedded) = peer.get_provider::<dyn IEmbeddedRootProvider>() else { break };
            let Some(embedded_hit) = embedded.get_peer_from_point(p) else { break };
            peer = embedded_hit;
        }

        Some(peer)
    }

    /// Starts following the focused element of the keyboard device.
    pub fn start_tracking_focus(&self) {
        if let Some(keyboard_device) = KeyboardDevice::instance() {
            let weak = self.to_ref().downgrade();
            let subscription = keyboard_device.property_changed(move |property_name| {
                if let Some(this) = weak.upgrade() {
                    this.keyboard_device_property_changed(property_name);
                }
            });
            *self.keyboard_device_subscription.borrow_mut() = Some(subscription);
            self.on_focus_changed(keyboard_device.focused_element());
        }
    }

    /// Stops following the focused element of the keyboard device.
    pub fn stop_tracking_focus(&self) {
        if KeyboardDevice::instance().is_some() {
            let subscription = self.keyboard_device_subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
    }

    fn focus(&self) -> Option<Ref<Control>> {
        self.focus.borrow().as_ref().and_then(|focus| focus.upgrade())
    }

    fn on_focus_changed(&self, focus: Option<Ref<InputElement>>) {
        let old_focus = self.focus();
        let c = focus.and_then(|focus| focus.cast::<Control>());
        let owner = self.owner();

        let new_focus = c.filter(|c| owner.is_visual_ancestor_of(c));
        *self.focus.borrow_mut() = new_focus.as_ref().map(|focus| focus.downgrade());

        if new_focus != old_focus {
            // As in the reference, the peer of the newly focused control is
            // created here although the event does not carry it.
            let _peer = new_focus.as_ref().map(|focus| self.get_or_create(focus));
            for (_, handler) in self.focus_changed.snapshot().iter() {
                handler();
            }
        }
    }

    fn keyboard_device_property_changed(&self, property_name: &str) {
        if property_name == "FocusedElement" {
            let keyboard_device = KeyboardDevice::instance().expect("The keyboard device exists.");
            self.on_focus_changed(keyboard_device.focused_element());
        }
    }
}
