//! The port of `AtSpiNode.StateMapping.cs`.

use super::at_spi_constants::build_state_set;
use super::at_spi_node::AtSpiNode;
use super::at_spi_state::AtSpiState;
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::{
    IExpandCollapseProvider, IRangeValueProvider, ISelectionItemProvider, ISelectionProvider, IToggleProvider,
    IValueProvider, ToggleState,
};
use ferroui_controls::automation::{AutomationProperties, ExpandCollapseState};

/// What the states of a node are computed from: the answers of its peer
/// and of the providers it has.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StateFacts {
    pub(crate) is_enabled: bool,
    pub(crate) is_offscreen: bool,
    pub(crate) is_keyboard_focusable: bool,
    pub(crate) has_keyboard_focus: bool,
    pub(crate) toggle_state: Option<ToggleState>,
    pub(crate) expand_collapse_state: Option<ExpandCollapseState>,
    /// `IsSelected` of a selection item.
    pub(crate) selection_item: Option<bool>,
    pub(crate) can_select_multiple: bool,
    /// `IsReadOnly` of a value provider.
    pub(crate) value_read_only: Option<bool>,
    pub(crate) range_value_read_only: bool,
    pub(crate) is_required_for_form: bool,
    pub(crate) control_type: AutomationControlType,
}

impl AtSpiNode {
    pub(crate) fn compute_states(&self) -> Vec<u32> {
        self.compute_states_core()
    }

    fn compute_states_core(&self) -> Vec<u32> {
        let peer = self.peer();
        let facts = StateFacts {
            is_enabled: peer.is_enabled(),
            is_offscreen: peer.is_offscreen(),
            is_keyboard_focusable: peer.is_keyboard_focusable(),
            has_keyboard_focus: peer.has_keyboard_focus(),
            toggle_state: peer.get_provider::<dyn IToggleProvider>().map(|toggle| toggle.toggle_state()),
            expand_collapse_state: peer
                .get_provider::<dyn IExpandCollapseProvider>()
                .map(|expand_collapse| expand_collapse.expand_collapse_state()),
            selection_item: peer.get_provider::<dyn ISelectionItemProvider>().map(|item| item.is_selected()),
            can_select_multiple: peer
                .get_provider::<dyn ISelectionProvider>()
                .is_some_and(|selection| selection.can_select_multiple()),
            value_read_only: peer.get_provider::<dyn IValueProvider>().map(|value| value.is_read_only()),
            range_value_read_only: peer
                .get_provider::<dyn IRangeValueProvider>()
                .is_some_and(|range| range.is_read_only()),
            is_required_for_form: peer
                .cast::<ControlAutomationPeer>()
                .is_some_and(|control_peer| AutomationProperties::get_is_required_for_form(&control_peer.owner())),
            control_type: peer.get_automation_control_type(),
        };

        build_state_set(&states_of(&facts))
    }
}

