//! Port of `Diagnostics/ViewModels/TreePageViewModel.cs`.
//!
//! Upstream shows the nodes of a tree page in a tree view whose items are the nodes and whose
//! selected item is `SelectedNode`. The port shows them in a TreeDataGrid: the page holds a
//! hierarchical source of the nodes ([`TreePageViewModel::nodes_source`]) with one expander
//! column, whose children are the children of a node (a
//! [`TreeNodeCollection`](super::TreeNodeCollection), filled when the row of its node is shown or
//! expanded) and whose expansion is `IsExpanded` of a node; the selection of the rows and
//! `SelectedNode` follow each other (docs/porting/DEVIATIONS.md, Developer tools).

use super::{ControlDetailsViewModel, FilterViewModel, MainViewModel, TreeNode, ViewModelBase};
use ferroui_base::collections::FerroList;
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{ferro_markup_type, Ref, TypeInfo, Visual};
use ferroui_controls::{Control, GridLength};
use ferroui_controls_treedatagrid::models::{HierarchicalExpanderColumn, IViewColumn, TemplateColumn, TemplateColumnOptions};
use ferroui_controls_treedatagrid::TreeDataGridSourceRef;
use ferroui_treedatagrid_core::models::ColumnRef;
use ferroui_treedatagrid_core::selection::TreeSelectionModelSelectionChangedEventArgsOf;
use ferroui_treedatagrid_core::{Accessor, HierarchicalTreeDataGridSource, IndexPath, Models};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::{Rc, Weak};

/// The key of the data template of the cells of the nodes, a resource of the view of a tree page.
pub const TREE_NODE_CELL_TEMPLATE: &str = "TreeNodeCell";

// The list of the root nodes, delivered to the items of the tree view.
ferroui_controls::ferro_markup_list!(pub TreeNodeList: Rc<TreeNode>);

/// The page of a tree of the developer tools (the logical or the visual
/// tree): the root nodes, the selected node and the details of its control,
/// and the commands of the context menu of the tree.
pub struct TreePageViewModel {
    base: ViewModelBase,
    this: Weak<TreePageViewModel>,
    selected_node: RefCell<Option<Rc<TreeNode>>>,
    details: RefCell<Option<Rc<ControlDetailsViewModel>>>,
    pinned_properties: Rc<RefCell<HashSet<String>>>,
    clipboard_copy_requested: Rc<HandlerList<dyn Fn(&str)>>,
    main_view: Weak<MainViewModel>,
    properties_filter: Rc<FilterViewModel>,
    setters_filter: Rc<FilterViewModel>,
    nodes: RefCell<FerroList<Rc<TreeNode>>>,
    filter_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    // Deviation (DEVIATIONS.md, Developer tools): the source of the TreeDataGrid of the page, in
    // place of the items of upstream's tree view, and whether the selection of its rows is being
    // set from `SelectedNode` (or the reverse).
    nodes_source: Rc<HierarchicalTreeDataGridSource<TreeNode>>,
    updating_selection: Cell<bool>,
}

impl PartialEq for TreePageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for TreePageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl TreePageViewModel {
    /// `TreePageViewModel(MainViewModel mainView, TreeNode[] nodes, ISet<string> pinnedProperties)`.
    ///
    /// The main view model owns its pages, so a page refers to it weakly.
    /// `pinned_properties` is the set of the pinned properties that the
    /// pages share.
    pub fn new(
        main_view: &Rc<MainViewModel>,
        nodes: Vec<Rc<TreeNode>>,
        pinned_properties: Rc<RefCell<HashSet<String>>>,
    ) -> Rc<Self> {
        Self::with_weak_main_view(Rc::downgrade(main_view), nodes, pinned_properties)
    }

