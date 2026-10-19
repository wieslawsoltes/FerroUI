//! `ReactiveUIBindingExtensions.ToProperty.cs`: `ToProperty`, the output property of a reactive
//! object backed by an observable.

use crate::binding::bindings::{ObservableAsPropertyHelper, ObservableAsPropertyInitialValue};
use crate::IReactiveObject;
use ferroui_reactive::concurrency::IScheduler;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::IObservable;
use std::rc::Rc;

/// `ToProperty`: makes an [`ObservableAsPropertyHelper`] that follows the sequence and raises the
/// change notifications of a property of its owner; implemented for `Rc<dyn IObservable<T>>`.
///
/// The property is named by a path (`prop!(VM => full_name)`), whose name is the one raised. The
/// helper raises `PropertyChanging` and `PropertyChanged` through
/// [`IReactiveObject`](crate::IReactiveObject)'s raising methods, so the suppression and the delay
/// of the notifications apply. The owner is held weakly.
///
/// Upstream's forms with an `out` parameter return the helper as the other forms do, so they are
/// these; a view model stores the helper (in a `OnceCell` or a `RefCell<Option<..>>`) and reads
/// [`value`](ObservableAsPropertyHelper::value) in its getter.
// Deviation (DEVIATIONS.md, ReactiveUI): the property is a path, and the forms without an initial
// value start from `T::default()`, as upstream starts from `default(T)`.
pub trait ToPropertyMixins<T: Clone + PartialEq + 'static> {
    /// The helper of the property `path` of `owner`, from the default value of `T`
    /// (`ToProperty(TObj, Func<TObj, TRet>)`).
    fn to_property<VM: IReactiveObject>(self, owner: &Rc<VM>, path: PropertyPath<Rc<VM>, T>) -> ObservableAsPropertyHelper<T>
    where
        T: Default;

    /// [`to_property`](Self::to_property) that subscribes on the first read of the value when
    /// `defer_subscription` is set (`ToProperty(TObj, Func<TObj, TRet>, bool)`).
    fn to_property_deferred<VM: IReactiveObject>(self, owner: &Rc<VM>, path: PropertyPath<Rc<VM>, T>, defer_subscription: bool) -> ObservableAsPropertyHelper<T>
    where
        T: Default;

    /// [`to_property`](Self::to_property) that delivers the values on `scheduler`
    /// (`ToProperty(TObj, Func<TObj, TRet>, ISequencer?)`).
    fn to_property_on<VM: IReactiveObject>(self, owner: &Rc<VM>, path: PropertyPath<Rc<VM>, T>, scheduler: Option<Rc<dyn IScheduler>>) -> ObservableAsPropertyHelper<T>
    where
        T: Default;

    /// [`to_property`](Self::to_property) with both options (`ToProperty(TObj, Func<TObj, TRet>,
    /// bool, ISequencer?)`).
    fn to_property_deferred_on<VM: IReactiveObject>(
        self,
        owner: &Rc<VM>,
        path: PropertyPath<Rc<VM>, T>,
        defer_subscription: bool,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> ObservableAsPropertyHelper<T>
    where
        T: Default;

    /// The helper starting from `initial_value` (`ToProperty(TObj, Func<TObj, TRet>, TRet)`).
    fn to_property_with_initial_value<VM: IReactiveObject>(self, owner: &Rc<VM>, path: PropertyPath<Rc<VM>, T>, initial_value: T) -> ObservableAsPropertyHelper<T>;

    /// The helper with every option: the initial value or its factory, the deferred subscription
    /// and the scheduler (the forms with `TRet initialValue` or `Func<TRet> getInitialValue`,
    /// `bool deferSubscription` and `ISequencer? scheduler`).
    fn to_property_with<VM: IReactiveObject>(
        self,
        owner: &Rc<VM>,
        path: PropertyPath<Rc<VM>, T>,
        initial_value: ObservableAsPropertyInitialValue<T>,
        defer_subscription: bool,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> ObservableAsPropertyHelper<T>;

    /// The helper of the property named `property_name`, with every option (the forms that name
    /// the property with a string, `nameof(Property)`).
    fn to_property_by_name<VM: IReactiveObject>(
        self,
        owner: &Rc<VM>,
        property_name: &str,
        initial_value: ObservableAsPropertyInitialValue<T>,
        defer_subscription: bool,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> ObservableAsPropertyHelper<T>;
}

impl<T: Clone + PartialEq + 'static> ToPropertyMixins<T> for Rc<dyn IObservable<T>> {
    fn to_property<VM: IReactiveObject>(self, owner: &Rc<VM>, path: PropertyPath<Rc<VM>, T>) -> ObservableAsPropertyHelper<T>
    where
        T: Default,
    {
        self.to_property_with(owner, path, ObservableAsPropertyInitialValue::default(), false, None)
    }

    fn to_property_deferred<VM: IReactiveObject>(self, owner: &Rc<VM>, path: PropertyPath<Rc<VM>, T>, defer_subscription: bool) -> ObservableAsPropertyHelper<T>
    where
        T: Default,
    {
        self.to_property_with(owner, path, ObservableAsPropertyInitialValue::default(), defer_subscription, None)
    }

    fn to_property_on<VM: IReactiveObject>(self, owner: &Rc<VM>, path: PropertyPath<Rc<VM>, T>, scheduler: Option<Rc<dyn IScheduler>>) -> ObservableAsPropertyHelper<T>
    where
        T: Default,
    {
        self.to_property_with(owner, path, ObservableAsPropertyInitialValue::default(), false, scheduler)
    }

    fn to_property_deferred_on<VM: IReactiveObject>(
        self,
        owner: &Rc<VM>,
        path: PropertyPath<Rc<VM>, T>,
        defer_subscription: bool,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> ObservableAsPropertyHelper<T>
    where
        T: Default,
    {
        self.to_property_with(owner, path, ObservableAsPropertyInitialValue::default(), defer_subscription, scheduler)
    }

    fn to_property_with_initial_value<VM: IReactiveObject>(self, owner: &Rc<VM>, path: PropertyPath<Rc<VM>, T>, initial_value: T) -> ObservableAsPropertyHelper<T> {
        self.to_property_with(owner, path, ObservableAsPropertyInitialValue::Value(initial_value), false, None)
    }

    fn to_property_with<VM: IReactiveObject>(
        self,
        owner: &Rc<VM>,
        path: PropertyPath<Rc<VM>, T>,
        initial_value: ObservableAsPropertyInitialValue<T>,
        defer_subscription: bool,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> ObservableAsPropertyHelper<T> {
        self.to_property_by_name(owner, &path.path(), initial_value, defer_subscription, scheduler)
    }

    fn to_property_by_name<VM: IReactiveObject>(
        self,
        owner: &Rc<VM>,
        property_name: &str,
        initial_value: ObservableAsPropertyInitialValue<T>,
        defer_subscription: bool,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> ObservableAsPropertyHelper<T> {
        let changed_name = property_name.to_string();
        let changing_name = property_name.to_string();
        ObservableAsPropertyHelper::create(
            self,
            owner,
            move |owner: &Rc<VM>, _value: T| IReactiveObject::raise_property_changed(&**owner, changed_name.as_str()),
            Some(Rc::new(move |owner: &Rc<VM>, _value: T| IReactiveObject::raise_property_changing(&**owner, changing_name.as_str()))),
            initial_value,
            defer_subscription,
            scheduler,
        )
    }
}
