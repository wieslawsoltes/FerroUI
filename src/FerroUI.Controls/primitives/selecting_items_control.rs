use super::{ItemSelectionEventTriggers, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt, TextSearch};
use crate::assigned_binding::AssignedBinding;
use crate::items_source::{items_equal, ItemsChangedEventArgs};
use crate::navigable_containers::as_navigable_container;
use crate::selection::{
    ISelectionModel, InternalSelectionModel, SelectionModelExtensions, SelectionModelSelectionChangedEventArgs,
};
use crate::utils::BindingEvaluator;
use crate::{
    Control, ControlImpl, ItemsControl, ItemsControlImpl, ItemsControlImplExt, SelectionChangedEventArgs,
    SelectionMode,
};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction};
use ferroui_base::data::BindingMode;
use ferroui_base::input::{
    FocusChangedEventArgs, FocusManager, InputElement, InputElementImpl, InputElementImplExt, KeyEventArgs,
    KeyboardNavigation, NavigationDirection, PointerEventArgs, TextInputEventArgs,
};
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken,
    RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, AnyValue, AttachedProperty,
    BoxedValue, DirectProperty, DirectPropertyMetadata, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledElementImplExt, StyledProperty,
    StyledPropertyOptions, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;
use std::time::Duration;

/// The list held by the `SelectedItems` property: a notifying list of
/// untyped items. Equality is the identity of the list.
#[derive(Clone)]
pub struct SelectedItemsList(pub Rc<FerroList<Option<BoxedValue>>>);

impl SelectedItemsList {
    /// Creates an empty list.
    pub fn new() -> Self {
        Self(Rc::new(FerroList::new()))
    }

    /// Creates a list with the given items.
    pub fn from_items(items: impl IntoIterator<Item = Option<BoxedValue>>) -> Self {
        Self(Rc::new(FerroList::from_items(items)))
    }
}

impl Default for SelectedItemsList {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for SelectedItemsList {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Deref for SelectedItemsList {
    type Target = FerroList<Option<BoxedValue>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// When in a begin/end init block, or when the data context is updating, we
// need to defer changes to the selection model because we have no idea in
// which order properties will be set. Consider:
//
// - Both the items and the selected item are bound
// - The data context changes
// - The binding for the selected item updates first, producing an item
// - The items are searched to find the index of the new selected item
// - However the items aren't yet updated; the item is not found
// - The selected index is incorrectly set to -1
//
// This logic cannot be encapsulated in the selection model because the
// selection model can also be bound, consider:
//
// - Both the items and the selection are bound
// - The data context changes
// - The binding for the items updates first
// - The new items are assigned to the source of the selection
// - The binding for the selection updates, producing a new selection model
// - Both the old and new selection models have the incorrect source
#[derive(Default)]
struct UpdateState {
    update_count: i32,
    selection: Option<Rc<dyn ISelectionModel>>,
    selected_items: Option<Option<SelectedItemsList>>,
    selected_index: Option<i32>,
    selected_item: Option<Option<BoxedValue>>,
    selected_value: Option<Option<BoxedValue>>,
}

/// Resets a flag when dropped.
struct Reset<'a>(&'a Cell<bool>);

impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// An [`ItemsControl`] that maintains a selection.
///
/// [`SelectingItemsControl`] provides a base class for [`ItemsControl`]s
/// that maintain a selection (single or multiple). By default only its
/// `selected_index` and `selected_item` properties are visible; the current
/// multiple `selection` and `selected_items` together with the
/// `selection_mode` properties are protected, however a derived class can
/// expose these if it wishes to support multiple selection.
///
/// [`SelectingItemsControl`] maintains a selection respecting the current
/// `selection_mode` but it does not react to user input; this must be
/// handled in a derived class. It does, however, respond to
/// `is_selected_changed_event` events from items and updates the selection
/// accordingly.
#[repr(C)]
pub struct SelectingItemsControl {
    base: ItemsControl,
    text_search_term: RefCell<String>,
    text_search_timer: RefCell<Option<(Rc<DispatcherTimer>, Rc<dyn IDisposable>)>>,
    selection: RefCell<Option<Rc<dyn ISelectionModel>>>,
    selection_property_changed_token: Cell<u64>,
    selection_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    old_selected_index: Cell<i32>,
    old_selected_item: RefCell<Option<BoxedValue>>,
    old_selected_items: RefCell<Option<SelectedItemsList>>,
    selected_items_snapshot: RefCell<Vec<Option<BoxedValue>>>,
    selected_items_before_reset: RefCell<Option<Vec<Option<BoxedValue>>>>,
    ignore_container_selection_changed: Cell<bool>,
    update_state: RefCell<Option<UpdateState>>,
    has_scrolled_to_selected_item: Cell<bool>,
    selected_value_binding_evaluator: RefCell<Option<Ref<BindingEvaluator>>>,
    is_selection_change_active: Cell<bool>,
    unverified_selected_index: Cell<i32>,
    auto_scroll_requested_index: Cell<i32>,
}

ferro_class! {
    SelectingItemsControl: ItemsControl, virtuals SelectingItemsControlImpl: ItemsControlImpl {
        /// Analyses a pointer event received by a selectable element, and
        /// determines whether the action should trigger selection.
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
ferroui_base::ferro_class_info!(SelectingItemsControl { new: SelectingItemsControl::new });

ferro_impl_classes!(SelectingItemsControl: LayoutableImpl, InteractiveImpl, ControlImpl);

impl FerroObjectImpl for SelectingItemsControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        this.items().add_source_changed(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.on_items_view_source_changed();
            }
        }));