    /// The constructor with the main view model given by a weak reference,
    /// which may be empty: a page without a main view shows the implemented
    /// interfaces and takes no screenshots.
    pub fn with_weak_main_view(
        main_view: Weak<MainViewModel>,
        nodes: Vec<Rc<TreeNode>>,
        pinned_properties: Rc<RefCell<HashSet<String>>>,
    ) -> Rc<Self> {
        TreeNode::register_model();
        let nodes = FerroList::from_items(nodes);
        let nodes_source = HierarchicalTreeDataGridSource::new(Rc::new(nodes.clone()) as Models<TreeNode>);
        Self::create_nodes_columns(&nodes_source);

        let this = Rc::new_cyclic(|this: &Weak<TreePageViewModel>| TreePageViewModel {
            base: ViewModelBase::new(),
            this: this.clone(),
            selected_node: RefCell::new(None),
            details: RefCell::new(None),
            pinned_properties,
            clipboard_copy_requested: Rc::new(HandlerList::new()),
            main_view,
            properties_filter: FilterViewModel::new(),
            setters_filter: FilterViewModel::new(),
            nodes: RefCell::new(nodes),
            filter_subscriptions: RefCell::new(Vec::new()),
            nodes_source,
            updating_selection: Cell::new(false),
        });

        // Deviation (DEVIATIONS.md, Developer tools): the selected row of the TreeDataGrid is the
        // selected node (the two-way binding of `SelectedItem` of upstream's tree view).
        if let Some(row_selection) = this.nodes_source.row_selection() {
            let weak = Rc::downgrade(&this);
            row_selection.add_selection_changed(Rc::new(
                move |_: &TreeSelectionModelSelectionChangedEventArgsOf<TreeNode>| {
                    if let Some(this) = weak.upgrade() {
                        this.on_row_selection_changed();
                    }
                },
            ));
        }

        let weak = Rc::downgrade(&this);
        let properties = this.properties_filter.refresh_filter(move || {
            let details = weak.upgrade().and_then(|this| this.details());
            if let Some(properties_view) = details.and_then(|details| details.properties_view()) {
                properties_view.refresh();
            }
        });

        let weak = Rc::downgrade(&this);
        let setters = this.setters_filter.refresh_filter(move || {
            if let Some(details) = weak.upgrade().and_then(|this| this.details()) {
                details.update_style_filters();
            }
        });

        this.filter_subscriptions.borrow_mut().extend([properties, setters]);
        this
    }

    /// `ClipboardCopyRequested`: raised with a selector to copy to the
    /// clipboard. Disposing the returned handle removes the handler.
    pub fn clipboard_copy_requested(&self, handler: impl Fn(&str) + 'static) -> Rc<dyn IDisposable> {
        let token = self.clipboard_copy_requested.add(Rc::new(handler));
        let handlers = self.clipboard_copy_requested.clone();
        Disposable::create(move || {
            handlers.remove(token);
        })
    }

    fn raise_clipboard_copy_requested(&self, selector: &str) {
        for (_, handler) in self.clipboard_copy_requested.snapshot().iter() {
            handler(selector);
        }
    }

    /// The main view model; `None` once it is gone.
    pub fn main_view(&self) -> Option<Rc<MainViewModel>> {
        self.main_view.upgrade()
    }

    pub fn properties_filter(&self) -> Rc<FilterViewModel> {
        self.properties_filter.clone()
    }

    pub fn setters_filter(&self) -> Rc<FilterViewModel> {
        self.setters_filter.clone()
    }

    /// The root nodes of the tree.
    pub fn nodes(&self) -> FerroList<Rc<TreeNode>> {
        self.nodes.borrow().clone()
    }

    /// `protected set` of `Nodes`.
    pub fn set_nodes(&self, value: Vec<Rc<TreeNode>>) {
        let nodes = FerroList::from_items(value);
        *self.nodes.borrow_mut() = nodes.clone();

        // Deviation (DEVIATIONS.md, Developer tools): the root rows of the TreeDataGrid, with the
        // selected node selected again when it is still in the tree.
        self.updating_selection.set(true);
        self.nodes_source.set_items(Rc::new(nodes) as Models<TreeNode>);
        let selected = self.selected_node();
        let reselected = self.select_row(selected.as_ref());
        self.updating_selection.set(false);
        if !reselected {
            self.set_selected_node(None);
        }
    }

    /// The column of the TreeDataGrid of the page: the expander of a node, around the cell of the
    /// node (the data template [`TREE_NODE_CELL_TEMPLATE`] of the view: what the item template of
    /// upstream's tree view shows), with the children of the node as the child rows and
    /// `IsExpanded` of the node as the expansion of its row.
    ///
    /// Deviation (DEVIATIONS.md, Developer tools): upstream shows the nodes in a tree view.
    fn create_nodes_columns(source: &HierarchicalTreeDataGridSource<TreeNode>) {
        let options = TemplateColumnOptions::<TreeNode>::new();
        options.set_can_user_resize_column(Some(false));
        options.set_can_user_sort_column(Some(false));
        let cell_column = TemplateColumn::from_resource_keys(
            None,
            ResourceKey::from(TREE_NODE_CELL_TEMPLATE),
            None,
            Some(GridLength::AUTO),
            Some(Rc::new(options)),
        );
        source.columns().add(HierarchicalExpanderColumn::new(
            cell_column as Rc<dyn IViewColumn<TreeNode>>,
            // The children of a node are filled the first time they are asked for: when the row
            // of the node is shown (whether it has children decides its expander) or expanded.
            |node: &Rc<TreeNode>| Some(node.children() as Models<TreeNode>),
            None,
            Some(
                Accessor::member("IsExpanded", |node: &Rc<TreeNode>| node.is_expanded())
                    .with_setter(|node: &Rc<TreeNode>, value: bool| node.set_is_expanded(value)),
            ),
        ) as ColumnRef<TreeNode>);
    }

