//! Typed property paths for bindings to view models.
//!
//! Signals are the default for state a component owns. A view model is declared with
//! `#[derive(ViewModel)]`, and one declaration gives the accessors, the name its changes are raised
//! with, and the typed property a binding takes, so they cannot disagree:
//!
//! ```ignore
//! #[derive(ViewModel)]
//! #[computed(greeting: String, depends_on(name))]
//! struct Person {
//!     #[property] name: RefCell<String>,
//!     property_changed: Event<str>,
//! }
//!
//! stack_panel()
//!     .data_context(person as BoxedValue)
//!     .child(text_box().text(Person::NAME.two_way()))
//!     .child(text_block().text(Person::GREETING.one_way()))
//! ```
//!
//! A property that does not exist, a value type that does not fit, or a two-way binding of a
//! read-only property does not compile. The result is the existing compiled binding with one
//! typed path element: no reflection and no path text.
//!
//! A model that already implements `INotifyPropertyChanged` with setters of its own raises names
//! the compiler cannot see. It is bound with [`ModelProperty::with_raised_name`], whose name is
//! **not checked**: a name that differs from what the model raises compiles, shows the first
//! value, and then does not update.

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
    /// A property of a model with setters of its own, which raises its changes with the text
    /// `raised_name`.
    ///
    /// **The name is not checked.** The accessors are, but a name that differs from what the
    /// model raises compiles, shows the first value, and then does not update. A model declared
    /// with `#[derive(ViewModel)]` has its properties as constants instead (`Person::NAME`).
    pub const fn with_raised_name(raised_name: &'static str, get: fn(&S) -> V, set: fn(&S, V)) -> Self {
        Self { name: raised_name, get, set }
    }

    /// A property that `#[derive(ViewModel)]` declared, with the name its setter raises.
    #[doc(hidden)]
    pub const fn __declared(name: &'static str, get: fn(&S) -> V, set: fn(&S, V)) -> Self {
        Self { name, get, set }
    }

    /// The text the property's changes are raised with.
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

impl<S: 'static, V: 'static> ReadOnlyModelProperty<S, V> {
    /// A read-only property of a model with setters of its own, which raises its changes with the
    /// text `raised_name`.
    ///
    /// **The name is not checked.** The getter is, but a name that differs from what the model
    /// raises compiles, shows the first value, and then does not update. A model declared with
    /// `#[derive(ViewModel)]` has its properties as constants instead (`Person::GREETING`).
    pub const fn with_raised_name(raised_name: &'static str, get: fn(&S) -> V) -> Self {
        Self { name: raised_name, get }
    }

    /// A property that `#[derive(ViewModel)]` declared, with the name its model raises.
    #[doc(hidden)]
    pub const fn __declared(name: &'static str, get: fn(&S) -> V) -> Self {
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
/// use ferroui_base::data::model::Event;
/// use ferroui_view::prelude::*;
/// use std::cell::RefCell;
///
/// #[derive(ViewModel)]
/// struct Person {
///     #[property]
///     name: RefCell<String>,
///     property_changed: Event<str>,
/// }
///
/// assert_eq!(Person::NAME.name(), "name");
/// let _ = text_block().text(Person::NAME.one_way());
/// ```
///
/// A property that does not exist does not compile, so the name cannot be misspelled:
///
/// ```compile_fail
/// use ferroui_base::data::model::Event;
/// use ferroui_view::prelude::*;
/// use std::cell::RefCell;
///
/// #[derive(ViewModel)]
/// struct Person {
///     #[property]
///     name: RefCell<String>,
///     property_changed: Event<str>,
/// }
///
/// let _ = text_block().text(Person::NMAE.one_way());
/// ```
///
/// Nor does a computed property that depends on a property the model does not have:
///
/// ```compile_fail
/// use ferroui_base::data::model::Event;
/// use ferroui_view::prelude::*;
/// use std::cell::RefCell;
///
/// #[derive(ViewModel)]
/// #[computed(greeting: String, depends_on(nmae))]
/// struct Person {
///     #[property]
///     name: RefCell<String>,
///     property_changed: Event<str>,
/// }
///
/// impl Person {
///     fn greeting(&self) -> String { String::new() }
/// }
/// ```
///
/// Nor a value of another type:
///
/// ```compile_fail
/// use ferroui_base::data::model::Event;
/// use ferroui_view::prelude::*;
/// use std::cell::Cell;
///
/// #[derive(ViewModel)]
/// struct Person {
///     #[property]
///     age: Cell<i32>,
///     property_changed: Event<str>,
/// }
///
/// let _ = text_block().text(Person::AGE.one_way());
/// ```
///
/// Nor a two-way binding of a read-only property:
///
/// ```compile_fail
/// use ferroui_base::data::model::Event;
/// use ferroui_view::prelude::*;
/// use std::cell::RefCell;
///
/// #[derive(ViewModel)]
/// #[computed(greeting: String, depends_on(name))]
/// struct Person {
///     #[property]
///     name: RefCell<String>,
///     property_changed: Event<str>,
/// }
///
/// impl Person {
///     fn greeting(&self) -> String { String::new() }
/// }
///
/// let _ = text_box().text(Person::GREETING.two_way());
/// ```
pub struct ModelBinding<S, V> {
    binding: Rc<CompiledBinding>,
    _marker: PhantomData<fn() -> (S, V)>,
}

/// A value type that a binding can give a property of type `T`: `T` itself or the `T` of an
/// `Option<T>`.
#[diagnostic::on_unimplemented(
    message = "a binding of a `{Self}` cannot set a property of type `{T}`",
    label = "the value type of this property of the model does not fit",
    note = "a property of type `V` takes a binding of `V`, and a property of type `Option<V>` too"
)]
pub trait BindableTo<T> {}

impl<T> BindableTo<T> for T {}

impl<T> BindableTo<Option<T>> for T {}

impl<T: StyledValue, S: 'static, V: BindableTo<T> + 'static> IntoProp<T, markers::FromModel<V>> for ModelBinding<S, V> {
    fn into_prop(self) -> PropValue {
        PropValue::Binding(self.binding)
    }
}
