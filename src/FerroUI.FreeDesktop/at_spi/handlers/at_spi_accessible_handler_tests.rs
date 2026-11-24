//! The tests of `AtSpiAccessibleHandlerTests.cs` (a test file of the unit tests of the controls
//! upstream, to which the AT-SPI project makes its internals visible; here they are tests of the
//! crate that has the module).
//!
//! The handlers of the port answer from a node (`AtSpiAccessibleHandler::get_role_async(node)`);
//! upstream's tests make a handler of the node and ask it.

use super::at_spi_accessible_handler::AtSpiAccessibleHandler;
use super::at_spi_collection_handler::{matches_rule, MatchSubject};
use crate::at_spi::at_spi_node::AtSpiNode;
use crate::at_spi::at_spi_role::AtSpiRole;
use crate::at_spi::at_spi_server::AtSpiServer;
use crate::at_spi::dbus::types::{AtSpiAttributeSet, AtSpiMatchRule};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, ListBoxItem, TextBlock};
use std::rc::Rc;

const HEADING_ROLE: u32 = 83;
const DOCUMENT_ROLE: i32 = 82;
const LINK_ROLE: i32 = 88;
const TREE_ITEM_ROLE: i32 = 91;
const TITLE_BAR_ROLE: i32 = 104;

/// What the tests ask the handler of: the node of the control's peer (the handler of the
/// reference holds the server and the node).
struct Handler {
    // The server owns the node by its peer.
    _server: Rc<AtSpiServer>,
    node: Rc<AtSpiNode>,
}

impl Handler {
    fn get_role_async(&self) -> u32 {
        AtSpiAccessibleHandler::get_role_async(&self.node)
    }

    fn get_role_name_async(&self) -> &'static str {
        AtSpiAccessibleHandler::get_role_name_async(&self.node)
    }

    fn get_localized_role_name_async(&self) -> String {
        AtSpiAccessibleHandler::get_localized_role_name_async(&self.node)
    }

    fn description(&self) -> String {
        AtSpiAccessibleHandler::description(&self.node)
    }

    fn get_attributes_async(&self) -> AtSpiAttributeSet {
        AtSpiAccessibleHandler::get_attributes_async(&self.node)
    }
}

fn create_handler(control: &Control, server: Rc<AtSpiServer>) -> Handler {
    let peer = ControlAutomationPeer::create_peer_for_element(control);
    let node = server.get_or_create_node(&peer);
    Handler { _server: server, node }
}

fn text_block(text: &str) -> ferroui_base::Ref<TextBlock> {
    let control = TextBlock::new();
    control.set_text(Some(text));
    control
}

fn exposes_heading_role_and_level(level: i32) {
    let _app = UnitTestApplication::start(TestServices::default());
    let control = text_block("Section");
    AutomationProperties::set_heading_level(&control, level);
    let server = AtSpiServer::new();
    let handler = create_handler(&control, server.clone());

    assert_eq!(HEADING_ROLE, AtSpiRole::Heading as u32);
    assert_eq!(AtSpiRole::Heading as u32, handler.get_role_async());
    assert_eq!("heading", handler.get_role_name_async());
    assert_eq!("heading", handler.get_localized_role_name_async());
    assert_eq!(Some(&level.to_string()), handler.get_attributes_async().get("level"));
    server.dispose();
}

#[test]
fn exposes_a_named_landmark_and_the_full_description() {
    use ferroui_controls::automation::peers::AutomationLandmarkType;
    let _app = UnitTestApplication::start(TestServices::default());
    let control = ferroui_controls::Border::new();
    AutomationProperties::set_landmark_type(&control, Some(AutomationLandmarkType::Custom));
    AutomationProperties::set_localized_landmark_type(&control, Some("Current viewers"));
    AutomationProperties::set_help_text(&control, Some("Help"));
    let server = AtSpiServer::new();
    let handler = create_handler(&control, server.clone());

    assert_eq!(AtSpiRole::Landmark as u32, handler.get_role_async());
    assert_eq!("landmark", handler.get_role_name_async());
    assert_eq!("Current viewers", handler.get_localized_role_name_async());
    // The help text is the description until the element has a full description.
    assert_eq!("Help", handler.description());
    AutomationProperties::set_full_description(&control, Some("The people watching now"));
    assert_eq!("The people watching now", handler.description());
    AutomationProperties::set_localized_landmark_type(&control, None);
    assert_eq!("landmark", handler.get_localized_role_name_async());
    server.dispose();
}

#[test]
fn exposes_heading_role_and_level_rows() {
    for level in 1..=6 {
        exposes_heading_role_and_level(level);
    }
}

fn unset_heading_preserves_label_role(level: i32) {
    let _app = UnitTestApplication::start(TestServices::default());
    let control = text_block("Label");
    AutomationProperties::set_heading_level(&control, level);
    let server = AtSpiServer::new();
    let handler = create_handler(&control, server.clone());

    assert_eq!(AtSpiRole::Label as u32, handler.get_role_async());
    assert_eq!("label", handler.get_role_name_async());
    assert!(!handler.get_attributes_async().contains_key("level"));
    server.dispose();
}

#[test]
fn unset_heading_preserves_label_role_rows() {
    for level in [0, -1] {
        unset_heading_preserves_label_role(level);
    }
}

