//! `WhenPropertyChangedFixture` (`DynamicData.Tests/Binding/WhenPropertyChangedFixture.cs`): the
//! models and helpers shared by the parts of the partial class (`UnitTests` and
//! `IntegrationTests`, one module per upstream file).
//!
//! The models hold their nullable children as `RefCell<Option<Rc<..>>>` with getters that return
//! `Option<Rc<..>>`, so that `x => x.Child!.Age` is
//! `PropertyChain::nullable(prop!(ParentModel => child)).then(prop!(ChildModel => age))`.
//! Upstream's `decimal` conversions use [`Decimal`], a local model of .NET's `decimal`.

#![allow(dead_code)]

use crate::domain::Randomizer;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `ObservedAmount`.
pub(crate) const OBSERVED_AMOUNT: f64 = 41.375;

/// The generator of `UnitTests` (`_randomizer`, a field initialized for each test).
pub(crate) fn unit_tests_randomizer() -> Randomizer {
    Randomizer::new(0x35C1_709B)
}

/// A .NET `decimal`: a mantissa and a scale, the number of digits after the point (`Scale`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Decimal {
    mantissa: i128,
    scale: u8,
}

impl Decimal {
    /// The largest scale of a `decimal`.
    const MAX_SCALE: i32 = 28;

    /// `(decimal)value` of a `double`: the value rounded to 15 significant digits, without
    /// trailing zeros.
    pub(crate) fn from_f64(value: f64) -> Self {
        if value == 0.0 || !value.is_finite() {
            return Self { mantissa: 0, scale: 0 };
        }
        let text = format!("{value:.14e}");
        let (digits, exponent) = text.split_once('e').expect("an exponent");
        let exponent: i32 = exponent.parse().expect("a number");
        let negative = digits.starts_with('-');
        let digits: String = digits.chars().filter(char::is_ascii_digit).collect();
        let mut mantissa: i128 = digits.parse().expect("digits");
        let mut scale = 14 - exponent;
        while scale < 0 {
            mantissa *= 10;
            scale += 1;
        }
        while scale > Self::MAX_SCALE {
            mantissa = (mantissa + 5) / 10;
            scale -= 1;
        }
        let mut decimal = Self { mantissa: if negative { -mantissa } else { mantissa }, scale: scale as u8 };
        while decimal.scale > 0 && decimal.mantissa % 10 == 0 {
            decimal.mantissa /= 10;
            decimal.scale -= 1;
        }
        decimal
    }

    /// `(decimal)value` of a `long`.
    pub(crate) fn from_i64(value: i64) -> Self {
        Self { mantissa: i128::from(value), scale: 0 }
    }

    /// `decimal.Truncate`: the integral part.
    pub(crate) fn truncate(self) -> Self {
        Self { mantissa: self.mantissa / 10i128.pow(u32::from(self.scale)), scale: 0 }
    }

    /// `Scale`.
    pub(crate) fn scale(self) -> u8 {
        self.scale
    }
}

/// `ConvertAmount`: `(decimal)amount`.
pub(crate) fn convert_amount(amount: f64) -> Decimal {
    Decimal::from_f64(amount)
}

/// `TruncateAmount`: `decimal.Truncate((decimal)amount)`.
pub(crate) fn truncate_amount(amount: f64) -> Decimal {
    Decimal::from_f64(amount).truncate()
}

/// `NewDeepChain`.
pub(crate) fn new_deep_chain(leaf: i32) -> Rc<Deep1> {
    Deep1::new(Some(new_deep2(leaf)))
}

/// `NewDeep2`.
pub(crate) fn new_deep2(leaf: i32) -> Rc<Deep2> {
    Deep2::new(Some(new_deep3(leaf)))
}

/// `NewDeep3`.
pub(crate) fn new_deep3(leaf: i32) -> Rc<Deep3> {
    Deep3::new(Some(new_deep4(leaf)))
}

/// `NewDeep4`.
pub(crate) fn new_deep4(leaf: i32) -> Rc<Deep4> {
    Deep4::new(Some(Deep5::new(leaf)))
}

/// `Item`: an id and a value that raises `Value` whenever it is set.
pub(crate) struct Item {
    changed: Event<str>,
    id: i32,
    value: Cell<i32>,
}

impl Item {
    /// `new Item { Id = id, Value = value }`.
    pub(crate) fn new(id: i32, value: i32) -> Rc<Self> {
        Rc::new(Self { changed: Event::new(), id, value: Cell::new(value) })
    }

    /// `Id`.
    pub(crate) fn id(&self) -> i32 {
        self.id
    }

    /// `Value`.
    pub(crate) fn value(&self) -> i32 {
        self.value.get()
    }

    /// Sets `Value` and raises it.
    pub(crate) fn set_value(&self, value: i32) {
        self.value.set(value);
        self.changed.raise("Value");
    }
}

