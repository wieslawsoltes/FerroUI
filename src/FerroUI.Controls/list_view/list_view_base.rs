//! WinUI's `ListViewBase` and the types of its members (`dxaml/xcp/dxaml/lib/ListViewBase_Partial*.cpp`
//! and the metadata of `dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Controls.cs`
//! of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md), on FerroUI's `SelectingItemsControl`
//! (docs/porting/winui-calculator-gaps.md, section 6.2, LV-1).

use crate::generators::RecycleKey;
use crate::primitives::{
    SelectingItemsControl, SelectingItemsControlImpl, SelectingItemsControlImplExt, TemplateAppliedEventArgs,
    TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::templates::IDataTemplate;
use crate::{Control, ControlImpl, ItemsControlImpl, ItemsControlImplExt, ScrollViewer, SelectionMode};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyboardNavigation, KeyboardNavigationMode,
    PointerEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_markup_enum, ferro_markup_type, ferro_property, instantiate, BoxedValue,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Ref, StyledElementImpl,
    StyledProperty, Vector, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// How the items of a list view are selected (WinUI's `ListViewSelectionMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ListViewSelectionMode {
    /// The user cannot select items.
    None = 0,
    /// One item at a time (WinUI's default).
    #[default]
    Single = 1,
    /// Several items; a tap toggles an item.
    Multiple = 2,
    /// Several items with the modifier keys (Ctrl toggles, Shift selects a range).
    Extended = 3,
}

ferro_markup_enum!(ListViewSelectionMode { None, Single, Multiple, Extended }, { namespace: "FerroUI.Controls" });

impl ListViewSelectionMode {
    /// The selection flags of the base that give the mode (decision of section 6.2).
    pub fn selection_mode(self) -> SelectionMode {
        match self {
            ListViewSelectionMode::None | ListViewSelectionMode::Single => SelectionMode::SINGLE,
            ListViewSelectionMode::Multiple => SelectionMode::MULTIPLE | SelectionMode::TOGGLE,
            ListViewSelectionMode::Extended => SelectionMode::MULTIPLE,
        }
    }
}

/// Where `ScrollIntoView` puts the item (WinUI's `ScrollIntoViewAlignment`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ScrollIntoViewAlignment {
    /// The least scrolling that shows the item.
    #[default]
    Default = 0,
    /// The item at the start of the viewport.
    Leading = 1,
}

ferro_markup_enum!(ScrollIntoViewAlignment { Default, Leading }, { namespace: "FerroUI.Controls" });

/// How a list reorders its items (WinUI's `ListViewReorderMode`). Accepted; reordering is not
/// ported (section 6.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ListViewReorderMode {
    #[default]
    Disabled = 0,
    Enabled = 1,
}

ferro_markup_enum!(ListViewReorderMode { Disabled, Enabled }, { namespace: "FerroUI.Controls" });

/// When a list loads more items (WinUI's `IncrementalLoadingTrigger`). Accepted; incremental
/// loading is not ported (section 6.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum IncrementalLoadingTrigger {
    None = 0,
    #[default]
    Edge = 1,
}

ferro_markup_enum!(IncrementalLoadingTrigger { None, Edge }, { namespace: "FerroUI.Controls" });

/// A range of item indexes (WinUI's `ItemIndexRange`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ItemIndexRange {
    first_index: i32,
    length: u32,
}

impl ItemIndexRange {
    pub fn new(first_index: i32, length: u32) -> Self {
        Self { first_index, length }
    }

    pub fn first_index(&self) -> i32 {
        self.first_index
    }

    pub fn length(&self) -> u32 {
        self.length
    }

    /// The last index of the range (WinUI's `LastIndex`: `first_index + length - 1`).
    pub fn last_index(&self) -> i32 {
        self.first_index + self.length as i32 - 1
    }
}

/// The arguments of [`ListViewBase::item_click`] (WinUI's `ItemClickEventArgs`).
#[derive(Clone, Debug)]
pub struct ItemClickEventArgs {
    clicked_item: Option<BoxedValue>,
}

impl ItemClickEventArgs {
    /// The item that was clicked (the data item, or the container when the item is its own).
    pub fn clicked_item(&self) -> Option<BoxedValue> {
        self.clicked_item.clone()
    }
}

