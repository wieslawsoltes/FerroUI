use crate::generators::RecycleKey;
use crate::items_source::{items_equal, ItemsChangedEventArgs, ItemsView};
use crate::primitives::{
    ItemSelectionEventTriggers, SelectedItemsList, SelectingItemsControl, TemplatedControlImpl,
};
use crate::{
    Control, ControlImpl, ItemsControl, ItemsControlImpl, ItemsControlImplExt, SelectionChangedEventArgs, SelectionMode, TreeViewItem,
};
use ferroui_base::collections::{NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs};
use ferroui_base::input::{
    FocusChangedEventArgs, ICustomKeyboardNavigation, InputElement, InputElementImpl, KeyEventArgs, KeyModifiers,
    NavigationDirection, NavigationMethod, PointerEventArgs,
};
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, DirectProperty, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Displays a hierarchical tree of data.
#[repr(C)]
pub struct TreeView {
    base: ItemsControl,
    selected_item: RefCell<Option<BoxedValue>>,
    selected_items: RefCell<Option<SelectedItemsList>>,
    selected_items_subscription: Cell<Option<u64>>,
    syncing_selected_items: Cell<bool>,
}

ferro_class! {
    TreeView: ItemsControl, virtuals TreeViewImpl: ItemsControlImpl {
        /// Analyses a pointer event received by a selectable element, and
        /// determines whether the action should trigger selection on press,
        /// on release, or not at all.
        fn should_trigger_selection(this, selectable: &Visual, event_args: &PointerEventArgs) -> bool;
        /// Analyses a key event received by a selectable element, and
        /// determines whether the action should trigger selection.
        fn should_trigger_selection_key(this, selectable: &Visual, event_args: &KeyEventArgs) -> bool;
        /// Updates the selection based on an event that may have originated
        /// in a container that belongs to the control.
        ///
        /// Returns true if the event originated from a container that
        /// belongs to the control and triggered selection; otherwise false.
        fn update_selection_from_event(this, container: &Ref<Control>, event_args: &dyn IRoutedEventArgs) -> bool;
    }
}
ferroui_base::ferro_class_info!(TreeView { new: TreeView::new });

ferro_impl_classes!(
    TreeView: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    TemplatedControlImpl
);

impl ControlImpl for TreeView {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::TreeViewAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for TreeView {}

impl InputElementImpl for TreeView {
    fn as_custom_keyboard_navigation(this: &Self) -> Option<&dyn ICustomKeyboardNavigation> {
        Some(this)
    }

    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        if e.navigation_method == NavigationMethod::Directional {
            e.set_handled(this.update_selection_from_event_source(
                e.source(),
                true,
                e.key_modifiers.contains(KeyModifiers::SHIFT),
                false,
                false,
            ));
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        let direction = e.key.to_navigation_direction(KeyModifiers::NONE);

        if let (Some(direction), false) = (direction.filter(|direction| direction.is_directional()), e.handled()) {
            if this.selected_item().is_some() {
                let from = this.get_container_from_event_source(e.source());
                let next = this.get_container_in_direction(from, direction, true);

                if let Some(next) = next {
                    e.set_handled(next.focus_with(NavigationMethod::Directional, KeyModifiers::NONE));
                }
            } else {
                this.set_selected_item(this.items_view().get_at(0));
            }
        }

        if !e.handled() {
            let keymap = this.get_platform_settings().map(|settings| settings.hotkey_configuration());
            let select_all = keymap.is_some_and(|keymap| keymap.select_all.iter().any(|g| g.matches(Some(e))));

            if this.selection_mode() == SelectionMode::MULTIPLE && select_all {
                this.select_all();
                e.set_handled(true);
            }
        }
    }
}

impl ItemsControlImpl for TreeView {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        TreeViewItem::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<TreeViewItem>(item)
    }

    fn container_for_item_prepared_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        Self::parent_container_for_item_prepared_override(this, container, item, index);

        // Once the container has been full prepared and added to the tree,
        // any bindings from styles or item container themes are guaranteed
        // to be applied.
        if container.is_set(SelectingItemsControl::is_selected_property().as_property()) {
            // The is selected property is set on the container: there is a
            // style or item container theme which has bound the property.
            // Update our selection based on the selection state of the
            // container.
            let container_is_selected = SelectingItemsControl::get_is_selected(container);
            this.update_selection_from_container(container, container_is_selected, false, true, false);
        }