        let weak = this.to_ref().downgrade();
        this.items().add_pre_collection_changed(Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
            if let Some(this) = weak.upgrade() {
                this.on_items_view_pre_collection_changed(e);
            }
        }));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::auto_scroll_to_selected_item_property().as_property() {
            this.auto_scroll_to_selected_item_if_necessary(this.get_anchor_index());
        } else if change.property() == Visual::is_visible_property().as_property() {
            if change.get_new_value::<bool>() {
                this.auto_scroll_to_selected_item_if_necessary(this.get_anchor_index());
            }
        } else if change.property() == Self::selection_mode_property().as_property() {
            let selection = this.selection.borrow().clone();
            if let Some(selection) = selection {
                let new_value = change.get_new_value::<SelectionMode>();
                selection.set_single_select(!new_value.contains(SelectionMode::MULTIPLE));
            }
        } else if change.property() == Self::wrap_selection_property().as_property() {
            this.set_wrap_focus(this.wrap_selection());
        } else if change.property() == Self::selected_value_property().as_property() {
            if this.is_selection_change_active.get() {
                return;
            }

            let new_value = change.get_new_value::<Option<BoxedValue>>();

            if let Some(state) = this.update_state.borrow_mut().as_mut() {
                state.selected_value = Some(new_value);
                return;
            }

            this.select_item_with_value(&new_value);
        } else if change.property() == Self::selected_value_binding_property().as_property() {
            let idx = this.selected_index();

            // If no selection is active, don't do anything as the selected
            // value is already null.
            if idx == -1 {
                return;
            }

            let Some(value) = change.get_new_value::<Option<AssignedBinding>>() else {
                // Clearing the selected value binding makes the selected
                // value the item itself.
                this.set_current_value(Self::selected_value_property(), this.selected_item());
                return;
            };

            let selected_item = this.selected_item();

            this.is_selection_change_active.set(true);
            let _reset = Reset(&this.is_selection_change_active);

            let binding_evaluator = this.get_selected_value_binding_evaluator(&value);

            // Re-evaluate the selected value with the new binding.
            this.set_current_value(Self::selected_value_property(), binding_evaluator.evaluate(&selected_item));
        }
    }
}

impl StyledElementImpl for SelectingItemsControl {
    fn begin_init(this: &Self) {
        Self::parent_begin_init(this);
        this.begin_updating();
    }

    fn try_end_init(this: &Self) -> Result<(), ferroui_base::InitializationError> {
        Self::parent_try_end_init(this)?;
        this.end_updating();
        Ok(())
    }

    fn on_data_context_begin_update(this: &Self) {
        Self::parent_on_data_context_begin_update(this);
        this.begin_updating();
    }

    fn on_data_context_end_update(this: &Self) {
        Self::parent_on_data_context_end_update(this);
        this.end_updating();
    }

    fn on_initialized(this: &Self) {
        Self::parent_on_initialized(this);

        let selection = this.selection.borrow().clone();
        this.try_initialize_selection_source(selection.as_ref(), this.update_state.borrow().is_none());
    }
}

impl VisualImpl for SelectingItemsControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        this.auto_scroll_to_selected_item_if_necessary(this.get_anchor_index());
    }
}

impl InputElementImpl for SelectingItemsControl {
    fn on_text_input(this: &Self, e: &TextInputEventArgs) {
        if !e.handled() {
            if !this.is_text_search_enabled() {
                return;
            }

            this.stop_text_search_timer();

            if let Some(text) = &e.text {
                this.text_search_term.borrow_mut().push_str(text);
            }

            let term = this.text_search_term.borrow().clone();
            let new_index = this.get_index_from_text_search(&term);
            if new_index >= 0 {
                this.set_selected_index(new_index);
            }

            this.start_text_search_timer();

            e.set_handled(true);
        }

        Self::parent_on_text_input(this, e);
    }
}

impl TemplatedControlImpl for SelectingItemsControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        this.auto_scroll_to_selected_item_if_necessary(this.get_anchor_index());
    }
}

impl ItemsControlImpl for SelectingItemsControl {
    fn on_items_view_collection_changed(this: &Self, e: &ItemsChangedEventArgs<'_>) {
        Self::parent_on_items_view_collection_changed(this, e);

        // Do not change the selected index during initialization.
        if this.update_state.borrow().is_some() {
            return;
        }

        if this.always_selected() && this.selected_index() == -1 && this.item_count() > 0 {
            this.set_selected_index(this.choose_first_visible_and_enabled_index());
        }
    }

    fn prepare_container_for_item_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        // Ensure that the selection model is created at this point so that
        // accessing it in `container_for_item_prepared_override` doesn't
        // cause it to be initialized (which can make containers become
        // deselected when they're synced with the empty selection mode).
        this.get_or_create_selection_model();

        Self::parent_prepare_container_for_item_override(this, container, item, index);
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
        if !container.is_set(Self::is_selected_property().as_property()) {
            // The is selected property is not set on the container: update
            // the container selection based on the current selection as
            // understood by this control.
            this.mark_container_selected(container, this.selection().is_selected(index));
        } else {
            // The is selected property is set on the container: there is a
            // style or item container theme which has bound the property.
            // Update our selection based on the selection state of the
            // container.
            let container_is_selected = Self::get_is_selected(container);
            this.update_selection(index, container_is_selected, false, true, false, false);
        }

        if this.selection().anchor_index() == index {
            KeyboardNavigation::set_tab_once_active_element(this, Some(container.clone().upcast()));
        }

        if this.always_selected() {
            if this.selected_index() == -1 && container.is_visible() && container.is_enabled() {
                this.set_selected_index(index);
            } else if index == this.selected_index() && (!container.is_visible() || !container.is_enabled()) {
                this.move_selection_to_first_visible_and_enabled_item();
            }
        }
    }

    fn container_index_changed_override(this: &Self, container: &Ref<Control>, old_index: i32, new_index: i32) {
        Self::parent_container_index_changed_override(this, container, old_index, new_index);
        this.mark_container_selected(container, this.selection().is_selected(new_index));
    }

    fn clear_container_for_item_override(this: &Self, element: &Ref<Control>) {
        Self::parent_clear_container_for_item_override(this, element);

        this.ignore_container_selection_changed.set(true);
        let _reset = Reset(&this.ignore_container_selection_changed);
        element.clear_value(Self::is_selected_property());
    }
}

impl SelectingItemsControlImpl for SelectingItemsControl {
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

        let container_index = this.index_from_container(container);
        if container_index == -1 {
            return false;
        }

        let pointer_event = event_args.downcast_ref::<PointerEventArgs>();
        let key_event = event_args.downcast_ref::<KeyEventArgs>();
        let is_focus_event = event_args.is::<FocusChangedEventArgs>();

        let triggers = match (pointer_event, key_event) {
            (Some(pointer_event), _) => this.should_trigger_selection(container, pointer_event),
            (None, Some(key_event)) => this.should_trigger_selection_key(container, key_event),
            (None, None) => is_focus_event,
        };

        if !triggers {
            return false;
        }

