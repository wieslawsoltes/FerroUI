//! `POCOObservableForProperty`.

use crate::binding::bindings::BindingAffinity;
use crate::binding::interfaces::{ICreatesObservableForProperty, ObservedChange};
use crate::binding::observable_for_property::ObservedPropertyKey;
use ferroui_reactive::disposables::Disposable;
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// The properties already warned about (`HasWarned`).
    static HAS_WARNED: RefCell<HashSet<ObservedPropertyKey>> = RefCell::new(HashSet::new());
}

/// Observes a property of an object that has no change notification, at the lowest affinity
/// (`POCOObservableForProperty`).
///
/// The sequence produces one change on subscription, which carries the object and the property
/// but no value, and then nothing. Unless warnings are suppressed, the first observation of each
/// type and property writes a debug message.
#[derive(Debug, Default, Clone, Copy)]
pub struct POCOObservableForProperty;

impl POCOObservableForProperty {
    /// The observer (`new POCOObservableForProperty()`).
    pub fn new() -> Self {
        Self
    }

    /// Writes the warning the first time a property of a type is observed (`WarnOnce`); returns
    /// whether it was written.
    pub fn warn_once(owner: &PropertyOwner, property_name: &str) -> bool {
        let key = ObservedPropertyKey::of(owner, property_name);
        let first = HAS_WARNED.with(|warned| warned.borrow_mut().insert(key));
        if first && cfg!(debug_assertions) {
            // `Debug.WriteLine` of upstream: written in debug builds only.
            eprintln!(
                "[ReactiveUI.Binding] Warning: The property {property_name} is on a POCO type and won't send change notifications, WhenAny will only return a single value!"
            );
        }
        first
    }
}

impl ICreatesObservableForProperty for POCOObservableForProperty {
    /// [`BindingAffinity::FALLBACK`] for every object and property, so that any other observer
    /// outranks it.
    fn get_affinity_for_object(&self, _owner: &PropertyOwner, _property_name: &str, _before_changed: bool) -> i32 {
        BindingAffinity::FALLBACK
    }

    fn get_notification_for_property(
        &self,
        owner: &PropertyOwner,
        property_name: &str,
        _before_changed: bool,
        suppress_warnings: bool,
    ) -> Rc<dyn IObservable<ObservedChange<PropertyOwner, ()>>> {
        if !suppress_warnings {
            Self::warn_once(owner, property_name);
        }
        Rc::new(LeadOnce { change: ObservedChange::new(owner.clone(), property_name, ()) })
    }
}

/// One change on subscription, then nothing, never completing (`LeadSignal` over
/// `ImmutableNeverSignal`).
struct LeadOnce {
    change: ObservedChange<PropertyOwner, ()>,
}

impl IObservable<ObservedChange<PropertyOwner, ()>> for LeadOnce {
    fn subscribe(&self, observer: Rc<dyn IObserver<ObservedChange<PropertyOwner, ()>>>) -> Rc<dyn IDisposable> {
        observer.on_next(self.change.clone());
        Disposable::empty()
    }
}
