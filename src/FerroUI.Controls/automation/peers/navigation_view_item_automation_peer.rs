//! WinUI's `NavigationViewItemAutomationPeer` (`controls/dev/NavigationView/NavigationViewItemAutomationPeer.cpp`
//! and `.h` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright
//! (c) Microsoft Corporation, see NOTICE.md).

use crate::{NavigationView, NavigationViewItem, NavigationViewItemBase};
use crate::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
    ControlAutomationPeer, ControlAutomationPeerImpl,
};
use crate::automation::provider::{
    IExpandCollapseProvider, ISelectionItemProvider, ISelectionProvider, ProviderAdapter,
};
use crate::automation::{
    ElementNotEnabledException, ExpandCollapsePatternIdentifiers, ExpandCollapseState,
    SelectionItemPatternIdentifiers,
};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt, Ref};
use std::cell::RefCell;
use std::rc::Rc;

/// The name of an item without content and name (#286's text; WinUI's peer has no name then).
const NAVIGATION_ITEM_NAME: &str = "Navigation item";

/// The automation peer of a [`NavigationViewItem`]: a list item (a tab item in the top pane) with
/// the selection item pattern and, for an item with children, the expand and collapse pattern;
/// like WinUI's peer at the pin it has no invoke pattern.
#[repr(C)]
pub struct NavigationViewItemAutomationPeer {
    base: ControlAutomationPeer,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}
ferro_class!(NavigationViewItemAutomationPeer: ControlAutomationPeer);
ferro_class_info!(NavigationViewItemAutomationPeer {
    markup: { namespace: "FerroUI.Automation.Peers" },
    interfaces: [
        Rc<dyn ISelectionItemProvider> => ProviderAdapter::as_selection_item_provider,
        Rc<dyn IExpandCollapseProvider> => ProviderAdapter::as_expand_collapse_provider,
    ],
});
impl FerroObjectImpl for NavigationViewItemAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let subscription = this.owner().property_changed(move |args| {
            if !args.is_effective_value_change() {
                return;
            }
            if let Some(peer) = weak.upgrade() {
                let property = args.property();
                if property == NavigationViewItem::is_expanded_property().as_property() {
                    let (old, _) = args.get_old_and_new_value::<bool>();
                    peer.raise_property_changed_event(
                        ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                        Some(Rc::new(if old {
                            ExpandCollapseState::Expanded
                        } else {
                            ExpandCollapseState::Collapsed
                        }) as BoxedValue),
                        Some(Rc::new(peer.expand_collapse_state()) as BoxedValue),
                    );
                } else if property == NavigationViewItemBase::is_selected_property().as_property() {
                    let (old, _) = args.get_old_and_new_value::<bool>();
                    peer.raise_property_changed_event(
                        SelectionItemPatternIdentifiers::is_selected_property(),
                        Some(Rc::new(old) as BoxedValue),
                        Some(Rc::new(peer.owner().is_selected()) as BoxedValue),
                    );
                }
            }
        });
        *this.subscription.borrow_mut() = Some(subscription);
    }
}
impl ControlAutomationPeerImpl for NavigationViewItemAutomationPeer {}
impl AutomationPeerImpl for NavigationViewItemAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "NavigationViewItem".into()
    }
    fn get_automation_control_type_core(this: &Self) -> AutomationControlType {
        if this.owner().is_top_navigation() {
            AutomationControlType::TabItem
        } else {
            AutomationControlType::ListItem
        }
    }
    fn get_name_core(this: &Self) -> Option<String> {
        Self::parent_get_name_core(this)
            .filter(|name| !name.is_empty())
            .or_else(|| {
                this.owner()
                    .content()
                    .map(|content| ValueTypes::to_display_string(Some(&content)))
                    .filter(|name| !name.is_empty())
            })
            .or_else(|| Some(NAVIGATION_ITEM_NAME.into()))
    }
    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        if provider_type == ValueType::of::<Rc<dyn IExpandCollapseProvider>>()
            && !this.owner().has_children()
        {
            None
        } else {
            Self::parent_get_provider_core(this, provider_type)
        }
    }
}
impl NavigationViewItemAutomationPeer {
    /// Creates the peer of `owner`.
    pub fn new(owner: &NavigationViewItem) -> Ref<Self> {
        instantiate(Self {
            base: ControlAutomationPeer::construct(owner),
            subscription: RefCell::new(None),
        })
    }
    /// Gets the item of the peer.
    pub fn owner(&self) -> Ref<NavigationViewItem> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("NavigationViewItem peer owner")
    }
    /// Gets whether the item is expanded.
    pub fn expand_collapse_state(&self) -> ExpandCollapseState {
        if self.owner().is_expanded() {
            ExpandCollapseState::Expanded
        } else {
            ExpandCollapseState::Collapsed
        }
    }
    fn navigation_view(&self) -> Option<Ref<NavigationView>> {
        let mut current = self.owner().parent();
        while let Some(element) = current {
            if let Some(control) = element.cast::<NavigationView>() {
                return Some(control);
            }
            current = element.parent();
        }
        None
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.navigation_view().and_then(|control| {
            ControlAutomationPeer::create_peer_for_element(&control)
                .get_provider::<dyn ISelectionProvider>()
        })
    }
    fn change_selection(&self, value: bool) -> Result<(), ElementNotEnabledException> {
        self.ensure_enabled()?;
        let owner = self.owner();
        if value {
            owner.invoke();
        }
        if !value || owner.selects_on_invoked() {
            owner.set_current_value(NavigationViewItemBase::is_selected_property(), value);
        }
        Ok(())
    }
}
impl Drop for NavigationViewItemAutomationPeer {
    fn drop(&mut self) {
        if let Some(subscription) = self.subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}
impl ISelectionItemProvider for ProviderAdapter<NavigationViewItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_selected(&self) -> bool {
        self.0.owner().is_selected()
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.0.selection_container()
    }
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.change_selection(true)
    }
    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        self.0.change_selection(false)
    }
    fn select(&self) -> Result<(), ElementNotEnabledException> {
        self.0.change_selection(true)
    }
}
impl IExpandCollapseProvider for ProviderAdapter<NavigationViewItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        self.0.expand_collapse_state()
    }
    fn shows_menu(&self) -> bool {
        self.0.owner().in_flyout()
    }
    fn expand(&self) -> Result<(), ElementNotEnabledException> {
        self.0.ensure_enabled()?;
        let owner = self.0.owner();
        if let Some(navigation_view) = self.0.navigation_view() {
            // The source peer delegates to NavigationView, including its active
            // source precedence and child-presence gate. A retained provider
            // must obey that gate after the collection changes as well.
            navigation_view.expand(&owner);
        } else {
            // Standalone native items remain independently expandable.
            owner.set_current_value(NavigationViewItem::is_expanded_property(), true);
        }
        Ok(())
    }
    fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        self.0.ensure_enabled()?;
        let owner = self.0.owner();
        if let Some(navigation_view) = self.0.navigation_view() {
            navigation_view.collapse(&owner);
        } else {
            owner.set_current_value(NavigationViewItem::is_expanded_property(), false);
        }
        Ok(())
    }
}
