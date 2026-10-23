// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the crate root for license information.

use super::{AutomationControlType, AutomationPeer, AutomationPeerImpl, ControlAutomationPeer};
use crate::automation::provider::{
    IInvokeProvider, ITextChildProvider, ITextRangeProvider, ProviderAdapter, ProviderError,
    RichTextAutomationHost, RichTextAutomationState, TextProviderError,
};
use crate::automation::{
    AccessibilityView, AutomationLiveSetting, AutomationProperties, ElementNotEnabledException,
};
use crate::documents::Hyperlink;
use ferroui_base::{ferro_class, instantiate, FerroObjectImpl, Rect, Ref, WeakRef};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Accessibility for a nonvisual hyperlink in a rich document. The owner/name,
/// Invoke and geometry contracts follow WinUI's HyperlinkAutomationPeer; the
/// TextChild projection uses FerroUI's shared live document range service.
#[repr(C)]
pub struct HyperlinkAutomationPeer {
    base: AutomationPeer,
    owner: WeakRef<Hyperlink>,
    host: RichTextAutomationHost,
    container: WeakRef<AutomationPeer>,
    parent: RefCell<Option<WeakRef<AutomationPeer>>>,
    state: Weak<RichTextAutomationState>,
}

ferro_class!(HyperlinkAutomationPeer: AutomationPeer);
ferroui_base::ferro_class_info!(HyperlinkAutomationPeer {
    interfaces: [
        Rc<dyn IInvokeProvider> => ProviderAdapter::as_invoke_provider,
        Rc<dyn ITextChildProvider> => ProviderAdapter::as_text_child_provider,
    ]
});

impl FerroObjectImpl for HyperlinkAutomationPeer {}

