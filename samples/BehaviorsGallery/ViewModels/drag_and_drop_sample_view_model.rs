//! Port of `ViewModels/DragAndDropSampleViewModel.cs`.

use super::items::{item_list, ItemList};
use super::{DragItemViewModel, DragNodeViewModel, ViewModelBase};
use ferroui_base::ferro_markup_type;
use ferroui_controls::primitives::SelectedItemsList;
use ferroui_controls::ItemsSource;
use ferroui_reactiveui::reactive_object;
use std::cell::RefCell;
use std::rc::Rc;

/// The view model of the drag and drop pages: a list of items, a tree of nodes, the selected nodes
/// of the tree, and the two lists of the managed drag and drop.
pub struct DragAndDropSampleViewModel {
    base: ViewModelBase,
    items: RefCell<ItemList>,
    nodes: RefCell<ItemList>,
    selected_tree_nodes: SelectedItemsList,
    managed_source_items: RefCell<ItemList>,
    managed_target_items: RefCell<ItemList>,
    has_multiple_tree_nodes_selected: RefCell<bool>,
}
reactive_object!(DragAndDropSampleViewModel, base);
super::reactive_accessors!(DragAndDropSampleViewModel {
    items, set_items, "Items": ItemList;
    nodes, set_nodes, "Nodes": ItemList;
    managed_source_items, set_managed_source_items, "ManagedSourceItems": ItemList;
    managed_target_items, set_managed_target_items, "ManagedTargetItems": ItemList;
    has_multiple_tree_nodes_selected, set_has_multiple_tree_nodes_selected, "HasMultipleTreeNodesSelected": bool;
});

impl PartialEq for DragAndDropSampleViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A node with three children, each with the node as its parent.
fn node_with_children(title: &str) -> Rc<DragNodeViewModel> {
    let node = DragNodeViewModel::new(title);
    let children: Vec<Rc<DragNodeViewModel>> = (0..3)
        .map(|index| {
            let child = DragNodeViewModel::new(&format!("{title}-{index}"));
            child.set_parent(Some(&node));
            child
        })
        .collect();
    node.set_nodes(Some(item_list(children)));
    node
}

impl DragAndDropSampleViewModel {
    /// `DragAndDropSampleViewModel()`.
    pub fn new() -> Rc<Self> {
        let view_model = Rc::new(Self {
            base: ViewModelBase::new(),
            items: RefCell::new(item_list(
                (0..5).map(|index| DragItemViewModel::new(&format!("Item{index}"))),
            )),
            nodes: RefCell::new(item_list([
                node_with_children("Node0"),
                node_with_children("Node1"),
                node_with_children("Node2"),
            ])),
            selected_tree_nodes: SelectedItemsList::new(),
            managed_source_items: RefCell::new(item_list(
                (0..4).map(|index| DragItemViewModel::new(&format!("PreviewItem{index}"))),
            )),
            managed_target_items: RefCell::new(item_list(
                (4..6).map(|index| DragItemViewModel::new(&format!("PreviewItem{index}"))),
            )),
            has_multiple_tree_nodes_selected: RefCell::new(false),
        });

        let weak = Rc::downgrade(&view_model);
        view_model
            .selected_tree_nodes
            .add_collection_changed(Rc::new(move |_| {
                if let Some(view_model) = weak.upgrade() {
                    view_model.on_selected_tree_nodes_changed();
                }
            }));
        view_model
    }

    /// `SelectedTreeNodes`.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors gallery): the list is the selected items list of the
    /// controls (a list of optional untyped values), so that the two-way binding of the selected
    /// items of the tree view of the drag and drop page shares it.
    pub fn selected_tree_nodes(&self) -> SelectedItemsList {
        self.selected_tree_nodes.clone()
    }

    fn on_selected_tree_nodes_changed(&self) {
        self.set_has_multiple_tree_nodes_selected(self.selected_tree_nodes.count() > 1);
    }
}

type Vm = Rc<DragAndDropSampleViewModel>;

ferro_markup_type!(class DragAndDropSampleViewModel {
    this: Rc<DragAndDropSampleViewModel>,
    handles: [DragAndDropSampleViewModel, Rc<DragAndDropSampleViewModel>, Option<Rc<DragAndDropSampleViewModel>>],
    constructors: [() => DragAndDropSampleViewModel::new],
    properties: [
        Items: ItemsSource { get: |this: &Vm| ItemsSource::from(this.items()) },
        Nodes: ItemsSource { get: |this: &Vm| ItemsSource::from(this.nodes()) },
        SelectedTreeNodes: Option<SelectedItemsList> { get: |this: &Vm| Some(this.selected_tree_nodes()) },
        ManagedSourceItems: ItemsSource { get: |this: &Vm| ItemsSource::from(this.managed_source_items()) },
        ManagedTargetItems: ItemsSource { get: |this: &Vm| ItemsSource::from(this.managed_target_items()) },
        HasMultipleTreeNodesSelected: bool {
            get: |this: &Vm| this.has_multiple_tree_nodes_selected(),
            set: |this: &Vm, value: bool| this.set_has_multiple_tree_nodes_selected(value)
        },
    ],
    notify_property_changed: DragAndDropSampleViewModel,
});
