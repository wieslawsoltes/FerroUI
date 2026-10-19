//! `FerroObjectObservableForProperty` (ReactiveUI.Binding's integration,
//! `...ObjectObservableForProperty.cs`).

use crate::{FerroObjectHandles, FerroPropertySubscription};
use ferroui_base::{FerroObject, FerroProperty, FerroPropertyChangedEventArgs, FerroPropertyRegistry};
use ferroui_reactive::Observable;
use ferroui_reactiveui::{BindingAffinity, ICreatesObservableForProperty, ObservedChange};
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// Observes a property of a FerroUI object through the registered property of that name
/// (`FerroObjectObservableForProperty`).
///
/// The observable raises after the value changes. The property system raises no notification
/// before a change, so a request for one receives the same observable.
///
/// The object is the `object` of the [`PropertyOwner`], in one of the forms
/// [`FerroObjectHandles::object_of`] recognises; the steps of
/// [`ferro_property`](crate::ferro_property) give it.
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): the affinity is computed on the object (as
/// every observer of the port does), and the registered property is found by the class of the
/// object. The changes carry no value, as the changes of every observer of the port do: the
/// reader reads the value from the object.
#[derive(Debug, Default, Clone, Copy)]
pub struct FerroObjectObservableForProperty;

impl FerroObjectObservableForProperty {
    /// The affinity of a registered property, the same as a dependency property of WPF
    /// (`FerroPropertyAffinity`).
    pub const FERRO_PROPERTY_AFFINITY: i32 = BindingAffinity::WPF_DEPENDENCY_OBJECT;

    /// The observer (`new FerroObjectObservableForProperty()`).
    pub fn new() -> Self {
        Self
    }

    /// The property registered on the class of `object` under `property_name`, or `None` when the
    /// class registers none by that name; the comparison is case-sensitive (`FindProperty`).
    pub fn find_property(object: &FerroObject, property_name: &str) -> Option<&'static FerroProperty> {
        // An attached property is named `Owner.Name`; a step of a path never is.
        if property_name.contains('.') {
            return None;
        }
        FerroPropertyRegistry::instance().find_registered_for(object, property_name)
    }
}

impl ICreatesObservableForProperty for FerroObjectObservableForProperty {
    /// [`FERRO_PROPERTY_AFFINITY`](Self::FERRO_PROPERTY_AFFINITY) when the owner is a FerroUI
    /// object whose class registers a property named `property_name`; zero otherwise.
    /// `before_changed` is ignored.
    fn get_affinity_for_object(&self, owner: &PropertyOwner, property_name: &str, _before_changed: bool) -> i32 {
        let Some(object) = owner.object.as_ref().and_then(|object| FerroObjectHandles::object_of(&**object)) else {
            return 0;
        };
        if Self::find_property(&object, property_name).is_some() {
            Self::FERRO_PROPERTY_AFFINITY
        } else {
            0
        }
    }

    /// A sequence that raises a change each time the registered property changes on the object;
    /// subscribing attaches a handler to the object's property-changed notification, and disposing
    /// the subscription detaches it. `before_changed` is ignored: the changes are always after the
    /// change.
    ///
    /// # Panics
    /// Panics when the owner has no object (`ArgumentNullException`), when the object is not a
    /// FerroUI object (`ArgumentException`), and when its class registers no property named
    /// `property_name` (`MissingMemberException`); in a debug build the last also writes a message
    /// unless `suppress_warnings` is set.
    fn get_notification_for_property(
        &self,
        owner: &PropertyOwner,
        property_name: &str,
        _before_changed: bool,
        suppress_warnings: bool,
    ) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>> {
        let Some(sender) = owner.object.as_ref() else {
            panic!("ArgumentNullException: Value cannot be null. (Parameter 'sender')");
        };
        let Some(object) = FerroObjectHandles::object_of(&**sender) else {
            panic!("ArgumentException: The sender is not a FerroObject. (Parameter 'sender')");
        };
        let type_name = object.get_type().name();
        let Some(property) = Self::find_property(&object, property_name) else {
            if !suppress_warnings && cfg!(debug_assertions) {
                // `Debug.WriteLine` of upstream: written in debug builds only.
                eprintln!("[ReactiveUI.Binding.Ferro] Error: Couldn't find FerroUI property {property_name} on {type_name}");
            }
            panic!("MissingMemberException: Member '{type_name}.{property_name}' not found.");
        };

        let owner = owner.clone();
        let expression = property_name.to_string();
        Observable::create(move |observer: Rc<dyn IObserver<ObservedChange<PropertyOwner, ()>>>| {
            let sender = owner.clone();
            let expression = expression.clone();
            let token = object.add_property_changed_handler(Rc::new(move |args: &FerroPropertyChangedEventArgs<'_>| {
                if args.property() != property {
                    return;
                }
                observer.on_next(ObservedChange::new(sender.clone(), expression.clone(), ()));
            }));
            let subscription: Rc<dyn IDisposable> = Rc::new(FerroPropertySubscription::new(object.clone(), token));
            subscription
        })
    }
}
