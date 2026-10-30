// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::controls::DragPreviewControl;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
    ControlAutomationPeer, ControlAutomationPeerImpl,
};
use ferroui_controls::automation::provider::{IValueProvider, ProviderError};
use ferroui_controls::automation::ValuePatternIdentifiers;
use std::rc::Rc;
#[repr(C)]
pub(crate) struct DragPreviewControlAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(DragPreviewControlAutomationPeer:ControlAutomationPeer);
ferroui_base::ferro_class_info!(DragPreviewControlAutomationPeer {interfaces:[Rc<dyn IValueProvider> => DragPreviewControlAutomationPeer::as_value_provider]});
ferro_impl_classes!(DragPreviewControlAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for DragPreviewControlAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "DragPreviewControl".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Pane
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|name| !name.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(&this.owner(), "Drag preview", None))
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        Some(Helper::format_state(&[
            (
                "Status",
                owner
                    .status()
                    .map(|v| Rc::new(v) as ferroui_base::BoxedValue),
            ),
            ("ShowContent", Some(Rc::new(owner.show_content()))),
            ("PreviewWidth", Some(Rc::new(owner.preview_content_width()))),
            (
                "PreviewHeight",
                Some(Rc::new(owner.preview_content_height())),
            ),
        ]))
    }
}
struct ValueProvider(Ref<DragPreviewControlAutomationPeer>);
impl IValueProvider for ValueProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        true
    }
    fn value(&self) -> Option<String> {
        Some(self.0.owner().status().unwrap_or_default())
    }
    fn set_value(&self, _value: Option<&str>) -> Result<(), ProviderError> {
        Err("Drag preview status is read-only.".into())
    }
}
impl DragPreviewControlAutomationPeer {
    pub(crate) fn new(owner: &DragPreviewControl) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn owner(&self) -> Ref<DragPreviewControl> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("native drag-preview peer owner")
    }
    fn as_value_provider(this: Ref<Self>) -> Rc<dyn IValueProvider> {
        Rc::new(ValueProvider(this))
    }
    pub(crate) fn notify_status_changed(
        &self,
        old_value: Option<String>,
        new_value: Option<String>,
    ) {
        let old = old_value.unwrap_or_default();
        let new = new_value.unwrap_or_default();
        if old != new {
            self.raise_property_changed_event(
                ValuePatternIdentifiers::value_property(),
                Some(Rc::new(old)),
                Some(Rc::new(new)),
            );
        }
    }
}
