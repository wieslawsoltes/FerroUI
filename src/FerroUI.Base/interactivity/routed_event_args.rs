use super::{RoutedEvent, RoutingStrategies};
use crate::{FerroObject, Nullable, Ref};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// Implemented by every routed event args type.
///
/// Routed event args form a small class hierarchy rooted at
/// [`RoutedEventArgs`]: a derived args type embeds its base as its first
/// field (named `base`) and derefs to it. This trait gives the runtime view
/// of that hierarchy, so that an event raised with derived args can be
/// delivered to a handler written against any of its base args types.
///
/// Implement it with [`ferro_routed_event_args!`](crate::ferro_routed_event_args).
pub trait IRoutedEventArgs: Any {
    /// The base routed event args: the routing state of the event.
    fn as_routed_event_args(&self) -> &RoutedEventArgs;

    /// Returns the args viewed as the args type identified by `type_id`, if
    /// the args are of that type or of a type derived from it.
    fn query_args(&self, type_id: TypeId) -> Option<&dyn Any>;

    /// Whether this args type is the args type identified by `type_id` or is
    /// derived from it.
    fn is_args_type(type_id: TypeId) -> bool
    where
        Self: Sized;

    /// A shared handle to these args: what untyped code (a handler wired up
    /// by markup) is given, since a borrow cannot be held in an untyped
    /// value. The handle refers to the live args: the state handlers change
    /// (`handled`, the source, a cancel flag, ..) is shared with them.
    fn share(&self) -> Rc<dyn IRoutedEventArgs>;
}

/// Handles compare by identity (reference equality): two handles are equal
/// when they refer to the same args, so that they can be held in untyped
/// values.
impl PartialEq for dyn IRoutedEventArgs {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_routed_event_args() == other.as_routed_event_args()
    }
}

impl dyn IRoutedEventArgs {
    /// Returns the args as `T` if they are of type `T` or of a type derived
    /// from it (C# `e as T`).
    #[inline]
    pub fn downcast_ref<T: IRoutedEventArgs>(&self) -> Option<&T> {
        self.query_args(TypeId::of::<T>()).and_then(<dyn Any>::downcast_ref::<T>)
    }

    /// Whether the args are of type `T` or of a type derived from it.
    #[inline]
    pub fn is<T: IRoutedEventArgs>(&self) -> bool {
        self.query_args(TypeId::of::<T>()).is_some()
    }
}

impl Deref for dyn IRoutedEventArgs {
    type Target = RoutedEventArgs;

    #[inline]
    fn deref(&self) -> &RoutedEventArgs {
        self.as_routed_event_args()
    }
}

/// Provides state information and data specific to a routed event.
///
/// The routing state (`handled`, `route`, `source`) is mutated while the
/// event travels along its route, so it is interiorly mutable: handlers
/// receive the args by shared reference.
///
/// The args are a reference object: a copy shares the routing state with
/// the args it was made from (marking a copy as handled marks the event as
/// handled), and copies compare by identity. The same holds for the args
/// types derived from this one, whose own mutable state is shared too.
#[derive(Clone, Default)]
pub struct RoutedEventArgs {
    state: Rc<RoutedEventArgsState>,
}

#[derive(Default)]
struct RoutedEventArgsState {
    handled: Cell<bool>,
    routed_event: Cell<Option<RoutedEvent>>,
    route: Cell<RoutingStrategies>,
    source: RefCell<Option<Ref<FerroObject>>>,
}

impl PartialEq for RoutedEventArgs {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl RoutedEventArgs {
    /// Creates args with no routed event.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates args for a routed event.
    pub fn with_event<T: ?Sized>(routed_event: &RoutedEvent<T>) -> Self {
        let result = Self::default();
        result.state.routed_event.set(Some(routed_event.as_routed_event()));
        result
    }

    /// Creates args for a routed event with a source.
    pub fn with_event_and_source<T: ?Sized>(
        routed_event: &RoutedEvent<T>,
        source: impl Into<Nullable<FerroObject>>,
    ) -> Self {
        let result = Self::with_event(routed_event);
        result.state.source.replace(source.into().0);
        result
    }

    /// Whether the routed event has already been handled.
    ///
    /// Once handled, the event keeps travelling along its route but is only
    /// delivered to handlers registered for handled events too.
    #[inline]
    pub fn handled(&self) -> bool {
        self.state.handled.get()
    }