    /// The source of the TreeDataGrid of the page: the rows of the nodes.
    pub fn nodes_source(&self) -> Rc<HierarchicalTreeDataGridSource<TreeNode>> {
        self.nodes_source.clone()
    }

    /// The source of the TreeDataGrid of the page, as the grid takes it.
    pub fn nodes_source_ref(&self) -> TreeDataGridSourceRef {
        TreeDataGridSourceRef::of::<TreeNode, _>(self.nodes_source.clone())
    }

    /// The index path of the row of a node in the source: the index of each of its ancestors
    /// among the children of its parent, from the root node down to the node; `None` when the
    /// node is not in the trees of the page.
    pub fn index_path_of(&self, node: &Rc<TreeNode>) -> Option<IndexPath> {
        let mut indexes = Vec::new();
        let mut current = node.clone();
        loop {
            match current.parent() {
                Some(parent) => {
                    let index = parent.children().index_of(&current);
                    if index < 0 {
                        return None;
                    }
                    indexes.push(index);
                    current = parent;
                }
                None => {
                    let index = self.nodes().index_of(&current)?;
                    indexes.push(index as i32);
                    break;
                }
            }
        }
        indexes.reverse();
        Some(IndexPath::from_indexes(&indexes))
    }

    /// The index of the row of the selected node among the rows the TreeDataGrid shows; `-1`
    /// when no node is selected or its row is not shown.
    pub fn selected_row_index(&self) -> i32 {
        match self.selected_node().and_then(|node| self.index_path_of(&node)) {
            Some(index) => self.nodes_source.get_row_index(&index, 0),
            None => -1,
        }
    }

    /// Selects the row of a node, expanding the rows of its ancestors; `false` when the node is
    /// not in the trees of the page (the selection is cleared then).
    fn select_row(&self, node: Option<&Rc<TreeNode>>) -> bool {
        let Some(selection) = self.nodes_source.row_selection() else {
            return false;
        };
        match node.and_then(|node| self.index_path_of(node)) {
            Some(index) => {
                self.nodes_source.expand(&index.parent());
                selection.base().set_selected_index(index);
                true
            }
            None => {
                selection.base().clear();
                false
            }
        }
    }

    /// The selection of the rows changed: the selected row is the selected node.
    fn on_row_selection_changed(&self) {
        if self.updating_selection.get() {
            return;
        }
        let selected = self.nodes_source.row_selection().and_then(|selection| selection.base().selected_item());
        self.updating_selection.set(true);
        self.set_selected_node(selected);
        self.updating_selection.set(false);
    }

    pub fn selected_node(&self) -> Option<Rc<TreeNode>> {
        self.selected_node.borrow().clone()
    }

    /// Selects a node, and shows the details of its object.
    pub fn set_selected_node(&self, value: Option<Rc<TreeNode>>) {
        if self.base.raise_and_set_if_changed(&self.selected_node, value.clone(), "SelectedNode") {
            let details = match (value.clone(), self.this.upgrade()) {
                (Some(value), Some(this)) => {
                    Some(ControlDetailsViewModel::new(&this, &value.visual(), self.pinned_properties.clone()))
                }
                _ => None,
            };
            self.set_details(details);
            if let Some(details) = self.details() {
                details.update_properties_view(self.show_implemented_interfaces());
                details.update_style_filters();
            }

            // Deviation (DEVIATIONS.md, Developer tools): the row of the node is selected, and the
            // rows of its ancestors expanded, also when they were not shown (upstream's tree view
            // selects the item of a node only when its container exists).
            if !self.updating_selection.get() {
                self.updating_selection.set(true);
                self.select_row(value.as_ref());
                self.updating_selection.set(false);
            }
        }
    }

    /// The details of the object of the selected node.
    pub fn details(&self) -> Option<Rc<ControlDetailsViewModel>> {
        self.details.borrow().clone()
    }

    fn set_details(&self, value: Option<Rc<ControlDetailsViewModel>>) {
        let old_value = self.details();

        // `RaiseAndSetIfChanged`: the details are a reference type, equal only to themselves.
        let changed = match (&old_value, &value) {
            (Some(old), Some(new)) => !Rc::ptr_eq(old, new),
            (None, None) => false,
            _ => true,
        };
        if changed {
            *self.details.borrow_mut() = value;
            self.base.raise_property_changed("Details");
            if let Some(old_value) = old_value {
                old_value.dispose();
            }
        }
    }

