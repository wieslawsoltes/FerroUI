//! Port of `ViewModels/TreeViewPageViewModel.cs`.

use super::random::Random;
use ferroui_base::collections::FerroList;
use ferroui_base::data::model::{BindableList, Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand;
use ferroui_controls::primitives::SelectedItemsList;
use ferroui_controls::{box_item, unbox_item, SelectionMode};
use mini_mvvm::{MiniCommand, ViewModelBase};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

ferroui_controls::ferro_markup_list!(pub NodeList: Rc<Node>);


/// The view model of the tree view page.
pub struct TreeViewPageViewModel {
    base: ViewModelBase,
    root: Rc<Node>,
    selection_mode: Cell<SelectionMode>,
    items: Rc<BindableList<Rc<Node>>>,
    /// The selected nodes: the untyped list a tree view shares with its
    /// `SelectedItems` property.
    selected_items: SelectedItemsList,
    add_item_command: Rc<MiniCommand>,
    remove_item_command: Rc<MiniCommand>,
    select_random_item_command: Rc<MiniCommand>,
}

impl PartialEq for TreeViewPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for TreeViewPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl TreeViewPageViewModel {
    pub fn new() -> Rc<TreeViewPageViewModel> {
        Rc::new_cyclic(|this: &Weak<TreeViewPageViewModel>| {
            let root = Node::new();

            let command = |action: fn(&TreeViewPageViewModel)| {
                let this = this.clone();
                MiniCommand::create(move || {
                    if let Some(this) = this.upgrade() {
                        action(&this);
                    }
                })
            };

            Self {
                base: ViewModelBase::new(),
                items: root.children(),
                root,
                selection_mode: Cell::new(SelectionMode::default()),
                selected_items: SelectedItemsList::new(),
                add_item_command: command(Self::add_item),
                remove_item_command: command(Self::remove_item),
                select_random_item_command: command(Self::select_random_item),
            }
        })
    }

    pub fn items(&self) -> Rc<BindableList<Rc<Node>>> {
        self.items.clone()
    }

    pub fn selected_items(&self) -> SelectedItemsList {
        self.selected_items.clone()
    }

    pub fn add_item_command(&self) -> Rc<MiniCommand> {
        self.add_item_command.clone()
    }

    pub fn remove_item_command(&self) -> Rc<MiniCommand> {
        self.remove_item_command.clone()
    }

    pub fn select_random_item_command(&self) -> Rc<MiniCommand> {
        self.select_random_item_command.clone()
    }

    pub fn selection_mode(&self) -> SelectionMode {
        self.selection_mode.get()
    }

    pub fn set_selection_mode(&self, value: SelectionMode) {
        self.selected_items.clear();
        self.base.raise_and_set_if_changed_cell(&self.selection_mode, value, "SelectionMode");
    }

    /// `(Node)SelectedItems[index]`.
    fn selected_node(&self, index: usize) -> Rc<Node> {
        match unbox_item::<Rc<Node>>(&self.selected_items.get(index)) {
            Some(node) => node,
            None => panic!("Unable to cast the selected item to type 'Node'."),
        }
    }

    fn add_item(&self) {
        let parent_item = if self.selected_items.count() > 0 { self.selected_node(0) } else { self.root.clone() };
        parent_item.add_item();
    }

    /// Removes the selected nodes. A node that is removed from the tree is
    /// removed from the selection by the tree view that shares the list of
    /// selected items; the loop relies on it, as upstream.
    fn remove_item(&self) {
        while self.selected_items.count() > 0 {
            let last_item = self.selected_node(0);
            Self::recursive_remove(&self.items, &last_item);
        }
    }

    fn recursive_remove(items: &Rc<BindableList<Rc<Node>>>, selected_item: &Rc<Node>) -> bool {
        if items.items().remove(selected_item) {
            return true;
        }

        for item in items.items().to_vec() {
            if item.are_children_initialized() && Self::recursive_remove(&item.children(), selected_item) {
                return true;
            }
        }

        false
    }

    fn select_random_item(&self) {
        let mut random = Random::new();
        let depth = random.next_max(4);
        let mut node = self.root.clone();

        for _ in 0..depth {
            let i = random.next_max(10);
            // An index outside of the children is an exception, as upstream.
            node = node.children().items().get(i as usize);
        }

        self.selected_items.clear();
        self.selected_items.add(box_item(&node));
    }
}

ferro_markup_type!(class TreeViewPageViewModel {
    this: Rc<TreeViewPageViewModel>,
    handles: [TreeViewPageViewModel, Rc<TreeViewPageViewModel>, Option<Rc<TreeViewPageViewModel>>],
    constructors: [() => TreeViewPageViewModel::new],
    properties: [
        // A list a binding delivers to an items source property.
        Items: FerroList<Rc<Node>> { get: |this: &Rc<TreeViewPageViewModel>| this.items().items().clone() },
        SelectedItems: SelectedItemsList { get: |this: &Rc<TreeViewPageViewModel>| this.selected_items() },
        AddItemCommand: Rc<dyn ICommand> {
            get: |this: &Rc<TreeViewPageViewModel>| this.add_item_command().as_command()
        },
        RemoveItemCommand: Rc<dyn ICommand> {
            get: |this: &Rc<TreeViewPageViewModel>| this.remove_item_command().as_command()
        },
        SelectRandomItemCommand: Rc<dyn ICommand> {
            get: |this: &Rc<TreeViewPageViewModel>| this.select_random_item_command().as_command()
        },
        SelectionMode: SelectionMode {
            get: |this: &Rc<TreeViewPageViewModel>| this.selection_mode(),
            set: |this: &Rc<TreeViewPageViewModel>, value: SelectionMode| this.set_selection_mode(value)
        },
    ],
    notify_property_changed: TreeViewPageViewModel,
});

/// `TreeViewPageViewModel.Node`: a node of the tree; its children are
/// created when they are first asked for.
pub struct Node {
    /// The parent is held weakly: a node is owned by the children of its
    /// parent.
    parent: Weak<Node>,
    header: String,
    children: RefCell<Option<Rc<BindableList<Rc<Node>>>>>,
    child_index: Cell<i32>,
    this: Weak<Node>,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Node {
    pub fn new() -> Rc<Node> {
        Rc::new_cyclic(|this| Self {
            parent: Weak::new(),
            header: "Item".to_string(),
            children: RefCell::new(None),
            child_index: Cell::new(10),
            this: this.clone(),
        })
    }

    pub fn with_parent(parent: &Rc<Node>, index: i32) -> Rc<Node> {
        Rc::new_cyclic(|this| Self {
            parent: Rc::downgrade(parent),
            header: format!("{} {index}", parent.header),
            children: RefCell::new(None),
            child_index: Cell::new(10),
            this: this.clone(),
        })
    }

    pub fn parent(&self) -> Option<Rc<Node>> {
        self.parent.upgrade()
    }

    pub fn header(&self) -> String {
        self.header.clone()
    }

    pub fn are_children_initialized(&self) -> bool {
        self.children.borrow().is_some()
    }

    pub fn children(&self) -> Rc<BindableList<Rc<Node>>> {
        if let Some(children) = &*self.children.borrow() {
            return children.clone();
        }
        let children = self.create_children();
        *self.children.borrow_mut() = Some(children.clone());
        children
    }

    pub fn add_item(&self) {
        let index = self.child_index.get();
        self.child_index.set(index + 1);
        self.children().items().add(Node::with_parent(&self.to_rc(), index));
    }

    pub fn remove_item(&self, child: &Rc<Node>) {
        self.children().items().remove(child);
    }

    fn to_rc(&self) -> Rc<Node> {
        self.this.upgrade().expect("a node is a shared object")
    }

    fn create_children(&self) -> Rc<BindableList<Rc<Node>>> {
        let this = self.to_rc();
        BindableList::new((0..10).map(|i| Node::with_parent(&this, i)))
    }
}

/// `ToString()`: the header.
impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.header)
    }
}

