use super::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::{ContentControl, TextBlock};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, FerroObjectImpl, Ref};

/// An automation peer which represents a [`ContentControl`].
#[repr(C)]
pub struct ContentControlAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(ContentControlAutomationPeer: ControlAutomationPeer);
ferroui_base::ferro_class_info!(ContentControlAutomationPeer {});

impl FerroObjectImpl for ContentControlAutomationPeer {}
impl ControlAutomationPeerImpl for ContentControlAutomationPeer {}

impl AutomationPeerImpl for ContentControlAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let result = Self::parent_get_name_core(this);

        if !is_null_or_white_space(&result) {
            result
        } else {
            let owner = this.owner();
            let child_control = owner.presenter().and_then(|presenter| presenter.child());
            let child_peer = child_control.as_ref().map(|child| ControlAutomationPeer::create_peer_for_element(child));
            let child_name = child_control
                .as_ref()
                .and_then(|child| child.cast::<TextBlock>())
                .and_then(|text_block| text_block.text())
                .or_else(|| child_peer.map(|peer| peer.get_name()));

            if !is_null_or_white_space(&child_name) {
                child_name
            } else {
                owner.content().map(|content| ValueTypes::to_display_string(Some(&content)))
            }
        }
    }

    fn get_help_text_core(this: &Self) -> Option<String> {
        let child_control = this.owner().presenter().and_then(|presenter| presenter.child());
        let child_peer = child_control.as_ref().map(|child| ControlAutomationPeer::create_peer_for_element(child));

        Self::parent_get_help_text_core(this).or_else(|| child_peer.map(|peer| peer.get_help_text()))
    }

    fn is_content_element_core(_this: &Self) -> bool {
        false
    }

    fn is_control_element_core(_this: &Self) -> bool {
        false
    }
}

fn is_null_or_white_space(value: &Option<String>) -> bool {
    value.as_deref().is_none_or(|value| value.chars().all(char::is_whitespace))
}

impl ContentControlAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    /// The class has no public constructor: it is the base of the peers of
    /// the content control classes.
    pub fn construct(owner: &ContentControl) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Gets the owning content control.
    pub fn owner(&self) -> Ref<ContentControl> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a content control.")
    }
}
