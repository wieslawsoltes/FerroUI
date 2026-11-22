//! Typed property paths for bindings to view models.
//!
//! Signals are the default for state a component owns. A view model that already raises
//! `INotifyPropertyChanged` is bound through a [`ModelProperty`]: a constant that names the
//! property in change notifications and holds its typed accessors.
//!
//! ```ignore
//! const NAME: ModelProperty<Person, String> = ModelProperty::new("Name", Person::name, Person::set_name);
//! const GREETING: ReadOnlyModelProperty<Person, String> = ReadOnlyModelProperty::new("Greeting", Person::greeting);
//!
//! stack_panel()
//!     .data_context(person as BoxedValue)
//!     .child(text_box().text(NAME.two_way()))
//!     .child(text_block().text(GREETING.one_way()))
//! ```
//!
//! An accessor that does not exist, a value type that does not fit the property, or a two-way
//! binding of a read-only property does not compile. The result is the existing compiled binding
//! with one typed path element: no reflection and no path text. The name is the text the model
//! raises its change with, and it is the one part the compiler cannot check.

use crate::builder::{markers, IntoProp};
use crate::element::PropValue;
use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::data::{BindingMode, CompiledBinding, CompiledBindingPathBuilder};
use ferroui_base::{PropertyValue, StyledValue};
use std::marker::PhantomData;
use std::rc::Rc;

/// A read-write property of the view model `S` with a value of type `V`.
pub struct ModelProperty<S: 'static, V: 'static> {
    name: &'static str,
    get: fn(&S) -> V,
    set: fn(&S, V),
}

/// A read-only property of the view model `S` with a value of type `V`.
pub struct ReadOnlyModelProperty<S: 'static, V: 'static> {
    name: &'static str,
    get: fn(&S) -> V,
}

impl<S: 'static, V: 'static> Clone for ModelProperty<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: 'static, V: 'static> Copy for ModelProperty<S, V> {}

impl<S: 'static, V: 'static> Clone for ReadOnlyModelProperty<S, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S: 'static, V: 'static> Copy for ReadOnlyModelProperty<S, V> {}

impl<S: 'static, V: 'static> ModelProperty<S, V> {
    /// The property `name` (the text its changes are raised with) with its accessors.
    pub const fn new(name: &'static str, get: fn(&S) -> V, set: fn(&S, V)) -> Self {
        Self { name, get, set }
    }

    /// The text the property's changes are raised with.
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

impl<S: 'static, V: 'static> ReadOnlyModelProperty<S, V> {
    /// The property `name` (the text its changes are raised with) with its getter.
    pub const fn new(name: &'static str, get: fn(&S) -> V) -> Self {
        Self { name, get }
    }

    /// The text the property's changes are raised with.
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

fn binding<S, V>(name: &str, get: fn(&S) -> V, set: Option<fn(&S, V)>, mode: BindingMode) -> ModelBinding<S, V>
where
    S: INotifyPropertyChanged + PartialEq + 'static,
    V: PropertyValue,
{
    let set = set.map(|set| Rc::new(set) as Rc<dyn Fn(&S, V)>);
    let path = CompiledBindingPathBuilder::new().typed_property::<S, V>(name, get, set).build();
    ModelBinding { binding: CompiledBinding::new(path).with_mode(mode), _marker: PhantomData }
}

impl<S: INotifyPropertyChanged + PartialEq + 'static, V: PropertyValue> ModelProperty<S, V> {
    /// The control follows the model.
    pub fn one_way(self) -> ModelBinding<S, V> {
        binding(self.name, self.get, Some(self.set), BindingMode::OneWay)
    }

    /// The control follows the model, and a change of the control is written to the model.
    pub fn two_way(self) -> ModelBinding<S, V> {
        binding(self.name, self.get, Some(self.set), BindingMode::TwoWay)
    }

    /// The control takes the value of the model once.
    pub fn one_time(self) -> ModelBinding<S, V> {
        binding(self.name, self.get, Some(self.set), BindingMode::OneTime)
    }

    /// A change of the control is written to the model, and the control does not follow it.
    pub fn one_way_to_source(self) -> ModelBinding<S, V> {
        binding(self.name, self.get, Some(self.set), BindingMode::OneWayToSource)
    }
}

impl<S: INotifyPropertyChanged + PartialEq + 'static, V: PropertyValue> ReadOnlyModelProperty<S, V> {
    /// The control follows the model.
    pub fn one_way(self) -> ModelBinding<S, V> {
        binding(self.name, self.get, None, BindingMode::OneWay)
    }

    /// The control takes the value of the model once.
    pub fn one_time(self) -> ModelBinding<S, V> {
        binding(self.name, self.get, None, BindingMode::OneTime)
    }
}

/// A binding to a property of a view model, which a setter of a property of type `V` or
/// `Option<V>` takes. It reads the `DataContext` of the control.
///
/// ```
/// use ferroui_base::data::model::{Event, INotifyPropertyChanged};
/// use ferroui_view::prelude::*;
/// #[derive(PartialEq)]
/// struct Person;
/// impl INotifyPropertyChanged for Person {
///     fn property_changed(&self) -> &Event<str> { unimplemented!() }
/// }
/// impl Person { fn name(&self) -> String { String::new() } }
/// const NAME: ReadOnlyModelProperty<Person, String> = ReadOnlyModelProperty::new("Name", Person::name);
/// let _ = text_block().text(NAME.one_way());
/// ```
///
/// A property that does not exist does not compile:
///
/// ```compile_fail
/// use ferroui_view::prelude::*;
/// struct Person;
/// impl Person { fn name(&self) -> String { String::new() } }
/// const NAME: ReadOnlyModelProperty<Person, String> = ReadOnlyModelProperty::new("Name", Person::nmae);
/// ```
///
/// Nor does a value of another type, or a two-way binding of a read-only property:
///
/// ```compile_fail
/// use ferroui_base::data::model::{Event, INotifyPropertyChanged};
/// use ferroui_view::prelude::*;
/// #[derive(PartialEq)]
/// struct Person;
/// impl INotifyPropertyChanged for Person {
///     fn property_changed(&self) -> &Event<str> { unimplemented!() }
/// }
/// impl Person { fn age(&self) -> i32 { 1 } }
/// const AGE: ReadOnlyModelProperty<Person, i32> = ReadOnlyModelProperty::new("Age", Person::age);
/// let _ = text_block().text(AGE.two_way());
/// ```
pub struct ModelBinding<S, V> {
    binding: Rc<CompiledBinding>,
    _marker: PhantomData<fn() -> (S, V)>,
}

/// A value type that a binding can give a property of type `T`: `T` itself or the `T` of an
/// `Option<T>`.
pub trait BindableTo<T> {}

impl<T> BindableTo<T> for T {}

impl<T> BindableTo<Option<T>> for T {}

impl<T: StyledValue, S: 'static, V: BindableTo<T> + 'static> IntoProp<T, markers::FromModel<V>> for ModelBinding<S, V> {
    fn into_prop(self) -> PropValue {
        PropValue::Binding(self.binding)
    }
}