        this.update_selection(
            container_index,
            true,
            ItemSelectionEventTriggers::has_range_selection_modifier(container, event_args),
            ItemSelectionEventTriggers::has_toggle_selection_modifier(container, event_args),
            pointer_event.is_some_and(|e| e.properties().is_right_button_pressed),
            is_focus_event,
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

ferroui_base::ferro_properties! { impl SelectingItemsControl, also [
    SelectingItemsControl::wrap_selection_property,
] {
    ferro_property!(
        /// Defines the `AutoScrollToSelectedItem` property.
        pub fn auto_scroll_to_selected_item_property() -> StyledProperty<bool> {
            FerroProperty::register::<SelectingItemsControl, _>("AutoScrollToSelectedItem", true)
        }
    );

    ferro_property!(
        /// Defines the `SelectedIndex` property.
        pub fn selected_index_property() -> DirectProperty<SelectingItemsControl, i32> {
            FerroProperty::register_direct_with::<SelectingItemsControl, _>(
                "SelectedIndex",
                |o| o.selected_index(),
                Some(|o, v| o.set_selected_index(v)),
                DirectPropertyMetadata::new(Some(-1)).with_default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `SelectedItem` property.
        pub fn selected_item_property() -> DirectProperty<SelectingItemsControl, Option<BoxedValue>> {
            FerroProperty::register_direct_with::<SelectingItemsControl, _>(
                "SelectedItem",
                |o| o.selected_item(),
                Some(|o, v| o.set_selected_item(v)),
                DirectPropertyMetadata::new(Some(None))
                    .with_default_binding_mode(BindingMode::TwoWay)
                    .with_enable_data_validation(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `SelectedValue` property.
        pub fn selected_value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register_with::<SelectingItemsControl, _>(
                "SelectedValue",
                StyledPropertyOptions::new(None).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `SelectedValueBinding` property.
        pub fn selected_value_binding_property() -> StyledProperty<Option<AssignedBinding>> {
            FerroProperty::register::<SelectingItemsControl, _>("SelectedValueBinding", None)
        }
    );

    ferro_property!(
        /// Defines the `SelectedItems` property.
        pub fn selected_items_property() -> DirectProperty<SelectingItemsControl, Option<SelectedItemsList>> {
            FerroProperty::register_direct::<SelectingItemsControl, _>(
                "SelectedItems",
                |o| o.selected_items(),
                Some(|o, v| o.set_selected_items(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `Selection` property.
        pub fn selection_property() -> DirectProperty<SelectingItemsControl, Option<Rc<dyn ISelectionModel>>> {
            FerroProperty::register_direct::<SelectingItemsControl, _>(
                "Selection",
                |o| Some(o.selection()),
                Some(|o, v| o.set_selection(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `SelectionMode` property.
        pub fn selection_mode_property() -> StyledProperty<SelectionMode> {
            FerroProperty::register::<SelectingItemsControl, _>("SelectionMode", SelectionMode::SINGLE)
        }
    );

    ferro_property!(
        /// Defines the `IsSelected` attached property on a container
        /// [`Control`].
        pub fn is_selected_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached_with::<SelectingItemsControl, Control, _>(
                "IsSelected",
                StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `IsTextSearchEnabled` property.
        pub fn is_text_search_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<SelectingItemsControl, _>("IsTextSearchEnabled", false)
        }
    );
} }

impl SelectingItemsControl {
    ferro_routed_event!(
        /// Event that should be raised by containers when their selection
        /// state changes to notify the parent [`SelectingItemsControl`]
        /// that their selection state has changed.
        pub fn is_selected_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<SelectingItemsControl, _>("IsSelectedChanged", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `SelectionChanged` event.
        pub fn selection_changed_event() -> RoutedEvent<SelectionChangedEventArgs> {
            RoutedEvent::register::<SelectingItemsControl, _>("SelectionChanged", RoutingStrategies::BUBBLE)
        }
    );

    ferro_property!(for SelectingItemsControl;
        /// Defines the `WrapSelection` property.
        pub fn wrap_selection_property() -> StyledProperty<bool> {
            FerroProperty::register::<SelectingItemsControl, _>("WrapSelection", false)
        }
    );

    pub(crate) fn static_constructor() {
        Self::is_selected_changed_event()
            .add_class_handler::<SelectingItemsControl>(|x, e| x.container_selection_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ItemsControl::construct(),
            text_search_term: RefCell::new(String::new()),
            text_search_timer: RefCell::new(None),
            selection: RefCell::new(None),
            selection_property_changed_token: Cell::new(0),
            selection_changed_subscription: RefCell::new(None),
            old_selected_index: Cell::new(0),
            old_selected_item: RefCell::new(None),
            old_selected_items: RefCell::new(None),
            selected_items_snapshot: RefCell::new(Vec::new()),
            selected_items_before_reset: RefCell::new(None),
            ignore_container_selection_changed: Cell::new(false),
            update_state: RefCell::new(None),
            has_scrolled_to_selected_item: Cell::new(false),
            selected_value_binding_evaluator: RefCell::new(None),
            is_selection_change_active: Cell::new(false),
            unverified_selected_index: Cell::new(-1),
            auto_scroll_requested_index: Cell::new(-1),
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
        self.add_handler(Self::selection_changed_event(), handler)
    }

    /// Gets or sets a value indicating whether to automatically scroll to
    /// newly selected items.
    pub fn auto_scroll_to_selected_item(&self) -> bool {
        self.get_value(Self::auto_scroll_to_selected_item_property())
    }

    pub fn set_auto_scroll_to_selected_item(&self, value: bool) {
        self.set_value(Self::auto_scroll_to_selected_item_property(), value)
    }

    /// Gets or sets the index of the selected item.
    pub fn selected_index(&self) -> i32 {
        // When a begin/end init or data context update is in place we
        // return the value to be updated here, even though it's not yet
        // active and the property changed notification has not yet been
        // raised. If we don't do this then the old value will be written
        // back to the source when two-way bound, and the update value will
        // be lost.
        let pending = self.update_state.borrow().as_ref().map(|state| state.selected_index);
        if let Some(pending) = pending {
            return match pending {
                Some(selected_index) => selected_index,
                None => self.try_get_existing_selection().map_or(-1, |selection| selection.selected_index()),
            };
        }

        self.selection().selected_index()
    }

    pub fn set_selected_index(&self, value: i32) {
        if let Some(state) = self.update_state.borrow_mut().as_mut() {
            state.selected_index = Some(value);
            return;
        }

        self.selection().set_selected_index(value);
    }

    /// Gets or sets the selected item.
    pub fn selected_item(&self) -> Option<BoxedValue> {
        // See the selected index getter for more information.
        let pending = self.update_state.borrow().as_ref().map(|state| state.selected_item.clone());
        if let Some(pending) = pending {
            return match pending {
                Some(selected_item) => selected_item,
                None => self.try_get_existing_selection().and_then(|selection| selection.selected_item()),
            };
        }

        self.selection().selected_item()
    }

    pub fn set_selected_item(&self, value: Option<BoxedValue>) {
        if let Some(state) = self.update_state.borrow_mut().as_mut() {
            state.selected_item = Some(value);
            return;
        }

        self.selection().set_selected_item(value);
    }

    /// Gets the binding instance which will be used to get the value for
    /// the `selected_value` property of the selected item.
    pub fn selected_value_binding(&self) -> Option<AssignedBinding> {
        self.get_value(Self::selected_value_binding_property())
    }

    pub fn set_selected_value_binding(&self, value: Option<AssignedBinding>) {
        self.set_value(Self::selected_value_binding_property(), value)
    }

    /// Gets or sets the value of the selected item, obtained using
    /// `selected_value_binding`.
    pub fn selected_value(&self) -> Option<BoxedValue> {
        self.get_value(Self::selected_value_property())
    }

    pub fn set_selected_value(&self, value: Option<BoxedValue>) {
        self.set_value(Self::selected_value_property(), value)
    }

    /// Gets or sets the selected items.
    ///
    /// By default returns a new list that can be manipulated to change the
    /// selection. This property can be set to an existing list to keep the
    /// selection of the control in sync with it.
    pub fn selected_items(&self) -> Option<SelectedItemsList> {
        // See the selected index setter for more information.
        let pending = self.update_state.borrow().as_ref().and_then(|state| state.selected_items.clone());
        if let Some(pending) = pending {
            return pending;
        }

        let selection = self.selection();
        if let Some(ism) = selection.as_internal_selection_model() {
            let result = SelectedItemsList(ism.writable_selected_items());
            *self.old_selected_items.borrow_mut() = Some(result.clone());
            return Some(result);
        }

        None
    }

    pub fn set_selected_items(&self, value: Option<SelectedItemsList>) {
        if let Some(state) = self.update_state.borrow_mut().as_mut() {
            state.selected_items = Some(value);
            return;
        }

        let selection = self.selection();
        match selection.as_internal_selection_model() {
            Some(i) => i.set_writable_selected_items(value.map(|v| v.0)),
            None => panic!("Cannot set both Selection and SelectedItems."),
        }
    }

    /// Gets or sets the model that holds the current selection.
    pub fn selection(&self) -> Rc<dyn ISelectionModel> {
        let pending = self.update_state.borrow().as_ref().and_then(|state| state.selection.clone());
        match pending {
            Some(selection) => selection,
            None => self.get_or_create_selection_model(),
        }
    }

    pub fn set_selection(&self, value: Option<Rc<dyn ISelectionModel>>) {
        let value = value.unwrap_or_else(|| self.create_default_selection_model());

        if let Some(state) = self.update_state.borrow_mut().as_mut() {
            state.selection = Some(value);
            return;
        }

        let current = self.selection.borrow().clone();
        if current.as_ref().is_some_and(|current| crate::selection::selection_model_ptr_eq(current, &value)) {
            return;
        }

        if let Some(source) = value.source() {
            if !source.ptr_eq(&self.items_view().source()) {
                panic!(
                    "The supplied ISelectionModel already has an assigned Source but this \
                     collection is different to the Items on the control."
                );
            }
        }

        let old_selection = current.as_ref().map(|selection| selection.selected_items().to_vec());
        self.deinitialize_selection_model(current.as_ref());
        *self.selection.borrow_mut() = Some(value.clone());

        if let Some(old_selection) = old_selection.filter(|items| !items.is_empty()) {
            self.raise_event(&SelectionChangedEventArgs::new(
                Some(Self::selection_changed_event()),
                old_selection,
                Vec::new(),
            ));
        }

        self.initialize_selection_model(&value);
        let selected_items = self.selected_items();
        let old_selected_items = self.old_selected_items.borrow().clone();
        if old_selected_items != selected_items {
            self.raise_direct_property_changed(Self::selected_items_property(), &old_selected_items, &selected_items);
            *self.old_selected_items.borrow_mut() = selected_items;
        }
    }

    /// Gets or sets a value that specifies whether a user can jump to a
    /// value by typing.
    pub fn is_text_search_enabled(&self) -> bool {
        self.get_value(Self::is_text_search_enabled_property())
    }

    pub fn set_is_text_search_enabled(&self, value: bool) {
        self.set_value(Self::is_text_search_enabled_property(), value)
    }

    /// Gets or sets a value which indicates whether to wrap around when the
    /// first or last item is reached.
    pub fn wrap_selection(&self) -> bool {
        self.get_value(Self::wrap_selection_property())
    }

    pub fn set_wrap_selection(&self, value: bool) {
        self.set_value(Self::wrap_selection_property(), value)
    }

    /// Gets or sets the selection mode.
    ///
    /// Note that the selection mode only applies to selections made via
    /// user interaction. Multiple selections can be made programmatically
    /// regardless of the value of this property.
    pub fn selection_mode(&self) -> SelectionMode {
        self.get_value(Self::selection_mode_property())
    }

    pub fn set_selection_mode(&self, value: SelectionMode) {
        self.set_value(Self::selection_mode_property(), value)
    }

    /// Gets a value indicating whether the `ALWAYS_SELECTED` flag is set.
    pub fn always_selected(&self) -> bool {
        self.selection_mode().contains(SelectionMode::ALWAYS_SELECTED)
    }

    /// Gets the value of the `IsSelected` attached property on a control.
    pub fn get_is_selected(control: &Control) -> bool {
        control.get_value(Self::is_selected_property())
    }

    /// Sets the value of the `IsSelected` attached property on a control.
    pub fn set_is_selected(control: &Control, value: bool) {
        control.set_value(Self::is_selected_property(), value)
    }

    /// Tries to get the container that was the source of an event.
    ///
    /// Returns the container or `None` if the event did not originate in a
    /// container.
    pub fn get_container_from_event_source(&self, event_source: Option<&Ref<Visual>>) -> Option<Ref<Control>> {
        let this: Ref<ferroui_base::StyledElement> = self.to_ref().upcast();
        let mut current = event_source.cloned();

        while let Some(visual) = current {
            if let Some(control) = visual.cast::<Control>() {
                if control.parent().as_ref() == Some(&this) && self.index_from_container(&control) != -1 {
                    return Some(control);
                }
            }

            current = visual.visual_parent();
        }

        None
    }

    /// Returns the [`SelectingItemsControl`] that owns the specified
    /// container control.
    pub fn selecting_items_control_from_item_container(container: &Ref<Control>) -> Option<Ref<SelectingItemsControl>> {
        ItemsControl::items_control_from_item_container(container).and_then(|c| c.cast::<SelectingItemsControl>())
    }

    fn on_items_view_pre_collection_changed(&self, e: &ItemsChangedEventArgs<'_>) {
        if e.action == NotifyCollectionChangedAction::Reset && !self.selected_items_snapshot.borrow().is_empty() {
            let snapshot = self.selected_items_snapshot.borrow().clone();
            *self.selected_items_before_reset.borrow_mut() = Some(snapshot);
        }
    }

    pub(crate) fn get_anchor_index(&self) -> i32 {
        let selection = if self.update_state.borrow().is_some() {
            self.try_get_existing_selection()
        } else {
            Some(self.selection())
        };
        selection.map_or(-1, |selection| selection.anchor_index())
    }

    fn try_get_existing_selection(&self) -> Option<Rc<dyn ISelectionModel>> {
        let pending = self.update_state.borrow().as_ref().and_then(|state| state.selection.clone());
        match pending {
            Some(selection) => Some(selection),
            None => self.selection.borrow().clone(),
        }
    }

    /// Moves the selection in the specified direction relative to the
    /// current selection.
    ///
    /// Returns true if the selection was moved; otherwise false.
    pub fn move_selection(&self, direction: NavigationDirection, wrap: bool, range_modifier: bool) -> bool {
        let focused = FocusManager::get_focus_manager(self).and_then(|focus| focus.get_focused_element());
        let focused: Option<Ref<Visual>> = focused.map(Ref::upcast);
        let from = self
            .get_container_from_event_source(focused.as_ref())
            .or_else(|| self.container_from_index(self.selection().anchor_index()));
        self.move_selection_from(from.as_ref(), direction, wrap, range_modifier)
    }

    /// Moves the selection in the specified direction relative to the
    /// specified container.
    ///
    /// Returns true if the selection was moved; otherwise false.
    pub fn move_selection_from(
        &self,
        from: Option<&Ref<Control>>,
        mut direction: NavigationDirection,
        wrap: bool,
        range_modifier: bool,
    ) -> bool {
        let Some(panel) = self.presenter().and_then(|presenter| presenter.panel()) else { return false };
        let Some(container) = as_navigable_container(&panel) else { return false };

        if from.is_none() {
            direction = match direction {
                NavigationDirection::Down => NavigationDirection::First,
                NavigationDirection::Up => NavigationDirection::Last,
                NavigationDirection::Right => NavigationDirection::First,
                NavigationDirection::Left => NavigationDirection::Last,
                other => other,
            };
        }

        let from: Option<Ref<InputElement>> = from.map(|from| from.clone().upcast());
        let next = ItemsControl::get_next_control(container, direction, from.as_ref(), wrap);

        if let Some(next) = next.and_then(|next| next.cast::<Control>()) {
            let index = self.index_from_container(&next);

            if index != -1 {
                self.update_selection(index, true, range_modifier, false, false, false);
                next.focus();
                return true;
            }
        }

        false
    }

    /// Updates the selection for an item based on user interaction.
    ///
    /// `index` is the index of the item, `select` whether the item should
    /// be selected or unselected, `range_modifier` whether the range
    /// modifier is enabled (i.e. shift key), `toggle_modifier` whether the
    /// toggle modifier is enabled (i.e. ctrl key), `right_button` whether
    /// the event is a right-click and `from_focus` whether the event is a
    /// focus event.
    pub fn update_selection(
        &self,
        index: i32,
        select: bool,
        range_modifier: bool,
        toggle_modifier: bool,
        right_button: bool,
        from_focus: bool,
    ) {
        if index < 0 || index >= self.item_count() {
            return;
        }

        let mode = self.selection_mode();
        let multi = mode.contains(SelectionMode::MULTIPLE);
        let toggle = toggle_modifier || mode.contains(SelectionMode::TOGGLE);
        let range = multi && range_modifier;
        let selection = self.selection();

        if !select {
            selection.deselect(index);
        } else if right_button {
            if !selection.is_selected(index) {
                self.set_selected_index(index);
            }
        } else if range {
            let operation = SelectionModelExtensions::batch_update(&selection);
            if !toggle_modifier {
                selection.clear();
            }
            selection.select_range(selection.anchor_index(), index);
            operation.dispose();
        } else if !from_focus && toggle {
            if multi {
                if selection.is_selected(index) {
                    selection.deselect(index);
                } else {
                    selection.select(index);
                }
            } else {
                self.set_selected_index(if self.selected_index() == index { -1 } else { index });
            }
        } else if !toggle {
            let operation = SelectionModelExtensions::batch_update(&selection);
            selection.clear();
            selection.select(index);
            operation.dispose();
        }
    }

    /// Updates the selection for a container based on user interaction.
    #[deprecated(note = "Call update_selection_from_event instead.")]
    pub fn update_selection_for_container(
        &self,
        container: &Ref<Control>,
        select: bool,
        range_modifier: bool,
        toggle_modifier: bool,
        right_button: bool,
        from_focus: bool,
    ) {
        let index = self.index_from_container(container);

        if index != -1 {
            self.update_selection(index, select, range_modifier, toggle_modifier, right_button, from_focus);
        }
    }

    /// Updates the selection based on an event source that may have
    /// originated in a container that belongs to the control.
    ///
    /// Returns true if the event originated from a container that belongs
    /// to the control; otherwise false.
    #[deprecated(note = "Call update_selection_from_event instead.")]
    pub fn update_selection_from_event_source(
        &self,
        event_source: Option<&Ref<Visual>>,
        select: bool,
        range_modifier: bool,
        toggle_modifier: bool,
        right_button: bool,
        from_focus: bool,
    ) -> bool {
        let Some(container) = self.get_container_from_event_source(event_source) else { return false };

        #[allow(deprecated)]
        self.update_selection_for_container(
            &container,
            select,
            range_modifier,
            toggle_modifier,
            right_button,
            from_focus,
        );
        true
    }

    fn get_or_create_selection_model(&self) -> Rc<dyn ISelectionModel> {
        let existing = self.selection.borrow().clone();
        if let Some(existing) = existing {
            return existing;
        }

        let selection = self.create_default_selection_model();
        *self.selection.borrow_mut() = Some(selection.clone());
        self.initialize_selection_model(&selection);

        // Initializing the model raises events: the handlers may have
        // replaced it.
        let current = self.selection.borrow().clone();
        current.unwrap_or(selection)
    }

    fn on_items_view_source_changed(&self) {
        if self.update_state.borrow().is_none() {
            let selection = self.selection.borrow().clone();
            self.try_initialize_selection_source(selection.as_ref(), true);
        }
    }

    /// Called when a property changes on the selection model.
    fn on_selection_model_property_changed(&self, property_name: &str) {
        match property_name {
            "AnchorIndex" => {
                self.has_scrolled_to_selected_item.set(false);

                let anchor_index = self.get_anchor_index();
                KeyboardNavigation::set_tab_once_active_element(
                    self,
                    self.container_from_index(anchor_index).map(Ref::upcast),
                );
                self.auto_scroll_to_selected_item_if_necessary(anchor_index);
            }
            "SelectedIndex" => {
                let selected_index = self.selected_index();
                let old_selected_index = self.old_selected_index.get();
                if old_selected_index != selected_index {
                    self.raise_direct_property_changed(
                        Self::selected_index_property(),
                        &old_selected_index,
                        &selected_index,
                    );
                    self.old_selected_index.set(selected_index);
                }
            }
            "SelectedItem" => {
                let selected_item = self.selected_item();
                let old_selected_item = self.old_selected_item.borrow().clone();
                if !items_equal(&selected_item, &old_selected_item) {
                    self.raise_direct_property_changed(
                        Self::selected_item_property(),
                        &old_selected_item,
                        &selected_item,
                    );
                    *self.old_selected_item.borrow_mut() = selected_item;
                }
            }
            "WritableSelectedItems" => {
                let old_selected_items = self.old_selected_items.borrow().clone();
                // The reference compares the old list with the read-only
                // selected items of the internal model, which is never the
                // same object: the change is raised whenever the selection
                // is an internal model or there was a list before.
                let is_internal = self.selection().as_internal_selection_model().is_some();
                if is_internal || old_selected_items.is_some() {
                    let selected_items = self.selected_items();
                    self.raise_direct_property_changed(
                        Self::selected_items_property(),
                        &old_selected_items,
                        &selected_items,
                    );
                    *self.old_selected_items.borrow_mut() = selected_items;
                }
            }
            "Source" => self.clear_value(Self::selected_value_property()),
            _ => {}
        }
    }

    /// Called when the selection changed event is raised on the selection
    /// model.
    fn on_selection_model_selection_changed(&self, e: &dyn SelectionModelSelectionChangedEventArgs) {
        let mark = |index: i32, selected: bool| {
            if let Some(container) = self.container_from_index(index) {
                self.mark_container_selected(&container, selected);
            }
        };

        let selected_indexes = e.selected_indexes();
        for i in 0..selected_indexes.count() {
            mark(selected_indexes.get(i), true);
        }

        let deselected_indexes = e.deselected_indexes();
        for i in 0..deselected_indexes.count() {
            mark(deselected_indexes.get(i), false);
        }

        if !self.is_selection_change_active.get() {
            self.update_selected_value_from_item();
        }

        {
            let selected_items = self.selection().selected_items();
            let mut snapshot = self.selected_items_snapshot.borrow_mut();
            snapshot.clear();
            snapshot.extend(selected_items.iter());
        }
        *self.selected_items_before_reset.borrow_mut() = None;

        self.raise_selection_changed(
            || e.deselected_items().to_vec(),
            || e.selected_items().to_vec(),
        );

        self.verify_selected_index();
    }

    /// Called when the lost selection event is raised on the selection
    /// model.
    fn on_selection_model_lost_selection(&self) {
        let before_reset = self.selected_items_before_reset.take();
        if let Some(before_reset) = before_reset.filter(|items| !items.is_empty()) {
            self.raise_selection_changed(|| before_reset, Vec::new);
        }

        if self.always_selected() && self.items_view().count() > 0 {
            self.set_selected_index(self.choose_first_visible_and_enabled_index());
        }
    }

    fn raise_selection_changed(
        &self,
        removed_items: impl FnOnce() -> Vec<Option<BoxedValue>>,
        added_items: impl FnOnce() -> Vec<Option<BoxedValue>>,
    ) {
        let route = self.build_event_route(Self::selection_changed_event());

        if route.has_handlers() {
            self.raise_event(&SelectionChangedEventArgs::new(
                Some(Self::selection_changed_event()),
                removed_items(),
                added_items(),
            ));
        }
    }

    fn select_item_with_value(&self, value: &Option<BoxedValue>) {
        if self.item_count() == 0 || self.is_selection_change_active.get() {
            return;
        }

        self.is_selection_change_active.set(true);
        let _reset = Reset(&self.is_selection_change_active);

        match self.find_item_with_value(value) {
            Some(si) => self.set_selected_item(si),
            None => self.set_selected_item(None),
        }
    }

    /// The item whose value is `value`; `None` if there is no such item
    /// (the unset result of the reference).
    fn find_item_with_value(&self, value: &Option<BoxedValue>) -> Option<Option<BoxedValue>> {
        if self.item_count() == 0 || value.is_none() {
            return None;
        }

        let items = self.items_view().clone();

        let Some(binding) = self.selected_value_binding() else {
            // No selected value binding set, the selected value is the item
            // itself. Still verify the value passed in is in the items
            // list.
            let index = items.index_of(value);

            return if index >= 0 { Some(value.clone()) } else { None };
        };

        let binding_evaluator = self.get_selected_value_binding_evaluator(&binding);

        // Matching UWP behavior, if duplicates are present, return the
        // first item matching the selected value provided.
        for item in items.iter() {
            let item_value = binding_evaluator.evaluate(&item);

            if items_equal(&item_value, value) {
                binding_evaluator.clear_data_context();
                return Some(item);
            }
        }

        binding_evaluator.clear_data_context();

        None
    }

    fn update_selected_value_from_item(&self) {
        if self.is_selection_change_active.get() {
            return;
        }

        let binding = self.selected_value_binding();
        let item = self.selected_item();

        self.is_selection_change_active.set(true);
        let _reset = Reset(&self.is_selection_change_active);

        match binding {
            Some(binding) if item.is_some() => {
                let binding_evaluator = self.get_selected_value_binding_evaluator(&binding);
                self.set_current_value(Self::selected_value_property(), binding_evaluator.evaluate(&item));
            }
            _ => {
                // No selected value binding, the selected value is the item
                // itself.
                self.set_current_value(Self::selected_value_property(), item);
            }
        }
    }

    fn auto_scroll_to_selected_item_if_necessary(&self, anchor_index: i32) {
        if self.auto_scroll_to_selected_item()
            && self.presenter().is_some()
            && anchor_index >= 0
            && self.is_attached_to_visual_tree()
        {
            if !self.has_scrolled_to_selected_item.get() {
                self.scroll_into_view(anchor_index);
                self.auto_scroll_requested_index.set(anchor_index);
                self.has_scrolled_to_selected_item.set(true);
            }
        } else if self.auto_scroll_requested_index.get() >= 0 {
            // The conditions for auto-scrolling no longer hold, cancel the
            // scroll we requested earlier.
            if let Some(presenter) = self.presenter() {
                presenter.cancel_scroll_into_view(self.auto_scroll_requested_index.get());
            }
            self.auto_scroll_requested_index.set(-1);
        }
    }

    /// Called when a container raises the `is_selected_changed_event`.
    fn container_selection_changed(&self, e: &RoutedEventArgs) {
        if !self.ignore_container_selection_changed.get() {
            let this: Ref<ferroui_base::StyledElement> = self.to_ref().upcast();
            let control = e.source().and_then(|source| source.cast::<Control>());
            if let Some(control) = control.filter(|control| control.parent().as_ref() == Some(&this)) {
                let index = self.index_from_container(&control);
                if index >= 0 {
                    if Self::get_is_selected(&control) {
                        self.selection().select(index);
                    } else {
                        self.selection().deselect(index);
                    }
                }
            }
        }

        if !e.is_source(self) {
            e.set_handled(true);
        }
    }

    /// Sets the `IsSelected` property on a container.
    fn mark_container_selected(&self, container: &Control, selected: bool) {
        self.ignore_container_selection_changed.set(true);
        let _reset = Reset(&self.ignore_container_selection_changed);

        container.set_current_value(Self::is_selected_property(), selected);
    }

    /// Finds the index of the first item that is visible and enabled.
    ///
    /// The second value is true if the returned index is known to be
    /// visible and enabled; false if the returned index is a non-realized,
    /// non-visual item and its container state is unknown.
    fn get_first_visible_and_enabled_index(&self) -> (i32, bool) {
        let count = self.item_count();
        if count == 0 {
            return (-1, true);
        }

        for i in 0..count {
            if let Some(container) = self.container_from_index(i) {
                if container.is_visible() && container.is_enabled() {
                    return (i, true);
                }

                continue;
            }

            let item = self.items_view().get_at(i as usize);
            let Some(item) = item else { continue };

            let value: &dyn AnyValue = &*item;
            if let Some(c) = value.downcast_ref::<Ref<Control>>() {
                if c.is_visible() && c.is_enabled() {
                    return (i, true);
                }
            } else if let Some(v) = value.downcast_ref::<Ref<Visual>>() {
                if v.is_visible() {
                    return (i, true);
                }
            } else {
                // The container isn't realized so its visibility/enabled
                // state is unknown.
                return (i, false);
            }
        }

        (-1, true)
    }

    fn choose_first_visible_and_enabled_index(&self) -> i32 {
        let (index, verified) = self.get_first_visible_and_enabled_index();
        self.unverified_selected_index.set(if verified { -1 } else { index });
        index
    }

    fn verify_selected_index(&self) {
        if self.unverified_selected_index.get() == -1 {
            return;
        }

        if !self.always_selected() {
            self.unverified_selected_index.set(-1);
            return;
        }

        // The selection has moved elsewhere in the meantime.
        if self.unverified_selected_index.get() != self.selected_index() {
            self.unverified_selected_index.set(-1);
            return;
        }

        // Still not realized: `container_for_item_prepared_override` will
        // check it.
        let Some(container) = self.container_from_index(self.unverified_selected_index.get()) else { return };

        self.unverified_selected_index.set(-1);

        if !container.is_visible() || !container.is_enabled() {
            self.move_selection_to_first_visible_and_enabled_item();
        }
    }

    fn move_selection_to_first_visible_and_enabled_item(&self) {
        let index = self.get_first_realized_visible_and_enabled_index();
        if index != self.selected_index() {
            self.set_selected_index(index);
        }
    }

    fn get_first_realized_visible_and_enabled_index(&self) -> i32 {
        let count = self.item_count();
        for i in 0..count {
            if let Some(container) = self.container_from_index(i) {
                if container.is_visible() && container.is_enabled() {
                    return i;
                }
            }
        }
        -1
    }

    /// Update the containers using the current selection.
    fn update_container_selection(&self) {
        if let Some(panel) = self.presenter().and_then(|presenter| presenter.panel()) {
            let selection = self.selection();
            for container in panel.children().snapshot().iter() {
                self.mark_container_selected(container, selection.is_selected(self.index_from_container(&container.clone())));
            }
        }
    }

    fn create_default_selection_model(&self) -> Rc<dyn ISelectionModel> {
        let model = InternalSelectionModel::new();
        model.set_single_select(!self.selection_mode().contains(SelectionMode::MULTIPLE));
        model
    }

    fn initialize_selection_model(&self, model: &Rc<dyn ISelectionModel>) {
        if self.update_state.borrow().is_none() {
            self.try_initialize_selection_source(Some(model), false);
        }

        let weak = self.to_ref().downgrade();
        let token = model.property_changed().add(Rc::new(move |name: &str| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_model_property_changed(name);
            }
        }));
        self.selection_property_changed_token.set(token);

        let weak = self.to_ref().downgrade();
        let subscription = model.selection_changed(Rc::new(move |e: &dyn SelectionModelSelectionChangedEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_model_selection_changed(e);
            }
        }));
        *self.selection_changed_subscription.borrow_mut() = Some(subscription);

        let weak = self.to_ref().downgrade();
        model.lost_selection(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.on_selection_model_lost_selection();
            }
        }));

        if model.single_select() {
            self.set_selection_mode(self.selection_mode() & !SelectionMode::MULTIPLE);
        } else {
            self.set_selection_mode(self.selection_mode() | SelectionMode::MULTIPLE);
        }

        self.old_selected_index.set(model.selected_index());
        *self.old_selected_item.borrow_mut() = model.selected_item();
        {
            let selected_items = model.selected_items();
            let mut snapshot = self.selected_items_snapshot.borrow_mut();
            snapshot.clear();
            snapshot.extend(selected_items.iter());
        }

        if self.update_state.borrow().is_none() && self.always_selected() && model.count() == 0 {
            model.set_selected_index(self.choose_first_visible_and_enabled_index());
        }

        self.update_container_selection();

        if self.selected_index() != -1 {
            self.raise_event(&SelectionChangedEventArgs::new(
                Some(Self::selection_changed_event()),
                Vec::new(),
                self.selection().selected_items().to_vec(),
            ));
        }
    }

    fn try_initialize_selection_source(
        &self,
        selection: Option<&Rc<dyn ISelectionModel>>,
        should_select_item_from_selected_value: bool,
    ) {
        let Some(selection) = selection else { return };
        let Some(source) = self.items_view().try_get_initialized_source() else { return };

        // The internal selection model keeps the selected index and
        // selected item values before the items source is set. However, the
        // selected value isn't part of that model, so we have to set the
        // selected item from the selected value manually now that we have a
        // source.
        //
        // While this works, this is messy: we effectively have "lazy
        // selection initialization" in 3 places:
        //  - the update state (all selection properties, for begin/end
        //    init)
        //  - the internal selection model (selected index/selected item)
        //  - this control (selected value)
        //
        // There's the opportunity to have a single place responsible for
        // this logic.
        if should_select_item_from_selected_value
            && selection.selected_index() == -1
            && selection.selected_item().is_none()
        {
            if let Some(item) = self.find_item_with_value(&self.selected_value()) {
                selection.set_selected_item(item);
            }
        }

        selection.set_source(Some(source));
    }

    fn deinitialize_selection_model(&self, model: Option<&Rc<dyn ISelectionModel>>) {
        if let Some(model) = model {
            model.property_changed().remove(self.selection_property_changed_token.get());
            if let Some(subscription) = self.selection_changed_subscription.take() {
                subscription.dispose();
            }
        }
    }

    fn begin_updating(&self) {
        let mut state = self.update_state.borrow_mut();
        let state = state.get_or_insert_with(UpdateState::default);
        state.update_count += 1;
    }

    fn end_updating(&self) {
        let finished = {
            let mut state = self.update_state.borrow_mut();
            match state.as_mut() {
                Some(update_state) => {
                    update_state.update_count -= 1;
                    update_state.update_count == 0
                }
                None => false,
            }
        };

        if !finished {
            return;
        }

        let Some(mut state) = self.update_state.take() else { return };

        if let Some(selection) = state.selection.take() {
            self.set_selection(Some(selection));
        }

        let current = self.selection.borrow().clone();
        if let Some(s) = current.as_ref().and_then(|selection| selection.as_internal_selection_model()) {
            s.update(
                self.items_view().try_get_initialized_source(),
                state.selected_items.clone().map(|items| items.map(|items| items.0)),
            );
        } else {
            if let Some(selected_items) = state.selected_items.clone() {
                self.set_selected_items(selected_items);
            }

            self.try_initialize_selection_source(Some(&self.selection()), false);
        }

        if let Some(selected_value) = &state.selected_value {
            if let Some(item) = self.find_item_with_value(selected_value) {
                state.selected_item = Some(item);
            }
        }

        // Selected index vs selected item:
        // - If only one has a value, use it
        // - If both have a value, prefer the one having a "non-empty"
        //   value, e.g. not -1 nor null
        // - If both have a "non-empty" value, prefer the index
        if let Some(selected_index) = state.selected_index {
            match state.selected_item.clone() {
                Some(selected_item) if selected_index < 0 => self.set_selected_item(selected_item),
                _ => self.set_selected_index(selected_index),
            }
        } else if let Some(selected_item) = state.selected_item.clone() {
            self.set_selected_item(selected_item);
        }

        if self.always_selected() && self.selected_index() == -1 && self.item_count() > 0 {
            self.set_selected_index(self.choose_first_visible_and_enabled_index());
        }
    }

    fn start_text_search_timer(&self) {
        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_secs(1));
        let weak = self.to_ref().downgrade();
        let tick = timer.tick(move |_| {
            if let Some(this) = weak.upgrade() {
                this.text_search_timer_tick();
            }
        });
        timer.start();
        *self.text_search_timer.borrow_mut() = Some((timer, tick));
    }

    fn stop_text_search_timer(&self) {
        let Some((timer, tick)) = self.text_search_timer.take() else { return };

        tick.dispose();
        timer.stop();
    }

    fn text_search_timer_tick(&self) {
        self.text_search_term.borrow_mut().clear();
        self.stop_text_search_timer();
    }

    fn get_index_from_text_search(&self, text_search_term: &str) -> i32 {
        if text_search_term.is_empty() {
            return -1;
        }

        let count = self.items().count();
        if count == 0 {
            return -1;
        }

        let text_binding = TextSearch::get_text_binding(self).or_else(|| self.display_member_binding());
        let text_binding_evaluator = BindingEvaluator::try_create(text_binding.as_ref());

        let mut result = -1;

        for i in 0..count {
            let text = TextSearch::get_effective_text(&self.items().get_at(i), text_binding_evaluator.as_ref());
            if starts_with_ordinal_ignore_case(&text, text_search_term) {
                result = i as i32;
                break;
            }
        }

        if let Some(evaluator) = text_binding_evaluator {
            evaluator.dispose();
        }

        result
    }

    fn get_selected_value_binding_evaluator(&self, binding: &AssignedBinding) -> Ref<BindingEvaluator> {
        let existing = self.selected_value_binding_evaluator.borrow().clone();
        let evaluator = match existing {
            Some(evaluator) => evaluator,
            None => {
                let evaluator = BindingEvaluator::new();
                *self.selected_value_binding_evaluator.borrow_mut() = Some(evaluator.clone());
                evaluator
            }
        };
        evaluator.update_binding(binding);
        evaluator
    }
}

/// Whether `text` starts with `prefix`, comparing without regard to case
/// and culture.
fn starts_with_ordinal_ignore_case(text: &str, prefix: &str) -> bool {
    let mut text = text.chars().flat_map(char::to_uppercase);
    for expected in prefix.chars().flat_map(char::to_uppercase) {
        if text.next() != Some(expected) {
            return false;
        }
    }
    true
}