ferro_markup_type!(class Node as "TreeViewPageViewModel+Node" {
    this: Rc<Node>,
    handles: [Node, Rc<Node>, Option<Rc<Node>>],
    constructors: [() => Node::new, (Rc<Node>, i32) => |parent: Rc<Node>, index: i32| Node::with_parent(&parent, index)],
    properties: [
        Parent: Option<Rc<Node>> { get: |this: &Rc<Node>| this.parent() },
        Header: String { get: |this: &Rc<Node>| this.header() },
        AreChildrenInitialized: bool { get: |this: &Rc<Node>| this.are_children_initialized() },
        // A list a binding delivers to an items source property.
        Children: FerroList<Rc<Node>> { get: |this: &Rc<Node>| this.children().items().clone() },
    ],
    methods: [
        fn AddItem() => |this: &Rc<Node>| this.add_item(),
        fn RemoveItem(Rc<Node>) => |this: &Rc<Node>, child: Rc<Node>| this.remove_item(&child),
        fn ToString() -> String => |this: &Rc<Node>| this.to_string(),
    ],
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn a_node_creates_ten_children_when_they_are_first_asked_for() {
        let root = Node::new();
        assert!(!root.are_children_initialized());
        let children = root.children().items().to_vec();
        assert!(root.are_children_initialized());
        assert_eq!(10, children.len());
        assert_eq!("Item 0", children[0].header());
        assert_eq!("Item 9", children[9].to_string());
        assert!(Rc::ptr_eq(&root, &children[3].parent().expect("the parent")));
        assert!(root.parent().is_none());
        assert_eq!("Item 3 7", children[3].children().items().get(7).header());

        root.add_item();
        root.add_item();
        assert_eq!("Item 11", root.children().items().get(11).header());
        root.remove_item(&children[0]);
        assert_eq!(11, root.children().items().count());
    }

    #[test]
    fn add_adds_to_the_selected_node_or_to_the_root() {
        let view_model = TreeViewPageViewModel::new();
        view_model.add_item_command().execute(None);
        assert_eq!(11, view_model.items().items().count());

        let node = view_model.items().items().get(2);
        view_model.selected_items().add(box_item(&node));
        view_model.add_item_command().execute(None);
        assert_eq!(11, node.children().items().count());
        assert_eq!("Item 2 10", node.children().items().get(10).header());
    }

    #[test]
    fn a_node_is_removed_from_wherever_it_is_in_the_tree() {
        let view_model = TreeViewPageViewModel::new();
        let node = view_model.items().items().get(4).children().items().get(5);
        assert!(TreeViewPageViewModel::recursive_remove(&view_model.items(), &node));
        assert_eq!(9, view_model.items().items().get(4).children().items().count());
        assert!(!TreeViewPageViewModel::recursive_remove(&view_model.items(), &node));
        // The children of the other nodes were not created by the search.
        assert!(!view_model.items().items().get(3).are_children_initialized());
    }

    #[test]
    fn select_random_selects_one_node_and_the_mode_clears_the_selection() {
        let view_model = TreeViewPageViewModel::new();
        view_model.select_random_item_command().execute(None);
        assert_eq!(1, view_model.selected_items().count());
        let node = view_model.selected_node(0);
        assert!(node.header().starts_with("Item"));

        view_model.set_selection_mode(SelectionMode::MULTIPLE);
        assert_eq!(0, view_model.selected_items().count());
        assert_eq!(SelectionMode::MULTIPLE, view_model.selection_mode());
    }
}
