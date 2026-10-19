//! `BuilderMixins` of a built application (`ReactiveUI.Shared/Mixins/BuilderMixins.Instance.cs`):
//! `WithInstance`, which resolves services and hands them to an action.

use super::IReactiveUIInstance;
use ferroui_splat::{IReadonlyDependencyResolver, ReadonlyDependencyResolverExt};
use std::rc::Rc;

/// The services that [`ReactiveUIInstanceMixins::with_instance`] resolves and hands to its action:
/// one service as an `Option<Rc<S>>`, or a tuple of 1 to 16 of them. Each is `None` when the
/// resolver has no registration for it (upstream's `T?`). The service may be a trait object.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream overloads `WithInstance` for actions of 1 to 16
/// arguments; the action here takes the services as one argument, whose type, a tuple for more
/// than one, says what to resolve.
pub trait ResolveServices: Sized {
    /// The services, resolved from `resolver`.
    fn resolve(resolver: &dyn IReadonlyDependencyResolver) -> Self;
}

impl<S: ?Sized + 'static> ResolveServices for Option<Rc<S>> {
    fn resolve(resolver: &dyn IReadonlyDependencyResolver) -> Self {
        resolver.get_service::<S>()
    }
}

/// Implements [`ResolveServices`] for a tuple of optional services.
macro_rules! resolve_services_tuple {
    ($($service:ident),+) => {
        impl<$($service: ?Sized + 'static),+> ResolveServices for ($(Option<Rc<$service>>,)+) {
            fn resolve(resolver: &dyn IReadonlyDependencyResolver) -> Self {
                ($(resolver.get_service::<$service>(),)+)
            }
        }
    };
}

resolve_services_tuple!(T1);
resolve_services_tuple!(T1, T2);
resolve_services_tuple!(T1, T2, T3);
resolve_services_tuple!(T1, T2, T3, T4);
resolve_services_tuple!(T1, T2, T3, T4, T5);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6, T7);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
resolve_services_tuple!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

/// The resolution of services from a built application (`BuilderMixins.WithInstance`, and the
/// `WithInstance` overloads of the builder).
///
/// ```
/// use ferroui_reactiveui::{ReactiveUIInstanceMixins, RxAppBuilder};
/// use std::rc::Rc;
///
/// RxAppBuilder::reset_for_testing();
/// let builder = RxAppBuilder::create_reactive_ui_builder();
/// builder.with_registration(|resolver| {
///     use ferroui_splat::MutableDependencyResolverExt;
///     resolver.register_constant(Rc::new(42_i32));
/// });
/// let mut seen = None;
/// builder.build_app().with_instance(|(number, text): (Option<Rc<i32>>, Option<Rc<String>>)| {
///     seen = Some((number.map(|number| *number), text.is_none()));
/// });
/// assert_eq!(seen, Some((Some(42), true)));
/// ```
pub trait ReactiveUIInstanceMixins: IReactiveUIInstance {
    /// Resolves the services that the type of the argument of `action` names and calls `action`
    /// with them; nothing happens when the application has no resolver
    /// (`WithInstance<T1, ..., Tn>(Action<T1?, ..., Tn?>)`). Returns the application, to chain the
    /// calls.
    fn with_instance<R: ResolveServices>(&self, action: impl FnOnce(R)) -> &Self {
        if let Some(current) = self.current() {
            action(R::resolve(&*current));
        }
        self
    }
}

impl<I: IReactiveUIInstance + ?Sized> ReactiveUIInstanceMixins for I {}
