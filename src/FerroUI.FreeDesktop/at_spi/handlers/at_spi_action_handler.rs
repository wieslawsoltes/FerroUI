//! The port of `AtSpiActionHandler.cs`: `org.a11y.atspi.Action`. The
//! actions of a node are those of the providers its peer has.

use super::{item_at, to_i32};
use crate::at_spi::at_spi_constants::ACTION_VERSION;
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::descriptions::ACTION;
use crate::at_spi::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use crate::at_spi::dbus::types::AtSpiAction;
use ferroui_controls::automation::peers::AutomationPeer;
use ferroui_controls::automation::provider::{
    IExpandCollapseProvider, IInvokeProvider, IScrollProvider, ISelectionItemProvider, IToggleProvider, ScrollAmount,
};
use ferroui_controls::automation::ExpandCollapseState;
use std::rc::Weak;
use zbus::zvariant::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ActionEntry {
    pub(crate) action_name: &'static str,
    pub(crate) localized_name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) key_binding: String,
}

impl ActionEntry {
    fn new(action_name: &'static str, localized_name: &'static str, description: &'static str, key_binding: String) -> Self {
        Self { action_name, localized_name, description, key_binding }
    }
}

/// What the action list of a peer is built from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ActionFacts {
    pub(crate) invoke: bool,
    pub(crate) accelerator_key: Option<String>,
    pub(crate) toggle: bool,
    pub(crate) expand_collapse: bool,
    /// Whether a scroll provider scrolls vertically and horizontally.
    pub(crate) scroll: Option<(bool, bool)>,
    pub(crate) selection_item: bool,
}

pub(crate) struct AtSpiActionHandler {
    node: Weak<AtSpiNode>,
    actions: Vec<ActionEntry>,
}

impl AtSpiActionHandler {
    pub(crate) fn new(_server: Weak<AtSpiServer>, node: Weak<AtSpiNode>) -> Self {
        let actions = node.upgrade().map(|node| Self::build_action_list(node.peer())).unwrap_or_default();
        Self { node, actions }
    }

    fn version(&self) -> u32 {
        ACTION_VERSION
    }

    fn n_actions(&self) -> i32 {
        to_i32(self.actions.len())
    }

    fn get_description_async(&self, index: i32) -> String {
        item_at(&self.actions, index).map(|entry| entry.description.to_string()).unwrap_or_default()
    }

    fn get_name_async(&self, index: i32) -> String {
        item_at(&self.actions, index).map(|entry| entry.action_name.to_string()).unwrap_or_default()
    }

    fn get_localized_name_async(&self, index: i32) -> String {
        item_at(&self.actions, index).map(|entry| entry.localized_name.to_string()).unwrap_or_default()
    }

    fn get_key_binding_async(&self, index: i32) -> String {
        item_at(&self.actions, index).map(|entry| entry.key_binding.clone()).unwrap_or_default()
    }

    fn get_actions_async(&self) -> Vec<AtSpiAction> {
        self.actions
            .iter()
            .map(|entry| (entry.localized_name.to_string(), entry.description.to_string(), entry.key_binding.clone()))
            .collect()
    }

    fn do_action_async(&self, index: i32) -> Result<bool, DBusError> {
        let Some(action) = item_at(&self.actions, index) else { return Ok(false) };
        self.execute_action(action.action_name)?;
        Ok(true)
    }

    /// A provider that refuses (its element is not enabled) makes the
    /// call fail, as the exception of the reference does.
    fn execute_action(&self, action_name: &str) -> Result<(), DBusError> {
        let Some(node) = self.node.upgrade() else { return Ok(()) };
        let peer = node.peer();
        let scroll = |horizontal: ScrollAmount, vertical: ScrollAmount| match peer.get_provider::<dyn IScrollProvider>() {
            Some(provider) => provider.scroll(horizontal, vertical).map_err(DBusError::failed),
            None => Ok(()),
        };

        match action_name {
            "click" => match peer.get_provider::<dyn IInvokeProvider>() {
                Some(provider) => provider.invoke().map_err(DBusError::failed),
                None => Ok(()),
            },
            "toggle" => match peer.get_provider::<dyn IToggleProvider>() {
                Some(provider) => provider.toggle().map_err(DBusError::failed),
                None => Ok(()),
            },
            "expand or collapse" => match peer.get_provider::<dyn IExpandCollapseProvider>() {
                Some(expand_collapse_action) => {
                    if expand_collapse_action.expand_collapse_state() == ExpandCollapseState::Collapsed {
                        expand_collapse_action.expand().map_err(DBusError::failed)
                    } else {
                        expand_collapse_action.collapse().map_err(DBusError::failed)
                    }
                }
                None => Ok(()),
            },
            "scroll up" => scroll(ScrollAmount::NoAmount, ScrollAmount::SmallDecrement),
            "scroll down" => scroll(ScrollAmount::NoAmount, ScrollAmount::SmallIncrement),
            "scroll left" => scroll(ScrollAmount::SmallDecrement, ScrollAmount::NoAmount),
            "scroll right" => scroll(ScrollAmount::SmallIncrement, ScrollAmount::NoAmount),
            // Provisional: activate selectable items (TabItem, ListBoxItem) via Action.
            "select" => match peer.get_provider::<dyn ISelectionItemProvider>() {
                Some(provider) => provider.select().map_err(DBusError::failed),
                None => Ok(()),
            },
            _ => Ok(()),
        }
    }

