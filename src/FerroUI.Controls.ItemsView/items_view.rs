// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.

//! `ItemsView` of WinUI: a list or grid of items over an [`ItemsRepeater`] in a scroll viewer, with
//! selection modes, a current item and an event for invoked items.

use crate::item_container::{ItemContainer, ItemContainerInteractionTrigger};
use crate::selector;
use ferroui_controls_repeater::selection::{IndexPath, SelectionModel};
use ferroui_controls_repeater::{ItemCollectionTransitionProvider, ItemsRepeater, Layout};
use ferroui_base::input::{InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::Subscription;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Point, Ref, StyledElementImpl, StyledProperty,
    VisualImpl,
};
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::{Control, ControlImpl, ItemsSource, ScrollViewer};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// How the items of an [`ItemsView`] are selected (WinUI `ItemsViewSelectionMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ItemsViewSelectionMode {
    None = 0,
    #[default]
    Single = 1,
    Multiple = 2,
    Extended = 3,
}

/// The item an [`ItemsView`] reported as invoked.
#[derive(Clone)]
pub struct ItemsViewItemInvokedEventArgs {
    pub invoked_item: Option<BoxedValue>,
}

/// The selection of an [`ItemsView`] changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemsViewSelectionChangedEventArgs;

/// A list or grid of items (WinUI `ItemsView`). Its template (`Themes/WinUI/ItemsView.xaml`) holds a
/// [`ScrollViewer`], WinUI's `ScrollView`, with the [`ItemsRepeater`].
#[repr(C)]
pub struct ItemsView {
    base: TemplatedControl,
    scroll_viewer: RefCell<Option<Ref<ScrollViewer>>>,
    repeater: RefCell<Option<Ref<ItemsRepeater>>>,
    selection_model: Rc<SelectionModel>,
    current_item_index: Cell<i32>,
    syncing_selection: Cell<bool>,
    item_invoked: HandlerList<dyn Fn(&ItemsViewItemInvokedEventArgs)>,
    selection_changed: HandlerList<dyn Fn(&ItemsViewSelectionChangedEventArgs)>,
    repeater_subscriptions: RefCell<Vec<Subscription>>,
    selection_subscription: RefCell<Option<Subscription>>,
}

ferro_class!(ItemsView: TemplatedControl);
ferro_class_info!(ItemsView {
    new: ItemsView::new,
    markup: {
        namespace: "FerroUI.Controls",
        property_attributes: [ItemTemplate: [InheritDataTypeFromItems("ItemsSource")]],
    },
});
ferro_impl_classes!(ItemsView: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, ControlImpl);

ferro_properties! { impl ItemsView {
    pub fn items_source_property() -> StyledProperty<Option<ItemsSource>> {
        FerroProperty::register::<Self, _>("ItemsSource", None)
    }
    pub fn item_template_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<Self, _>("ItemTemplate", None)
    }
    pub fn layout_property() -> StyledProperty<Option<Ref<Layout>>> {
        FerroProperty::register::<Self, _>("Layout", None)
    }
    pub fn is_item_invoked_enabled_property() -> StyledProperty<bool> {
        FerroProperty::register::<Self, _>("IsItemInvokedEnabled", false)
    }
    pub fn selection_mode_property() -> StyledProperty<ItemsViewSelectionMode> {
        FerroProperty::register::<Self, _>("SelectionMode", ItemsViewSelectionMode::Single)
    }
    pub fn item_transition_provider_property() -> StyledProperty<Option<Ref<ItemCollectionTransitionProvider>>> {
        FerroProperty::register::<Self, _>("ItemTransitionProvider", None)
    }
} }

impl FerroObjectImpl for ItemsView {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_single_select();
        let weak = this.to_ref().downgrade();
        *this.selection_subscription.borrow_mut() = Some(Subscription::from(this.selection_model.selection_changed(move |_| {
            if let Some(view) = weak.upgrade() {
                view.on_model_selection_changed();
            }
        })));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == Self::items_source_property().as_property() {
            this.current_item_index.set(-1);
            this.selection_model.set_source(this.items_source());
        } else if property == Self::selection_mode_property().as_property() {
            this.update_single_select();
            this.update_containers();
        }
    }
}

