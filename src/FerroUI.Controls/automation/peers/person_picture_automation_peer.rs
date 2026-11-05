//! WinUI's `PersonPictureAutomationPeer` (`controls/dev/PersonPicture/PersonPictureAutomationPeer.cpp`
//! and `.h` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright
//! (c) Microsoft Corporation, see NOTICE.md).

use super::{AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::PersonPicture;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref};

/// The automation peer of a [`PersonPicture`]: text, whose name is the automation name the picture
/// sets (the person and the badge), as in WinUI.
#[repr(C)]
pub struct PersonPictureAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(PersonPictureAutomationPeer: ControlAutomationPeer);
ferro_class_info!(PersonPictureAutomationPeer { markup: { namespace: "FerroUI.Automation.Peers" } });
ferroui_base::ferro_impl_classes!(PersonPictureAutomationPeer: FerroObjectImpl);

impl ControlAutomationPeerImpl for PersonPictureAutomationPeer {}

impl AutomationPeerImpl for PersonPictureAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Text
    }

    fn get_class_name_core(_this: &Self) -> String {
        "PersonPicture".to_owned()
    }
}

impl PersonPictureAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &PersonPicture) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &PersonPicture) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}
