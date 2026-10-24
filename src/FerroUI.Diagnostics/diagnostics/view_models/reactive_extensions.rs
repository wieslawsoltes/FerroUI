//! Port of `Diagnostics/ViewModels/ReactiveExtensions.cs`.

use ferroui_base::data::model::INotifyPropertyChanged;
use ferroui_base::reactive::{Disposable, IObservable, IObserver, Observable};
use std::rc::Rc;

/// Observables of the properties of view models.
pub struct ReactiveExtensions;

impl ReactiveExtensions {
    /// `vm.GetObservable(x => x.Property, fireImmediately)`: an observable of
    /// the values of a property of a view model, produced again whenever the
    /// view model notifies a change of the property.
    ///
    /// The original takes the property as an expression and reads its name
    /// and its value through reflection; here the name of the property and
    /// the function that reads it are passed. When `fire_immediately` is set,
    /// each observer receives the current value when it subscribes.
    pub fn get_observable<TOwner, TValue>(
        vm: &Rc<TOwner>,
        property_name: &'static str,
        property: impl Fn(&TOwner) -> TValue + 'static,
        fire_immediately: bool,
    ) -> Rc<dyn IObservable<TValue>>
    where
        TOwner: INotifyPropertyChanged + ?Sized + 'static,
        TValue: 'static,
    {
        let vm = vm.clone();
        let property: Rc<dyn Fn(&TOwner) -> TValue> = Rc::new(property);
        Observable::create(move |o: Rc<dyn IObserver<TValue>>| {
            let fire = {
                let vm = vm.clone();
                let property = property.clone();
                Rc::new(move || o.on_next(property(&*vm)))
            };

            if fire_immediately {
                fire();
            }

            let on_property_changed: Rc<dyn Fn(&str)> = Rc::new(move |e: &str| {
                if e == property_name {
                    fire();
                }
            });

            let token = vm.property_changed().add(on_property_changed);

            let vm = vm.clone();
            Disposable::create(move || {
                vm.property_changed().remove(token);
            })
        })
    }
}