        // The is selected property is not set on the container: update the
        // container selection based on the current selection as understood
        // by this control.
        this.mark_container_selected(container, this.selected_items().contains(item));

        // If the newly realized container is the selected container, scroll
        // to it after layout.
        if this.auto_scroll_to_selected_item() && items_equal(&this.selected_item(), item) {
            container.bring_into_view();
        }
    }

    fn on_items_view_collection_changed(this: &Self, e: &ItemsChangedEventArgs<'_>) {
        Self::parent_on_items_view_collection_changed(this, e);

        match e.action {
            NotifyCollectionChangedAction::Remove | NotifyCollectionChangedAction::Replace => {
                for i in e.old_items {
                    this.selected_items().remove(&i);
                }
            }
            NotifyCollectionChangedAction::Reset => this.selected_items().clear(),
            _ => {}
        }
    }
}

impl TreeViewImpl for TreeView {
    fn should_trigger_selection(_this: &Self, selectable: &Visual, event_args: &PointerEventArgs) -> bool {
        ItemSelectionEventTriggers::should_trigger_selection(selectable, event_args)
    }

    fn should_trigger_selection_key(_this: &Self, selectable: &Visual, event_args: &KeyEventArgs) -> bool {
        ItemSelectionEventTriggers::should_trigger_selection_key(selectable, event_args)
    }

    fn update_selection_from_event(this: &Self, container: &Ref<Control>, event_args: &dyn IRoutedEventArgs) -> bool {
        let args = event_args.as_routed_event_args();

        if args.handled() {
            return false;
        }

        let pointer_event = event_args.downcast_ref::<PointerEventArgs>();
        let key_event = event_args.downcast_ref::<KeyEventArgs>();

        let triggers = match (pointer_event, key_event) {
            (Some(pointer_event), _) => this.should_trigger_selection(container, pointer_event),
            (None, Some(key_event)) => this.should_trigger_selection_key(container, key_event),
            (None, None) => false,
        };

        if !triggers {
            return false;
        }

        this.update_selection_from_container(
            container,
            true,
            ItemSelectionEventTriggers::has_range_selection_modifier(container, event_args),
            ItemSelectionEventTriggers::has_toggle_selection_modifier(container, event_args),
            pointer_event.is_some_and(|e| e.properties().is_right_button_pressed),
        );

        if pointer_event.is_some() {
            crate::platform::PlatformFeedbackExtensions::perform_feedback(
                &**container as &InputElement,
                crate::platform::FeedbackAction::click(),
            );
        }

        args.set_handled(true);
        true
    }
}

impl ICustomKeyboardNavigation for TreeView {
    fn get_next(&self, element: &Ref<InputElement>, direction: NavigationDirection) -> (bool, Option<Ref<InputElement>>) {
        if direction == NavigationDirection::Next || direction == NavigationDirection::Previous {
            if !self.is_visual_ancestor_of(element) {
                let selected_item = self.selected_item.borrow().clone();
                let result = match selected_item {
                    Some(_) => self.tree_container_from_item(&selected_item),
                    None => self.container_from_index(0),
                };

                // The selected item may not be in the tree view.
                return (result.is_some(), result.map(Ref::upcast));
            }

            return (true, None);
        }

        (false, None)
    }
}

