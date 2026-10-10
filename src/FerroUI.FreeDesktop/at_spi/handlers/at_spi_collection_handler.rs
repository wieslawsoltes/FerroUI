//! The port of `AtSpiCollectionHandler.cs`: `org.a11y.atspi.Collection`.
//!
//! At the tracked commit the reference has this handler and registers
//! it for no node (`AtSpiNode.BuildAndRegisterHandlers` does not create
//! one, and `GetSupportedInterfaces` does not list the interface). The
//! port is the same: the handler is here, with its tests, and no node
//! has it.
#![allow(dead_code)]

use super::{node_of, server_of};
use crate::at_spi::at_spi_constants::{COLLECTION_VERSION, TOOLKIT_NAME};
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::descriptions::COLLECTION;
use crate::at_spi::dbus::interface::{args, reply, CallResult, DBusError, DBusInterface, InterfaceDescription};
use crate::at_spi::dbus::types::{AtSpiMatchRule, AtSpiMatchRuleWire, AtSpiObjectReference};
use std::collections::{BTreeSet, HashMap};
use std::rc::{Rc, Weak};
use zbus::zvariant::{OwnedObjectPath, Value};

// `MatchType`
const MATCH_INVALID: i32 = 0;
const MATCH_ALL: i32 = 1;
const MATCH_ANY: i32 = 2;
const MATCH_NONE: i32 = 3;
const MATCH_EMPTY: i32 = 4;

/// What a rule is matched against: a node as its states, its role, its
/// interfaces and its attributes.
pub(crate) struct MatchSubject {
    pub(crate) states: Vec<u32>,
    pub(crate) role: u32,
    pub(crate) interfaces: BTreeSet<&'static str>,
    pub(crate) attributes: HashMap<String, String>,
}

impl MatchSubject {
    fn of(node: &AtSpiNode) -> Self {
        // Build node attributes (same as AccessibleHandler.GetAttributesAsync)
        let mut attributes = HashMap::from([("toolkit".to_string(), TOOLKIT_NAME.to_string())]);
        if !node.peer().get_name().is_empty() {
            attributes.insert("explicit-name".to_string(), "true".to_string());
        }

        Self {
            states: node.compute_states(),
            role: AtSpiNode::to_at_spi_role(node.peer().get_automation_control_type(), Some(node.peer())) as u32,
            interfaces: node.get_supported_interfaces(),
            attributes,
        }
    }
}

pub(crate) fn matches_rule(subject: &MatchSubject, rule: &AtSpiMatchRule) -> bool {
    let matched = matches_states(&subject.states, &rule.states, rule.state_match_type)
        && matches_roles(subject.role, &rule.roles, rule.role_match_type)
        && matches_interfaces(&subject.interfaces, &rule.interfaces, rule.interface_match_type)
        && matches_attributes(&subject.attributes, &rule.attributes, rule.attribute_match_type);

    if rule.invert {
        !matched
    } else {
        matched
    }
}

fn matches_states(node_states: &[u32], rule_states: &[i32], match_type: i32) -> bool {
    if match_type == MATCH_INVALID || match_type == MATCH_EMPTY {
        return match_type != MATCH_EMPTY || is_empty_bit_set(rule_states);
    }

    if is_empty_bit_set(rule_states) {
        return true;
    }

    let node_low = node_states.first().copied().unwrap_or(0);
    let node_high = node_states.get(1).copied().unwrap_or(0);
    let rule_low = rule_states.first().map_or(0, |word| *word as u32);
    let rule_high = rule_states.get(1).map_or(0, |word| *word as u32);

    match match_type {
        MATCH_ALL => (node_low & rule_low) == rule_low && (node_high & rule_high) == rule_high,
        MATCH_ANY => (node_low & rule_low) != 0 || (node_high & rule_high) != 0,
        MATCH_NONE => (node_low & rule_low) == 0 && (node_high & rule_high) == 0,
        _ => true,
    }
}

fn matches_roles(role: u32, rule_roles: &[i32], match_type: i32) -> bool {
    if match_type == MATCH_INVALID || match_type == MATCH_EMPTY {
        return match_type != MATCH_EMPTY || is_empty_bit_set(rule_roles);
    }

    if is_empty_bit_set(rule_roles) {
        return true;
    }

    let bucket = (role / 32) as usize;
    let bit = role % 32;
    let is_set = rule_roles.get(bucket).is_some_and(|word| (*word as u32) & (1u32 << bit) != 0);

    match match_type {
        MATCH_ALL | MATCH_ANY => is_set,
        MATCH_NONE => !is_set,
        _ => true,
    }
}

