// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See ../../NOTICE.md.

use crate::InfoBar;
use crate::automation::peers::{
    AutomationControlType, AutomationPeerImpl, AutomationPeerImplExt,
    ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::{AutomationElementIdentifiers, AutomationProperties};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref,
};
use std::cell::RefCell;
use std::rc::Rc;

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
        this.owner().property_changed(move |change| {
            if change.property() == AutomationProperties::name_property().as_property()
                || change.property() == AutomationProperties::labeled_by_property().as_property()
                || change.property() == InfoBar::title_property().as_property()
                || change.property() == InfoBar::message_property().as_property()
            {
                if let Some(peer) = weak.upgrade() {
                    let next = peer.get_name();
                    let previous = peer.last_name.replace(next.clone());
                    if previous != next {
                        peer.raise_property_changed_event(
                            AutomationElementIdentifiers::name_property(),
                            Some(Rc::new(previous)),
                            Some(Rc::new(next)),
                        );
                    }
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
        "InfoBar".into()
    }
    fn get_name_core(this: &Self) -> Option<String> {
        Self::parent_get_name_core(this)
            .filter(|name| !name.trim().is_empty())
            .or_else(|| {
                let owner = this.owner();
                Some(
                    [owner.title(), owner.message()]
                        .into_iter()
                        .filter(|part| !part.is_empty())
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            })
    }
}
impl InfoBarAutomationPeer {
    pub fn new(owner: &InfoBar) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            last_name: RefCell::new(String::new()),
        })
    }
    pub fn owner(&self) -> Ref<InfoBar> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("InfoBarAutomationPeer owns an InfoBar")
    }
}
