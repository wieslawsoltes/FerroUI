//! WinUI's `InfoBarAutomationPeer` (`controls/dev/InfoBar/InfoBarAutomationPeer.cpp` and `.h` of
//! microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::{AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt, ControlAutomationPeer, ControlAutomationPeerImpl};
use crate::automation::{AutomationElementIdentifiers, AutomationLiveSetting, AutomationProperties};
use crate::{InfoBar, InfoBarSeverity};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::cell::RefCell;
use std::rc::Rc;

/// The automation peer of an [`InfoBar`]: a status bar, as in WinUI.
///
/// WinUI raises UI Automation notifications when the bar opens and closes
/// (`RaiseNotificationEvent` with the text of the severity, the title and the message), which
/// FerroUI's automation has no counterpart of. The peer exposes the bar instead: its name is the
/// title and the message of the bar when the application gives it none, it raises a change of the
/// name when they change, and its live setting is assertive for warnings and errors and polite
/// otherwise when the application sets none (WinUI's processing of the notifications,
/// `GetProcessingForSeverity`).
#[repr(C)]
pub struct InfoBarAutomationPeer {
    base: ControlAutomationPeer,
    last_name: RefCell<String>,
}

ferro_class!(InfoBarAutomationPeer: ControlAutomationPeer);
ferro_class_info!(InfoBarAutomationPeer { markup: { namespace: "FerroUI.Automation.Peers" } });

impl FerroObjectImpl for InfoBarAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        *this.last_name.borrow_mut() = this.get_name();
        let weak = this.to_ref().downgrade();
        let _subscription = this.owner().property_changed(move |change| {
            let property = change.property();
            if property == AutomationProperties::name_property().as_property()
                || property == AutomationProperties::labeled_by_property().as_property()
                || property == InfoBar::title_property().as_property()
                || property == InfoBar::message_property().as_property()
            {
                if let Some(peer) = weak.upgrade() {
                    peer.name_may_have_changed();
                }
            }
        });
    }
}

impl ControlAutomationPeerImpl for InfoBarAutomationPeer {}

impl AutomationPeerImpl for InfoBarAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::StatusBar
    }

    fn get_class_name_core(_this: &Self) -> String {
        "InfoBar".to_string()
    }

    fn get_name_core(this: &Self) -> Option<String> {
        Self::parent_get_name_core(this).filter(|name| !name.trim().is_empty()).or_else(|| {
            let owner = this.owner();
            let parts: Vec<String> = [owner.title(), owner.message()].into_iter().filter(|part| !part.is_empty()).collect();
            Some(parts.join(" "))
        })
    }

    fn get_live_setting_core(this: &Self) -> AutomationLiveSetting {
        let explicit = Self::parent_get_live_setting_core(this);
        if explicit != AutomationLiveSetting::Off {
            return explicit;
        }
        match this.owner().severity() {
            InfoBarSeverity::Warning | InfoBarSeverity::Error => AutomationLiveSetting::Assertive,
            InfoBarSeverity::Informational | InfoBarSeverity::Success => AutomationLiveSetting::Polite,
        }
    }
}

impl InfoBarAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &InfoBar) -> Self {
        Self { base: ControlAutomationPeer::construct(owner), last_name: RefCell::new(String::new()) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &InfoBar) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }

    /// Gets the owning info bar.
    pub fn owner(&self) -> Ref<InfoBar> {
        ControlAutomationPeer::owner(self).cast().expect("The owner of the peer is an info bar.")
    }

    /// Raises a change of the name when the name differs from the last one reported.
    fn name_may_have_changed(&self) {
        let name = self.get_name();
        let previous = self.last_name.replace(name.clone());
        if previous != name {
            self.raise_property_changed_event(
                AutomationElementIdentifiers::name_property(),
                Some(Rc::new(previous)),
                Some(Rc::new(name)),
            );
        }
    }
}
