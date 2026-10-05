use crate::animation::{Animatable, IClock, TransitionBase, TransitionBaseImpl, TransitionInstance};
use crate::data::BindingPriority;
use crate::reactive::{IDisposable, IObservable};
use crate::{AnyValue, BoxedValue, PropertyValue, Upcast};
use std::rc::Rc;

/// Defines how a property of value type `T` should be animated using a
/// transition.
///
/// A transition class derives from [`TransitionBase`], is declared with
/// [`ferro_transition_class!`](crate::ferro_transition_class) and implements
/// this trait.
pub trait Transition<T: PropertyValue>: TransitionBaseImpl + Upcast<TransitionBase> {
    /// Creates the observable of the values of the transition from
    /// `old_value` to `new_value`, driven by `progress` (from 0 to 1).
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: T,
        new_value: T,
    ) -> Rc<dyn IObservable<T>>;
}

/// The implementation of [`TransitionBaseImpl::apply`] for a transition of
/// value type `T`.
pub fn apply_transition<C: Transition<T>, T: PropertyValue>(
    this: &C,
    control: &Animatable,
    clock: Rc<dyn IClock>,
    old_value: &BoxedValue,
    new_value: &BoxedValue,
) -> Rc<dyn IDisposable> {
    let base: &TransitionBase = this.upcast();
    let Some(property) = base.property() else {
        panic!("Transition has no property specified.");
    };
    let typed = |value: &BoxedValue| -> T {
        let value: &dyn AnyValue = &**value;
        match value.downcast_ref::<T>() {
            Some(value) => value.clone(),
            None => panic!(
                "Unable to cast object of type '{}' to type '{}'.",
                value.type_name(),
                std::any::type_name::<T>()
            ),
        }
    };
    let Some(styled) = property.as_styled::<T>() else {
        panic!(
            "Unable to cast property '{}' of type '{}' to a property of type '{}'.",
            property,
            property.property_type_name(),
            std::any::type_name::<T>()
        );
    };
    let instance = TransitionInstance::new(clock, base.delay(), base.duration());
    control.report_transition_instance(&instance);
    let progress: Rc<dyn IObservable<f64>> = instance;
    let transition = C::do_transition(this, progress, typed(old_value), typed(new_value));
    control.bind(styled, transition, BindingPriority::Animation)
}

/// Declares a transition class for properties of the given value type:
/// the class struct deriving from
/// [`TransitionBase`](crate::animation::TransitionBase), its constructors
/// and the override that applies it. The
/// [`Transition`](crate::animation::Transition) implementation supplies the
/// values.
///
/// ```ignore
/// ferro_transition_class!(
///     /// Transition class that handles properties with `f64` types.
///     DoubleTransition: f64
/// );
/// ```
#[macro_export]
macro_rules! ferro_transition_class {
    ($(#[$meta:meta])* $name:ident : $value:ty) => {
        $(#[$meta])*
        #[repr(C)]
        pub struct $name {
            base: $crate::animation::TransitionBase,
        }

        $crate::ferro_class!($name: TransitionBase);
        $crate::ferro_class_info!($name { new: $name::new });

        impl $crate::FerroObjectImpl for $name {}

        impl $crate::animation::TransitionBaseImpl for $name {
            fn apply(
                this: &Self,
                control: &$crate::animation::Animatable,
                clock: ::std::rc::Rc<dyn $crate::animation::IClock>,
                old_value: &$crate::BoxedValue,
                new_value: &$crate::BoxedValue,
            ) -> ::std::rc::Rc<dyn $crate::reactive::IDisposable> {
                $crate::animation::apply_transition::<$name, $value>(this, control, clock, old_value, new_value)
            }

            fn value_type(_this: &Self) -> (::std::any::TypeId, &'static str) {
                (::std::any::TypeId::of::<$value>(), ::std::any::type_name::<$value>())
            }
        }

        impl $name {
            /// Creates the class data.
            pub fn construct() -> Self {
                Self { base: $crate::animation::TransitionBase::construct() }
            }

            pub fn new() -> $crate::Ref<Self> {
                $crate::instantiate(Self::construct())
            }

            /// Creates a transition of `property` that takes `duration`.
            pub fn with_property(
                property: &'static $crate::FerroProperty,
                duration: $crate::animation::TimeSpan,
            ) -> $crate::Ref<Self> {
                let result = Self::new();
                result.set_property(Some(property));
                result.set_duration(duration);
                result
            }
        }
    };
}
