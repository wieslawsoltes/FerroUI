//! Port of `Diagnostics/ViewModels/PropertyViewModel.cs`.
//!
//! The abstract class is the state the property view models share ([`PropertyViewModel`]),
//! which [`FerroPropertyViewModel`] and [`ClrPropertyViewModel`] embed and dereference to. A
//! property view model of either class is held as a [`PropertyViewModelRef`]: the shared
//! handle typed with the abstract class, through which its abstract members dispatch.
//!
//! The `Type`-typed members of the original are [`ValueType`]s: the type of the values of a
//! property, the type of its current value, and the type that declares it.

use super::{ClrPropertyViewModel, FerroPropertyViewModel, ViewModelBase};
use crate::diagnostics::TypeExtesnions;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::metadata::MarkupType;
use ferroui_base::{BoxedValue, FerroProperty};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// The key of a property view model in the index of the properties of an object (`object Key`):
/// the registered property, or the name of a plain property.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PropertyViewModelKey {
    Property(&'static FerroProperty),
    Name(String),
}

/// The state of the abstract class `PropertyViewModel`: the base of the view models of the
/// properties of an object, and whether the property is pinned.
pub struct PropertyViewModel {
    base: ViewModelBase,
    is_pinned: RefCell<bool>,
}

impl INotifyPropertyChanged for PropertyViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl PropertyViewModel {
    /// The state of the class, for the classes that derive from it.
    pub fn construct() -> Self {
        Self { base: ViewModelBase::new(), is_pinned: RefCell::new(false) }
    }

    /// The base class of the view models (`RaiseAndSetIfChanged`, `RaisePropertyChanged`).
    pub fn view_model_base(&self) -> &ViewModelBase {
        &self.base
    }

    pub fn is_pinned(&self) -> bool {
        *self.is_pinned.borrow()
    }

    /// Stores `IsPinned` without raising the notification; `true` when the value changed. The
    /// setter of the property is [`PropertyViewModelRef::set_is_pinned`], which runs the
    /// override of `OnPropertyChanged` of the deriving class before the handlers.
    pub(crate) fn store_is_pinned(&self, value: bool) -> bool {
        if *self.is_pinned.borrow() == value {
            return false;
        }
        *self.is_pinned.borrow_mut() = value;
        true
    }

    /// `RaisePropertyChanged`, for the deriving classes.
    pub fn raise_property_changed(&self, property_name: &str) {
        self.base.raise_property_changed(property_name);
    }
}

/// The text of a type as the developer tools show it (`Type.GetTypeName()`).
pub(crate) fn type_name(type_: ValueType) -> String {
    type_.get_type_name()
}

/// The dotted full name of a type (`Type.FullName`): the name of a class, of a type with markup
/// metadata, or the name the value types know.
pub(crate) fn type_full_name(type_: ValueType) -> String {
    if let Some(class) = ValueTypes::class_of(type_) {
        return class.full_name();
    }
    if let Some(markup) = MarkupType::find_by_handle(type_.id()) {
        return markup.full_name();
    }
    ValueTypes::type_full_name(type_)
}

/// The type of a value as the developer tools show it (`value.GetType()`): the run-time class of
/// an object, the type a nullable form holds, else the type of the value. `None` for null.
pub(crate) fn runtime_type_of(value: Option<&BoxedValue>) -> Option<ValueType> {
    let value = ValueTypes::normalize(value?.clone())?;

    if let Some(object) = ValueTypes::as_object(&*value) {
        let class = object.get_type();
        if let Some(handle) = class.handle() {
            return Some(ValueType::new(handle, class.name()));
        }
    }

    Some(ValueType::of_value(&*value))
}

/// The shared handle of a property view model of either class: a reference typed with the
/// abstract class `PropertyViewModel`.
#[derive(Clone)]
pub enum PropertyViewModelRef {
    Ferro(Rc<FerroPropertyViewModel>),
    Clr(Rc<ClrPropertyViewModel>),
}

/// The kind and the name of the property.
impl std::fmt::Debug for PropertyViewModelRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Self::Ferro(_) => "Ferro",
            Self::Clr(_) => "Clr",
        };
        f.debug_tuple(kind).field(&self.name()).finish()
    }
}

/// Compares by identity, as the reference type this mirrors.
impl PartialEq for PropertyViewModelRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq::<PropertyViewModel>(&**self, &**other)
    }
}

impl Deref for PropertyViewModelRef {
    type Target = PropertyViewModel;

    fn deref(&self) -> &PropertyViewModel {
        match self {
            Self::Ferro(property) => property,
            Self::Clr(property) => property,
        }
    }
}

impl INotifyPropertyChanged for PropertyViewModelRef {
    fn property_changed(&self) -> &Event<str> {
        (**self).property_changed()
    }
}

impl PropertyViewModelRef {
    pub fn key(&self) -> PropertyViewModelKey {
        match self {
            Self::Ferro(property) => property.key(),
            Self::Clr(property) => property.key(),
        }
    }

    pub fn name(&self) -> String {
        match self {
            Self::Ferro(property) => property.name(),
            Self::Clr(property) => property.name(),
        }
    }

