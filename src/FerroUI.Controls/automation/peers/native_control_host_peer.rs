use super::{
    AutomationPeer, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl, InteropAutomationPeer,
};
use crate::NativeControlHost;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref};

/// An automation peer which represents a [`NativeControlHost`] (internal
/// in the reference).
#[repr(C)]
pub struct NativeControlHostPeer {
    base: ControlAutomationPeer,
}

ferro_class!(NativeControlHostPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(NativeControlHostPeer {});

impl FerroObjectImpl for NativeControlHostPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        // The host holds the handler and the peer; the handler holds the
        // peer weakly.
        let Some(owner) = this.owner().cast::<NativeControlHost>() else { return };
        let weak = this.to_ref().downgrade();
        owner.native_control_handle_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.on_native_control_handle_changed();
            }
        });
    }
}

impl ControlAutomationPeerImpl for NativeControlHostPeer {
    fn get_children_core(this: &Self) -> Option<Vec<Ref<AutomationPeer>>> {
        if let Some(host) = this.owner().cast::<NativeControlHost>() {
            if let Some(native_control_handle) = host.native_control_handle() {
                return Some(vec![InteropAutomationPeer::new(native_control_handle).upcast()]);
            }
        }
        None
    }
}

impl AutomationPeerImpl for NativeControlHostPeer {}

impl NativeControlHostPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &NativeControlHost) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &NativeControlHost) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    fn on_native_control_handle_changed(&self) {
        self.invalidate_children();
    }
}