fn matches_interfaces(node_interfaces: &BTreeSet<&'static str>, rule_interfaces: &[String], match_type: i32) -> bool {
    if match_type == MATCH_INVALID || match_type == MATCH_EMPTY {
        return match_type != MATCH_EMPTY || rule_interfaces.is_empty();
    }

    if rule_interfaces.is_empty() {
        return true;
    }

    let present = |name: &String| node_interfaces.contains(resolve_interface_name(name).as_str());
    match match_type {
        MATCH_ALL => rule_interfaces.iter().all(present),
        MATCH_ANY => rule_interfaces.iter().any(present),
        MATCH_NONE => !rule_interfaces.iter().any(present),
        _ => true,
    }
}

fn resolve_interface_name(name: &str) -> String {
    // ATs may pass short names like "Action" or full names like "org.a11y.atspi.Action"
    if name.contains('.') {
        name.to_string()
    } else {
        format!("org.a11y.atspi.{name}")
    }
}

fn matches_attributes(
    node_attrs: &HashMap<String, String>,
    rule_attrs: &HashMap<String, String>,
    match_type: i32,
) -> bool {
    if match_type == MATCH_INVALID || match_type == MATCH_EMPTY {
        return match_type != MATCH_EMPTY || rule_attrs.is_empty();
    }

    if rule_attrs.is_empty() {
        return true;
    }

    let matches = |(key, value): (&String, &String)| node_attrs.get(key) == Some(value);
    match match_type {
        MATCH_ALL => rule_attrs.iter().all(matches),
        MATCH_ANY => rule_attrs.iter().any(matches),
        MATCH_NONE => !rule_attrs.iter().any(matches),
        _ => true,
    }
}

fn is_empty_bit_set(values: &[i32]) -> bool {
    values.iter().all(|value| *value == 0)
}

fn full(count: i32, results: &[AtSpiObjectReference]) -> bool {
    count > 0 && results.len() >= usize::try_from(count).unwrap_or(usize::MAX)
}

pub(crate) struct AtSpiCollectionHandler {
    server: Weak<AtSpiServer>,
    node: Weak<AtSpiNode>,
}

impl AtSpiCollectionHandler {
    pub(crate) fn new(server: Weak<AtSpiServer>, node: Weak<AtSpiNode>) -> Self {
        Self { server, node }
    }

    fn version() -> u32 {
        COLLECTION_VERSION
    }

    fn get_matches_async(
        server: &AtSpiServer,
        node: &Rc<AtSpiNode>,
        rule: &AtSpiMatchRule,
        _sortby: u32,
        count: i32,
        traverse: bool,
    ) -> Vec<AtSpiObjectReference> {
        let mut results = Vec::new();
        Self::collect_matches(server, node, rule, count, traverse, &mut results);
        results
    }

    // GetMatchesTo: find matches after currentObject in tree order
    fn get_matches_to_async(
        server: &AtSpiServer,
        node: &Rc<AtSpiNode>,
        current_object: &str,
        rule: &AtSpiMatchRule,
        count: i32,
        traverse: bool,
    ) -> Vec<AtSpiObjectReference> {
        let (mut results, mut found) = (Vec::new(), false);
        Self::collect_matches_ordered(server, node, rule, count, traverse, &mut results, current_object, &mut found, true);
        results
    }

    // GetMatchesFrom: find matches before currentObject in tree order
    fn get_matches_from_async(
        server: &AtSpiServer,
        node: &Rc<AtSpiNode>,
        current_object: &str,
        rule: &AtSpiMatchRule,
        count: i32,
        traverse: bool,
    ) -> Vec<AtSpiObjectReference> {
        let (mut results, mut found) = (Vec::new(), false);
        Self::collect_matches_ordered(server, node, rule, count, traverse, &mut results, current_object, &mut found, false);
        results
    }

    // Not implemented in most toolkits
    fn get_active_descendant_async(server: &AtSpiServer) -> AtSpiObjectReference {
        server.get_null_reference()
    }

