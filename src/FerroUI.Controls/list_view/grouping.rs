//! WinUI's grouped data of a list view (`CollectionViewSource`, `ICollectionViewGroup`,
//! `GroupStyle`, `dxaml/xcp/dxaml/lib/CollectionViewSource_Partial.cpp`,
//! `GroupedDataCollectionView_Partial.cpp` and `GroupStyle_Partial.cpp` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).
//!
//! The view is a flat list of the items of all groups, so an index of the list view is an index of
//! an item, as in WinUI: the headers are not items, cannot be selected and take no index. The
//! panel asks the view which group an index is in ([`GroupedItemsView::group_of`]).

use crate::items_source::{IItemsList, ItemsChangedEventArgs, ItemsChangedHandler};
use crate::templates::IDataTemplate;
use crate::utils::BindingEvaluator;
use crate::{AssignedBinding, ItemsSource};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::ReflectionBinding;
use ferroui_base::styling::ControlTheme;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// One group of a grouped source (WinUI's `ICollectionViewGroup`).
pub struct CollectionViewGroup {
    group: Option<BoxedValue>,
    group_items: ItemsSource,
}

impl CollectionViewGroup {
    /// A group of `group_items`, whose header shows `group`.
    pub fn new(group: Option<BoxedValue>, group_items: ItemsSource) -> Rc<Self> {
        Rc::new(Self { group, group_items })
    }

    /// The object the header of the group shows (WinUI's `Group`).
    pub fn group(&self) -> Option<BoxedValue> {
        self.group.clone()
    }

    /// The items of the group (WinUI's `GroupItems`).
    pub fn group_items(&self) -> &ItemsSource {
        &self.group_items
    }
}

// The group as the data of a template (WinUI's `x:Bind Group` of an `ICollectionViewGroup`, the
// items of the zoomed-out view of a semantic zoom); registered by `CollectionViewSource`.
ferroui_base::ferro_markup_type!(class CollectionViewGroup {
    this: Rc<CollectionViewGroup>,
    handles: [CollectionViewGroup, Rc<CollectionViewGroup>, Option<Rc<CollectionViewGroup>>],
    properties: [
        Group: Option<BoxedValue> { get: |this: &Rc<CollectionViewGroup>| this.group() },
    ],
});

/// A group is itself: two groups are equal when they are the same group.
impl PartialEq for CollectionViewGroup {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The items of all groups as one list (WinUI's `ICollectionView` of a grouped source).
pub struct GroupedItemsView {
    this: Weak<GroupedItemsView>,
    source: Option<ItemsSource>,
    /// The property path of the items of each group of `source` (WinUI's `ItemsPath`).
    items_path: Option<Rc<str>>,
    groups: RefCell<Vec<Rc<CollectionViewGroup>>>,
    /// The flat index of the first item of each group, and the count of all items at the end.
    starts: RefCell<Vec<usize>>,
    handlers: RefCell<Vec<(u64, Rc<ItemsChangedHandler>)>>,
    next_handler: Cell<u64>,
    subscriptions: RefCell<Vec<(ItemsSource, u64)>>,
    source_subscription: Cell<Option<u64>>,
}

impl GroupedItemsView {
    /// A view of `groups`, which does not change.
    pub fn new(groups: Vec<Rc<CollectionViewGroup>>) -> Rc<Self> {
        Self::create(None, None, groups)
    }

    /// A view of the groups of `source`: each item is a [`CollectionViewGroup`], or a list that is
    /// both the header and the items of its group. The view follows the changes of `source`.
    pub fn from_source(source: ItemsSource) -> Rc<Self> {
        let groups = Self::read_groups(&source, None);
        Self::create(Some(source), None, groups)
    }

    /// A view of the groups of `source` whose items are the value of the property path
    /// `items_path` of each group (WinUI's `CollectionViewSource.ItemsPath`); the group is the
    /// header. An empty path reads the groups as [`from_source`](Self::from_source) does.
    pub fn from_source_with_items_path(source: ItemsSource, items_path: &str) -> Rc<Self> {
        let items_path: Option<Rc<str>> = (!items_path.is_empty()).then(|| items_path.into());
        let groups = Self::read_groups(&source, items_path.as_deref());
        Self::create(Some(source), items_path, groups)
    }

    fn create(
        source: Option<ItemsSource>,
        items_path: Option<Rc<str>>,
        groups: Vec<Rc<CollectionViewGroup>>,
    ) -> Rc<Self> {
        let view = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            source,
            items_path,
            groups: RefCell::new(groups),
            starts: RefCell::new(Vec::new()),
            handlers: RefCell::new(Vec::new()),
            next_handler: Cell::new(0),
            subscriptions: RefCell::new(Vec::new()),
            source_subscription: Cell::new(None),
        });
        view.update_starts();
        view
    }

