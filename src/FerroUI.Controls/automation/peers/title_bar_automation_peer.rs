//! WinUI's `TitleBarAutomationPeer` (`controls/dev/TitleBar/TitleBarAutomationPeer.cpp` and `.h` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::{AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::TitleBar;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref};

/// The automation peer of a [`TitleBar`]: a title bar whose name is its title when the
/// application gives it none, as in WinUI.
#[repr(C)]
pub struct TitleBarAutomationPeer {
    base: ControlAutomationPeer,
}

ferro_class!(TitleBarAutomationPeer: ControlAutomationPeer);
ferro_class_info!(TitleBarAutomationPeer { markup: { namespace: "FerroUI.Automation.Peers" } });

impl FerroObjectImpl for TitleBarAutomationPeer {}

impl ControlAutomationPeerImpl for TitleBarAutomationPeer {}

impl AutomationPeerImpl for TitleBarAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::TitleBar
    }

    fn get_class_name_core(_this: &Self) -> String {
        "TitleBar".to_string()
    }

    fn get_name_core(this: &Self) -> Option<String> {
        Self::parent_get_name_core(this).filter(|name| !name.is_empty()).or_else(|| Some(this.owner().title()))
    }
}

impl TitleBarAutomationPeer {
    pub fn construct(owner: &TitleBar) -> Self {
        Self { base: ControlAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TitleBar) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// The title bar of the peer.
    pub fn owner(&self) -> Ref<TitleBar> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is a title bar.")
    }
}