impl TemplatedControlImpl for ItemsView {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.repeater_subscriptions.borrow_mut().clear();
        *this.scroll_viewer.borrow_mut() = e.name_scope().find_as::<ScrollViewer>("PART_ScrollView");
        let repeater = e.name_scope().find_as::<ItemsRepeater>("PART_ItemsRepeater");
        if let Some(repeater) = &repeater {
            let mut subscriptions = Vec::new();
            let weak = this.to_ref().downgrade();
            subscriptions.push(Subscription::from(repeater.element_prepared(move |args| {
                if let Some(view) = weak.upgrade() {
                    view.on_element_prepared(&args.element, args.index);
                }
            })));
            let weak = this.to_ref().downgrade();
            subscriptions.push(Subscription::from(repeater.element_clearing(move |args| {
                if let Some(view) = weak.upgrade() {
                    view.on_element_clearing(&args.element);
                }
            })));
            let weak = this.to_ref().downgrade();
            subscriptions.push(Subscription::from(repeater.element_index_changed(move |args| {
                if let Some(view) = weak.upgrade() {
                    view.on_element_index_changed(&args.element, args.new_index);
                }
            })));
            *this.repeater_subscriptions.borrow_mut() = subscriptions;
        }
        *this.repeater.borrow_mut() = repeater;
        Self::parent_on_apply_template(this, e);
    }
}

impl InputElementImpl for ItemsView {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if e.handled() {
            return;
        }
        let ctrl = e.key_modifiers.contains(KeyModifiers::CONTROL) || e.key_modifiers.contains(KeyModifiers::META);
        if e.key == Key::A && ctrl && matches!(this.selection_mode(), ItemsViewSelectionMode::Multiple | ItemsViewSelectionMode::Extended) {
            this.select_all();
            e.set_handled(true);
            return;
        }
        let target = match e.key {
            Key::Home => Some(0),
            Key::End => Some(this.item_count() - 1),
            Key::Left | Key::Right | Key::Up | Key::Down => this.index_in_direction(e.key),
            _ => return,
        };
        let Some(target) = target.filter(|target| *target >= 0 && *target < this.item_count()) else { return };
        if this.move_current_to(target, e.key_modifiers) {
            e.set_handled(true);
        }
    }
}