pub(crate) fn states_of(facts: &StateFacts) -> Vec<AtSpiState> {
    let mut states = Vec::new();

    if facts.is_enabled {
        states.push(AtSpiState::Enabled);
        states.push(AtSpiState::Sensitive);
    }

    if !facts.is_offscreen {
        states.push(AtSpiState::Visible);
        states.push(AtSpiState::Showing);
    }

    if facts.is_keyboard_focusable {
        states.push(AtSpiState::Focusable);
    }

    if facts.has_keyboard_focus {
        states.push(AtSpiState::Focused);
    }

    // Toggle state
    if let Some(toggle_state) = facts.toggle_state {
        states.push(AtSpiState::Checkable);
        match toggle_state {
            ToggleState::On => states.push(AtSpiState::Checked),
            ToggleState::Indeterminate => states.push(AtSpiState::Indeterminate),
            ToggleState::Off => {}
        }
    }

    // Expand/collapse state
    if let Some(expand_collapse_state) = facts.expand_collapse_state {
        states.push(AtSpiState::Expandable);
        match expand_collapse_state {
            ExpandCollapseState::Expanded => states.push(AtSpiState::Expanded),
            ExpandCollapseState::Collapsed => states.push(AtSpiState::Collapsed),
            _ => {}
        }
    }

    // Selection item states
    if let Some(is_selected) = facts.selection_item {
        states.push(AtSpiState::Selectable);
        if is_selected {
            states.push(AtSpiState::Selected);
        }
    }

    // Multi-selectable container
    if facts.can_select_multiple {
        states.push(AtSpiState::MultiSelectable);
    }

    // Value provider states (text editable/read-only)
    if let Some(read_only) = facts.value_read_only {
        if read_only {
            states.push(AtSpiState::ReadOnly);
        } else {
            states.push(AtSpiState::Editable);
        }
    }

    // Range value read-only
    if facts.range_value_read_only {
        states.push(AtSpiState::ReadOnly);
    }

    // Required for form
    if facts.is_required_for_form {
        states.push(AtSpiState::Required);
    }

    // Window-level active state and text entry states
    if facts.control_type == AutomationControlType::Window {
        states.push(AtSpiState::Active);
    }
    if facts.control_type == AutomationControlType::Edit {
        states.push(AtSpiState::SingleLine);
    }

    states
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    fn facts() -> StateFacts {
        StateFacts {
            is_enabled: false,
            is_offscreen: true,
            is_keyboard_focusable: false,
            has_keyboard_focus: false,
            toggle_state: None,
            expand_collapse_state: None,
            selection_item: None,
            can_select_multiple: false,
            value_read_only: None,
            range_value_read_only: false,
            is_required_for_form: false,
            control_type: AutomationControlType::None,
        }
    }

    #[test]
    fn nothing_gives_no_state() {
        assert_eq!(states_of(&facts()), vec![]);
    }

    #[test]
    fn an_enabled_visible_focused_control() {
        let states = states_of(&StateFacts {
            is_enabled: true,
            is_offscreen: false,
            is_keyboard_focusable: true,
            has_keyboard_focus: true,
            ..facts()
        });
        assert_eq!(
            states,
            vec![
                AtSpiState::Enabled,
                AtSpiState::Sensitive,
                AtSpiState::Visible,
                AtSpiState::Showing,
                AtSpiState::Focusable,
                AtSpiState::Focused
            ]
        );
    }

    #[test]
    fn toggle_states() {
        let of = |state| states_of(&StateFacts { toggle_state: Some(state), ..facts() });
        assert_eq!(of(ToggleState::Off), vec![AtSpiState::Checkable]);
        assert_eq!(of(ToggleState::On), vec![AtSpiState::Checkable, AtSpiState::Checked]);
        assert_eq!(of(ToggleState::Indeterminate), vec![AtSpiState::Checkable, AtSpiState::Indeterminate]);
    }

    #[test]
    fn expand_collapse_states() {
        let of = |state| states_of(&StateFacts { expand_collapse_state: Some(state), ..facts() });
        assert_eq!(of(ExpandCollapseState::Expanded), vec![AtSpiState::Expandable, AtSpiState::Expanded]);
        assert_eq!(of(ExpandCollapseState::Collapsed), vec![AtSpiState::Expandable, AtSpiState::Collapsed]);
        assert_eq!(of(ExpandCollapseState::LeafNode), vec![AtSpiState::Expandable]);
    }

    #[test]
    fn selection_value_and_form_states() {
        assert_eq!(states_of(&StateFacts { selection_item: Some(false), ..facts() }), vec![AtSpiState::Selectable]);
        assert_eq!(
            states_of(&StateFacts { selection_item: Some(true), can_select_multiple: true, ..facts() }),
            vec![AtSpiState::Selectable, AtSpiState::Selected, AtSpiState::MultiSelectable]
        );
        assert_eq!(states_of(&StateFacts { value_read_only: Some(true), ..facts() }), vec![AtSpiState::ReadOnly]);
        assert_eq!(states_of(&StateFacts { value_read_only: Some(false), ..facts() }), vec![AtSpiState::Editable]);
        assert_eq!(states_of(&StateFacts { range_value_read_only: true, ..facts() }), vec![AtSpiState::ReadOnly]);
        assert_eq!(states_of(&StateFacts { is_required_for_form: true, ..facts() }), vec![AtSpiState::Required]);
    }

    #[test]
    fn a_window_is_active_and_an_edit_is_a_single_line() {
        assert_eq!(
            states_of(&StateFacts { control_type: AutomationControlType::Window, ..facts() }),
            vec![AtSpiState::Active]
        );
        assert_eq!(
            states_of(&StateFacts { control_type: AutomationControlType::Edit, value_read_only: Some(false), ..facts() }),
            vec![AtSpiState::Editable, AtSpiState::SingleLine]
        );
    }
}