impl INotifyPropertyChanged for Item {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

/// `KeyedActivable`: an id and a flag that raises `Activated` when it changes.
pub(crate) struct KeyedActivable {
    changed: Event<str>,
    id: i32,
    activated: Cell<bool>,
}

impl KeyedActivable {
    /// `new KeyedActivable(id)`.
    pub(crate) fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { changed: Event::new(), id, activated: Cell::new(false) })
    }

    /// `Id`.
    pub(crate) fn id(&self) -> i32 {
        self.id
    }

    /// `Activated`.
    pub(crate) fn activated(&self) -> bool {
        self.activated.get()
    }

    /// Sets `Activated`, raising it when it changes.
    pub(crate) fn set_activated(&self, value: bool) {
        if self.activated.get() == value {
            return;
        }
        self.activated.set(value);
        self.changed.raise("Activated");
    }
}

impl INotifyPropertyChanged for KeyedActivable {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

/// `Money`: the whole units of an amount, made by an explicit conversion
/// (`explicit operator Money(double)`, `From<f64>` here).
pub(crate) struct Money {
    whole_units: i32,
}

impl Money {
    /// `WholeUnits`.
    pub(crate) fn whole_units(&self) -> i32 {
        self.whole_units
    }
}

impl From<f64> for Money {
    fn from(amount: f64) -> Self {
        Self { whole_units: amount as i32 }
    }
}

/// `ObservablePrice`: amounts that raise their names whenever they are set, and a child set at
/// construction. The handlers are counted (`HandlerCount`), and an access to the event is a
/// subscription (`WasSubscribed`).
///
/// Upstream's `ReadError` and `ChildReadError`, which make the getters throw, are not ported: a
/// getter cannot fail.
pub(crate) struct ObservablePrice {
    changed: Event<str>,
    amount: Cell<f64>,
    other_amount: Cell<f64>,
    child: Option<Rc<ObservablePrice>>,
    was_subscribed: Cell<bool>,
}

impl ObservablePrice {
    /// `new ObservablePrice { Amount = amount }`.
    pub(crate) fn new(amount: f64) -> Rc<Self> {
        Self::create(amount, 0.0, None)
    }

    /// `new ObservablePrice { Amount = amount, OtherAmount = other_amount }`.
    pub(crate) fn with_amounts(amount: f64, other_amount: f64) -> Rc<Self> {
        Self::create(amount, other_amount, None)
    }

    /// `new ObservablePrice { Child = child }`.
    pub(crate) fn with_child(child: Rc<ObservablePrice>) -> Rc<Self> {
        Self::create(0.0, 0.0, Some(child))
    }

    fn create(amount: f64, other_amount: f64, child: Option<Rc<ObservablePrice>>) -> Rc<Self> {
        Rc::new(Self { changed: Event::new(), amount: Cell::new(amount), other_amount: Cell::new(other_amount), child, was_subscribed: Cell::new(false) })
    }

    /// `Amount`.
    pub(crate) fn amount(&self) -> f64 {
        self.amount.get()
    }

    /// Sets `Amount` and raises it.
    pub(crate) fn set_amount(&self, value: f64) {
        self.amount.set(value);
        self.changed.raise("Amount");
    }

    /// `OtherAmount`.
    pub(crate) fn other_amount(&self) -> f64 {
        self.other_amount.get()
    }

    /// Sets `OtherAmount` and raises it.
    pub(crate) fn set_other_amount(&self, value: f64) {
        self.other_amount.set(value);
        self.changed.raise("OtherAmount");
    }

    /// `Child`.
    pub(crate) fn child(&self) -> Option<Rc<ObservablePrice>> {
        self.child.clone()
    }

    /// `HandlerCount`: the handlers of `PropertyChanged`.
    pub(crate) fn handler_count(&self) -> usize {
        self.changed.handler_count()
    }

    /// `WasSubscribed`: whether a handler was ever added (the event was ever reached through
    /// `INotifyPropertyChanged`).
    pub(crate) fn was_subscribed(&self) -> bool {
        self.was_subscribed.get()
    }
}

impl INotifyPropertyChanged for ObservablePrice {
    /// The event; reaching it is the `add` accessor of upstream, which sets `WasSubscribed`. The
    /// setters raise the event directly.
    fn property_changed(&self) -> &Event<str> {
        self.was_subscribed.set(true);
        &self.changed
    }
}

/// `IHasAge`: an object with an age that raises `PropertyChanged`.
pub(crate) trait IHasAge: INotifyPropertyChanged {
    /// `Age`.
    fn age(&self) -> i32;
}

/// `TestModel`: a value that raises `Value` whenever it is set.
pub(crate) struct TestModel {
    changed: Event<str>,
    value: Cell<i32>,
}

impl TestModel {
    /// `new TestModel { Value = value }`.
    pub(crate) fn new(value: i32) -> Rc<Self> {
        Rc::new(Self { changed: Event::new(), value: Cell::new(value) })
    }