impl ItemsView {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            scroll_viewer: RefCell::new(None),
            repeater: RefCell::new(None),
            selection_model: SelectionModel::new(),
            current_item_index: Cell::new(-1),
            syncing_selection: Cell::new(false),
            item_invoked: HandlerList::new(),
            selection_changed: HandlerList::new(),
            repeater_subscriptions: RefCell::new(Vec::new()),
            selection_subscription: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn items_source(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_source_property())
    }

    pub fn set_items_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::items_source_property(), value);
    }

    pub fn item_template(&self) -> Option<BoxedValue> {
        self.get_value(Self::item_template_property())
    }

    pub fn set_item_template(&self, value: Option<BoxedValue>) {
        self.set_value(Self::item_template_property(), value);
    }

    pub fn layout(&self) -> Option<Ref<Layout>> {
        self.get_value(Self::layout_property())
    }

    pub fn set_layout(&self, value: Option<Ref<Layout>>) {
        self.set_value(Self::layout_property(), value);
    }

    pub fn is_item_invoked_enabled(&self) -> bool {
        self.get_value(Self::is_item_invoked_enabled_property())
    }

    pub fn set_is_item_invoked_enabled(&self, value: bool) {
        self.set_value(Self::is_item_invoked_enabled_property(), value);
    }

    pub fn selection_mode(&self) -> ItemsViewSelectionMode {
        self.get_value(Self::selection_mode_property())
    }

    pub fn set_selection_mode(&self, value: ItemsViewSelectionMode) {
        self.set_value(Self::selection_mode_property(), value);
    }

    pub fn item_transition_provider(&self) -> Option<Ref<ItemCollectionTransitionProvider>> {
        self.get_value(Self::item_transition_provider_property())
    }

    pub fn set_item_transition_provider(&self, value: Option<Ref<ItemCollectionTransitionProvider>>) {
        self.set_value(Self::item_transition_provider_property(), value);
    }

    /// `ScrollView`: the scroll viewer of the template, once it is applied. WinUI's is a
    /// `ScrollView`.
    pub fn scroll_view(&self) -> Option<Ref<ScrollViewer>> {
        self.scroll_viewer.borrow().clone()
    }

    pub(crate) fn items_repeater(&self) -> Option<Ref<ItemsRepeater>> {
        self.repeater.borrow().clone()
    }

    /// The index of the current item, -1 when there is none.
    pub fn current_item_index(&self) -> i32 {
        self.current_item_index.get()
    }

    /// The first selected item.
    pub fn selected_item(&self) -> Option<BoxedValue> {
        self.selection_model.selected_item()
    }

    /// The selected items.
    pub fn selected_items(&self) -> Vec<Option<BoxedValue>> {
        self.selection_model.selected_items().to_vec().unwrap_or_default()
    }

    pub fn select(&self, item_index: i32) {
        if self.selection_mode() != ItemsViewSelectionMode::None {
            let _ = self.selection_model.select(item_index);
        }
    }

    pub fn deselect(&self, item_index: i32) {
        let _ = self.selection_model.deselect(item_index);
    }

    pub fn is_selected(&self, item_index: i32) -> bool {
        self.selection_model.is_selected(item_index).unwrap_or(false)
    }

    pub fn select_all(&self) {
        if matches!(self.selection_mode(), ItemsViewSelectionMode::Multiple | ItemsViewSelectionMode::Extended) {
            let _ = self.selection_model.select_all();
        }
    }

    pub fn deselect_all(&self) {
        self.selection_model.clear_selection();
    }

    /// Selects the items that are not selected and deselects the others.
    pub fn invert_selection(&self) {
        if !matches!(self.selection_mode(), ItemsViewSelectionMode::Multiple | ItemsViewSelectionMode::Extended) {
            return;
        }
        for index in 0..self.item_count() {
            let _ = if self.is_selected(index) {
                self.selection_model.deselect(index)
            } else {
                self.selection_model.select(index)
            };
        }
    }

    /// `ItemInvoked`.
    pub fn item_invoked(&self, handler: impl Fn(&ItemsViewItemInvokedEventArgs) + 'static) -> Subscription {
        let token = self.item_invoked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(ferroui_base::reactive::Disposable::create(move || {
            if let Some(view) = weak.upgrade() {
                view.item_invoked.remove(token);
            }
        }))
    }

    /// `SelectionChanged`.
    pub fn selection_changed(&self, handler: impl Fn(&ItemsViewSelectionChangedEventArgs) + 'static) -> Subscription {
        let token = self.selection_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(ferroui_base::reactive::Disposable::create(move || {
            if let Some(view) = weak.upgrade() {
                view.selection_changed.remove(token);
            }
        }))
    }

    fn update_single_select(&self) {
        let mode = self.selection_mode();
        self.selection_model.set_single_select(false);
        match mode {
            ItemsViewSelectionMode::None => self.selection_model.clear_selection(),
            ItemsViewSelectionMode::Single => self.selection_model.set_single_select(true),
            ItemsViewSelectionMode::Multiple | ItemsViewSelectionMode::Extended => {}
        }
    }

    fn item_count(&self) -> i32 {
        self.items_repeater()
            .and_then(|repeater| repeater.items_source_view())
            .map_or(0, |view| view.count() as i32)
    }

    fn element_index(&self, container: &ItemContainer) -> i32 {
        let Some(repeater) = self.items_repeater() else { return -1 };
        repeater.get_element_index(&container.to_ref().upcast())
    }

    fn on_element_prepared(&self, element: &Ref<Control>, index: i32) {
        if let Some(container) = element.cast::<ItemContainer>() {
            container.set_owner(Some(self));
            container.set_is_multiple(self.selection_mode() == ItemsViewSelectionMode::Multiple);
            self.sync_container(&container, index);
        }
    }

    fn on_element_clearing(&self, element: &Ref<Control>) {
        if let Some(container) = element.cast::<ItemContainer>() {
            container.set_owner(None);
        }
    }

    fn on_element_index_changed(&self, element: &Ref<Control>, index: i32) {
        if let Some(container) = element.cast::<ItemContainer>() {
            self.sync_container(&container, index);
        }
    }

    fn sync_container(&self, container: &ItemContainer, index: i32) {
        let was_syncing = self.syncing_selection.replace(true);
        container.set_is_selected(self.is_selected(index));
        self.syncing_selection.set(was_syncing);
    }

    /// Brings the realized containers up to the selection and the selection mode.
    fn update_containers(&self) {
        let Some(repeater) = self.items_repeater() else { return };
        let multiple = self.selection_mode() == ItemsViewSelectionMode::Multiple;
        for element in repeater.realized_elements() {
            if let Some(container) = element.cast::<ItemContainer>() {
                container.set_is_multiple(multiple);
                let index = repeater.get_element_index(&element);
                if index >= 0 {
                    self.sync_container(&container, index);
                }
            }
        }
    }

    fn on_model_selection_changed(&self) {
        self.update_containers();
        for (_, handler) in self.selection_changed.snapshot().iter() {
            handler(&ItemsViewSelectionChangedEventArgs);
        }
    }

    pub(crate) fn on_container_selected_changed(&self, container: &ItemContainer, selected: bool) {
        if self.syncing_selection.get() {
            return;
        }
        let index = self.element_index(container);
        if index < 0 {
            return;
        }
        if selected {
            self.select(index);
            // A view without selection keeps the container unselected.
            if !self.is_selected(index) {
                self.sync_container(container, index);
            }
        } else {
            self.deselect(index);
        }
    }

    pub(crate) fn on_container_interaction(
        &self,
        container: &ItemContainer,
        trigger: ItemContainerInteractionTrigger,
        modifiers: KeyModifiers,
    ) {
        use ItemContainerInteractionTrigger::*;
        let index = self.element_index(container);
        if index < 0 {
            return;
        }
        if matches!(trigger, PointerReleased | EnterKey | SpaceKey) {
            self.current_item_index.set(index);
            let ctrl = modifiers.contains(KeyModifiers::CONTROL) || modifiers.contains(KeyModifiers::META);
            let shift = modifiers.contains(KeyModifiers::SHIFT);
            selector::on_interacted(self.selection_mode(), &self.selection_model, &IndexPath::create_from(index), ctrl, shift);
        }
        if trigger != PointerReleased && self.can_raise_item_invoked(trigger) {
            let item = self.items_repeater().and_then(|repeater| repeater.items_source_view()).and_then(|view| view.get_at(index as usize));
            let args = ItemsViewItemInvokedEventArgs { invoked_item: item };
            for (_, handler) in self.item_invoked.snapshot().iter() {
                handler(&args);
            }
        }
    }

    /// `CanRaiseItemInvoked`.
    fn can_raise_item_invoked(&self, trigger: ItemContainerInteractionTrigger) -> bool {
        use ItemContainerInteractionTrigger::*;
        let no_selection = self.selection_mode() == ItemsViewSelectionMode::None;
        self.is_item_invoked_enabled()
            && !(no_selection && trigger == DoubleTap)
            && !(!no_selection && matches!(trigger, Tap | SpaceKey))
    }

    /// The realized item nearest to the current one in the direction of an arrow key.
    fn index_in_direction(&self, key: Key) -> Option<i32> {
        let repeater = self.items_repeater()?;
        let current = self.current_item_index.get();
        if current < 0 {
            return Some(0);
        }
        let from = repeater.try_get_element(current)?;
        let center = |element: &Ref<Control>| {
            let bounds = element.bounds();
            Point::new(bounds.x + bounds.width / 2.0, bounds.y + bounds.height / 2.0)
        };
        let origin = center(&from);
        let mut best: Option<(f64, i32)> = None;
        for element in repeater.realized_elements() {
            let index = repeater.get_element_index(&element);
            if index < 0 || index == current {
                continue;
            }
            let point = center(&element);
            let (along, across) = match key {
                Key::Left => (origin.x - point.x, point.y - origin.y),
                Key::Right => (point.x - origin.x, point.y - origin.y),
                Key::Up => (origin.y - point.y, point.x - origin.x),
                _ => (point.y - origin.y, point.x - origin.x),
            };
            if along <= 0.5 {
                continue;
            }
            // Items in the same row or column come first.
            let score = along + across.abs() * 4.0;
            if best.is_none_or(|(known, _)| score < known) {
                best = Some((score, index));
            }
        }
        best.map(|(_, index)| index)
    }

    fn move_current_to(&self, index: i32, modifiers: KeyModifiers) -> bool {
        let Some(repeater) = self.items_repeater() else { return false };
        let element = repeater.get_or_create_element(index);
        element.bring_into_view();
        element.focus();
        self.current_item_index.set(index);
        let ctrl = modifiers.contains(KeyModifiers::CONTROL) || modifiers.contains(KeyModifiers::META);
        let shift = modifiers.contains(KeyModifiers::SHIFT);
        selector::on_focused(self.selection_mode(), &self.selection_model, &IndexPath::create_from(index), ctrl, shift);
        true
    }
}
