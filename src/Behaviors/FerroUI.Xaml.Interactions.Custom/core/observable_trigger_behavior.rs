//! Port of `Core/ObservableTriggerBehavior.cs` (`ObservableTriggerBehavior<T>`).

use ferroui_base::data::core::plugins::ObservableValue;
use ferroui_base::reactive::{AnonymousObserver, IDisposable, IObservable, IObserver, Observable, ObservableError};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, WeakRef,
};
use ferroui_xaml_interactivity::{
    Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTrigger,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A trigger that subscribes to an observable and executes its actions whenever a new value is
/// produced. The emitted value is exposed through the [`value`](Self::value) property and passed
/// to the actions as a parameter.
///
/// Deviation (DEVIATIONS.md, Behaviors): classes of the class model are not generic. The
/// [`observable`](Self::observable) is an observable of untyped values
/// (`Rc<dyn IObservable<Option<BoxedValue>>>`, `Rc<dyn IObservable<BoxedValue>>` or the stream
/// value a binding reads from a property of type `IObservable<T>`, `ObservableValue`, held in an
/// untyped value), and [`value`](Self::value) is untyped, for the `IObservable<T>` and `T` of the
/// reference; the class can therefore be created from markup without type arguments.
#[repr(C)]
pub struct ObservableTriggerBehavior {
    base: StyledElementTrigger,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(ObservableTriggerBehavior: StyledElementTrigger);
ferro_class_info!(ObservableTriggerBehavior { new: ObservableTriggerBehavior::new });
ferroui_base::ferro_impl_classes!(ObservableTriggerBehavior: StyledElementImpl);

ferro_properties! {
    impl ObservableTriggerBehavior {
        /// Identifies the `Observable` property.
        pub fn observable_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ObservableTriggerBehavior, _>("Observable", None)
        }

        /// Identifies the `Value` property.
        pub fn value_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ObservableTriggerBehavior, _>("Value", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ObservableTriggerBehavior {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);
            this.subscribe();
        }

        fn on_detaching(this: &Self) {
            Self::parent_on_detaching(this);
            let subscription = this.subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ObservableTriggerBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::observable_property().as_property() {
                this.subscribe();
            }
        }
    }
}

/// The observable an untyped value holds, as an observable of optional untyped values.
///
/// The value of a property of type `IObservable<T>` that a binding reads (`{Binding Values}`)
/// is the stream value of the binding system (`ObservableValue`, or its nullable form): its
/// observable of untyped values.
fn observable_of(value: &BoxedValue) -> Option<Rc<dyn IObservable<Option<BoxedValue>>>> {
    if let Some(observable) = value.downcast_ref::<Rc<dyn IObservable<Option<BoxedValue>>>>() {
        return Some(observable.clone());
    }

    if let Some(observable) = value.downcast_ref::<ObservableValue>() {
        return Some(observable.0.clone());
    }

    if let Some(observable) = value.downcast_ref::<Option<ObservableValue>>() {
        return observable.as_ref().map(|observable| observable.0.clone());
    }

    if let Some(observable) = value.downcast_ref::<Rc<dyn IObservable<BoxedValue>>>() {
        let observable = observable.clone();
        return Some(Observable::create(move |observer| {
            observable.subscribe(Rc::new(ForwardingObserver { target: observer }))
        }));
    }

    None
}

/// Gives the values of an observable of untyped values to an observer of optional untyped values.
struct ForwardingObserver {
    target: Rc<dyn IObserver<Option<BoxedValue>>>,
}

impl IObserver<BoxedValue> for ForwardingObserver {
    fn on_next(&self, value: BoxedValue) {
        self.target.on_next(Some(value));
    }

    fn on_error(&self, error: ObservableError) {
        self.target.on_error(error);
    }

    fn on_completed(&self) {
        self.target.on_completed();
    }
}

impl ObservableTriggerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTrigger::construct(), subscription: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the observable sequence that triggers the actions. This is a property of the
    /// object model.
    pub fn observable(&self) -> Option<BoxedValue> {
        self.get_value(Self::observable_property())
    }

    pub fn set_observable(&self, value: Option<BoxedValue>) {
        self.set_value(Self::observable_property(), value)
    }

    /// Gets the last value received from the [`observable`](Self::observable). This is a
    /// property of the object model.
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }

    fn set_trigger_value(&self, value: Option<BoxedValue>) {
        self.set_current_value(Self::value_property(), value)
    }

    fn subscribe(&self) {
        // The reference disposes the previous subscription and keeps its handle until it is
        // replaced: values produced while subscribing again are not ignored.
        let subscription = self.subscription.borrow().clone();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        let Some(observable) = self.observable().as_ref().and_then(observable_of) else {
            return;
        };

        let behavior: WeakRef<ObservableTriggerBehavior> = self.to_ref().downgrade();
        let subscription = observable.subscribe(Rc::new(AnonymousObserver::new(move |value: Option<BoxedValue>| {
            let Some(behavior) = behavior.upgrade() else {
                return;
            };
            if behavior.subscription.borrow().is_none() {
                return;
            }

            let _ = Dispatcher::ui_thread().invoke_local(move || {
                behavior.set_trigger_value(value.clone());
                behavior.execute(value.as_ref());
            });
        })));
        *self.subscription.borrow_mut() = Some(subscription);
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        let associated_object = self.associated_object();
        let Some(associated_object) = associated_object.filter(|_| self.is_enabled()) else {
            return;
        };

        let sender: BoxedValue = Rc::new(associated_object);
        Interaction::execute_actions(Some(&sender), self.actions().as_ref(), parameter);
    }
}
