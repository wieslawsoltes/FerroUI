//! Port of `Diagnostics/ViewModels/EventsPageViewModel.cs`.
//!
//! Upstream shows the tree of the routed events in a tree view whose items are the nodes, hidden
//! by the filter through their `IsVisible`, and whose selected item is `SelectedNode`. The port
//! shows them in a TreeDataGrid: the page holds a hierarchical source of the visible nodes
//! ([`EventsPageViewModel::nodes_source`]) with one expander column, whose children are the
//! visible children of a node and whose expansion is `IsExpanded` of a node; the selection of the
//! rows and `SelectedNode` follow each other (docs/porting/DEVIATIONS.md, Developer tools).

use super::event_tree_node_base::sync_visible_nodes;
use super::{EventOwnerTreeNode, EventTreeNodeBase, FilterViewModel, FiredEvent, MainViewModel, ViewModelBase};
use crate::diagnostics::models::EventChainLink;
use ferroui_base::collections::FerroList;
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventRegistry};
use ferroui_base::TypeInfo;
use ferroui_controls::{Button, Control, GridLength, GridUnitType};
use ferroui_controls_treedatagrid::models::{HierarchicalExpanderColumn, IViewColumn, TemplateColumn, TemplateColumnOptions};
use ferroui_controls_treedatagrid::TreeDataGridSourceRef;
use ferroui_treedatagrid_core::models::ColumnRef;
use ferroui_treedatagrid_core::selection::TreeSelectionModelSelectionChangedEventArgsOf;
use ferroui_treedatagrid_core::{Accessor, HierarchicalTreeDataGridSource, IndexPath, Models};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::{Rc, Weak};

/// The key of the data template of the cells of the nodes of the tree of the events, a resource
/// of the view of the events page.
pub const EVENT_TREE_NODE_CELL_TEMPLATE: &str = "EventTreeNodeCell";

/// The events recorded when the page is created, and again with "Enable
/// default".
fn default_events() -> HashSet<RoutedEvent> {
    HashSet::from([
        Button::click_event().as_routed_event(),
        InputElement::key_down_event().as_routed_event(),
        InputElement::key_up_event().as_routed_event(),
        InputElement::text_input_event().as_routed_event(),
        InputElement::pointer_released_event().as_routed_event(),
        InputElement::pointer_pressed_event().as_routed_event(),
    ])
}

/// The view model of the events page: the tree of the registered routed
/// events grouped by the type that owns them, whose check boxes choose the
/// events to record, and the recorded events.
pub struct EventsPageViewModel {
    base: ViewModelBase,
    main_view_model: Weak<MainViewModel>,
    selected_event: RefCell<Option<Rc<FiredEvent>>>,
    selected_node: RefCell<Option<Rc<EventTreeNodeBase>>>,
    nodes: FerroList<Rc<EventTreeNodeBase>>,
    recorded_events: FerroList<Rc<FiredEvent>>,
    events_filter: Rc<FilterViewModel>,
    // Deviation (DEVIATIONS.md, Developer tools): the visible nodes (the root rows), the source of
    // the TreeDataGrid of the page, in place of the items of upstream's tree view, and whether the
    // selection of its rows is being set from `SelectedNode` (or the reverse).
    visible_nodes: Rc<FerroList<Rc<EventTreeNodeBase>>>,
    nodes_source: Rc<HierarchicalTreeDataGridSource<EventTreeNodeBase>>,
    updating_selection: Cell<bool>,
}