    fn read_groups(source: &ItemsSource, items_path: Option<&str>) -> Vec<Rc<CollectionViewGroup>> {
        let Some(path) = items_path else {
            return source.iter().filter_map(|item| Self::group_of_item(item)).collect();
        };
        // One evaluator of the path for all groups.
        let evaluator = BindingEvaluator::new();
        evaluator.update_binding(&AssignedBinding::new(ReflectionBinding::new(path)));
        let groups = source.iter().filter_map(|item| Self::group_at_path(&evaluator, item)).collect();
        evaluator.dispose();
        groups
    }

    /// The group of `item` whose items are the value of the path of `evaluator`; a group without
    /// a list at the path has no items.
    fn group_at_path(evaluator: &BindingEvaluator, item: Option<BoxedValue>) -> Option<Rc<CollectionViewGroup>> {
        let value = item?;
        let items = evaluator.evaluate(&Some(value.clone())).and_then(|items| Self::items_of_value(&items));
        Some(CollectionViewGroup::new(Some(value), items.unwrap_or_else(|| ItemsSource::from_items([]))))
    }

    /// The list `value` holds, if any.
    fn items_of_value(value: &BoxedValue) -> Option<ItemsSource> {
        if let Some(items) = value.downcast_ref::<ItemsSource>() {
            return Some(items.clone());
        }
        if let Some(items) = value.downcast_ref::<Option<ItemsSource>>() {
            return items.clone();
        }
        let converted = ValueTypes::try_convert_registered(value, ValueType::of::<ItemsSource>())?;
        converted.downcast_ref::<ItemsSource>().cloned()
    }

    fn group_of_item(item: Option<BoxedValue>) -> Option<Rc<CollectionViewGroup>> {
        let value = item?;
        if let Some(group) = value.downcast_ref::<Rc<CollectionViewGroup>>() {
            return Some(group.clone());
        }
        let converted = ValueTypes::try_convert_registered(&value, ValueType::of::<ItemsSource>())?;
        let items = converted.downcast_ref::<ItemsSource>()?.clone();
        Some(CollectionViewGroup::new(Some(value), items))
    }

    fn update_starts(&self) {
        let mut starts = self.starts.borrow_mut();
        starts.clear();
        let mut total = 0;
        for group in self.groups.borrow().iter() {
            starts.push(total);
            total += group.group_items().count();
        }
        starts.push(total);
    }

    /// The number of groups.
    pub fn group_count(&self) -> usize {
        self.groups.borrow().len()
    }

    /// The group at `index` (WinUI's `CollectionGroups`).
    pub fn group_at(&self, index: usize) -> Option<Rc<CollectionViewGroup>> {
        self.groups.borrow().get(index).cloned()
    }

    /// The flat index of the first item of the group at `index`.
    pub fn group_start(&self, index: usize) -> Option<usize> {
        let starts = self.starts.borrow();
        (index + 1 < starts.len()).then(|| starts[index])
    }

    /// The number of items of the group at `index`.
    pub fn group_len(&self, index: usize) -> usize {
        let starts = self.starts.borrow();
        if index + 1 < starts.len() {
            starts[index + 1] - starts[index]
        } else {
            0
        }
    }

    /// The group of the flat index `index`, and the position of the item in it.
    pub fn group_of(&self, index: usize) -> Option<(usize, usize)> {
        let starts = self.starts.borrow();
        if index >= *starts.last()? {
            return None;
        }
        // The last start at or before `index`: an empty group has the start of the one after it.
        let group = starts.partition_point(|start| *start <= index) - 1;
        Some((group, index - starts[group]))
    }

    fn raise(&self, e: &ItemsChangedEventArgs<'_>) {
        let handlers: Vec<_> = self.handlers.borrow().iter().map(|(_, handler)| handler.clone()).collect();
        for handler in handlers {
            handler(e);
        }
    }

