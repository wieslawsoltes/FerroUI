//! `AutoRefreshFixture` (`DynamicData.Tests/Cache/AutoRefreshFixture.cs`): the types shared by the
//! tests of `AutoRefresh` (`auto_refresh_fixture_base`, `auto_refresh_fixture_with_property_accessor`
//! and `auto_refresh_fixture_without_property_accessor`).

use ferroui_dynamic_data::binding::AbstractNotifyPropertyChanged;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

/// When a test notifies the source: before the subscription of the operator (`Immediate`) or after
/// it (`Asynchronous`) (`NotificationStrategy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NotificationStrategy {
    Immediate,
    Asynchronous,
}

/// An item that raises `PropertyChanged` for `Value` and `OtherValue` (`Item`). Items are held in
/// `Rc` and are equal only to themselves, as the reference type of upstream.
pub(crate) struct Item {
    notify: AbstractNotifyPropertyChanged,
    id: i32,
    other_value: Cell<i32>,
    value: Cell<i32>,
}

impl Item {
    /// `new Item() { Id = id }`.
    pub(crate) fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { notify: AbstractNotifyPropertyChanged::new(), id, other_value: Cell::new(0), value: Cell::new(0) })
    }

    /// The key of an item (`SelectId`).
    pub(crate) fn select_id(item: &Rc<Item>) -> i32 {
        item.id
    }

    /// The key (`Id`).
    pub(crate) fn id(&self) -> i32 {
        self.id
    }

    /// Whether `PropertyChanged` has handlers (`HasSubscriptions`).
    pub(crate) fn has_subscriptions(&self) -> bool {
        self.notify.property_changed().has_handlers()
    }

    /// `OtherValue`.
    pub(crate) fn other_value(&self) -> i32 {
        self.other_value.get()
    }

    /// Sets `OtherValue`, raising `OtherValue` when it changes.
    pub(crate) fn set_other_value(&self, value: i32) {
        self.notify.set_and_raise(&self.other_value, value, "OtherValue");
    }

    /// `Value`.
    pub(crate) fn value(&self) -> i32 {
        self.value.get()
    }

    /// Sets `Value`, raising `Value` when it changes.
    pub(crate) fn set_value(&self, value: i32) {
        self.notify.set_and_raise(&self.value, value, "Value");
    }

    /// Raises `PropertyChanged` for every property: an empty name (`RaiseAllPropertiesChanged`).
    pub(crate) fn raise_all_properties_changed(&self) {
        self.notify.on_property_changed("");
    }
}

impl INotifyPropertyChanged for Item {
    fn property_changed(&self) -> &Event<str> {
        self.notify.property_changed()
    }
}

impl PartialEq for Item {
    /// Reference equality, as upstream's class.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Item").field("id", &self.id).field("value", &self.value.get()).field("other_value", &self.other_value.get()).finish()
    }
}
