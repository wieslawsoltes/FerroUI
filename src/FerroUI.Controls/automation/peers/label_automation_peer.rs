use super::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::primitives::AccessText;
use crate::Label;
use ferroui_base::{ferro_class, instantiate, AnyValue, FerroObjectImpl, Ref};

/// An automation peer which represents a [`Label`].
#[repr(C)]
pub struct LabelAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(LabelAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(LabelAutomationPeer {});

impl FerroObjectImpl for LabelAutomationPeer {}
impl ControlAutomationPeerImpl for LabelAutomationPeer {}

impl AutomationPeerImpl for LabelAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "Text".to_owned()
    }

    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Text
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let owner = ControlAutomationPeer::owner(this).cast::<Label>().expect("The owner of the peer is a label.");
        let content = owner.content().and_then(|content| {
            let content: &dyn AnyValue = &*content;
            content.downcast_ref::<String>().cloned()
        });

        match content {
            Some(content) if !content.is_empty() => {
                Some(AccessText::remove_access_key_marker(Some(&content)).unwrap_or_default())
            }
            _ => Self::parent_get_name_core(this),
        }
    }
}

impl LabelAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &Label) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &Label) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
