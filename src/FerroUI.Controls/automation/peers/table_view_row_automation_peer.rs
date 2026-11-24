use super::{
    AutomationControlType, AutomationPeerImpl, ControlAutomationPeer, ControlAutomationPeerImpl,
    ListItemAutomationPeer,
};
use crate::automation::provider::{IRangeValueProvider, IValueProvider};
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::primitives::AccessText;
use crate::{ContentControl, Control, TableViewCell, TableViewRow, TextBlock};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, Visual};
use std::rc::Rc;

/// An automation peer of a row of a `TableView`: a data item named after the content of its
/// visible cells, joined by spaces, where it has no name of its own (upstream c4d24e9dab).
#[repr(C)]
pub struct TableViewRowAutomationPeer {
    base: ListItemAutomationPeer,
}

ferro_class!(TableViewRowAutomationPeer: ListItemAutomationPeer);
ferroui_base::ferro_class_info!(TableViewRowAutomationPeer {});

impl FerroObjectImpl for TableViewRowAutomationPeer {}
impl ControlAutomationPeerImpl for TableViewRowAutomationPeer {}

impl AutomationPeerImpl for TableViewRowAutomationPeer {
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::DataItem
    }

    fn get_name_core(this: &Self) -> Option<String> {
        let owner = this.owner();
        if let Some(name) = AutomationProperties::get_name(&owner).filter(|name| !name.trim().is_empty()) {
            return Some(name);
        }
        if let Some(name) = this.get_labeled_by().map(|peer| peer.get_name()).filter(|n| !n.trim().is_empty()) {
            return Some(name);
        }
        let Some(row) = owner.cast::<TableViewRow>() else { return Self::parent_get_name_core(this) };
        let Some(presenter) = row.cells_presenter() else { return Self::parent_get_name_core(this) };

        let mut builder = String::new();
        let mut has_visible_cell = false;
        let children = presenter.children();
        for child in children.iter() {
            let Some(cell) = child.cast::<TableViewCell>() else { continue };
            if !cell.is_visible() || AutomationProperties::get_accessibility_view(&cell) == AccessibilityView::Raw {
                continue;
            }
            has_visible_cell = true;
            append_cell_name(&row, &cell, &mut builder);
        }

        if !builder.is_empty() {
            return Some(builder);
        }
        if has_visible_cell || children.is_empty() {
            Self::parent_get_name_core(this)
        } else {
            None
        }
    }
}

impl TableViewRowAutomationPeer {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct(owner: &TableViewRow) -> Self {
        Self { base: ListItemAutomationPeer::construct(owner) }
    }

    /// Initializes a new peer of `owner`.
    pub fn new(owner: &TableViewRow) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}

fn is_row_content(row: &TableViewRow, content: &BoxedValue) -> bool {
    row.content().is_some_and(|row_content| Rc::ptr_eq(&row_content, content))
}

fn is_string(content: &BoxedValue) -> bool {
    content.downcast_ref::<String>().is_some()
}

fn append_cell_name(row: &TableViewRow, cell: &TableViewCell, builder: &mut String) {
    if let Some(name) = AutomationProperties::get_name(cell).filter(|name| !name.trim().is_empty()) {
        append_name(Some(&name), builder);
        return;
    }
    if let Some(labeled_by) = AutomationProperties::get_labeled_by(cell) {
        append_visual_name(row, labeled_by.upcast_ref::<Visual>(), builder, false);
        return;
    }
    if let Some(child) = cell.presenter().and_then(|presenter| presenter.child()) {
        append_visual_name(row, child.upcast_ref::<Visual>(), builder, true);
        return;
    }
    if let Some(content) = cell.content() {
        if is_string(&content) || !is_row_content(row, &content) {
            append_name(Some(&ValueTypes::to_display_string(Some(&content))), builder);
        }
    }
}

fn append_visual_name(row: &TableViewRow, visual: &Visual, builder: &mut String, allow_labeled_by: bool) {
    let Some(control) = visual.cast::<Control>() else {
        for child in visual.visual_children().iter() {
            append_visual_name(row, &child, builder, allow_labeled_by);
        }
        return;
    };
    if !control.is_visible() || AutomationProperties::get_accessibility_view(&control) == AccessibilityView::Raw {
        return;
    }
    if let Some(name) = AutomationProperties::get_name(&control).filter(|name| !name.trim().is_empty()) {
        append_name(Some(&name), builder);
        return;
    }
    if allow_labeled_by {
        if let Some(labeled_by) = AutomationProperties::get_labeled_by(&control) {
            let previous_length = builder.len();
            append_visual_name(row, labeled_by.upcast_ref::<Visual>(), builder, false);
            if builder.len() > previous_length {
                return;
            }
        }
    }
    if let Some(access_text) = control.cast::<AccessText>() {
        let text = access_text.inlines().and_then(|inlines| inlines.text()).or_else(|| access_text.text());
        append_name(AccessText::remove_access_key_marker(text.as_deref()).as_deref(), builder);
        return;
    }
    if let Some(text_block) = control.cast::<TextBlock>() {
        let text = text_block.inlines().and_then(|inlines| inlines.text()).or_else(|| text_block.text());
        append_name(text.as_deref(), builder);
        return;
    }
    let peer = ControlAutomationPeer::create_peer_for_element(&control);
    if let Some(value) = peer.get_provider::<dyn IValueProvider>() {
        append_name(value.value().as_deref(), builder);
        return;
    }
    if let Some(range) = peer.get_provider::<dyn IRangeValueProvider>() {
        append_name(Some(&range.value().to_string()), builder);
        return;
    }
    if !control.visual_children().is_empty() {
        for child in control.visual_children().iter() {
            append_visual_name(row, &child, builder, true);
        }
        return;
    }
    let shows_row_content = control
        .cast::<ContentControl>()
        .and_then(|content_control| content_control.content())
        .is_some_and(|content| is_row_content(row, &content) && !is_string(&content));
    if !shows_row_content {
        append_name(Some(&peer.get_name()), builder);
    }
}

fn append_name(name: Option<&str>, builder: &mut String) {
    let Some(name) = name.map(str::trim).filter(|name| !name.is_empty()) else { return };
    if !builder.is_empty() {
        builder.push(' ');
    }
    builder.push_str(name);
}

#[cfg(test)]
mod tests {
    use super::append_name;

    #[test]
    fn names_are_trimmed_and_joined_by_spaces() {
        let mut builder = String::new();
        append_name(Some("  First "), &mut builder);
        append_name(None, &mut builder);
        append_name(Some("   "), &mut builder);
        append_name(Some("Second"), &mut builder);
        assert_eq!(builder, "First Second");
    }
}