    /// Marks the routed event as handled or unhandled.
    #[inline]
    pub fn set_handled(&self, value: bool) {
        self.state.handled.set(value)
    }

    /// The routed event associated with these args.
    #[inline]
    pub fn routed_event(&self) -> Option<RoutedEvent> {
        self.state.routed_event.get()
    }

    /// Sets the routed event associated with these args.
    #[inline]
    pub fn set_routed_event<T: ?Sized>(&self, value: Option<&RoutedEvent<T>>) {
        self.state.routed_event.set(value.map(RoutedEvent::as_routed_event))
    }

    /// The routing strategy currently being applied to the event.
    #[inline]
    pub fn route(&self) -> RoutingStrategies {
        self.state.route.get()
    }

    /// Sets the routing strategy currently being applied to the event.
    #[inline]
    pub fn set_route(&self, value: RoutingStrategies) {
        self.state.route.set(value)
    }

    /// The object that raised the event.
    #[inline]
    pub fn source(&self) -> Option<Ref<FerroObject>> {
        self.state.source.borrow().clone()
    }

    /// Sets the object that raised the event.
    pub fn set_source(&self, value: impl Into<Nullable<FerroObject>>) {
        // The previous source is dropped after the borrow is released.
        let old = self.state.source.replace(value.into().0);
        drop(old);
    }

    /// Whether `object` is the source of the event (C# `e.Source == object`).
    #[inline]
    pub fn is_source(&self, object: &FerroObject) -> bool {
        match &*self.state.source.borrow() {
            Some(source) => std::ptr::eq::<FerroObject>(&**source, object),
            None => false,
        }
    }
}

impl IRoutedEventArgs for RoutedEventArgs {
    #[inline]
    fn as_routed_event_args(&self) -> &RoutedEventArgs {
        self
    }

    #[inline]
    fn query_args(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<RoutedEventArgs>() {
            Some(self)
        } else {
            None
        }
    }

    #[inline]
    fn is_args_type(type_id: TypeId) -> bool {
        type_id == TypeId::of::<RoutedEventArgs>()
    }

    fn share(&self) -> Rc<dyn IRoutedEventArgs> {
        Rc::new(self.clone())
    }
}

/// Declares a routed event args type derived from another one.
///
/// ```ignore
/// pub struct TextInputEventArgs {
///     base: RoutedEventArgs,      // always first, always named `base`
///     pub text: Option<String>,
/// }
///
/// ferro_routed_event_args!(TextInputEventArgs: RoutedEventArgs);
/// ```
///
/// Generates `Deref<Target = Base>` and the
/// [`IRoutedEventArgs`](crate::interactivity::IRoutedEventArgs)
/// implementation that makes the args deliverable to handlers written
/// against `TextInputEventArgs` or any of its base args types.
#[macro_export]
macro_rules! ferro_routed_event_args {
    ($name:ident : $base:ty) => {
        impl ::std::ops::Deref for $name {
            type Target = $base;

            #[inline]
            fn deref(&self) -> &$base {
                &self.base
            }
        }

        impl $crate::interactivity::IRoutedEventArgs for $name {
            #[inline]
            fn as_routed_event_args(&self) -> &$crate::interactivity::RoutedEventArgs {
                $crate::interactivity::IRoutedEventArgs::as_routed_event_args(&self.base)
            }

            #[inline]
            fn query_args(&self, type_id: ::std::any::TypeId) -> ::std::option::Option<&dyn ::std::any::Any> {
                if type_id == ::std::any::TypeId::of::<$name>() {
                    ::std::option::Option::Some(self)
                } else {
                    $crate::interactivity::IRoutedEventArgs::query_args(&self.base, type_id)
                }
            }

            #[inline]
            fn is_args_type(type_id: ::std::any::TypeId) -> bool {
                type_id == ::std::any::TypeId::of::<$name>()
                    || <$base as $crate::interactivity::IRoutedEventArgs>::is_args_type(type_id)
            }

            fn share(&self) -> ::std::rc::Rc<dyn $crate::interactivity::IRoutedEventArgs> {
                ::std::rc::Rc::new(::std::clone::Clone::clone(self))
            }
        }
    };
}
