//! `NotifyPropertyChangedObservable`.

use crate::binding::interfaces::ObservedChange;
use ferroui_reactive::disposables::Disposable;
use ferroui_system::component_model::{INotifyPropertyChanged, PropertyOwner};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A [`PropertyOwner`] held weakly, so that an observation does not keep alive the object it
/// observes (an object holds its events, and the events hold the observation).
#[derive(Clone, Default)]
pub(crate) struct WeakPropertyOwner {
    object: Option<Weak<dyn Any>>,
    notify: Option<Weak<dyn INotifyPropertyChanged>>,
}

impl WeakPropertyOwner {
    pub(crate) fn new(owner: &PropertyOwner) -> Self {
        Self { object: owner.object.as_ref().map(Rc::downgrade), notify: owner.notify.as_ref().map(Rc::downgrade) }
    }

    /// The owner, or `None` once its object is dropped.
    pub(crate) fn upgrade(&self) -> Option<PropertyOwner> {
        let object = match &self.object {
            Some(object) => Some(object.upgrade()?),
            None => None,
        };
        let notify = match &self.notify {
            Some(notify) => Some(notify.upgrade()?),
            None => None,
        };
        Some(PropertyOwner { object, notify })
    }
}

/// The notifications of one property of an object that raises `PropertyChanged`, or
/// `PropertyChanging` before a change (`NotifyPropertyChangedObservable`).
///
/// A notification of the property, or of every property (an empty name), produces one change that
/// carries the object and the name of the property, without a value.
// Deviation (DEVIATIONS.md, ReactiveUI): the subscription holds the object weakly.
pub struct NotifyPropertyChangedObservable {
    sender: WeakPropertyOwner,
    observed_property_name: String,
    before_changed: bool,
}

impl NotifyPropertyChangedObservable {
    /// The notifications of `expected_name` on `sender`: of `PropertyChanging` when
    /// `before_changed` is set and the object raises it, of `PropertyChanged` otherwise.
    pub fn new(sender: PropertyOwner, expected_name: impl Into<String>, before_changed: bool) -> Self {
        Self { sender: WeakPropertyOwner::new(&sender), observed_property_name: expected_name.into(), before_changed }
    }
}

impl IObservable<ObservedChange<PropertyOwner, ()>> for NotifyPropertyChangedObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<ObservedChange<PropertyOwner, ()>>>) -> Rc<dyn IDisposable> {
        let Some(sender) = self.sender.upgrade() else {
            return Disposable::empty();
        };
        let Some(notify) = sender.notify.clone() else {
            return Disposable::empty();
        };
        let slot: Rc<RefCell<Option<Rc<dyn IObserver<ObservedChange<PropertyOwner, ()>>>>>> = Rc::new(RefCell::new(Some(observer)));
        let handler_slot = slot.clone();
        let weak_sender = self.sender.clone();
        let name = self.observed_property_name.clone();
        let handler: Rc<dyn Fn(&str)> = Rc::new(move |notified: &str| {
            if !notified.is_empty() && notified != name {
                return;
            }
            let observer = handler_slot.borrow().clone();
            if let (Some(observer), Some(sender)) = (observer, weak_sender.upgrade()) {
                observer.on_next(ObservedChange::new(sender, name.clone(), ()));
            }
        });
        let changing = self.before_changed && notify.as_notify_property_changing().is_some();
        let token = match notify.as_notify_property_changing() {
            Some(changing_source) if changing => changing_source.property_changing().add(handler),
            _ => notify.property_changed().add(handler),
        };
        let weak_notify = Rc::downgrade(&notify);
        let weak_slot = Rc::downgrade(&slot);
        drop(slot);
        Disposable::create(move || {
            let Some(slot) = weak_slot.upgrade() else {
                return;
            };
            if slot.borrow_mut().take().is_none() {
                return;
            }
            let Some(notify) = weak_notify.upgrade() else {
                return;
            };
            if changing {
                if let Some(changing_source) = notify.as_notify_property_changing() {
                    changing_source.property_changing().remove(token);
                }
            } else {
                notify.property_changed().remove(token);
            }
        })
    }
}