    /// `Value`.
    pub(crate) fn value(&self) -> i32 {
        self.value.get()
    }

    /// Sets `Value` and raises it.
    pub(crate) fn set_value(&self, value: i32) {
        self.value.set(value);
        self.changed.raise("Value");
    }
}

impl INotifyPropertyChanged for TestModel {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

/// `ParentModel`: a child that may be null, raising `Child` whenever it is set.
pub(crate) struct ParentModel {
    changed: Event<str>,
    child: RefCell<Option<Rc<ChildModel>>>,
}

impl ParentModel {
    /// `new ParentModel { Child = child }`.
    pub(crate) fn new(child: Option<Rc<ChildModel>>) -> Rc<Self> {
        Rc::new(Self { changed: Event::new(), child: RefCell::new(child) })
    }

    /// `Child`.
    pub(crate) fn child(&self) -> Option<Rc<ChildModel>> {
        self.child.borrow().clone()
    }

    /// Sets `Child` and raises it.
    pub(crate) fn set_child(&self, value: Option<Rc<ChildModel>>) {
        *self.child.borrow_mut() = value;
        self.changed.raise("Child");
    }
}

impl INotifyPropertyChanged for ParentModel {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

/// `ChildModel`: an age that raises `Age` whenever it is set.
pub(crate) struct ChildModel {
    changed: Event<str>,
    age: Cell<i32>,
}

impl ChildModel {
    /// `new ChildModel { Age = age }`.
    pub(crate) fn new(age: i32) -> Rc<Self> {
        Rc::new(Self { changed: Event::new(), age: Cell::new(age) })
    }

    /// `Age`.
    pub(crate) fn age(&self) -> i32 {
        self.age.get()
    }

    /// Sets `Age` and raises it.
    pub(crate) fn set_age(&self, value: i32) {
        self.age.set(value);
        self.changed.raise("Age");
    }
}

impl INotifyPropertyChanged for ChildModel {
    fn property_changed(&self) -> &Event<str> {
        &self.changed
    }
}

impl IHasAge for ChildModel {
    fn age(&self) -> i32 {
        ChildModel::age(self)
    }
}

/// Declares a model whose child may be null, raising `Child` whenever it is set (`Deep1` to
/// `Deep4`, `Level1` to `Level3`).
macro_rules! parent_model {
    ($(#[$meta:meta])* $name:ident => $child:ident) => {
        $(#[$meta])*
        pub(crate) struct $name {
            changed: Event<str>,
            child: RefCell<Option<Rc<$child>>>,
        }

        impl $name {
            /// `new .. { Child = child }`.
            pub(crate) fn new(child: Option<Rc<$child>>) -> Rc<Self> {
                Rc::new(Self { changed: Event::new(), child: RefCell::new(child) })
            }

            /// `Child`.
            pub(crate) fn child(&self) -> Option<Rc<$child>> {
                self.child.borrow().clone()
            }

            /// Sets `Child` and raises it.
            pub(crate) fn set_child(&self, value: Option<Rc<$child>>) {
                *self.child.borrow_mut() = value;
                self.changed.raise("Child");
            }
        }

        impl INotifyPropertyChanged for $name {
            fn property_changed(&self) -> &Event<str> {
                &self.changed
            }
        }
    };
}

/// Declares a model with a leaf value that raises `Leaf` whenever it is set (`Deep5`, `Level4`).
macro_rules! leaf_model {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        pub(crate) struct $name {
            changed: Event<str>,
            leaf: Cell<i32>,
        }

        impl $name {
            /// `new .. { Leaf = leaf }`.
            pub(crate) fn new(leaf: i32) -> Rc<Self> {
                Rc::new(Self { changed: Event::new(), leaf: Cell::new(leaf) })
            }

            /// `Leaf`.
            pub(crate) fn leaf(&self) -> i32 {
                self.leaf.get()
            }

            /// Sets `Leaf` and raises it.
            pub(crate) fn set_leaf(&self, value: i32) {
                self.leaf.set(value);
                self.changed.raise("Leaf");
            }
        }

        impl INotifyPropertyChanged for $name {
            fn property_changed(&self) -> &Event<str> {
                &self.changed
            }
        }
    };
}

parent_model!(
    /// `Deep1`.
    Deep1 => Deep2
);
parent_model!(
    /// `Deep2`.
    Deep2 => Deep3
);
parent_model!(
    /// `Deep3`.
    Deep3 => Deep4
);
parent_model!(
    /// `Deep4`.
    Deep4 => Deep5
);
leaf_model!(
    /// `Deep5`.
    Deep5
);
parent_model!(
    /// `Level1`.
    Level1 => Level2
);
parent_model!(
    /// `Level2`.
    Level2 => Level3
);
parent_model!(
    /// `Level3`.
    Level3 => Level4
);
leaf_model!(
    /// `Level4`.
    Level4
);