/// Reference equality, as the class of the original.
impl PartialEq for EventsPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for EventsPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl EventsPageViewModel {
    /// C# `new EventsPageViewModel(MainViewModel mainViewModel)`: the tree of
    /// the routed events registered on this thread, with the default events
    /// enabled.
    ///
    /// The main view model is held weakly: it holds the page.
    pub fn new(main_view_model: Weak<MainViewModel>) -> Rc<Self> {
        // Deviation (DEVIATIONS.md, Developer tools): the static `DefaultEvents` of the original
        // is initialized before the constructor runs, which registers its events; here they
        // register on first access, so the set is made before the registered events are read.
        let _ = default_events();
        EventTreeNodeBase::register_model();
        let this = Rc::new_cyclic(|this: &Weak<EventsPageViewModel>| {
            // `GroupBy(e => e.OwnerType)`: the groups in the order of their first event.
            let mut groups: Vec<(&'static TypeInfo, Vec<RoutedEvent>)> = Vec::new();
            for e in RoutedEventRegistry::instance().get_all_registered() {
                let owner_type = e.owner_type();
                match groups.iter_mut().find(|(key, _)| std::ptr::eq(*key, owner_type)) {
                    Some((_, events)) => events.push(e),
                    None => groups.push((owner_type, vec![e])),
                }
            }
            // `OrderBy(e => e.Key.Name)` is a stable sort.
            groups.sort_by(|(x, _), (y, _)| x.name().cmp(y.name()));

            let nodes =
                FerroList::from_items(groups.into_iter().map(|(key, events)| EventOwnerTreeNode::new(key, events, this)));
            let visible_nodes = Rc::new(FerroList::from_items(nodes.to_vec()));
            let nodes_source =
                HierarchicalTreeDataGridSource::new(visible_nodes.clone() as Models<EventTreeNodeBase>);
            Self::create_nodes_columns(&nodes_source);

            Self {
                base: ViewModelBase::new(),
                main_view_model,
                selected_event: RefCell::new(None),
                selected_node: RefCell::new(None),
                nodes,
                recorded_events: FerroList::new(),
                events_filter: FilterViewModel::new(),
                visible_nodes,
                nodes_source,
                updating_selection: Cell::new(false),
            }
        });

        // Deviation (DEVIATIONS.md, Developer tools): the selected row of the TreeDataGrid is the
        // selected node (the two-way binding of `SelectedItem` of upstream's tree view).
        if let Some(row_selection) = this.nodes_source.row_selection() {
            let weak = Rc::downgrade(&this);
            row_selection.add_selection_changed(Rc::new(
                move |_: &TreeSelectionModelSelectionChangedEventArgsOf<EventTreeNodeBase>| {
                    if let Some(this) = weak.upgrade() {
                        this.on_row_selection_changed();
                    }
                },
            ));
        }

        let weak = Rc::downgrade(&this);
        // As upstream, the handler is never removed: the filter belongs to the page.
        let _ = this.events_filter.refresh_filter(move || {
            if let Some(this) = weak.upgrade() {
                this.update_event_filters();
            }
        });

        this.enable_default();
        this
    }

    /// The title of the page.
    pub fn name(&self) -> String {
        "Events".to_string()
    }

    /// The nodes of the types that own routed events, ordered by the names
    /// of the types.
    pub fn nodes(&self) -> FerroList<Rc<EventTreeNodeBase>> {
        self.nodes.clone()
    }

    /// The recorded events, the oldest first.
    pub fn recorded_events(&self) -> FerroList<Rc<FiredEvent>> {
        self.recorded_events.clone()
    }

    pub fn selected_event(&self) -> Option<Rc<FiredEvent>> {
        self.selected_event.borrow().clone()
    }

    pub fn set_selected_event(&self, value: Option<Rc<FiredEvent>>) {
        self.base.raise_and_set_if_changed(&self.selected_event, value, "SelectedEvent");
    }

    pub fn selected_node(&self) -> Option<Rc<EventTreeNodeBase>> {
        self.selected_node.borrow().clone()
    }

    pub fn set_selected_node(&self, value: Option<Rc<EventTreeNodeBase>>) {
        if self.base.raise_and_set_if_changed(&self.selected_node, value.clone(), "SelectedNode") {
            // Deviation (DEVIATIONS.md, Developer tools): the row of the node is selected, and the
            // row of its owner expanded.
            if !self.updating_selection.get() {
                self.updating_selection.set(true);
                self.select_row(value.as_ref());
                self.updating_selection.set(false);
            }
        }
    }

    /// The column of the TreeDataGrid of the page: the expander of a node, around the cell of the
    /// node (the data template [`EVENT_TREE_NODE_CELL_TEMPLATE`] of the view: the check box of
    /// `IsEnabled` with the name of the node, what the item template of upstream's tree view
    /// shows), with the visible children of the node as the child rows and `IsExpanded` of the
    /// node as the expansion of its row.
    ///
    /// Deviation (DEVIATIONS.md, Developer tools): upstream shows the nodes in a tree view.
    fn create_nodes_columns(source: &HierarchicalTreeDataGridSource<EventTreeNodeBase>) {
        let options = TemplateColumnOptions::<EventTreeNodeBase>::new();
        options.set_can_user_resize_column(Some(false));
        options.set_can_user_sort_column(Some(false));
        let cell_column = TemplateColumn::from_resource_keys(
            None,
            ResourceKey::from(EVENT_TREE_NODE_CELL_TEMPLATE),
            None,
            Some(GridLength::new(1.0, GridUnitType::Star)),
            Some(Rc::new(options)),
        );
        source.columns().add(HierarchicalExpanderColumn::new(
            cell_column as Rc<dyn IViewColumn<EventTreeNodeBase>>,
            |node: &Rc<EventTreeNodeBase>| {
                node.visible_children().map(|children| children as Models<EventTreeNodeBase>)
            },
            None,
            Some(
                Accessor::member("IsExpanded", |node: &Rc<EventTreeNodeBase>| node.is_expanded())
                    .with_setter(|node: &Rc<EventTreeNodeBase>, value: bool| node.set_is_expanded(value)),
            ),
        ) as ColumnRef<EventTreeNodeBase>);
    }

    /// The source of the TreeDataGrid of the page: the rows of the visible nodes.
    pub fn nodes_source(&self) -> Rc<HierarchicalTreeDataGridSource<EventTreeNodeBase>> {
        self.nodes_source.clone()
    }

    /// The source of the TreeDataGrid of the page, as the grid takes it.
    pub fn nodes_source_ref(&self) -> TreeDataGridSourceRef {
        TreeDataGridSourceRef::of::<EventTreeNodeBase, _>(self.nodes_source.clone())
    }

    /// The visible nodes of the owners of events: the root rows of the TreeDataGrid of the page.
    pub fn visible_nodes(&self) -> Rc<FerroList<Rc<EventTreeNodeBase>>> {
        self.visible_nodes.clone()
    }

    /// The index path of the row of a node in the source; `None` when the node is not visible.
    pub fn index_path_of(&self, node: &Rc<EventTreeNodeBase>) -> Option<IndexPath> {
        match node.parent() {
            Some(parent) => {
                let parent_index = self.visible_nodes.index_of(&parent)?;
                let index = parent.visible_children()?.index_of(node)?;
                Some(IndexPath::new(parent_index as i32).append(index as i32))
            }
            None => self.visible_nodes.index_of(node).map(|index| IndexPath::new(index as i32)),
        }
    }

    /// The index of the row of the selected node among the rows the TreeDataGrid shows; `-1`
    /// when no node is selected or its row is not shown.
    pub fn selected_row_index(&self) -> i32 {
        match self.selected_node().and_then(|node| self.index_path_of(&node)) {
            Some(index) => self.nodes_source.get_row_index(&index, 0),
            None => -1,
        }
    }

    /// Selects the row of a node, expanding the row of its owner; `false` when the node is not
    /// visible (the selection is cleared then).
    fn select_row(&self, node: Option<&Rc<EventTreeNodeBase>>) -> bool {
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

    /// The filter of the tree of the events.
    pub fn events_filter(&self) -> Rc<FilterViewModel> {
        self.events_filter.clone()
    }

    /// Removes the recorded events.
    pub fn clear(&self) {
        self.recorded_events.clear();
    }

    /// Stops recording every event.
    pub fn disable_all(&self) {
        self.evaluate_node_enabled(&|_: &EventTreeNodeBase| false);
    }

    /// Records the default events and no other.
    pub fn enable_default(&self) {
        let default_events = default_events();
        self.evaluate_node_enabled(&|node: &EventTreeNodeBase| {
            let event = node.as_event_tree_node().expect("a node of an event").event();
            default_events.contains(&event)
        });
    }

    /// Shows the element of a link of the chain of an event in the visual
    /// tree page.
    pub fn request_tree_navigate_to(&self, nav_target: &EventChainLink) {
        if let Some(control) = ValueTypes::as_object(&*nav_target.handler()).and_then(|handler| handler.cast::<Control>())
        {
            if let Some(main_view_model) = self.main_view() {
                main_view_model.request_tree_navigate_to(&control, true);
            }
        }
    }

    /// Selects the node of `evt` in the tree, if it is visible.
    pub fn select_event_by_type(&self, evt: RoutedEvent) {
        fn find_node(node: &Rc<EventTreeNodeBase>, event_type: RoutedEvent) -> Option<Rc<EventTreeNodeBase>> {
            if node.as_event_tree_node().is_some_and(|event_node| event_node.event() == event_type) {
                return Some(node.clone());
            }

            if let Some(children) = node.children() {
                for child in children.to_vec() {
                    let result = find_node(&child, event_type);

                    if result.is_some() {
                        return result;
                    }
                }
            }

            None
        }

        for node in self.nodes.to_vec() {
            let result = find_node(&node, evt);

            if let Some(result) = result.filter(|result| result.is_visible()) {
                self.set_selected_node(Some(result));

                break;
            }
        }
    }

    /// Sets `IsEnabled` of every node of an event to `eval` of the node.
    fn evaluate_node_enabled(&self, eval: &dyn Fn(&EventTreeNodeBase) -> bool) {
        fn process_node(node: &Rc<EventTreeNodeBase>, eval: &dyn Fn(&EventTreeNodeBase) -> bool) {
            if node.as_event_tree_node().is_some() {
                node.set_is_enabled(Some(eval(&**node)));
            }

            if let Some(children) = node.children() {
                for child_node in children.to_vec() {
                    process_node(&child_node, eval);
                }
            }
        }

        for node in self.nodes.to_vec() {
            process_node(&node, eval);
        }
    }

    /// Shows the nodes whose text matches the filter, their ancestors and
    /// their descendants.
    fn update_event_filters(&self) {
        fn filter_node(filter: &FilterViewModel, node: &Rc<EventTreeNodeBase>, is_parent_visible: bool) -> bool {
            let matches_filter = filter.filter(node.text());
            let mut has_visible_child = false;

            if let Some(children) = node.children() {
                for child_node in children.to_vec() {
                    has_visible_child |= filter_node(filter, &child_node, matches_filter);
                }
            }

            node.set_is_visible(has_visible_child || matches_filter || is_parent_visible);

            node.is_visible()
        }

        for node in self.nodes.to_vec() {
            filter_node(&self.events_filter, &node, false);
        }

        // Deviation (DEVIATIONS.md, Developer tools): the rows of the TreeDataGrid are the visible
        // nodes. As in upstream's tree view, hiding the selected node does not change
        // `SelectedNode`; its row is selected again when it is shown.
        self.updating_selection.set(true);
        for node in self.nodes.to_vec() {
            node.update_visible_children();
        }
        sync_visible_nodes(&self.nodes.to_vec(), &self.visible_nodes);
        let selected = self.selected_node();
        self.select_row(selected.as_ref());
        self.updating_selection.set(false);
    }

    /// The main view model the page belongs to.
    pub fn main_view(&self) -> Option<Rc<MainViewModel>> {
        self.main_view_model.upgrade()
    }
}

ferro_markup_type!(class EventsPageViewModel {
    this: Rc<EventsPageViewModel>,
    handles: [EventsPageViewModel, Rc<EventsPageViewModel>, Option<Rc<EventsPageViewModel>>],
    base: ViewModelBase,
    properties: [
        Name: String { get: EventsPageViewModel::name },
        Nodes: FerroList<Rc<EventTreeNodeBase>> { get: EventsPageViewModel::nodes },
        RecordedEvents: FerroList<Rc<FiredEvent>> { get: EventsPageViewModel::recorded_events },
        SelectedEvent: Option<Rc<FiredEvent>> {
            get: EventsPageViewModel::selected_event,
            set: EventsPageViewModel::set_selected_event
        },
        SelectedNode: Option<Rc<EventTreeNodeBase>> {
            get: EventsPageViewModel::selected_node,
            set: EventsPageViewModel::set_selected_node
        },
        EventsFilter: Rc<FilterViewModel> { get: EventsPageViewModel::events_filter },
        // Deviation (DEVIATIONS.md, Developer tools): the source of the TreeDataGrid of the page.
        NodesSource: TreeDataGridSourceRef { get: EventsPageViewModel::nodes_source_ref },
        MainView: Option<Rc<MainViewModel>> { get: EventsPageViewModel::main_view },
    ],
    methods: [
        fn Clear() => EventsPageViewModel::clear,
        fn DisableAll() => EventsPageViewModel::disable_all,
        fn EnableDefault() => EventsPageViewModel::enable_default,
        fn RequestTreeNavigateTo(Rc<EventChainLink>) =>
            (|this: &Rc<EventsPageViewModel>, nav_target: Rc<EventChainLink>| this.request_tree_navigate_to(&nav_target)),
        fn SelectEventByType(RoutedEvent) => EventsPageViewModel::select_event_by_type,
    ],
    notify_property_changed: EventsPageViewModel,
});
