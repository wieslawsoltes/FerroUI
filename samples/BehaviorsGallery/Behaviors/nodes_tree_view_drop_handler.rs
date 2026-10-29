//! Port of `Behaviors/NodesTreeViewDropHandler.cs`.

use crate::view_models::items::{as_item, boxed, index_of, ItemList};
use crate::view_models::{DragAndDropSampleViewModel, DragNodeViewModel};
use ferroui_base::input::{DragDropEffects, DragEventArgs};
use ferroui_base::{ferro_markup_type, BoxedValue, Ref, Visual};
use ferroui_controls::{Control, TreeView, TreeViewItem};
use ferroui_xaml_interactions_draganddrop::{
    BaseTreeViewDropHandler, DropHandlerBase, IDropHandler,
};
use std::rc::Rc;

/// The drop handler of the tree of nodes of the drag and drop pages: a copy inserts a copy of the
/// dragged node before or after the node it is dropped on (the half of the node the pointer is
/// over), a move moves it there (to another parent too), a link swaps the two nodes.
#[derive(Default)]
pub struct NodesTreeViewDropHandler;

impl PartialEq for NodesTreeViewDropHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The children of `parent`, or the root nodes of the view model.
fn nodes_of(
    parent: &Option<Rc<DragNodeViewModel>>,
    vm: &DragAndDropSampleViewModel,
) -> Option<ItemList> {
    match parent {
        Some(parent) => parent.nodes(),
        None => Some(vm.nodes()),
    }
}

/// Whether two optional nodes are the same node.
fn same_node(a: &Option<Rc<DragNodeViewModel>>, b: &Option<Rc<DragNodeViewModel>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

impl BaseTreeViewDropHandler for NodesTreeViewDropHandler {
    fn validate_tree_view(
        &self,
        tree_view: &Ref<TreeView>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        execute: bool,
    ) -> (bool, bool) {
        let tree_view_visual: &Visual = tree_view;
        let (Some(source_node), Some(vm), Some(target_node)) = (
            source_context.and_then(as_item::<DragNodeViewModel>),
            target_context.and_then(as_item::<DragAndDropSampleViewModel>),
            tree_view
                .get_visual_at(e.get_position(tree_view_visual))
                .and_then(|visual| visual.cast::<Control>())
                .and_then(|target_control| target_control.data_context())
                .and_then(|data_context| as_item::<DragNodeViewModel>(&data_context)),
        ) else {
            return (false, false);
        };
        if Rc::ptr_eq(&source_node, &target_node)
            || target_node.is_descendant_of(&source_node) // block moving parent to inside child
            || vm.has_multiple_tree_nodes_selected()
        {
            // moving multiple items is disabled because
            // when an item is clicked to be dragged (whilst pressing Ctrl),
            // it becomes unselected and won't be considered for movement.
            // TODO: find how to fix that.
            return (false, false);
        }

        let source_parent = source_node.parent();
        let target_parent = target_node.parent();
        let (Some(source_nodes), Some(target_nodes)) =
            (nodes_of(&source_parent, &vm), nodes_of(&target_parent, &vm))
        else {
            return (false, false);
        };
        let are_source_nodes_different_than_target_nodes = !source_nodes.ptr_eq(&target_nodes);

        let (Some(source_index), Some(target_index)) = (
            index_of(&source_nodes, &source_node),
            index_of(&target_nodes, &target_node),
        ) else {
            return (false, false);
        };

        let mut insert_index = target_index;

        if let Some(source) = e.source().and_then(|source| source.cast::<Control>()) {
            if let Some(tree_view_item) =
                source.find_logical_ancestor_of_type::<TreeViewItem>(false)
            {
                let item_visual: &Visual = &tree_view_item;
                if e.get_position(item_visual).y > tree_view_item.bounds().height / 2.0 {
                    insert_index = target_index + 1;
                }
            }
        }

        let mut adjusted_target_index = insert_index;
        if same_node(&source_parent, &target_parent) && adjusted_target_index > source_index {
            adjusted_target_index -= 1;
        }

        let effects = e.drag_effects();
        if effects == DragDropEffects::COPY {
            if execute {
                let clone = DragNodeViewModel::new(&format!(
                    "{}_copy",
                    source_node.title().unwrap_or_default()
                ));
                self.insert_item(&target_nodes, boxed(&clone), insert_index);
            }

            (true, are_source_nodes_different_than_target_nodes)
        } else if effects == DragDropEffects::MOVE {
            if execute {
                if !are_source_nodes_different_than_target_nodes {
                    self.move_item(&source_nodes, source_index, adjusted_target_index);
                } else {
                    source_node.set_parent(target_parent.as_ref());
                    source_nodes.remove_at(source_index);
                    let insertion_index = insert_index.min(target_nodes.count());
                    target_nodes.insert(insertion_index, boxed(&source_node));
                }
            }

            (true, are_source_nodes_different_than_target_nodes)
        } else if effects == DragDropEffects::LINK {
            if execute {
                if !are_source_nodes_different_than_target_nodes {
                    self.swap_item(&source_nodes, source_index, adjusted_target_index);
                } else {
                    source_node.set_parent(target_parent.as_ref());
                    target_node.set_parent(source_parent.as_ref());

                    self.swap_item_between(
                        &source_nodes,
                        &target_nodes,
                        source_index,
                        target_index,
                    );
                }
            }

            (true, are_source_nodes_different_than_target_nodes)
        } else {
            (false, false)
        }
    }
}

ferro_markup_type!(class NodesTreeViewDropHandler {
    this: Rc<NodesTreeViewDropHandler>,
    handles: [NodesTreeViewDropHandler, Rc<NodesTreeViewDropHandler>, Option<Rc<NodesTreeViewDropHandler>>],
    interfaces: [Rc<dyn IDropHandler>],
    constructors: [() => || Rc::new(NodesTreeViewDropHandler)],
});