    /// `Dispose()`: disposes the nodes and the details.
    pub fn dispose(&self) {
        for node in self.nodes().snapshot().iter() {
            node.dispose();
        }

        if let Some(details) = self.details() {
            details.dispose();
        }

        // Deviation (DEVIATIONS.md, Developer tools): the rows of the TreeDataGrid end their
        // subscriptions to the nodes.
        self.nodes_source.dispose();
    }

    /// The node of `control`, searched in the trees of the root nodes.
    pub fn find_node(&self, control: &Ref<Control>) -> Option<Rc<TreeNode>> {
        for node in self.nodes().snapshot().iter() {
            let result = self.find_node_in(node, control);

            if result.is_some() {
                return result;
            }
        }

        None
    }

    /// Selects the node of `control`, or of its nearest visual ancestor that
    /// has one, and expands its ancestors.
    pub fn select_control(&self, control: &Ref<Control>) {
        let mut node: Option<Rc<TreeNode>> = None;
        let mut c: Option<Ref<Control>> = Some(control.clone());

        while node.is_none() {
            let Some(current) = c.as_ref() else { break };
            node = self.find_node(current);

            if node.is_none() {
                c = current.get_visual_parent_of_type::<Control>();
            }
        }

        if let Some(node) = node {
            self.set_selected_node(Some(node.clone()));
            self.expand_node(node.parent());
        }
    }

    /// Requests the copy of the selector of the selected control.
    pub fn copy_selector(&self) {
        let current_visual = self.selected_node().and_then(|node| node.visual().cast::<Visual>());
        if let Some(current_visual) = current_visual {
            let selector = Self::get_visual_selector(&current_visual);

            self.raise_clipboard_copy_requested(&selector);
        }
    }

    /// Requests the copy of the selector of the selected control from the
    /// outermost templated parent, through the templates.
    pub fn copy_selector_from_template_parent(&self) {
        let mut parts = Vec::new();

        let mut current_visual = self.selected_node().and_then(|node| node.visual().cast::<Visual>());
        while let Some(visual) = current_visual {
            parts.push(Self::get_visual_selector(&visual));

            current_visual = visual.templated_parent().and_then(|parent| parent.cast::<Visual>());
        }

        if !parts.is_empty() {
            parts.reverse();
            let selector = parts.join(" /template/ ");

            self.raise_clipboard_copy_requested(&selector);
        }
    }

    /// Expands the selected node, its ancestors and all its descendants.
    pub fn expand_recursively(&self) {
        if let Some(selected_node) = self.selected_node() {
            self.expand_node(Some(selected_node.clone()));

            let mut stack = vec![selected_node];

            while let Some(item) = stack.pop() {
                item.set_is_expanded(true);
                for child in item.children().to_vec() {
                    stack.push(child);
                }
            }
        }
    }

    /// Collapses the selected node and all its descendants.
    pub fn collapse_children(&self) {
        if let Some(selected_node) = self.selected_node() {
            let mut stack = vec![selected_node];

            while let Some(item) = stack.pop() {
                item.set_is_expanded(false);
                for child in item.children().to_vec() {
                    stack.push(child);
                }
            }
        }
    }

    /// Takes a screenshot of the selected control.
    pub fn capture_node_screenshot(&self) {
        if let Some(main_view) = self.main_view() {
            main_view.shot(None);
        }
    }

    /// Brings the selected control into view.
    pub fn bring_into_view(&self) {
        if let Some(control) = self.selected_node().and_then(|node| node.visual().cast::<Control>()) {
            control.bring_into_view();
        }
    }

    /// Focuses the selected control.
    pub fn focus(&self) {
        if let Some(control) = self.selected_node().and_then(|node| node.visual().cast::<Control>()) {
            let _ = control.focus();
        }
    }

    /// The selector of a visual: `{assembly}namespace|Type`, then the name
    /// (`#name`), the classes (`.class`) and the pseudo-classes.
    fn get_visual_selector(visual: &Ref<Visual>) -> String {
        let name = visual.name().filter(|name| !name.is_empty()).map(|name| format!("#{name}")).unwrap_or_default();
        let classes = visual.classes().snapshot();
        let pseudo: String = classes.iter().filter(|c| c.starts_with(':')).map(String::as_str).collect();
        let classes: String = classes.iter().filter(|c| !c.starts_with(':')).map(|c| format!(".{c}")).collect();
        let type_ = visual.style_key();
        format!(
            "{{{}}}{}|{}{name}{classes}{pseudo}",
            Self::assembly_full_name(type_),
            type_.namespace(),
            type_.name()
        )
    }