ferro_markup_type!(class ItemClickEventArgs {
    namespace: "FerroUI.Controls",
    handles: [ItemClickEventArgs],
    properties: [
        ClickedItem: Option<BoxedValue> { get: |args: &ItemClickEventArgs| args.clicked_item() },
    ],
});

/// The arguments of [`ListViewBase::container_content_changing`] (WinUI's
/// `ContainerContentChangingEventArgs`): a container gets an item (`in_recycle_queue` false) or
/// is cleared to be reused (`true`). FerroUI raises phase 0 only.
#[derive(Debug)]
pub struct ContainerContentChangingEventArgs {
    item_container: Ref<Control>,
    item: Option<BoxedValue>,
    item_index: i32,
    in_recycle_queue: bool,
    handled: Cell<bool>,
}

impl ContainerContentChangingEventArgs {
    pub fn item_container(&self) -> Ref<Control> {
        self.item_container.clone()
    }

    pub fn item(&self) -> Option<BoxedValue> {
        self.item.clone()
    }

    pub fn item_index(&self) -> i32 {
        self.item_index
    }

    pub fn in_recycle_queue(&self) -> bool {
        self.in_recycle_queue
    }

    /// The phase of the change: 0, the only one FerroUI raises (WinUI's later phases come from
    /// `RegisterUpdateCallback`, not ported).
    pub fn phase(&self) -> u32 {
        0
    }

    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }
}

/// The arguments of [`ListViewBase::choosing_item_container`] (WinUI's
/// `ChoosingItemContainerEventArgs`): a handler may give the container for the item.
#[derive(Debug)]
pub struct ChoosingItemContainerEventArgs {
    item: Option<BoxedValue>,
    item_index: i32,
    item_container: RefCell<Option<Ref<Control>>>,
}

impl ChoosingItemContainerEventArgs {
    pub fn item(&self) -> Option<BoxedValue> {
        self.item.clone()
    }

    pub fn item_index(&self) -> i32 {
        self.item_index
    }

    pub fn item_container(&self) -> Option<Ref<Control>> {
        self.item_container.borrow().clone()
    }

    pub fn set_item_container(&self, value: Option<Ref<Control>>) {
        *self.item_container.borrow_mut() = value;
    }
}

type ItemClickHandler = dyn Fn(&Ref<ListViewBase>, &ItemClickEventArgs);
type ContainerContentChangingHandler = dyn Fn(&Ref<ListViewBase>, &ContainerContentChangingEventArgs);
type ChoosingItemContainerHandler = dyn Fn(&Ref<ListViewBase>, &ChoosingItemContainerEventArgs);

/// The base of [`ListView`](super::ListView) and `GridView` (WinUI's `ListViewBase`): a selecting
/// items control with WinUI's selection modes, item clicks, header and footer, selected ranges
/// and container events.
///
/// [`selection_mode`](Self::selection_mode) is WinUI's [`ListViewSelectionMode`], a property of
/// its own that sets the selection flags of the base (`None` keeps the input from selecting).
/// With [`is_item_click_enabled`](Self::is_item_click_enabled) a tap or Enter on an item raises
/// [`item_click`](Self::item_click) (WinUI's `OnItemClick`); with `None` it is the only effect.
/// The template part is `ScrollViewer`.
#[repr(C)]
pub struct ListViewBase {
    base: SelectingItemsControl,
    scroll_viewer: RefCell<Option<Ref<ScrollViewer>>>,
    item_click: HandlerList<ItemClickHandler>,
    container_content_changing: HandlerList<ContainerContentChangingHandler>,
    choosing_item_container: HandlerList<ChoosingItemContainerHandler>,
}

ferro_class!(ListViewBase: SelectingItemsControl);
ferroui_base::ferro_class_info!(ListViewBase {
    // No markup constructor: WinUI's constructor is protected (the base of ListView and GridView).
    markup: {
        namespace: "FerroUI.Controls",
        attributes: [TemplatePart("ScrollViewer", type(Ref<ScrollViewer>))],
    },
});
ferro_impl_classes!(ListViewBase: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl);