#[test]
fn exposes_list_position_and_size() {
    let _app = UnitTestApplication::start(TestServices::default());
    let control = ListBoxItem::new();
    AutomationProperties::set_position_in_set(&control, 3);
    AutomationProperties::set_size_of_set(&control, 10);
    let server = AtSpiServer::new();
    let handler = create_handler(&control, server.clone());
    let attributes = handler.get_attributes_async();

    assert_eq!(Some("3"), attributes.get("posinset").map(String::as_str));
    assert_eq!(Some("10"), attributes.get("setsize").map(String::as_str));
    assert_eq!(AtSpiRole::ListItem as u32, handler.get_role_async());
    server.dispose();
}

fn uses_at_spi_role_ids(control_type: AutomationControlType, expected: i32) {
    assert_eq!(expected, AtSpiNode::to_at_spi_role(control_type, None) as i32);
}

#[test]
fn uses_at_spi_role_ids_rows() {
    uses_at_spi_role_ids(AutomationControlType::Document, DOCUMENT_ROLE);
    uses_at_spi_role_ids(AutomationControlType::Hyperlink, LINK_ROLE);
    uses_at_spi_role_ids(AutomationControlType::TreeItem, TREE_ITEM_ROLE);
    uses_at_spi_role_ids(AutomationControlType::TitleBar, TITLE_BAR_ROLE);
}

fn omits_unset_list_position_and_size(value: i32) {
    let _app = UnitTestApplication::start(TestServices::default());
    let control = ListBoxItem::new();
    AutomationProperties::set_position_in_set(&control, value);
    AutomationProperties::set_size_of_set(&control, value);
    let server = AtSpiServer::new();
    let attributes = create_handler(&control, server.clone()).get_attributes_async();

    assert!(!attributes.contains_key("posinset"));
    assert!(!attributes.contains_key("setsize"));
    server.dispose();
}

#[test]
fn omits_unset_list_position_and_size_rows() {
    for value in [-1, 0] {
        omits_unset_list_position_and_size(value);
    }
}

fn exposes_list_position_and_size_independently(
    position: i32,
    size: i32,
    expected_position: Option<&str>,
    expected_size: Option<&str>,
) {
    let _app = UnitTestApplication::start(TestServices::default());
    let control = ListBoxItem::new();
    AutomationProperties::set_position_in_set(&control, position);
    AutomationProperties::set_size_of_set(&control, size);
    let server = AtSpiServer::new();
    let attributes = create_handler(&control, server.clone()).get_attributes_async();

    assert_eq!(expected_position, attributes.get("posinset").map(String::as_str));
    assert_eq!(expected_size, attributes.get("setsize").map(String::as_str));
    server.dispose();
}

#[test]
fn exposes_list_position_and_size_independently_rows() {
    exposes_list_position_and_size_independently(3, -1, Some("3"), None);
    exposes_list_position_and_size_independently(-1, 10, None, Some("10"));
}

#[test]
fn reads_updated_metadata() {
    let _app = UnitTestApplication::start(TestServices::default());
    let control = text_block("Section");
    let server = AtSpiServer::new();
    let handler = create_handler(&control, server.clone());
    assert!(!handler.get_attributes_async().contains_key("level"));

    AutomationProperties::set_heading_level(&control, 2);
    AutomationProperties::set_position_in_set(&control, 1);
    AutomationProperties::set_size_of_set(&control, 4);
    let attributes = handler.get_attributes_async();

    assert_eq!(Some("2"), attributes.get("level").map(String::as_str));
    assert_eq!(Some("1"), attributes.get("posinset").map(String::as_str));
    assert_eq!(Some("4"), attributes.get("setsize").map(String::as_str));
    assert_eq!(AtSpiRole::Heading as u32, handler.get_role_async());

    control.clear_value(AutomationProperties::heading_level_property());
    control.clear_value(AutomationProperties::position_in_set_property());
    control.clear_value(AutomationProperties::size_of_set_property());
    let attributes = handler.get_attributes_async();

    assert!(!attributes.contains_key("level"));
    assert!(!attributes.contains_key("posinset"));
    assert!(!attributes.contains_key("setsize"));
    assert_eq!(AtSpiRole::Label as u32, handler.get_role_async());
    server.dispose();
}

fn collection_matches_heading_and_list_attributes(key: &str, value: &str, expected: bool) {
    let _app = UnitTestApplication::start(TestServices::default());
    let heading = text_block("Section");
    AutomationProperties::set_heading_level(&heading, 2);
    AutomationProperties::set_position_in_set(&heading, 3);
    AutomationProperties::set_size_of_set(&heading, 4);
    let server = AtSpiServer::new();
    let node = server.get_or_create_node(&ControlAutomationPeer::create_peer_for_element(&heading));
    let rule = AtSpiMatchRule {
        states: Vec::new(),
        state_match_type: 0,
        attributes: AtSpiAttributeSet::from([(key.to_string(), value.to_string())]),
        attribute_match_type: 1,
        roles: Vec::new(),
        role_match_type: 0,
        interfaces: Vec::new(),
        interface_match_type: 0,
        invert: false,
    };

    // `AtSpiCollectionHandler.MatchesRule(node, rule)`: the port matches a rule against what
    // it reads of the node.
    assert_eq!(expected, matches_rule(&MatchSubject::of(&node), &rule));
    server.dispose();
}

#[test]
fn collection_matches_heading_and_list_attributes_rows() {
    collection_matches_heading_and_list_attributes("level", "2", true);
    collection_matches_heading_and_list_attributes("posinset", "3", true);
    collection_matches_heading_and_list_attributes("setsize", "4", true);
    collection_matches_heading_and_list_attributes("level", "1", false);
}
