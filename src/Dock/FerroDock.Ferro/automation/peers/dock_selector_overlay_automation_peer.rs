// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::dock_automation_peer_helper::DockAutomationPeerHelper as Helper;
use crate::{
    controls::DockSelectorOverlay,
    selectors::{DockSelectorItem, DockSelectorMode},
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
};
use ferroui_controls::automation::{
    peers::{
        AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
        ControlAutomationPeer, ControlAutomationPeerImpl,
    },
    provider::{
        IExpandCollapseProvider, IScrollProvider, ISelectionProvider, IValueProvider,
        ProviderError, ScrollAmount,
    },
    AutomationElementIdentifiers, ElementNotEnabledException, ExpandCollapsePatternIdentifiers,
    ExpandCollapseState, ScrollPatternIdentifiers, SelectionPatternIdentifiers,
    ValuePatternIdentifiers,
};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct DockSelectorOverlayAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(DockSelectorOverlayAutomationPeer:ControlAutomationPeer);
ferroui_base::ferro_class_info!(DockSelectorOverlayAutomationPeer {interfaces:[
    Rc<dyn IExpandCollapseProvider> => DockSelectorOverlayAutomationPeer::as_expand_provider,
    Rc<dyn ISelectionProvider> => DockSelectorOverlayAutomationPeer::as_selection_provider,
    Rc<dyn IScrollProvider> => DockSelectorOverlayAutomationPeer::as_scroll_provider,
    Rc<dyn IValueProvider> => DockSelectorOverlayAutomationPeer::as_value_provider
]});
ferro_impl_classes!(DockSelectorOverlayAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for DockSelectorOverlayAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "DockSelectorOverlay".into()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::List
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let base = Self::parent_get_name_core(this);
        if base.as_ref().is_some_and(|s| !s.trim().is_empty()) {
            return base;
        }
        Some(Helper::resolve_name(
            &this.owner(),
            &format!("{:?} selector", this.owner().mode()),
            None,
        ))
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        let selected = owner
            .selected_item()
            .map(|i| i.title())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "none".into());
        Some(Helper::format_state(&[
            ("Open", Some(Rc::new(owner.is_open()))),
            ("Mode", Some(Rc::new(owner.mode()))),
            (
                "ItemCount",
                Some(Rc::new(owner.items().map_or(0, |i| i.count()) as i32)),
            ),
            ("Selected", Some(Rc::new(selected))),
        ]))
    }
}
struct Provider(Ref<DockSelectorOverlayAutomationPeer>);
impl IExpandCollapseProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        state(self.0.owner().is_open())
    }
    fn shows_menu(&self) -> bool {
        false
    }
    fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.0.owner().set_is_open(true);
        Ok(())
    }
    fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.0.owner().set_is_open(false);
        Ok(())
    }
}
impl ISelectionProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn can_select_multiple(&self) -> bool {
        false
    }
    fn is_selection_required(&self) -> bool {
        false
    }
    fn get_selection(&self) -> Vec<Ref<AutomationPeer>> {
        if let Some(provider) = self.0.get_selection_provider() {
            let selection = provider.get_selection();
            if !selection.is_empty() {
                return selection;
            }
        }
        self.0.build_selection(self.0.owner().selected_item())
    }
}
impl IValueProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        true
    }
    fn value(&self) -> Option<String> {
        Some(
            self.0
                .owner()
                .selected_item()
                .map(|i| i.title())
                .unwrap_or_default(),
        )
    }
    fn set_value(&self, _value: Option<&str>) -> Result<(), ProviderError> {
        Err("Dock selector value is read-only.".into())
    }
}
impl IScrollProvider for Provider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn horizontally_scrollable(&self) -> bool {
        self.0
            .get_scroll_provider()
            .is_some_and(|p| p.horizontally_scrollable())
    }
    fn horizontal_scroll_percent(&self) -> f64 {
        self.0
            .get_scroll_provider()
            .map_or(ScrollPatternIdentifiers::NO_SCROLL, |p| {
                p.horizontal_scroll_percent()
            })
    }
    fn horizontal_view_size(&self) -> f64 {
        self.0
            .get_scroll_provider()
            .map_or(100.0, |p| p.horizontal_view_size())
    }
    fn vertically_scrollable(&self) -> bool {
        self.0
            .get_scroll_provider()
            .is_some_and(|p| p.vertically_scrollable())
    }
    fn vertical_scroll_percent(&self) -> f64 {
        self.0
            .get_scroll_provider()
            .map_or(ScrollPatternIdentifiers::NO_SCROLL, |p| {
                p.vertical_scroll_percent()
            })
    }
    fn vertical_view_size(&self) -> f64 {
        self.0
            .get_scroll_provider()
            .map_or(100.0, |p| p.vertical_view_size())
    }
    fn scroll(
        &self,
        horizontal: ScrollAmount,
        vertical: ScrollAmount,
    ) -> Result<(), ElementNotEnabledException> {
        self.0
            .get_scroll_provider()
            .map_or(Ok(()), |p| p.scroll(horizontal, vertical))
    }
    fn set_scroll_percent(
        &self,
        horizontal: f64,
        vertical: f64,
    ) -> Result<(), ElementNotEnabledException> {
        self.0
            .get_scroll_provider()
            .map_or(Ok(()), |p| p.set_scroll_percent(horizontal, vertical))
    }
}
fn state(open: bool) -> ExpandCollapseState {
    if open {
        ExpandCollapseState::Expanded
    } else {
        ExpandCollapseState::Collapsed
    }
}
impl DockSelectorOverlayAutomationPeer {
    pub(crate) fn new(owner: &DockSelectorOverlay) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
        })
    }
    fn owner(&self) -> Ref<DockSelectorOverlay> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("selector peer owner")
    }
    fn as_expand_provider(this: Ref<Self>) -> Rc<dyn IExpandCollapseProvider> {
        Rc::new(Provider(this))
    }
    fn as_selection_provider(this: Ref<Self>) -> Rc<dyn ISelectionProvider> {
        Rc::new(Provider(this))
    }
    fn as_scroll_provider(this: Ref<Self>) -> Rc<dyn IScrollProvider> {
        Rc::new(Provider(this))
    }
    fn as_value_provider(this: Ref<Self>) -> Rc<dyn IValueProvider> {
        Rc::new(Provider(this))
    }
    pub(crate) fn notify_is_open_changed(&self, old: bool, new: bool) {
        if old != new {
            self.raise_property_changed_event(
                ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                Some(Rc::new(state(old))),
                Some(Rc::new(state(new))),
            );
        }
    }
    pub(crate) fn notify_selected_item_changed(
        &self,
        old: Option<Rc<DockSelectorItem>>,
        new: Option<Rc<DockSelectorItem>>,
    ) {
        let old_selection = self.build_selection(old.clone());
        let new_selection = self.build_selection(new.clone());
        self.raise_property_changed_event(
            SelectionPatternIdentifiers::selection_property(),
            Some(Rc::new(old_selection)),
            Some(Rc::new(new_selection)),
        );
        let old = old.map(|i| i.title()).unwrap_or_default();
        let new = new.map(|i| i.title()).unwrap_or_default();
        if old != new {
            self.raise_property_changed_event(
                ValuePatternIdentifiers::value_property(),
                Some(Rc::new(old)),
                Some(Rc::new(new)),
            );
        }
    }
    pub(crate) fn notify_items_changed(&self) {
        self.raise_children_changed_event();
    }
    pub(crate) fn notify_mode_changed(&self, old: DockSelectorMode, new: DockSelectorMode) {
        let old = Helper::resolve_name(&self.owner(), &format!("{old:?} selector"), None);
        let new = Helper::resolve_name(&self.owner(), &format!("{new:?} selector"), None);
        if old != new {
            self.raise_property_changed_event(
                AutomationElementIdentifiers::name_property(),
                Some(Rc::new(old)),
                Some(Rc::new(new)),
            );
        }
    }
    fn build_selection(&self, item: Option<Rc<DockSelectorItem>>) -> Vec<Ref<AutomationPeer>> {
        Helper::to_selection_list(self.resolve_selection_peer(item))
    }
    fn get_scroll_provider(&self) -> Option<Rc<dyn IScrollProvider>> {
        {
            let list = self.owner().items_list()?;
            ControlAutomationPeer::create_peer_for_element(&list)
                .get_provider::<dyn IScrollProvider>()
        }
    }
    fn get_selection_provider(&self) -> Option<Rc<dyn ISelectionProvider>> {
        {
            let list = self.owner().items_list()?;
            ControlAutomationPeer::create_peer_for_element(&list)
                .get_provider::<dyn ISelectionProvider>()
        }
    }
    fn resolve_selection_peer(
        &self,
        item: Option<Rc<DockSelectorItem>>,
    ) -> Option<Ref<AutomationPeer>> {
        let items = self.owner().items_list()?;
        let item: BoxedValue = Rc::new(item?);
        if let Some(peer) = Helper::try_get_container_peer(&items, Some(&item)) {
            return Some(peer);
        }
        items.scroll_into_view_item(&Some(item.clone()));
        items.update_layout();
        Helper::try_get_container_peer(&items, Some(&item))
    }
}