    /// Follows the groups and their items while someone listens to the view.
    fn subscribe(&self) {
        if let Some(source) = &self.source {
            if self.source_subscription.get().is_none() {
                let this = self.this.clone();
                let token = source.list().add_collection_changed(Rc::new(move |_: &ItemsChangedEventArgs<'_>| {
                    if let Some(this) = this.upgrade() {
                        this.reload();
                    }
                }));
                self.source_subscription.set(token);
            }
        }
        let groups = self.groups.borrow().clone();
        let mut subscriptions = self.subscriptions.borrow_mut();
        for (position, group) in groups.iter().enumerate() {
            let this = self.this.clone();
            let items = group.group_items().clone();
            let handler = Rc::new(move |e: &ItemsChangedEventArgs<'_>| {
                if let Some(this) = this.upgrade() {
                    this.group_changed(position, e);
                }
            });
            if let Some(token) = items.list().add_collection_changed(handler) {
                subscriptions.push((items, token));
            }
        }
    }

    fn unsubscribe(&self) {
        for (items, token) in self.subscriptions.borrow_mut().drain(..) {
            items.list().remove_collection_changed(token);
        }
        if let (Some(source), Some(token)) = (&self.source, self.source_subscription.take()) {
            source.list().remove_collection_changed(token);
        }
    }

    fn reload(&self) {
        let Some(source) = &self.source else { return };
        let groups = Self::read_groups(source, self.items_path.as_deref());
        for (items, token) in self.subscriptions.borrow_mut().drain(..) {
            items.list().remove_collection_changed(token);
        }
        *self.groups.borrow_mut() = groups;
        self.update_starts();
        if !self.handlers.borrow().is_empty() {
            self.subscribe();
        }
        self.raise(&ItemsChangedEventArgs::RESET);
    }

    /// A change of the items of one group, at the flat indexes of the view.
    fn group_changed(&self, group: usize, e: &ItemsChangedEventArgs<'_>) {
        let offset = self.group_start(group).unwrap_or(0) as i32;
        self.update_starts();
        let shift = |index: i32| if index < 0 { index } else { index + offset };
        self.raise(&ItemsChangedEventArgs {
            action: e.action,
            new_items: e.new_items,
            old_items: e.old_items,
            new_starting_index: shift(e.new_starting_index),
            old_starting_index: shift(e.old_starting_index),
        });
    }
}

impl Drop for GroupedItemsView {
    fn drop(&mut self) {
        self.unsubscribe();
    }
}

