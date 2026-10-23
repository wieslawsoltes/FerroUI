//! The run-time type of a view model (`data.GetType()` of the generated source).
//!
//! Not an upstream file: a view model reaches a data template as the untyped value of a property
//! (`BoxedValue`), which may be the view model itself, the handle of a registered reference type,
//! an object of the class model or, with ReactiveUI, an untyped view model. [`RuntimeType::of_value`]
//! finds the type the generated tables are keyed by in each of them.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::BoxedValue;
use std::any::{Any, TypeId};
use std::fmt;

/// The run-time type of a view model: the `TypeId` the generated tables are keyed by and the path
/// of the type (`Type` of the generated source, with `FullName`).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeType {
    id: TypeId,
    name: &'static str,
}

impl RuntimeType {
    /// The type `T`.
    pub fn of<T: ?Sized + 'static>() -> Self {
        Self { id: TypeId::of::<T>(), name: std::any::type_name::<T>() }
    }

    /// The type of the id `id`, named `name`.
    pub fn new(id: TypeId, name: &'static str) -> Self {
        Self { id, name }
    }

    /// The run-time type of the view model `value` (`value.GetType()`):
    ///
    /// - an object of the class model (`Ref<T>`, or a registered handle of one) is of the handle
    ///   type of its class (`Ref<T>`), which the generator keys the classes by;
    /// - the handle `Rc<T>` of a registered reference type is of `T`;
    /// - with the feature `reactiveui`, an untyped view model (`UntypedViewModel`) is of the type
    ///   of the view model it holds;
    /// - any other value is of its own type.
    pub fn of_value(value: &BoxedValue) -> Self {
        if let Some(object) = ValueTypes::as_object(&**value) {
            let type_ = object.get_type();
            if let Some(handle) = type_.handle() {
                return Self { id: handle, name: type_.name() };
            }
        }
        #[cfg(feature = "reactiveui")]
        if let Some(untyped) = value.downcast_ref::<ferroui_reactiveui_controls::UntypedViewModel>() {
            return Self::of_any(&*untyped.0);
        }
        if let Some(object) = ValueTypes::reference_object(value) {
            return Self { id: object.value_type_id(), name: object.type_name() };
        }
        Self { id: value.value_type_id(), name: value.type_name() }
    }

    /// The run-time type of a view model held as `dyn Any` (the view models of ReactiveUI). Its
    /// path is not known: `std::any::type_name` needs the static type.
    pub fn of_any(value: &dyn Any) -> Self {
        Self { id: value.type_id(), name: "<view model>" }
    }

    /// The id of the type.
    pub fn id(&self) -> TypeId {
        self.id
    }

    /// The path of the type (`FullName`), as `std::any::type_name` writes it.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The path of the generic type definition of the type, without its generic arguments
    /// (`my_app::WidgetViewModel` of `my_app::WidgetViewModel<i32>`); `None` for a type that is
    /// not generic (`IsGenericType` and `GetGenericTypeDefinition()`).
    pub fn generic_definition(&self) -> Option<&'static str> {
        let at = self.name.find('<')?;
        Some(&self.name[..at])
    }
}

impl fmt::Debug for RuntimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl fmt::Display for RuntimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}