    pub fn group(&self) -> String {
        match self {
            Self::Ferro(property) => property.group(),
            Self::Clr(property) => property.group(),
        }
    }

    pub fn assigned_type(&self) -> ValueType {
        match self {
            Self::Ferro(property) => property.assigned_type(),
            Self::Clr(property) => property.assigned_type(),
        }
    }

    pub fn declaring_type(&self) -> Option<ValueType> {
        match self {
            Self::Ferro(property) => property.declaring_type(),
            Self::Clr(property) => property.declaring_type(),
        }
    }

    pub fn value(&self) -> Option<BoxedValue> {
        match self {
            Self::Ferro(property) => property.value(),
            Self::Clr(property) => property.value(),
        }
    }

    pub fn set_value(&self, value: Option<BoxedValue>) {
        match self {
            Self::Ferro(property) => property.set_value(value),
            Self::Clr(property) => property.set_value(value),
        }
    }

    pub fn priority(&self) -> String {
        match self {
            Self::Ferro(property) => property.priority(),
            Self::Clr(property) => property.priority(),
        }
    }

    pub fn is_attached(&self) -> Option<bool> {
        match self {
            Self::Ferro(property) => property.is_attached(),
            Self::Clr(property) => property.is_attached(),
        }
    }

    pub fn update(&self) {
        match self {
            Self::Ferro(property) => property.update(),
            Self::Clr(property) => property.update(),
        }
    }

    pub fn property_type(&self) -> ValueType {
        match self {
            Self::Ferro(property) => property.property_type(),
            Self::Clr(property) => property.property_type(),
        }
    }

    pub fn is_readonly(&self) -> bool {
        match self {
            Self::Ferro(property) => property.is_readonly(),
            Self::Clr(property) => property.is_readonly(),
        }
    }

    /// The text of the type of the property, followed by the type of its value in braces when
    /// the two differ.
    pub fn type_(&self) -> String {
        let property_type = self.property_type();
        let assigned_type = self.assigned_type();

        if property_type == assigned_type {
            type_name(property_type)
        } else {
            format!("{} {{{}}}", type_name(property_type), type_name(assigned_type))
        }
    }

    pub fn set_is_pinned(&self, value: bool) {
        if self.store_is_pinned(value) {
            // `RaisePropertyChanged`: the override of `OnPropertyChanged`, then the handlers.
            self.on_property_changed("IsPinned");
            self.raise_property_changed("IsPinned");
        }
    }

    /// The virtual `OnPropertyChanged`.
    pub(crate) fn on_property_changed(&self, property_name: &str) {
        match self {
            Self::Ferro(property) => property.on_property_changed(property_name),
            Self::Clr(property) => property.on_property_changed(property_name),
        }
    }

    /// The name of the class without `PropertyViewModel`, a colon, the full name of the
    /// declaring type, a dot and the name: the key a pinned property is remembered by.
    pub fn full_name(&self) -> String {
        // `GetType().Name.Replace("PropertyViewModel", "")`.
        let class_name = match self {
            Self::Ferro(_) => "FerroPropertyViewModel",
            Self::Clr(_) => "ClrPropertyViewModel",
        };
        let declaring_type = self.declaring_type().map(type_full_name).unwrap_or_default();

        format!("{}:{}.{}", class_name.replace("PropertyViewModel", ""), declaring_type, self.name())
    }

    /// The view model as a value of its class (the source of the bindings of an editor).
    pub fn boxed(&self) -> BoxedValue {
        match self {
            Self::Ferro(property) => property.clone(),
            Self::Clr(property) => property.clone(),
        }
    }

    /// The view model of an untyped value of one of the classes (`DataContext as
    /// PropertyViewModel`).
    pub fn from_boxed(value: &BoxedValue) -> Option<Self> {
        if let Some(property) = value.downcast_ref::<PropertyViewModelRef>() {
            return Some(property.clone());
        }
        if let Some(property) = value.downcast_ref::<Rc<FerroPropertyViewModel>>() {
            return Some(Self::Ferro(property.clone()));
        }
        if let Some(property) = value.downcast_ref::<Rc<ClrPropertyViewModel>>() {
            return Some(Self::Clr(property.clone()));
        }
        // The box that is the shared object itself.
        let any: Rc<dyn std::any::Any> = value.clone();
        let any = match any.downcast::<FerroPropertyViewModel>() {
            Ok(property) => return Some(Self::Ferro(property)),
            Err(any) => any,
        };
        any.downcast::<ClrPropertyViewModel>().ok().map(Self::Clr)
    }

    /// `as FerroPropertyViewModel`.
    pub fn as_ferro(&self) -> Option<&Rc<FerroPropertyViewModel>> {
        match self {
            Self::Ferro(property) => Some(property),
            Self::Clr(_) => None,
        }
    }

    /// `as ClrPropertyViewModel`.
    pub fn as_clr(&self) -> Option<&Rc<ClrPropertyViewModel>> {
        match self {
            Self::Ferro(_) => None,
            Self::Clr(property) => Some(property),
        }
    }
}