ferroui_base::ferro_properties! { impl ListViewBase {
    ferro_property!(
        /// Defines the `SelectionMode` property (WinUI's `ListViewSelectionMode`).
        pub fn list_view_selection_mode_property() -> StyledProperty<ListViewSelectionMode> {
            FerroProperty::register::<ListViewBase, _>("SelectionMode", ListViewSelectionMode::Single)
        }
    );

    ferro_property!(
        /// Defines the `IsItemClickEnabled` property: whether a tap or Enter raises `ItemClick`.
        pub fn is_item_click_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<ListViewBase, _>("IsItemClickEnabled", false)
        }
    );

    ferro_property!(
        /// Defines the `Header` property: the content above the items.
        pub fn header_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ListViewBase, _>("Header", None)
        }
    );

    ferro_property!(
        /// Defines the `HeaderTemplate` property.
        pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<ListViewBase, _>("HeaderTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `Footer` property: the content below the items.
        pub fn footer_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ListViewBase, _>("Footer", None)
        }
    );

    ferro_property!(
        /// Defines the `FooterTemplate` property.
        pub fn footer_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<ListViewBase, _>("FooterTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `SingleSelectionFollowsFocus` property: with `Single`, moving the focus
        /// selects (WinUI's default `true`).
        pub fn single_selection_follows_focus_property() -> StyledProperty<bool> {
            FerroProperty::register::<ListViewBase, _>("SingleSelectionFollowsFocus", true)
        }
    );

    ferro_property!(
        /// Defines the `IsMultiSelectCheckBoxEnabled` property (the check boxes of `Multiple`).
        pub fn is_multi_select_check_box_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<ListViewBase, _>("IsMultiSelectCheckBoxEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `ShowsScrollingPlaceholders` property. Accepted; FerroUI shows no
        /// placeholders.
        pub fn shows_scrolling_placeholders_property() -> StyledProperty<bool> {
            FerroProperty::register::<ListViewBase, _>("ShowsScrollingPlaceholders", true)
        }
    );

    ferro_property!(
        /// Defines the `IsSwipeEnabled` property. A no-op, as in WinUI 3.
        pub fn is_swipe_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<ListViewBase, _>("IsSwipeEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `CanDragItems` property. Accepted; dragging is not ported.
        pub fn can_drag_items_property() -> StyledProperty<bool> {
            FerroProperty::register::<ListViewBase, _>("CanDragItems", false)
        }
    );

    ferro_property!(
        /// Defines the `CanReorderItems` property. Accepted; reordering is not ported.
        pub fn can_reorder_items_property() -> StyledProperty<bool> {
            FerroProperty::register::<ListViewBase, _>("CanReorderItems", false)
        }
    );

    ferro_property!(
        /// Defines the `ReorderMode` property. Accepted; reordering is not ported.
        pub fn reorder_mode_property() -> StyledProperty<ListViewReorderMode> {
            FerroProperty::register::<ListViewBase, _>("ReorderMode", ListViewReorderMode::Disabled)
        }
    );

    ferro_property!(
        /// Defines the `IncrementalLoadingTrigger` property. Accepted; incremental loading is not
        /// ported.
        pub fn incremental_loading_trigger_property() -> StyledProperty<IncrementalLoadingTrigger> {
            FerroProperty::register::<ListViewBase, _>("IncrementalLoadingTrigger", IncrementalLoadingTrigger::Edge)
        }
    );

    ferro_property!(
        /// Defines the `IncrementalLoadingThreshold` property. Accepted.
        pub fn incremental_loading_threshold_property() -> StyledProperty<f64> {
            FerroProperty::register::<ListViewBase, _>("IncrementalLoadingThreshold", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `DataFetchSize` property. Accepted.
        pub fn data_fetch_size_property() -> StyledProperty<f64> {
            FerroProperty::register::<ListViewBase, _>("DataFetchSize", 3.0)
        }
    );
} }

impl FerroObjectImpl for ListViewBase {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.apply_selection_mode(this.selection_mode());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::list_view_selection_mode_property().as_property() {
            let mode = change.get_new_value::<ListViewSelectionMode>();
            this.apply_selection_mode(mode);
            if mode == ListViewSelectionMode::None {
                // WinUI clears the selection when the mode becomes None.
                this.selection().clear();
            }
        }
    }
}

impl ControlImpl for ListViewBase {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ListViewBaseAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for ListViewBase {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        *this.scroll_viewer.borrow_mut() = e.name_scope().find_as::<ScrollViewer>("ScrollViewer");
    }
}

impl InputElementImpl for ListViewBase {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        // The arrow keys move the focus and, with Single and `SingleSelectionFollowsFocus`, the
        // selection (WinUI's `OnKeyDown`); without a selection mode they only move the focus.
        if !e.handled() {
            if let Some(direction) = e.key.to_navigation_direction(ferroui_base::input::KeyModifiers::NONE) {
                if direction.is_directional() {
                    let follows =
                        this.selection_mode() == ListViewSelectionMode::Single && this.single_selection_follows_focus();
                    if this.selection_mode() != ListViewSelectionMode::None && follows {
                        let moved = this.move_selection(direction, this.wrap_selection(), false);
                        e.set_handled(moved);
                    }
                }
            }
        }
        Self::parent_on_key_down(this, e);
    }
}

impl ItemsControlImpl for ListViewBase {
    fn create_container_for_item_override(
        this: &Self,
        item: &Option<BoxedValue>,
        index: i32,
        recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        if let Some(container) = this.choose_item_container(item, index) {
            return container;
        }
        Self::parent_create_container_for_item_override(this, item, index, recycle_key)
    }

    fn container_for_item_prepared_override(
        this: &Self,
        container: &Ref<Control>,
        item: &Option<BoxedValue>,
        index: i32,
    ) {
        Self::parent_container_for_item_prepared_override(this, container, item, index);
        this.raise_container_content_changing(container, item.clone(), index, false);
    }

    fn clear_container_for_item_override(this: &Self, container: &Ref<Control>) {
        this.raise_container_content_changing(container, None, -1, true);
        Self::parent_clear_container_for_item_override(this, container);
    }
}

impl SelectingItemsControlImpl for ListViewBase {
    /// WinUI's `OnItemPrimaryInteractionGesture` and `OnItemClick`: a tap or Enter raises
    /// `ItemClick` when enabled, then selects as the mode says.
    fn update_selection_from_event(this: &Self, container: &Ref<Control>, event_args: &dyn IRoutedEventArgs) -> bool {
        let args = event_args.as_routed_event_args();
        if args.handled() {
            return false;
        }
        let index = this.index_from_container(container);
        if index == -1 {
            return false;
        }
        let pointer = event_args.downcast_ref::<PointerEventArgs>();
        let key = event_args.downcast_ref::<KeyEventArgs>();
        let invoked = match (pointer, key) {
            (Some(pointer), _) => this.should_trigger_selection(container, pointer),
            (None, Some(key)) => {
                matches!(key.key, Key::Return | Key::Space) && this.should_trigger_selection_key(container, key)
            }
            _ => false,
        };
        if invoked && this.is_item_click_enabled() && key.is_none_or(|key| key.key == Key::Return) {
            this.raise_item_click(index, container);
        }
        if this.selection_mode() == ListViewSelectionMode::None {
            if invoked {
                args.set_handled(true);
            }
            return invoked;
        }
        // With item clicks enabled, a tap does not select in WinUI unless the mode selects on
        // tap (`Single`, `Multiple`, `Extended` still select).
        Self::parent_update_selection_from_event(this, container, event_args)
    }
}

impl ListViewBase {
    fn static_constructor() {
        KeyboardNavigation::tab_navigation_property()
            .override_default_value::<ListViewBase>(KeyboardNavigationMode::Once);
        InputElement::focusable_property().override_default_value::<ListViewBase>(false);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: SelectingItemsControl::construct(),
            scroll_viewer: RefCell::new(None),
            item_click: HandlerList::new(),
            container_content_changing: HandlerList::new(),
            choosing_item_container: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn apply_selection_mode(&self, mode: ListViewSelectionMode) {
        SelectingItemsControl::set_selection_mode(self, mode.selection_mode());
    }

    /// WinUI's `SelectionMode`.
    pub fn selection_mode(&self) -> ListViewSelectionMode {
        self.get_value(Self::list_view_selection_mode_property())
    }

    pub fn set_selection_mode(&self, value: ListViewSelectionMode) {
        self.set_value(Self::list_view_selection_mode_property(), value)
    }

    pub fn is_item_click_enabled(&self) -> bool {
        self.get_value(Self::is_item_click_enabled_property())
    }

    pub fn set_is_item_click_enabled(&self, value: bool) {
        self.set_value(Self::is_item_click_enabled_property(), value)
    }

    pub fn header(&self) -> Option<BoxedValue> {
        self.get_value(Self::header_property())
    }

    pub fn set_header(&self, value: Option<BoxedValue>) {
        self.set_value(Self::header_property(), value)
    }

    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }

    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value)
    }

    pub fn footer(&self) -> Option<BoxedValue> {
        self.get_value(Self::footer_property())
    }

    pub fn set_footer(&self, value: Option<BoxedValue>) {
        self.set_value(Self::footer_property(), value)
    }

    pub fn footer_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::footer_template_property())
    }

    pub fn set_footer_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::footer_template_property(), value)
    }

    pub fn single_selection_follows_focus(&self) -> bool {
        self.get_value(Self::single_selection_follows_focus_property())
    }

    pub fn set_single_selection_follows_focus(&self, value: bool) {
        self.set_value(Self::single_selection_follows_focus_property(), value)
    }

    pub fn is_multi_select_check_box_enabled(&self) -> bool {
        self.get_value(Self::is_multi_select_check_box_enabled_property())
    }

    pub fn set_is_multi_select_check_box_enabled(&self, value: bool) {
        self.set_value(Self::is_multi_select_check_box_enabled_property(), value)
    }

    pub fn shows_scrolling_placeholders(&self) -> bool {
        self.get_value(Self::shows_scrolling_placeholders_property())
    }

    pub fn set_shows_scrolling_placeholders(&self, value: bool) {
        self.set_value(Self::shows_scrolling_placeholders_property(), value)
    }

    pub fn is_swipe_enabled(&self) -> bool {
        self.get_value(Self::is_swipe_enabled_property())
    }

    pub fn set_is_swipe_enabled(&self, value: bool) {
        self.set_value(Self::is_swipe_enabled_property(), value)
    }

    pub fn can_drag_items(&self) -> bool {
        self.get_value(Self::can_drag_items_property())
    }

    pub fn set_can_drag_items(&self, value: bool) {
        self.set_value(Self::can_drag_items_property(), value)
    }

    pub fn can_reorder_items(&self) -> bool {
        self.get_value(Self::can_reorder_items_property())
    }

    pub fn set_can_reorder_items(&self, value: bool) {
        self.set_value(Self::can_reorder_items_property(), value)
    }

    pub fn reorder_mode(&self) -> ListViewReorderMode {
        self.get_value(Self::reorder_mode_property())
    }

    pub fn set_reorder_mode(&self, value: ListViewReorderMode) {
        self.set_value(Self::reorder_mode_property(), value)
    }

    pub fn incremental_loading_trigger(&self) -> IncrementalLoadingTrigger {
        self.get_value(Self::incremental_loading_trigger_property())
    }

    pub fn set_incremental_loading_trigger(&self, value: IncrementalLoadingTrigger) {
        self.set_value(Self::incremental_loading_trigger_property(), value)
    }

    pub fn incremental_loading_threshold(&self) -> f64 {
        self.get_value(Self::incremental_loading_threshold_property())
    }

    pub fn set_incremental_loading_threshold(&self, value: f64) {
        self.set_value(Self::incremental_loading_threshold_property(), value)
    }

    pub fn data_fetch_size(&self) -> f64 {
        self.get_value(Self::data_fetch_size_property())
    }

    pub fn set_data_fetch_size(&self, value: f64) {
        self.set_value(Self::data_fetch_size_property(), value)
    }

    /// The scroll viewer of the template.
    pub fn scroll_viewer(&self) -> Option<Ref<ScrollViewer>> {
        self.scroll_viewer.borrow().clone()
    }

    /// Adds a handler of `ItemClick`. Dropping the guard removes it.
    pub fn item_click(&self, handler: impl Fn(&Ref<ListViewBase>, &ItemClickEventArgs) + 'static) -> Subscription {
        let token = self.item_click.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.item_click.remove(token);
            }
        }))
    }

    /// Adds a handler of `ContainerContentChanging` (phase 0).
    pub fn container_content_changing(
        &self,
        handler: impl Fn(&Ref<ListViewBase>, &ContainerContentChangingEventArgs) + 'static,
    ) -> Subscription {
        let token = self.container_content_changing.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.container_content_changing.remove(token);
            }
        }))
    }

    /// Adds a handler of `ChoosingItemContainer`.
    pub fn choosing_item_container(
        &self,
        handler: impl Fn(&Ref<ListViewBase>, &ChoosingItemContainerEventArgs) + 'static,
    ) -> Subscription {
        let token = self.choosing_item_container.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.choosing_item_container.remove(token);
            }
        }))
    }

    /// WinUI's `ChoosingItemContainer`: the container a handler gives for `item`, if any.
    pub(crate) fn choose_item_container(&self, item: &Option<BoxedValue>, index: i32) -> Option<Ref<Control>> {
        if self.choosing_item_container.is_empty() {
            return None;
        }
        let args = ChoosingItemContainerEventArgs {
            item: item.clone(),
            item_index: index,
            item_container: RefCell::new(None),
        };
        let owner = self.to_ref();
        for (_, handler) in self.choosing_item_container.snapshot().iter() {
            handler(&owner, &args);
        }
        args.item_container()
    }

    /// WinUI's `OnItemClick`: the clicked item is the data item (or the container).
    fn raise_item_click(&self, index: i32, container: &Ref<Control>) {
        let clicked_item = self.items_view().get_at(index as usize).or_else(|| Some(Control::boxed(container.clone())));
        let args = ItemClickEventArgs { clicked_item };
        let owner = self.to_ref();
        for (_, handler) in self.item_click.snapshot().iter() {
            handler(&owner, &args);
        }
    }

    /// Clicks the item at `index` as a tap would (the automation Invoke of an item).
    pub fn invoke_item(&self, index: i32) {
        if let Some(container) = self.container_from_index(index) {
            if self.is_item_click_enabled() {
                self.raise_item_click(index, &container);
            }
        }
    }

    fn raise_container_content_changing(
        &self,
        container: &Ref<Control>,
        item: Option<BoxedValue>,
        index: i32,
        in_recycle_queue: bool,
    ) {
        if self.container_content_changing.is_empty() {
            return;
        }
        let args = ContainerContentChangingEventArgs {
            item_container: container.clone(),
            item,
            item_index: index,
            in_recycle_queue,
            handled: Cell::new(false),
        };
        let owner = self.to_ref();
        for (_, handler) in self.container_content_changing.snapshot().iter() {
            handler(&owner, &args);
        }
    }

    /// The selected items as ranges of indexes (WinUI's `SelectedRanges`), in order.
    pub fn selected_ranges(&self) -> Vec<ItemIndexRange> {
        let mut indexes: Vec<i32> = self.selection().selected_indexes().iter().collect();
        indexes.sort_unstable();
        indexes.dedup();
        let mut ranges: Vec<ItemIndexRange> = Vec::new();
        for index in indexes {
            match ranges.last_mut() {
                Some(range) if range.last_index() + 1 == index => range.length += 1,
                _ => ranges.push(ItemIndexRange::new(index, 1)),
            }
        }
        ranges
    }

    /// Selects every item (WinUI's `SelectAll`; with `Multiple` and `Extended`).
    pub fn select_all(&self) {
        if matches!(self.selection_mode(), ListViewSelectionMode::Multiple | ListViewSelectionMode::Extended) {
            self.selection().select_all();
        }
    }

    /// Selects a range (WinUI's `SelectRange`).
    pub fn select_range(&self, range: ItemIndexRange) {
        if range.length() > 0 {
            self.selection().select_range(range.first_index(), range.last_index());
        }
    }

    /// Deselects a range (WinUI's `DeselectRange`).
    pub fn deselect_range(&self, range: ItemIndexRange) {
        if range.length() > 0 {
            self.selection().deselect_range(range.first_index(), range.last_index());
        }
    }

    /// Scrolls `item` into view (WinUI's `ScrollIntoView(item, alignment)`): with `Leading` the
    /// item comes to the start of the viewport.
    pub fn scroll_into_view_with_alignment(&self, item: &Option<BoxedValue>, alignment: ScrollIntoViewAlignment) {
        let index = self.items_view().index_of(item);
        if index < 0 {
            return;
        }
        self.scroll_into_view(index);
        if alignment != ScrollIntoViewAlignment::Leading {
            return;
        }
        let (Some(scroll_viewer), Some(container), Some(panel)) =
            (self.scroll_viewer(), self.container_from_index(index), self.items_panel_root())
        else {
            return;
        };
        let Some(position) = container.translate_point(Point::new(0.0, 0.0), &panel) else { return };
        let offset = scroll_viewer.offset();
        let horizontal = matches!(
            panel.cast::<crate::VirtualizingStackPanel>().map(|panel| panel.orientation()),
            Some(ferroui_base::layout::Orientation::Horizontal)
        );
        let target = if horizontal { Vector::new(position.x, offset.y) } else { Vector::new(offset.x, position.y) };
        scroll_viewer.set_offset(target);
    }
}