impl AutomationPeerImpl for HyperlinkAutomationPeer {
    fn bring_into_view_core(this: &Self) {
        if let Ok(range) = this.text_range() {
            let _ = range.scroll_into_view(true);
        }
    }
    fn get_accelerator_key_core(this: &Self) -> Option<String> {
        this.try_owner()
            .and_then(|link| AutomationProperties::get_accelerator_key(&link))
    }
    fn get_access_key_core(this: &Self) -> Option<String> {
        this.try_owner()
            .and_then(|link| AutomationProperties::get_access_key(&link))
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Hyperlink
    }
    fn get_automation_id_core(this: &Self) -> Option<String> {
        this.try_owner().and_then(|link| {
            AutomationProperties::get_automation_id(&link)
                .filter(|value| !value.is_empty())
                .or_else(|| link.name())
        })
    }
    fn get_bounding_rectangle_core(this: &Self) -> Rect {
        this.text_range()
            .and_then(|range| range.get_bounding_rectangles())
            .unwrap_or_default()
            .into_iter()
            .reduce(|first, next| first.union(next))
            .unwrap_or_default()
    }
    fn get_or_create_children_core(this: &Self) -> Rc<Vec<Ref<AutomationPeer>>> {
        let (Some(container), Some(state), Some(link)) = (
            this.container.upgrade(),
            this.state.upgrade(),
            this.try_owner(),
        ) else {
            return Rc::new(Vec::new());
        };
        state.children(&this.host, &container, Some(&link))
    }
    fn get_class_name_core(_this: &Self) -> String {
        "Hyperlink".to_owned()
    }
    fn get_labeled_by_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.try_owner()
            .and_then(|link| AutomationProperties::get_labeled_by(&link))
            .map(|control| ControlAutomationPeer::create_peer_for_element(&control))
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let link = this.try_owner()?;
        AutomationProperties::get_name(&link)
            .filter(|name| !name.is_empty())
            .or_else(|| Some(link.content_text()).filter(|name| !name.is_empty()))
            .or_else(|| link.navigate_uri())
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        this.try_owner()
            .and_then(|link| AutomationProperties::get_help_text(&link))
    }
    fn get_item_type_core(this: &Self) -> Option<String> {
        this.try_owner()
            .and_then(|link| AutomationProperties::get_item_type(&link))
    }
    fn get_item_status_core(this: &Self) -> Option<String> {
        this.try_owner()
            .and_then(|link| AutomationProperties::get_item_status(&link))
    }
    fn get_parent_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    fn has_keyboard_focus_core(this: &Self) -> bool {
        this.host.has_focus()
            && this.try_owner().is_some_and(|link| link.is_focused())
            && this.text_range().is_ok()
    }
    fn is_content_element_core(this: &Self) -> bool {
        this.try_owner().is_some_and(|link| {
            matches!(
                AutomationProperties::get_accessibility_view(&link),
                AccessibilityView::Default | AccessibilityView::Content
            )
        })
    }
    fn is_control_element_core(this: &Self) -> bool {
        this.try_owner().is_some_and(|link| {
            AutomationProperties::get_accessibility_view(&link) != AccessibilityView::Raw
        })
    }
    fn is_enabled_core(this: &Self) -> bool {
        // FerroUI hyperlinks have an explicit IsEnabled property, unlike the
        // pinned WinUI peer's unconditional true. Honor both it and the host.
        this.host.is_enabled()
            && this.try_owner().is_some_and(|link| link.is_enabled())
            && this.text_range().is_ok()
    }
    fn is_keyboard_focusable_core(this: &Self) -> bool {
        this.is_enabled() && this.try_owner().is_some_and(|link| link.is_tab_stop())
    }
    fn is_offscreen_core(this: &Self) -> bool {
        this.get_bounding_rectangle().is_empty()
    }
    fn set_focus_core(this: &Self) {
        if !this.is_keyboard_focusable() {
            return;
        }
        let (Some(link), Ok(range)) = (this.try_owner(), this.text_range()) else {
            return;
        };
        let (Some(container), Some(state)) = (this.container.upgrade(), this.state.upgrade())
        else {
            return;
        };
        let Ok(bounds) =
            state.range_bounds_for_child(&this.host, &container, &this.to_ref().upcast())
        else {
            return;
        };
        if this
            .host
            .focus_hyperlink(link, bounds.start)
            .unwrap_or(false)
        {
            let _ = range.scroll_into_view(true);
        }
    }
    fn show_context_menu_core(this: &Self) -> bool {
        this.container
            .upgrade()
            .is_some_and(|container| container.show_context_menu())
    }
    fn get_live_setting_core(this: &Self) -> AutomationLiveSetting {
        this.try_owner().map_or(AutomationLiveSetting::Off, |link| {
            AutomationProperties::get_live_setting(&link)
        })
    }
    fn get_control_type_override_core(this: &Self) -> AutomationControlType {
        this.try_owner()
            .and_then(|link| AutomationProperties::get_control_type_override(&link))
            .unwrap_or(AutomationControlType::Hyperlink)
    }
    fn get_class_name_override_core(this: &Self) -> String {
        this.try_owner()
            .and_then(|link| AutomationProperties::get_class_name_override(&link))
            .unwrap_or_else(|| "Hyperlink".to_owned())
    }
    fn get_automation_root_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.container
            .upgrade()
            .and_then(|container| container.get_automation_root())
    }
    fn get_visual_root_core(this: &Self) -> Option<Ref<AutomationPeer>> {
        this.container
            .upgrade()
            .and_then(|container| container.get_visual_root())
    }
    fn to_screen_core(this: &Self, rect: Rect) -> Option<Rect> {
        this.container
            .upgrade()
            .and_then(|container| container.to_screen(rect))
    }
    fn is_control_element_override_core(this: &Self) -> bool {
        this.try_owner()
            .and_then(|link| AutomationProperties::get_is_control_element_override(&link))
            .unwrap_or_else(|| this.is_control_element_core())
    }
    fn try_set_parent(this: &Self, parent: Option<Ref<AutomationPeer>>) -> bool {
        *this.parent.borrow_mut() = parent.map(|peer| peer.downgrade());
        true
    }
}

impl HyperlinkAutomationPeer {
    pub(crate) fn new(
        host: RichTextAutomationHost,
        container: &Ref<AutomationPeer>,
        owner: &Ref<Hyperlink>,
        state: Weak<RichTextAutomationState>,
    ) -> Ref<Self> {
        instantiate(Self {
            base: AutomationPeer::construct(),
            owner: owner.downgrade(),
            host,
            container: container.downgrade(),
            parent: RefCell::new(Some(container.downgrade())),
            state,
        })
    }

    pub fn try_owner(&self) -> Option<Ref<Hyperlink>> {
        self.owner.upgrade()
    }

    pub fn text_container(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.text_range()?;
        self.container
            .upgrade()
            .ok_or_else(|| TextProviderError::ElementNotAvailable.into())
    }

    pub fn text_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        let container = self
            .container
            .upgrade()
            .ok_or(TextProviderError::ElementNotAvailable)?;
        let state = self
            .state
            .upgrade()
            .ok_or(TextProviderError::ElementNotAvailable)?;
        state.range_for_child(&self.host, &container, &self.to_ref().upcast())
    }

    pub fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;
        if let Some(link) = self.try_owner() {
            link.navigate();
        }
        Ok(())
    }
}

impl IInvokeProvider for ProviderAdapter<HyperlinkAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        self.0.invoke()
    }
}

impl ITextChildProvider for ProviderAdapter<HyperlinkAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn text_container(&self) -> Result<Ref<AutomationPeer>, ProviderError> {
        self.0.text_container()
    }
    fn text_range(&self) -> Result<Rc<dyn ITextRangeProvider>, ProviderError> {
        self.0.text_range()
    }
}
