//! `PluginObservationSource`.

use super::PluginPropertyObservable;
use crate::binding::observable_for_property::find_observer_for_property;
use ferroui_system::component_model::PropertyOwner;
use ferroui_system::IObservable;
use std::rc::Rc;

/// Chooses between the observation that the caller built for one step of a path and a registered
/// property observer that outranks the mechanism it was built from (`PluginObservationSource`).
pub struct PluginObservationSource;

impl PluginObservationSource {
    /// The observation of the registered observer when one outranks `generated_affinity`, without
    /// a distinct gate; `generated` otherwise (`Choose<T>`).
    pub fn choose<T: Clone + 'static>(
        source: &PropertyOwner,
        property_name: &str,
        before_change: bool,
        generated_affinity: i32,
        getter: impl Fn() -> Option<T> + 'static,
        generated: Rc<dyn IObservable<T>>,
    ) -> Rc<dyn IObservable<T>> {
        match find_observer_for_property(source, property_name, before_change) {
            Some((plugin, affinity)) if affinity > generated_affinity => {
                Rc::new(PluginPropertyObservable::with_comparer(plugin, source, property_name, getter, before_change, None))
            }
            _ => generated,
        }
    }
}
