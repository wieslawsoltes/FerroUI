//! The writing side of a binding: a property path with a setter ([`SettableProperty`]) and the
//! macro [`settable_prop!`](crate::settable_prop) that writes one.
//!
//! Not an upstream file. Upstream writes a property through the expression that selects it (the
//! generated code assigns `view.NameText = value`; the run-time fallback sets the last member of
//! the expression chain by reflection). A [`PropertyPath`] has a getter only, so the bindings that
//! write a property take its path together with a setter.

use ferroui_system::component_model::PropertyPath;
use std::fmt;
use std::rc::Rc;

/// A property a binding reads, observes and writes: its [`PropertyPath`] (the getter, the names
/// and the observation of its steps) and a setter that writes the value at the end of the path.
///
/// [`settable_prop!`](crate::settable_prop) writes one from the getters, with the setter
/// `set_<last getter>` of the object that owns the last property (`x => x.Address.Street` sets
/// with `x.address().set_street(value)`). [`new`](Self::new) takes any setter, for a path whose
/// owner may be absent or a setter of another name.
// Deviation (DEVIATIONS.md, ReactiveUI): where upstream takes one expression for the property a
// binding writes, the port takes a property path with a setter.
pub struct SettableProperty<S, V> {
    path: PropertyPath<S, V>,
    set: Rc<dyn Fn(&S, V)>,
}

impl<S, V> Clone for SettableProperty<S, V> {
    fn clone(&self) -> Self {
        Self { path: self.path.clone(), set: self.set.clone() }
    }
}

impl<S: 'static, V: 'static> SettableProperty<S, V> {
    /// The property at the end of `path`, written by `set`.
    pub fn new(path: PropertyPath<S, V>, set: impl Fn(&S, V) + 'static) -> Self {
        Self { path, set: Rc::new(set) }
    }

    /// The path of the property.
    pub fn path(&self) -> &PropertyPath<S, V> {
        &self.path
    }

    /// The value of the property on `source`.
    pub fn get(&self, source: &S) -> V {
        self.path.get(source)
    }

    /// Writes `value` to the property on `source`.
    pub fn set(&self, source: &S, value: V) {
        (self.set)(source, value);
    }

    /// The setter of the property.
    pub fn setter(&self) -> Rc<dyn Fn(&S, V)> {
        self.set.clone()
    }
}

impl<S: 'static, V: 'static> fmt::Debug for SettableProperty<S, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SettableProperty").field("names", &self.path.names()).finish_non_exhaustive()
    }
}

/// A [`SettableProperty`] from getters: the path of [`prop!`](ferroui_system::prop) and the
/// setter `set_<last getter>` of the object that owns the last property.
///
/// `settable_prop!(View => name_text)` reads with `name_text()`, observes `NameText` and writes
/// with `set_name_text(value)`; `settable_prop!(Person => address.street)` writes with
/// `address().set_street(value)`. The steps take `as "Name"` as [`prop!`](ferroui_system::prop)
/// does; the setter is named after the getter all the same.
///
/// ```
/// use ferroui_reactiveui::settable_prop;
/// use ferroui_system::component_model::INotifyPropertyChanged;
/// use ferroui_system::events::Event;
/// use std::cell::RefCell;
/// use std::rc::Rc;
///
/// struct Person { name: RefCell<String>, changed: Event<str> }
/// impl Person {
///     fn name(&self) -> String { self.name.borrow().clone() }
///     fn set_name(&self, value: String) { *self.name.borrow_mut() = value; self.changed.raise("Name"); }
/// }
/// impl INotifyPropertyChanged for Person {
///     fn property_changed(&self) -> &Event<str> { &self.changed }
/// }
///
/// let property = settable_prop!(Person => name);
/// let person = Rc::new(Person { name: RefCell::new(String::new()), changed: Event::new() });
/// property.set(&person, "Ada".to_owned());
/// assert_eq!(property.get(&person), "Ada");
/// assert_eq!(property.path().path(), "Name");
/// ```
#[macro_export]
macro_rules! settable_prop {
    (@set $source:expr, $value:expr; $last:ident) => {
        $crate::__private::paste! { $source.[<set_ $last>]($value) }
    };
    (@set $source:expr, $value:expr; $head:ident $(. $tail:ident)+) => {
        $crate::settable_prop!(@set $source.$head(), $value; $($tail).+)
    };
    ($type_:ty => $first:ident $(as $name:literal)? $(. $rest:ident $(as $rest_name:literal)?)*) => {
        $crate::SettableProperty::new(
            $crate::__private::prop!($type_ => $first $(as $name)? $(. $rest $(as $rest_name)?)*),
            |source: &::std::rc::Rc<$type_>, value| {
                $crate::settable_prop!(@set source, value; $first $(. $rest)*);
            },
        )
    };
}
