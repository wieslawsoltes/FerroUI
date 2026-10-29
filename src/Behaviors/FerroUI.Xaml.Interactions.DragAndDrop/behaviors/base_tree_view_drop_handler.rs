//! Port of `Behaviors/BaseTreeViewDropHandler.cs`.

use crate::values::object_of;
use crate::DropHandlerBase;
use ferroui_base::input::DragEventArgs;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{BoxedValue, Ref, StyledElement, Visual};
use ferroui_controls::{Control, TreeView, TreeViewItem};

const ROW_DRAGGING_UP_STYLE_CLASS: &str = "DraggingUp";
const ROW_DRAGGING_DOWN_STYLE_CLASS: &str = "DraggingDown";
const TARGET_HIGHLIGHT_STYLE_CLASS: &str = "TargetHighlight";

/// Provides base drag-and-drop visuals and validation helpers for [`TreeView`] scenarios: while an
/// item is dragged over a tree view, the item under the pointer gets the class `DraggingUp` or
/// `DraggingDown` (the half of the item the pointer is over), or `TargetHighlight` when the dragged
/// item would move to another parent (the styles document of the crate styles these classes).
///
/// Deviation (DEVIATIONS.md, Behaviors): the abstract class of the reference is a trait, and every
/// type that implements it is a [`DropHandlerBase`] with the validation, execution and cancellation
/// of the class; the abstract overload `Validate(TreeView, ..)` of the reference is
/// [`validate_tree_view`](Self::validate_tree_view).
pub trait BaseTreeViewDropHandler {
    /// Validates a pending tree-view drag operation and optionally executes it (when `execute`).
    ///
    /// Returns whether the operation is valid, and whether the source item will be moved to a
    /// different parent.
    fn validate_tree_view(
        &self,
        tree_view: &Ref<TreeView>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        execute: bool,
    ) -> (bool, bool);
}

/// Every tree view drop handler is a drop handler base, with the members the class overrides.
impl<H: BaseTreeViewDropHandler> DropHandlerBase for H {
    fn validate(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        let source_is_control = e.source().is_some_and(|source| source.is::<Control>());
        if let Some(tree_view) = object_of(sender).and_then(|sender| sender.cast::<TreeView>()).filter(|_| source_is_control) {
            let (valid, will_source_item_change_parent) =
                self.validate_tree_view(&tree_view, e, source_context, target_context, false);
            let tree_view_visual: &Visual = &tree_view;
            let target_visual = tree_view.get_visual_at(e.get_position(tree_view_visual));
            if valid {
                if let Some(target_item) = find_tree_view_item_from_child_view(target_visual.as_deref().map(styled_element_of)) {
                    let target_item_visual: &Visual = &target_item;
                    let is_direction_up = e.get_position(target_item_visual).y < target_item.bounds().height / 2.0;
                    let item_to_apply_style = match target_item.parent().and_then(|parent| parent.cast::<TreeViewItem>()) {
                        Some(parent_item) if will_source_item_change_parent => parent_item,
                        _ => target_item,
                    };
                    apply_dragging_style_to_item(Some(&item_to_apply_style), is_direction_up, will_source_item_change_parent);
                    clear_dragging_style_from_all_items(sender, Some(&item_to_apply_style));
                }
            }

            return valid;
        }

        clear_dragging_style_from_all_items(sender, None);
        false
    }

    fn execute(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        clear_dragging_style_from_all_items(sender, None);
        let source_is_control = e.source().is_some_and(|source| source.is::<Control>());
        if let Some(tree_view) = object_of(sender).and_then(|sender| sender.cast::<TreeView>()).filter(|_| source_is_control) {
            let (valid, _) = self.validate_tree_view(&tree_view, e, source_context, target_context, true);
            return valid;
        }

        false
    }

    fn cancel(&self, sender: Option<&BoxedValue>, _e: &RoutedEventArgs) {
        clear_dragging_style_from_all_items(sender, None);
    }
}

/// A visual as the styled element it is.
fn styled_element_of(visual: &Visual) -> &StyledElement {
    visual
}

/// Finds the [`TreeViewItem`] associated with the provided visual descendant.
fn find_tree_view_item_from_child_view(source_child: Option<&StyledElement>) -> Option<Ref<TreeViewItem>> {
    source_child?.find_logical_ancestor_of_type::<TreeViewItem>(false)
}

fn clear_dragging_style_from_all_items(sender: Option<&BoxedValue>, except_this: Option<&Ref<TreeViewItem>>) {
    let Some(root_visual) = object_of(sender).and_then(|sender| sender.cast::<Visual>()) else {
        return;
    };

    clear_dragging_style_from_children(&root_visual, except_this);
}

fn clear_dragging_style_from_children(root: &StyledElement, except_this: Option<&Ref<TreeViewItem>>) {
    for item in root.get_logical_children().iter().filter_map(|child| child.cast::<TreeViewItem>()) {
        if except_this.is_some_and(|except_this| except_this.ptr_eq(&item)) {
            continue;
        }

        let classes = item.classes();
        classes.remove(ROW_DRAGGING_UP_STYLE_CLASS);
        classes.remove(ROW_DRAGGING_DOWN_STYLE_CLASS);
        classes.remove(TARGET_HIGHLIGHT_STYLE_CLASS);

        clear_dragging_style_from_children(&item, except_this);
    }
}

fn apply_dragging_style_to_item(
    item: Option<&Ref<TreeViewItem>>,
    is_direction_up: bool,
    will_source_item_be_moved_to_different_parent: bool,
) {
    let Some(item) = item else {
        return;
    };

    let classes = item.classes();
    if will_source_item_be_moved_to_different_parent {
        classes.remove(ROW_DRAGGING_DOWN_STYLE_CLASS);
        classes.remove(ROW_DRAGGING_UP_STYLE_CLASS);
        classes.add(TARGET_HIGHLIGHT_STYLE_CLASS);
    } else if is_direction_up {
        classes.remove(ROW_DRAGGING_DOWN_STYLE_CLASS);
        classes.remove(TARGET_HIGHLIGHT_STYLE_CLASS);
        classes.add(ROW_DRAGGING_UP_STYLE_CLASS);
    } else {
        classes.remove(ROW_DRAGGING_UP_STYLE_CLASS);
        classes.remove(TARGET_HIGHLIGHT_STYLE_CLASS);
        classes.add(ROW_DRAGGING_DOWN_STYLE_CLASS);
    }
}