ferroui_base::ferro_properties! { impl TreeView {
    ferro_property!(
        /// Defines the `AutoScrollToSelectedItem` property.
        pub fn auto_scroll_to_selected_item_property() -> StyledProperty<bool> {
            SelectingItemsControl::auto_scroll_to_selected_item_property().add_owner::<TreeView>()
        }
    );

    ferro_property!(
        /// Defines the `SelectedItem` property.
        pub fn selected_item_property() -> DirectProperty<TreeView, Option<BoxedValue>> {
            SelectingItemsControl::selected_item_property().add_owner::<TreeView>(
                |o| o.selected_item(),
                Some(|o, v| o.set_selected_item(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `SelectedItems` property.
        pub fn selected_items_property() -> DirectProperty<TreeView, Option<SelectedItemsList>> {
            FerroProperty::register_direct::<TreeView, _>(
                "SelectedItems",
                |o| Some(o.selected_items()),
                Some(|o, v| o.set_selected_items(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `SelectionMode` property.
        pub fn selection_mode_property() -> StyledProperty<SelectionMode> {
            SelectingItemsControl::selection_mode_property().add_owner::<TreeView>()
        }
    );
} }

impl TreeView {
    /// Initializes static members of the [`TreeView`] class.
    fn static_constructor() {
        SelectingItemsControl::is_selected_changed_event()
            .add_class_handler::<TreeView>(|x, e| x.container_selection_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ItemsControl::construct(),
            selected_item: RefCell::new(None),
            selected_items: RefCell::new(None),
            selected_items_subscription: Cell::new(None),
            syncing_selected_items: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Occurs when the control's selection changes.
    pub fn selection_changed(
        &self,
        handler: impl Fn(&Interactive, &SelectionChangedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(SelectingItemsControl::selection_changed_event(), handler)
    }

    /// Gets or sets a value indicating whether to automatically scroll to
    /// newly selected items.
    ///
    /// This property is of limited use with [`TreeView`] as it will only
    /// scroll to realized items. To scroll to a non-expanded item, you need
    /// to ensure that its ancestors are expanded.
    pub fn auto_scroll_to_selected_item(&self) -> bool {
        self.get_value(Self::auto_scroll_to_selected_item_property())
    }

    pub fn set_auto_scroll_to_selected_item(&self, value: bool) {
        self.set_value(Self::auto_scroll_to_selected_item_property(), value)
    }

    /// Gets or sets the selection mode.
    pub fn selection_mode(&self) -> SelectionMode {
        self.get_value(Self::selection_mode_property())
    }

    pub fn set_selection_mode(&self, value: SelectionMode) {
        self.set_value(Self::selection_mode_property(), value)
    }

    /// Gets or sets the selected item.
    ///
    /// Note that setting this property only currently works if the item is
    /// expanded to be visible.
    pub fn selected_item(&self) -> Option<BoxedValue> {
        self.selected_item.borrow().clone()
    }

    pub fn set_selected_item(&self, value: Option<BoxedValue>) {
        let selected_items = self.selected_items();

        self.set_and_raise(Self::selected_item_property(), &self.selected_item, value.clone());

        if value.is_some() {
            if selected_items.count() != 1 || !items_equal(&selected_items.get(0), &value) {
                self.select_single_item(value);
            }
        } else if self.selected_items().count() > 0 {
            self.selected_items().clear();
        }
    }

    /// Gets or sets the selected items.
    pub fn selected_items(&self) -> SelectedItemsList {
        let existing = self.selected_items.borrow().clone();
        match existing {
            Some(selected_items) => selected_items,
            None => {
                let selected_items = SelectedItemsList::new();
                *self.selected_items.borrow_mut() = Some(selected_items.clone());
                self.subscribe_to_selected_items();
                selected_items
            }
        }
    }

    pub fn set_selected_items(&self, value: Option<SelectedItemsList>) {
        // The reference rejects fixed size and read-only collections here;
        // the list type of the property is neither.
        self.unsubscribe_from_selected_items();
        *self.selected_items.borrow_mut() = Some(value.unwrap_or_default());
        self.subscribe_to_selected_items();
    }

    /// Expands the specified [`TreeViewItem`] and all its descendent
    /// [`TreeViewItem`]s.
    pub fn expand_sub_tree(&self, item: &TreeViewItem) {
        item.set_is_expanded(true);

        if item.items_panel_root().is_none() {
            if let Some(layout_manager) = self.get_layout_manager() {
                layout_manager.execute_layout_pass();
            }
        }

        if let Some(panel) = item.items_panel_root() {
            for child in panel.children().snapshot().iter() {
                if let Some(tree_view_item) = child.downcast_ref::<TreeViewItem>() {
                    self.expand_sub_tree(tree_view_item);
                }
            }
        }
    }

    /// Collapses the specified [`TreeViewItem`] and all its descendent
    /// [`TreeViewItem`]s.
    pub fn collapse_sub_tree(&self, item: &TreeViewItem) {
        item.set_is_expanded(false);

        if let Some(panel) = item.items_panel_root() {
            for child in panel.children().snapshot().iter() {
                if let Some(tree_view_item) = child.downcast_ref::<TreeViewItem>() {
                    self.collapse_sub_tree(tree_view_item);
                }
            }
        }
    }

    /// Selects all items in the [`TreeView`].
    ///
    /// Note that this method only selects nodes currently visible due to
    /// their parent nodes being expanded: it does not expand nodes.
    pub fn select_all(&self) {
        fn add_items(items_control: &ItemsControl, all_items: &mut Vec<Option<BoxedValue>>) {
            all_items.extend(items_control.items_view().iter());

            for child in items_control.get_realized_containers() {
                if let Some(child_items_control) = child.downcast_ref::<ItemsControl>() {
                    add_items(child_items_control, all_items);
                }
            }
        }

        let mut all_items = Vec::new();
        add_items(self, &mut all_items);
        Self::synchronize_items(&self.selected_items(), &all_items);
    }

    /// Deselects all items in the [`TreeView`].
    pub fn unselect_all(&self) {
        self.selected_items().clear();
    }

    /// Gets the currently realized containers of the whole tree.
    pub fn get_realized_tree_containers(&self) -> Vec<Ref<Control>> {
        fn get_realized_containers(items_control: &ItemsControl, result: &mut Vec<Ref<Control>>) {
            for container in items_control.get_realized_containers() {
                result.push(container.clone());
                if let Some(items_control_container) = container.downcast_ref::<ItemsControl>() {
                    get_realized_containers(items_control_container, result);
                }
            }
        }

        let mut result = Vec::new();
        get_realized_containers(self, &mut result);
        result
    }

    /// Returns the container of the specified item, searching the whole
    /// tree.
    pub fn tree_container_from_item(&self, item: &Option<BoxedValue>) -> Option<Ref<Control>> {
        fn tree_container_from_item(items_control: &ItemsControl, item: &Option<BoxedValue>) -> Option<Ref<Control>> {
            if let Some(container) = items_control.container_from_item(item) {
                return Some(container);
            }

            for child in items_control.get_realized_containers() {
                if let Some(child_items_control) = child.downcast_ref::<ItemsControl>() {
                    if let Some(child_container) = tree_container_from_item(child_items_control, item) {
                        return Some(child_container);
                    }
                }
            }

            None
        }

        tree_container_from_item(self, item)
    }

    /// Returns the item of the specified container, searching the whole
    /// tree.
    pub fn tree_item_from_container(&self, container: &Ref<Control>) -> Option<BoxedValue> {
        fn tree_item_from_container(items_control: &ItemsControl, container: &Ref<Control>) -> Option<BoxedValue> {
            if let Some(item) = items_control.item_from_container(container) {
                return Some(item);
            }

            for child in items_control.get_realized_containers() {
                if let Some(child_items_control) = child.downcast_ref::<ItemsControl>() {
                    if let Some(child_item) = tree_item_from_container(child_items_control, container) {
                        return Some(child_item);
                    }
                }
            }

            None
        }

        tree_item_from_container(self, container)
    }

    /// Subscribes to the collection changed event of the selected items.
    fn subscribe_to_selected_items(&self) {
        let selected_items = self.selected_items.borrow().clone();

        if let Some(selected_items) = selected_items {
            let weak = self.to_ref().downgrade();
            let token = selected_items.add_collection_changed(Rc::new(
                move |e: &NotifyCollectionChangedEventArgs<'_, Option<BoxedValue>>| {
                    if let Some(this) = weak.upgrade() {
                        this.selected_items_collection_changed(&e.into());
                    }
                },
            ));
            self.selected_items_subscription.set(Some(token));
        }

        self.selected_items_collection_changed(&ItemsChangedEventArgs::RESET);
    }

    fn select_single_item(&self, item: Option<BoxedValue>) {
        let old_value = self.selected_item.borrow().clone();
        self.syncing_selected_items.set(true);
        self.selected_items().clear();
        *self.selected_item.borrow_mut() = item.clone();
        self.selected_items().add(item);
        self.syncing_selected_items.set(false);

        let new_value = self.selected_item.borrow().clone();
        self.raise_direct_property_changed(Self::selected_item_property(), &old_value, &new_value);
    }

    /// Called when the collection changed event of the selected items is
    /// raised.
    fn selected_items_collection_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        let mut added: Option<Vec<Option<BoxedValue>>> = None;
        let mut removed: Option<Vec<Option<BoxedValue>>> = None;

        match e.action {
            NotifyCollectionChangedAction::Add => {
                self.selected_items_added(e.new_items);

                let selected_item = self.selected_item();

                if self.auto_scroll_to_selected_item()
                    && selected_item.is_some()
                    && items_equal(&e.new_items.get(0), &selected_item)
                {
                    if let Some(container) = self.tree_container_from_item(&selected_item) {
                        container.bring_into_view();
                    }
                }

                added = Some(e.new_items.to_vec());
            }
            NotifyCollectionChangedAction::Remove => {
                if !self.syncing_selected_items.get() {
                    if self.selected_items().count() == 0 {
                        self.set_selected_item(None);
                    } else {
                        let selected_item = self.selected_item.borrow().clone();
                        let selected_index = self.selected_items().index_of(&selected_item);

                        if selected_index.is_none() {
                            let old = selected_item;
                            let new = self.selected_items().get(0);
                            *self.selected_item.borrow_mut() = new.clone();

                            self.raise_direct_property_changed(Self::selected_item_property(), &old, &new);
                        }
                    }
                }

                for item in e.old_items {
                    self.mark_item_selected(&item, false);
                }

                removed = Some(e.old_items.to_vec());
            }
            NotifyCollectionChangedAction::Reset => {
                for container in self.get_realized_tree_containers() {
                    self.mark_container_selected(&container, false);
                }

                if self.selected_items().count() > 0 {
                    let selected_items = self.selected_items().to_vec();
                    self.selected_items_added(ItemsView::from_slice(&selected_items));

                    added = Some(selected_items);
                } else if !self.syncing_selected_items.get() {
                    self.set_selected_item(None);
                }
            }
            NotifyCollectionChangedAction::Replace => {
                for item in e.old_items {
                    self.mark_item_selected(&item, false);
                }

                for item in e.new_items {
                    self.mark_item_selected(&item, true);
                }

                let first = self.selected_items().get(0);
                if !items_equal(&self.selected_item(), &first) && !self.syncing_selected_items.get() {
                    let old_item = self.selected_item();
                    *self.selected_item.borrow_mut() = first.clone();
                    self.raise_direct_property_changed(Self::selected_item_property(), &old_item, &first);
                }

                added = Some(e.new_items.to_vec());
                removed = Some(e.old_items.to_vec());
            }
            _ => {}
        }

        if added.as_ref().is_some_and(|added| !added.is_empty())
            || removed.as_ref().is_some_and(|removed| !removed.is_empty())
        {
            let changed = SelectionChangedEventArgs::new(
                Some(SelectingItemsControl::selection_changed_event()),
                removed.unwrap_or_default(),
                added.unwrap_or_default(),
            );
            self.raise_event(&changed);
        }
    }

    fn mark_item_selected(&self, item: &Option<BoxedValue>, selected: bool) {
        if let Some(container) = self.tree_container_from_item(item) {
            self.mark_container_selected(&container, selected);
        }
    }

    fn selected_items_added(&self, items: ItemsView<'_>) {
        if items.is_empty() {
            return;
        }

        for item in items {
            self.mark_item_selected(&item, true);
        }

        if self.selected_item().is_none() && !self.syncing_selected_items.get() {
            self.set_and_raise(Self::selected_item_property(), &self.selected_item, items.get(0));
        }
    }

    /// Unsubscribes from the collection changed event of the selected
    /// items.
    fn unsubscribe_from_selected_items(&self) {
        let selected_items = self.selected_items.borrow().clone();

        if let (Some(selected_items), Some(token)) = (selected_items, self.selected_items_subscription.take()) {
            selected_items.remove_collection_changed(token);
        }
    }

    fn get_container_in_direction(
        &self,
        from: Option<Ref<TreeViewItem>>,
        direction: NavigationDirection,
        into_children: bool,
    ) -> Option<Ref<TreeViewItem>> {
        // The parent of the item is the tree view or another item; anything
        // else (or no item to navigate from) has nothing to navigate to.
        let from = from?;
        let parent_item = from.parent().and_then(|parent| parent.cast::<TreeViewItem>());
        let parent: Ref<ItemsControl> = match &parent_item {
            Some(parent_item) => parent_item.clone().upcast(),
            None => from.parent().and_then(|parent| parent.cast::<TreeView>())?.upcast(),
        };

        let index = parent.index_from_container(&from.clone().upcast());
        let as_item = |container: Option<Ref<Control>>| container.and_then(|container| container.cast::<TreeViewItem>());

        match direction {
            NavigationDirection::Up => {
                if index > 0 {
                    let previous = as_item(parent.container_from_index(index - 1))?;
                    if previous.is_expanded() && previous.item_count() > 0 {
                        as_item(previous.container_from_index(previous.item_count() - 1))
                    } else {
                        Some(previous)
                    }
                } else {
                    parent_item
                }
            }
            NavigationDirection::Down | NavigationDirection::Right => {
                if from.is_expanded() && into_children && from.item_count() > 0 {
                    as_item(from.container_from_index(0))
                } else if index < parent.item_count() - 1 {
                    as_item(parent.container_from_index(index + 1))
                } else if let Some(parent_item) = parent_item {
                    self.get_container_in_direction(Some(parent_item), direction, false)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Updates the selection for an item based on user interaction.
    ///
    /// `select` is whether the item should be selected or unselected,
    /// `range_modifier` whether the range modifier is enabled (i.e. the
    /// shift key), `toggle_modifier` whether the toggle modifier is enabled
    /// (i.e. the control key) and `right_button` whether the event is a
    /// right-click.
    pub fn update_selection_from_container(
        &self,
        container: &Ref<Control>,
        select: bool,
        range_modifier: bool,
        toggle_modifier: bool,
        right_button: bool,
    ) {
        let Some(item) = self.tree_item_from_container(container) else { return };
        let item = Some(item);

        let mut selected_container = None;

        if self.selected_item().is_some() {
            selected_container = self.tree_container_from_item(&self.selected_item());
        }

        let mode = self.selection_mode();
        let toggle = toggle_modifier || mode.contains(SelectionMode::TOGGLE);
        let multi = mode.contains(SelectionMode::MULTIPLE);
        let range = multi && range_modifier && selected_container.is_some();

        if !select {
            self.selected_items().remove(&item);
        } else if right_button {
            if !self.selected_items().contains(&item) {
                self.select_single_item(item);
            }
        } else if !toggle && !range {
            self.select_single_item(item);
        } else if multi && range {
            Self::synchronize_items(
                &self.selected_items(),
                &self.get_items_in_range(
                    selected_container.and_then(|container| container.cast::<TreeViewItem>()),
                    container.clone().cast::<TreeViewItem>(),
                ),
            );
        } else {
            let i = self.selected_items().index_of(&item);

            if i.is_some() {
                self.selected_items().remove(&item);
            } else if multi {
                self.selected_items().add(item);
            } else {
                self.set_selected_item(item);
            }
        }
    }

    /// Finds which node is first in the hierarchy.
    fn find_first_node(tree_view: &TreeView, node_a: &Ref<TreeViewItem>, node_b: &Ref<TreeViewItem>) -> Option<Ref<TreeViewItem>> {
        Self::find_in_containers(tree_view, node_a, node_b)
    }

    fn find_in_containers(
        items_control: &ItemsControl,
        node_a: &Ref<TreeViewItem>,
        node_b: &Ref<TreeViewItem>,
    ) -> Option<Ref<TreeViewItem>> {
        for container in items_control.get_realized_containers() {
            let node = Self::find_first_node_in(container.cast::<TreeViewItem>(), node_a, node_b);

            if node.is_some() {
                return node;
            }
        }

        None
    }

    fn find_first_node_in(
        node: Option<Ref<TreeViewItem>>,
        node_a: &Ref<TreeViewItem>,
        node_b: &Ref<TreeViewItem>,
    ) -> Option<Ref<TreeViewItem>> {
        let node = node?;

        if node == *node_a {
            return Some(node_a.clone());
        }

        if node == *node_b {
            return Some(node_b.clone());
        }

        Self::find_in_containers(&node, node_a, node_b)
    }

    /// Returns all items that belong to containers between `from` and `to`.
    /// The range is inclusive.
    fn get_items_in_range(
        &self,
        from: Option<Ref<TreeViewItem>>,
        to: Option<Ref<TreeViewItem>>,
    ) -> Vec<Option<BoxedValue>> {
        let mut items = Vec::new();

        let (Some(mut from), Some(mut to)) = (from, to) else { return items };

        let Some(first_item) = Self::find_first_node(self, &from, &to) else { return items };

        let mut was_reversed = false;

        if first_item == to {
            std::mem::swap(&mut from, &mut to);

            was_reversed = true;
        }

        let mut node = Some(from);

        while let Some(current) = node.take() {
            if current == to {
                break;
            }

            let item = self.tree_item_from_container(&current.clone().upcast());

            if item.is_some() {
                items.push(item);
            }

            node = self.get_container_in_direction(Some(current), NavigationDirection::Down, true);
        }

        let to_item = self.tree_item_from_container(&to.upcast());

        if to_item.is_some() {
            items.push(to_item);
        }

        if was_reversed {
            items.reverse();
        }

        items
    }

    /// Updates the selection based on an event that may have originated in
    /// a container that belongs to the control.
    ///
    /// `event_source` is the control that raised the event; the other
    /// arguments are those of
    /// [`update_selection_from_container`](Self::update_selection_from_container).
    ///
    /// Returns true if the event originated from a container that belongs
    /// to the control; otherwise false.
    pub fn update_selection_from_event_source(
        &self,
        event_source: Option<Ref<FerroObject>>,
        select: bool,
        range_modifier: bool,
        toggle_modifier: bool,
        right_button: bool,
    ) -> bool {
        let container = self.get_container_from_event_source(event_source);

        if let Some(container) = container {
            self.update_selection_from_container(
                &container.upcast(),
                select,
                range_modifier,
                toggle_modifier,
                right_button,
            );
            return true;
        }

        false
    }

    /// Tries to get the container that was the source of an event.
    ///
    /// Returns the container or `None` if the event did not originate in a
    /// container.
    pub fn get_container_from_event_source(&self, event_source: Option<Ref<FerroObject>>) -> Option<Ref<TreeViewItem>> {
        let source = event_source.and_then(|source| source.cast::<Visual>())?;
        let item = source.get_self_and_visual_ancestors().find_map(|visual| visual.cast::<TreeViewItem>())?;

        if item.tree_view_owner().is_some_and(|owner| owner == self.to_ref()) {
            Some(item)
        } else {
            None
        }
    }

    /// Called when a container raises the is selected changed event.
    fn container_selection_changed(&self, e: &RoutedEventArgs) {
        let source = e.source();
        let container = source.as_ref().and_then(|source| source.clone().cast::<TreeViewItem>());

        if let Some(container) = container {
            if container.tree_view_owner().is_some_and(|owner| owner == self.to_ref()) {
                let container: Ref<Control> = container.upcast();

                if let Some(item) = self.tree_item_from_container(&container) {
                    let item = Some(item);
                    let container_is_selected = SelectingItemsControl::get_is_selected(&container);
                    let our_is_selected = self.selected_items().contains(&item);

                    if container_is_selected != our_is_selected {
                        if container_is_selected {
                            self.selected_items().add(item);
                        } else {
                            self.selected_items().remove(&item);
                        }
                    }
                }
            }
        }

        if !e.is_source(self) {
            e.set_handled(true);
        }
    }

    /// Sets the selected state of a container.
    fn mark_container_selected(&self, container: &Control, selected: bool) {
        container.set_current_value(SelectingItemsControl::is_selected_property(), selected);
    }

    /// Makes a list of objects equal another (though doesn't preserve
    /// order).
    fn synchronize_items(items: &SelectedItemsList, desired: &[Option<BoxedValue>]) {
        let items_count = items.count();
        let desired_count = desired.len();

        if items_count == 0 && desired_count > 0 {
            // Add all desired.
            for item in desired {
                items.add(item.clone());
            }
        } else if items_count > 0 && desired_count == 0 {
            // Remove all.
            items.clear();
        } else {
            // Intersect.
            fn except(first: &[Option<BoxedValue>], second: &[Option<BoxedValue>]) -> Vec<Option<BoxedValue>> {
                let mut result: Vec<Option<BoxedValue>> = Vec::new();
                for item in first {
                    if !second.iter().any(|other| items_equal(other, item))
                        && !result.iter().any(|other| items_equal(other, item))
                    {
                        result.push(item.clone());
                    }
                }
                result
            }

            let list = items.to_vec();
            let to_remove = except(&list, desired);
            let to_add = except(desired, &list);

            for i in &to_remove {
                items.remove(i);
            }

            for i in to_add {
                items.add(i);
            }
        }
    }
}