    /// `type.Assembly.FullName`.
    ///
    /// Deviation (DEVIATIONS.md, Developer tools): the port has no assembly identity (version,
    /// culture and public key token); the name of the assembly that declares the type stands for
    /// it, or the name of its crate when the assembly is not registered.
    fn assembly_full_name(type_: &'static TypeInfo) -> String {
        let module_path = type_.module_path();
        match MarkupAssembly::of_module(module_path) {
            Some(assembly) => assembly.name.to_string(),
            None => module_path.split("::").next().unwrap_or_default().to_string(),
        }
    }

    fn expand_node(&self, node: Option<Rc<TreeNode>>) {
        if let Some(node) = node {
            node.set_is_expanded(true);
            self.expand_node(node.parent());
        }
    }

    fn find_node_in(&self, node: &Rc<TreeNode>, control: &Ref<Control>) -> Option<Rc<TreeNode>> {
        if node.visual().ptr_eq(control) {
            return Some(node.clone());
        } else {
            for child in node.children().to_vec() {
                let result = self.find_node_in(&child, control);

                if result.is_some() {
                    return result;
                }
            }
        }

        None
    }

    /// Updates the properties of the details with the current setting of
    /// the main view (shown with the implemented interfaces when there is
    /// no main view).
    pub fn update_properties_view(&self) {
        if let Some(details) = self.details() {
            details.update_properties_view(self.show_implemented_interfaces());
        }
    }

    /// `MainView?.ShowImplementedInterfaces ?? true`.
    fn show_implemented_interfaces(&self) -> bool {
        self.main_view().is_none_or(|main_view| main_view.show_implemented_interfaces())
    }
}

ferro_markup_type!(class TreePageViewModel {
    this: Rc<TreePageViewModel>,
    handles: [TreePageViewModel, Rc<TreePageViewModel>, Option<Rc<TreePageViewModel>>],
    properties: [
        MainView: Option<Rc<MainViewModel>> { get: TreePageViewModel::main_view },
        PropertiesFilter: Rc<FilterViewModel> { get: TreePageViewModel::properties_filter },
        SettersFilter: Rc<FilterViewModel> { get: TreePageViewModel::setters_filter },
        Nodes: FerroList<Rc<TreeNode>> { get: TreePageViewModel::nodes },
        // Deviation (DEVIATIONS.md, Developer tools): the source of the TreeDataGrid of the page.
        NodesSource: TreeDataGridSourceRef { get: TreePageViewModel::nodes_source_ref },
        SelectedNode: Option<Rc<TreeNode>> {
            get: TreePageViewModel::selected_node,
            set: TreePageViewModel::set_selected_node
        },
        Details: Option<Rc<ControlDetailsViewModel>> { get: TreePageViewModel::details },
    ],
    methods: [
        fn CopySelector() => TreePageViewModel::copy_selector,
        fn CopySelectorFromTemplateParent() => TreePageViewModel::copy_selector_from_template_parent,
        fn ExpandRecursively() => TreePageViewModel::expand_recursively,
        fn CollapseChildren() => TreePageViewModel::collapse_children,
        fn CaptureNodeScreenshot() => TreePageViewModel::capture_node_screenshot,
        fn BringIntoView() => TreePageViewModel::bring_into_view,
        fn Focus() => TreePageViewModel::focus,
    ],
    notify_property_changed: TreePageViewModel,
});

/// What the untyped value conversions must know about the view models of
/// the trees: they are reference types, and the children of a node are
/// delivered to the items of a tree view. Registered once with
/// `ValueTypes::register_global` from the `register_types()` of the crate.
pub fn register_tree_value_types() {
    use ferroui_base::data::core::ValueTypes;
    use ferroui_controls::ItemsSource;

    ValueTypes::register_reference::<TreeNode>();
    ValueTypes::register_reference::<super::TreeNodeCollection>();
    ValueTypes::register_reference::<TreePageViewModel>();
    ValueTypes::register_reference::<FilterViewModel>();
    // The children are delivered as the shared handle, or as the object itself (the untyped
    // form of a reference type).
    ItemsSource::register_binding_conversion_with::<Rc<super::TreeNodeCollection>>(|children| {
        ItemsSource::new(children.clone())
    });
    ItemsSource::register_binding_conversion_with::<super::TreeNodeCollection>(|children| {
        ItemsSource::new(children.to_rc())
    });
}
