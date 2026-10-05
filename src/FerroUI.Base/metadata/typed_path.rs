//! The typed path hook of a plain property declared in markup metadata.
//!
//! A compiled binding whose path is a single plain property of a model type
//! is evaluated without boxing (`TypedBindingExpression<TSource, TValue>`)
//! when the path element has typed accessors. Generated code builds such an
//! element with `CompiledBindingPathBuilder::typed_property::<S, V>(..)`; a
//! path built at run time from metadata gets it from
//! [`MarkupProperty::typed_path_element`](super::MarkupProperty::typed_path_element),
//! which the declaration macros generate for every `properties:` entry from
//! its accessors.
//!
//! The hook produces the typed element when the instance type of the
//! declaration (`this:`) is `Rc<S>` with `S: PartialEq` (a shared reference
//! type, the `TSource : class` of the managed original) and the property
//! type is a property value type: an element that follows the property
//! change notifications of the source when `S: INotifyPropertyChanged`, one
//! that reads the value once per source otherwise. For every other
//! declaration (value types, handles of contracts, classes of the object
//! model) it produces nothing and the caller uses the untyped element. The
//! choice is made by method resolution at the declaration (the most
//! specific form that applies is selected), so a declaration needs no
//! syntax for it.

use crate::data::core::TypedClrPropertyInfo;
use crate::data::model::INotifyPropertyChanged;
use crate::data::{BindingError, CompiledBindingPathBuilder};
use crate::PropertyValue;
use std::marker::PhantomData;
use std::rc::Rc;

/// Adds the typed element of a plain property to a path under
/// construction: `(builder, accepts_null) -> builder`, or `None` if the
/// property has no typed form.
pub type TypedPathElement = fn(&CompiledBindingPathBuilder, bool) -> Option<CompiledBindingPathBuilder>;

/// The getter of a declared property, as the hook receives it.
pub type TypedPathGetter<This, V> = Rc<dyn Fn(&This) -> Result<V, BindingError>>;
/// The setter of a declared property, as the hook receives it.
pub type TypedPathSetter<This, V> = Rc<dyn Fn(&This, V)>;

/// Selects the typed form of a property of instance type `This` and
/// property type `V`; see the [module documentation](self).
#[doc(hidden)]
pub struct TypedPathProbe<This, V>(PhantomData<(This, V)>);

impl<This, V> TypedPathProbe<This, V> {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

/// The declarations of a shared type that raises property change
/// notifications (selected first).
#[doc(hidden)]
pub trait TypedPathNotifying<This, V> {
    fn typed_path_element(
        &self,
        builder: &CompiledBindingPathBuilder,
        accepts_null: bool,
        name: &str,
        get: Option<TypedPathGetter<This, V>>,
        set: Option<TypedPathSetter<This, V>>,
    ) -> Option<CompiledBindingPathBuilder>;
}

impl<S: INotifyPropertyChanged + PartialEq + 'static, V: PropertyValue> TypedPathNotifying<Rc<S>, V>
    for &TypedPathProbe<Rc<S>, V>
{
    fn typed_path_element(
        &self,
        builder: &CompiledBindingPathBuilder,
        accepts_null: bool,
        name: &str,
        get: Option<TypedPathGetter<Rc<S>, V>>,
        set: Option<TypedPathSetter<Rc<S>, V>>,
    ) -> Option<CompiledBindingPathBuilder> {
        let info = TypedClrPropertyInfo::<S, V>::from_handle_accessors(name, get, set);
        Some(builder.typed_property_info_with(info, accepts_null))
    }
}

/// The declarations of any other shared type (selected second).
#[doc(hidden)]
pub trait TypedPathShared<This, V> {
    fn typed_path_element(
        &self,
        builder: &CompiledBindingPathBuilder,
        accepts_null: bool,
        name: &str,
        get: Option<TypedPathGetter<This, V>>,
        set: Option<TypedPathSetter<This, V>>,
    ) -> Option<CompiledBindingPathBuilder>;
}

impl<S: PartialEq + 'static, V: PropertyValue> TypedPathShared<Rc<S>, V> for &&TypedPathProbe<Rc<S>, V> {
    fn typed_path_element(
        &self,
        builder: &CompiledBindingPathBuilder,
        accepts_null: bool,
        name: &str,
        get: Option<TypedPathGetter<Rc<S>, V>>,
        set: Option<TypedPathSetter<Rc<S>, V>>,
    ) -> Option<CompiledBindingPathBuilder> {
        let info = TypedClrPropertyInfo::<S, V>::from_handle_accessors(name, get, set);
        Some(builder.typed_plain_property_info_with(info, accepts_null))
    }
}

/// Every other declaration: no typed form (selected last).
#[doc(hidden)]
pub trait TypedPathFallback<This, V> {
    fn typed_path_element(
        &self,
        _builder: &CompiledBindingPathBuilder,
        _accepts_null: bool,
        _name: &str,
        _get: Option<TypedPathGetter<This, V>>,
        _set: Option<TypedPathSetter<This, V>>,
    ) -> Option<CompiledBindingPathBuilder> {
        None
    }
}

impl<This, V> TypedPathFallback<This, V> for TypedPathProbe<This, V> {}