    fn collect_matches(
        server: &AtSpiServer,
        parent: &Rc<AtSpiNode>,
        rule: &AtSpiMatchRule,
        count: i32,
        traverse: bool,
        results: &mut Vec<AtSpiObjectReference>,
    ) {
        if full(count, results) {
            return;
        }

        for child_node in parent.ensure_children() {
            if matches_rule(&MatchSubject::of(&child_node), rule) {
                results.push(server.get_reference(Some(&child_node)));
                if full(count, results) {
                    return;
                }
            }

            if traverse {
                Self::collect_matches(server, &child_node, rule, count, traverse, results);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_matches_ordered(
        server: &AtSpiServer,
        parent: &Rc<AtSpiNode>,
        rule: &AtSpiMatchRule,
        count: i32,
        traverse: bool,
        results: &mut Vec<AtSpiObjectReference>,
        target_path: &str,
        past_target: &mut bool,
        after: bool,
    ) {
        if full(count, results) {
            return;
        }

        for child_node in parent.ensure_children() {
            if full(count, results) {
                return;
            }

            if child_node.path() == target_path {
                *past_target = true;
                if traverse {
                    Self::collect_matches_ordered(
                        server, &child_node, rule, count, traverse, results, target_path, past_target, after,
                    );
                }
                continue;
            }

            let should_include = if after { *past_target } else { !*past_target };
            if should_include && matches_rule(&MatchSubject::of(&child_node), rule) {
                results.push(server.get_reference(Some(&child_node)));
                if full(count, results) {
                    return;
                }
            }

            if traverse {
                Self::collect_matches_ordered(
                    server, &child_node, rule, count, traverse, results, target_path, past_target, after,
                );
            }
        }
    }
}

fn wire(results: &[AtSpiObjectReference]) -> Vec<(String, OwnedObjectPath)> {
    results.iter().map(AtSpiObjectReference::to_wire).collect()
}

impl DBusInterface for AtSpiCollectionHandler {
    fn description(&self) -> &'static InterfaceDescription {
        &COLLECTION
    }

    fn call(&self, member: &str, body: &zbus::message::Body) -> CallResult {
        let (server, node) = (server_of(&self.server)?, node_of(&self.node)?);
        match member {
            "GetMatches" => {
                let (rule, sortby, count, traverse) = args::<(AtSpiMatchRuleWire, u32, i32, bool)>(body)?;
                reply((wire(&Self::get_matches_async(&server, &node, &rule.into(), sortby, count, traverse)),))
            }
            "GetMatchesTo" => {
                let (current_object, rule, _sortby, _tree, _limit_scope, count, traverse) =
                    args::<(OwnedObjectPath, AtSpiMatchRuleWire, u32, u32, bool, i32, bool)>(body)?;
                let results =
                    Self::get_matches_to_async(&server, &node, current_object.as_str(), &rule.into(), count, traverse);
                reply((wire(&results),))
            }
            "GetMatchesFrom" => {
                let (current_object, rule, _sortby, _tree, count, traverse) =
                    args::<(OwnedObjectPath, AtSpiMatchRuleWire, u32, u32, i32, bool)>(body)?;
                let results =
                    Self::get_matches_from_async(&server, &node, current_object.as_str(), &rule.into(), count, traverse);
                reply((wire(&results),))
            }
            "GetActiveDescendant" => reply((Self::get_active_descendant_async(&server).to_wire(),)),
            _ => Err(DBusError::unknown_method()),
        }
    }

    fn get_property(&self, name: &str) -> Option<Value<'static>> {
        (name == "version").then(|| Value::from(Self::version()))
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;
    use crate::at_spi::at_spi_constants::{build_state_set, IFACE_ACCESSIBLE, IFACE_ACTION, IFACE_COMPONENT};
    use crate::at_spi::at_spi_role::AtSpiRole;
    use crate::at_spi::at_spi_state::AtSpiState;

    fn button() -> MatchSubject {
        MatchSubject {
            states: build_state_set(&[AtSpiState::Enabled, AtSpiState::Focusable, AtSpiState::Checkable]),
            role: AtSpiRole::PushButton as u32,
            interfaces: BTreeSet::from([IFACE_ACCESSIBLE, IFACE_COMPONENT, IFACE_ACTION]),
            attributes: HashMap::from([("toolkit".to_string(), TOOLKIT_NAME.to_string())]),
        }
    }

    fn states(states: &[AtSpiState]) -> Vec<i32> {
        build_state_set(states).into_iter().map(|word| word as i32).collect()
    }

    fn role_set(role: AtSpiRole) -> Vec<i32> {
        let mut words = vec![0i32; 4];
        words[(role as u32 / 32) as usize] = (1u32 << (role as u32 % 32)) as i32;
        words
    }

    #[test]
    fn an_empty_rule_matches_and_can_be_inverted() {
        assert!(matches_rule(&button(), &AtSpiMatchRule::default()));
        assert!(!matches_rule(&button(), &AtSpiMatchRule { invert: true, ..Default::default() }));
    }

    #[test]
    fn states_match_all_any_none_and_empty() {
        let rule = |words: Vec<i32>, match_type| AtSpiMatchRule { states: words, state_match_type: match_type, ..Default::default() };
        let enabled_focused = states(&[AtSpiState::Enabled, AtSpiState::Focused]);
        assert!(!matches_rule(&button(), &rule(enabled_focused.clone(), MATCH_ALL)));
        assert!(matches_rule(&button(), &rule(enabled_focused.clone(), MATCH_ANY)));
        assert!(!matches_rule(&button(), &rule(enabled_focused, MATCH_NONE)));
        assert!(matches_rule(&button(), &rule(states(&[AtSpiState::Checked]), MATCH_NONE)));
        // The high word: checkable is state 41.
        assert!(matches_rule(&button(), &rule(states(&[AtSpiState::Checkable]), MATCH_ALL)));
        assert!(!matches_rule(&button(), &rule(states(&[AtSpiState::ReadOnly]), MATCH_ALL)));
        // "Empty" matches only a rule without states.
        assert!(!matches_rule(&button(), &rule(states(&[AtSpiState::Enabled]), MATCH_EMPTY)));
        assert!(matches_rule(&button(), &rule(vec![0, 0], MATCH_EMPTY)));
    }

    #[test]
    fn roles_match_by_their_bit() {
        let rule = |role, match_type| AtSpiMatchRule { roles: role_set(role), role_match_type: match_type, ..Default::default() };
        assert!(matches_rule(&button(), &rule(AtSpiRole::PushButton, MATCH_ANY)));
        assert!(matches_rule(&button(), &rule(AtSpiRole::PushButton, MATCH_ALL)));
        assert!(!matches_rule(&button(), &rule(AtSpiRole::PushButton, MATCH_NONE)));
        assert!(!matches_rule(&button(), &rule(AtSpiRole::Entry, MATCH_ANY)));
        assert!(matches_rule(&button(), &rule(AtSpiRole::Entry, MATCH_NONE)));
    }

    #[test]
    fn interfaces_match_by_short_and_full_names() {
        let rule = |names: &[&str], match_type| AtSpiMatchRule {
            interfaces: names.iter().map(|name| name.to_string()).collect(),
            interface_match_type: match_type,
            ..Default::default()
        };
        assert!(matches_rule(&button(), &rule(&["Action"], MATCH_ALL)));
        assert!(matches_rule(&button(), &rule(&["org.a11y.atspi.Action", "Component"], MATCH_ALL)));
        assert!(!matches_rule(&button(), &rule(&["Action", "Text"], MATCH_ALL)));
        assert!(matches_rule(&button(), &rule(&["Action", "Text"], MATCH_ANY)));
        assert!(matches_rule(&button(), &rule(&["Text"], MATCH_NONE)));
    }

    #[test]
    fn attributes_match_by_key_and_value() {
        let rule = |entries: &[(&str, &str)], match_type| AtSpiMatchRule {
            attributes: entries.iter().map(|(key, value)| (key.to_string(), value.to_string())).collect(),
            attribute_match_type: match_type,
            ..Default::default()
        };
        assert!(matches_rule(&button(), &rule(&[("toolkit", TOOLKIT_NAME)], MATCH_ALL)));
        assert!(!matches_rule(&button(), &rule(&[("toolkit", "other")], MATCH_ALL)));
        assert!(!matches_rule(&button(), &rule(&[("toolkit", TOOLKIT_NAME), ("explicit-name", "true")], MATCH_ALL)));
        assert!(matches_rule(&button(), &rule(&[("toolkit", TOOLKIT_NAME), ("explicit-name", "true")], MATCH_ANY)));
        assert!(matches_rule(&button(), &rule(&[("explicit-name", "true")], MATCH_NONE)));
    }
}