impl IItemsList for GroupedItemsView {
    fn count(&self) -> usize {
        self.starts.borrow().last().copied().unwrap_or(0)
    }

    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        let (group, position) = self.group_of(index)?;
        self.groups.borrow()[group].group_items().get_at(position)
    }

    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        let groups = self.groups.borrow();
        for (position, group) in groups.iter().enumerate() {
            let found = group.group_items().index_of(item);
            if found >= 0 {
                return self.starts.borrow()[position] as i32 + found;
            }
        }
        -1
    }

    fn is_notifying(&self) -> bool {
        true
    }

    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        if self.handlers.borrow().is_empty() {
            self.subscribe();
        }
        let token = self.next_handler.get();
        self.next_handler.set(token + 1);
        self.handlers.borrow_mut().push((token, handler));
        Some(token)
    }

    fn remove_collection_changed(&self, token: u64) {
        self.handlers.borrow_mut().retain(|(known, _)| *known != token);
        if self.handlers.borrow().is_empty() {
            self.unsubscribe();
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The source of a grouped list (WinUI's `CollectionViewSource`): with `IsSourceGrouped`, its
/// [`view`](Self::view) is a [`GroupedItemsView`] of the groups of `Source`, whose items are the
/// value of `ItemsPath` of each group when it is set; without, the source.
#[repr(C)]
pub struct CollectionViewSource {
    base: FerroObject,
    view: RefCell<Option<ItemsSource>>,
}

ferro_class!(CollectionViewSource: FerroObject);
ferroui_base::ferro_class_info!(CollectionViewSource {
    new: CollectionViewSource::new,
    markup: { namespace: "FerroUI.Controls" },
});

ferroui_base::ferro_properties! { impl CollectionViewSource {
    ferro_property!(
        /// Defines the `Source` property.
        pub fn source_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<CollectionViewSource, _>("Source", None)
        }
    );

    ferro_property!(
        /// Defines the `IsSourceGrouped` property.
        pub fn is_source_grouped_property() -> StyledProperty<bool> {
            FerroProperty::register::<CollectionViewSource, _>("IsSourceGrouped", false)
        }
    );

    ferro_property!(
        /// Defines the `ItemsPath` property: the property path of the items of each group of a
        /// grouped source (WinUI's `PropertyPath`, as a path string); empty, each group is its
        /// own list of items.
        pub fn items_path_property() -> StyledProperty<String> {
            FerroProperty::register::<CollectionViewSource, _>("ItemsPath", String::new())
        }
    );
} }

impl FerroObjectImpl for CollectionViewSource {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == Self::source_property().as_property()
            || change.property() == Self::is_source_grouped_property().as_property()
            || change.property() == Self::items_path_property().as_property()
        {
            *this.view.borrow_mut() = None;
        }
    }
}

impl CollectionViewSource {
    fn static_constructor() {
        // The groups of `collection_groups` as the data of templates (their markup type), with the
        // class rather than in the list of every application.
        ferroui_base::metadata::MarkupType::register_all(&[
            <CollectionViewGroup as ferroui_base::metadata::MarkupTyped>::MARKUP,
        ]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), view: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn source(&self) -> Option<ItemsSource> {
        self.get_value(Self::source_property())
    }

    pub fn set_source(&self, value: Option<ItemsSource>) {
        self.set_value(Self::source_property(), value)
    }

    pub fn is_source_grouped(&self) -> bool {
        self.get_value(Self::is_source_grouped_property())
    }

    pub fn set_is_source_grouped(&self, value: bool) {
        self.set_value(Self::is_source_grouped_property(), value)
    }

    pub fn items_path(&self) -> String {
        self.get_value(Self::items_path_property())
    }

    pub fn set_items_path(&self, value: impl Into<String>) {
        self.set_value(Self::items_path_property(), value.into())
    }

    /// The items a list view shows (WinUI's `View`).
    pub fn view(&self) -> Option<ItemsSource> {
        if let Some(view) = self.view.borrow().as_ref() {
            return Some(view.clone());
        }
        let source = self.source()?;
        let view = match self.is_source_grouped() {
            true => ItemsSource::new(GroupedItemsView::from_source_with_items_path(source, &self.items_path())),
            false => source,
        };
        *self.view.borrow_mut() = Some(view.clone());
        Some(view)
    }

    /// The groups of the view (WinUI's `View.CollectionGroups`), without a grouped source none.
    pub fn collection_groups(&self) -> Vec<Rc<CollectionViewGroup>> {
        let Some(view) = self.view() else { return Vec::new() };
        let Some(grouped) = view.downcast_ref::<GroupedItemsView>() else { return Vec::new() };
        (0..grouped.group_count()).filter_map(|index| grouped.group_at(index)).collect()
    }
}

/// How the headers of the groups of a list view look (WinUI's `GroupStyle`).
#[repr(C)]
pub struct GroupStyle {
    base: FerroObject,
}

ferro_class!(GroupStyle: FerroObject);
ferroui_base::ferro_class_info!(GroupStyle {
    new: GroupStyle::new,
    markup: { namespace: "FerroUI.Controls" },
});
ferro_impl_classes!(GroupStyle: FerroObjectImpl);

ferroui_base::ferro_properties! { impl GroupStyle {
    ferro_property!(
        /// Defines the `HeaderTemplate` property.
        pub fn header_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<GroupStyle, _>("HeaderTemplate", None)
        }
    );

    ferro_property!(
        /// Defines the `HeaderContainerStyle` property: the control theme of the header.
        pub fn header_container_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<GroupStyle, _>("HeaderContainerStyle", None)
        }
    );

    ferro_property!(
        /// Defines the `HidesIfEmpty` property.
        pub fn hides_if_empty_property() -> StyledProperty<bool> {
            FerroProperty::register::<GroupStyle, _>("HidesIfEmpty", false)
        }
    );
} }

impl GroupStyle {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn header_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::header_template_property())
    }

    pub fn set_header_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::header_template_property(), value)
    }

    pub fn header_container_style(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::header_container_style_property())
    }

    pub fn set_header_container_style(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::header_container_style_property(), value)
    }

    pub fn hides_if_empty(&self) -> bool {
        self.get_value(Self::hides_if_empty_property())
    }

    pub fn set_hides_if_empty(&self, value: bool) {
        self.set_value(Self::hides_if_empty_property(), value)
    }
}