    fn build_action_list(peer: &AutomationPeer) -> Vec<ActionEntry> {
        let invoke = peer.get_provider::<dyn IInvokeProvider>().is_some();
        action_list(&ActionFacts {
            invoke,
            accelerator_key: if invoke { peer.get_accelerator_key() } else { None },
            toggle: peer.get_provider::<dyn IToggleProvider>().is_some(),
            expand_collapse: peer.get_provider::<dyn IExpandCollapseProvider>().is_some(),
            scroll: peer
                .get_provider::<dyn IScrollProvider>()
                .map(|scroll| (scroll.vertically_scrollable(), scroll.horizontally_scrollable())),
            selection_item: peer.get_provider::<dyn ISelectionItemProvider>().is_some(),
        })
    }
}

pub(crate) fn action_list(facts: &ActionFacts) -> Vec<ActionEntry> {
    let mut actions = Vec::new();
    let entry = |name, localized, description| ActionEntry::new(name, localized, description, String::new());

    if facts.invoke {
        let accelerator_key = facts.accelerator_key.clone().unwrap_or_default();
        actions.push(ActionEntry::new("click", "Click", "Performs the default action", accelerator_key));
    }

    if facts.toggle {
        actions.push(entry("toggle", "Toggle", "Toggles the control state"));
    }

    if facts.expand_collapse {
        actions.push(entry("expand or collapse", "Expand or Collapse", "Expands or collapses the control"));
    }

    if let Some((vertically, horizontally)) = facts.scroll {
        if vertically {
            actions.push(entry("scroll up", "Scroll Up", "Scrolls the view up"));
            actions.push(entry("scroll down", "Scroll Down", "Scrolls the view down"));
        }
        if horizontally {
            actions.push(entry("scroll left", "Scroll Left", "Scrolls the view left"));
            actions.push(entry("scroll right", "Scroll Right", "Scrolls the view right"));
        }
    }

    if facts.selection_item && !facts.invoke {
        actions.push(entry("select", "Select", "Selects this item"));
    }

    actions
}

impl DBusInterface for AtSpiActionHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &ACTION
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        match member {
            "GetDescription" => reply((self.get_description_async(args::<i32>(body)?),)),
            "GetName" => reply((self.get_name_async(args::<i32>(body)?),)),
            "GetLocalizedName" => reply((self.get_localized_name_async(args::<i32>(body)?),)),
            "GetKeyBinding" => reply((self.get_key_binding_async(args::<i32>(body)?),)),
            "GetActions" => reply((self.get_actions_async(),)),
            "DoAction" => reply((self.do_action_async(args::<i32>(body)?)?,)),
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        Some(match name {
            "version" => Value::from(self.version()),
            "NActions" => Value::from(self.n_actions()),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    fn names(facts: &ActionFacts) -> Vec<&'static str> {
        action_list(facts).iter().map(|entry| entry.action_name).collect()
    }

    #[test]
    fn a_button_clicks_with_its_accelerator_key() {
        let actions =
            action_list(&ActionFacts { invoke: true, accelerator_key: Some("Ctrl+S".to_string()), ..Default::default() });
        assert_eq!(
            actions,
            vec![ActionEntry::new("click", "Click", "Performs the default action", "Ctrl+S".to_string())]
        );
        assert_eq!(names(&ActionFacts::default()), Vec::<&str>::new());
    }

    #[test]
    fn the_actions_follow_the_providers_in_a_fixed_order() {
        let all = ActionFacts {
            invoke: true,
            accelerator_key: None,
            toggle: true,
            expand_collapse: true,
            scroll: Some((true, true)),
            selection_item: true,
        };
        // An item that can be invoked is not also selected through an action.
        assert_eq!(
            names(&all),
            vec!["click", "toggle", "expand or collapse", "scroll up", "scroll down", "scroll left", "scroll right"]
        );
        assert_eq!(names(&ActionFacts { scroll: Some((false, true)), ..Default::default() }), vec!["scroll left", "scroll right"]);
        assert_eq!(names(&ActionFacts { scroll: Some((false, false)), ..Default::default() }), Vec::<&str>::new());
        assert_eq!(names(&ActionFacts { selection_item: true, ..Default::default() }), vec!["select"]);
        assert_eq!(names(&ActionFacts { toggle: true, selection_item: true, ..Default::default() }), vec!["toggle", "select"]);
    }
}
