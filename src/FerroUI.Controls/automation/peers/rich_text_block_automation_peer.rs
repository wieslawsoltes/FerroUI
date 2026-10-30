use super::{AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::RichTextBlock;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Ref};

/// An automation peer which represents a [`RichTextBlock`]: a text element named by
/// the text of its document. The UI Automation text pattern of WinUI's
/// `RichTextBlockAutomationPeer` comes with the text object model
/// (`docs/porting/winui-rich-text.md`, R8).
#[repr(C)]
pub struct RichTextBlockAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(RichTextBlockAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(RichTextBlockAutomationPeer {});
ferroui_base::ferro_impl_classes!(RichTextBlockAutomationPeer: FerroObjectImpl, ControlAutomationPeerImpl);

impl AutomationPeerImpl for RichTextBlockAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Text
    }

    fn get_name_core(this: &Self) -> Option<String> {
        Some(this.owner().document().text())
    }

    fn is_control_element_core(this: &Self) -> bool {
        // Not a control element when the view is part of a control template.
        this.owner().templated_parent().is_none() && Self::parent_is_control_element_core(this)
    }
}

impl RichTextBlockAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &RichTextBlock) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &RichTextBlock) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning rich text view.
    pub fn owner(&self) -> Ref<RichTextBlock> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a rich text view.")
    }
}
