use super::RoutedEvent;
use crate::{StaticType, TypeInfo};
use std::cell::RefCell;
use std::collections::HashMap;

/// Tracks registered routed events.
///
/// Like routed event definitions, the registry is per-thread.
pub struct RoutedEventRegistry {
    registered_routed_events: RefCell<HashMap<&'static TypeInfo, Vec<RoutedEvent>>>,
}

impl Default for RoutedEventRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutedEventRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self { registered_routed_events: RefCell::new(HashMap::new()) }
    }

    /// The routed event registry instance of the current thread.
    pub fn instance() -> &'static RoutedEventRegistry {
        thread_local! {
            static INSTANCE: &'static RoutedEventRegistry = Box::leak(Box::new(RoutedEventRegistry::new()));
        }
        INSTANCE.with(|instance| *instance)
    }

    /// Registers a routed event on a type.
    ///
    /// You won't usually want to call this method directly: use
    /// [`RoutedEvent::register`], which calls it.
    pub fn register<T: ?Sized>(&self, type_: &'static TypeInfo, event: RoutedEvent<T>) {
        self.registered_routed_events.borrow_mut().entry(type_).or_default().push(event.as_routed_event());
    }

    /// Returns all routed events registered with the registry.
    pub fn get_all_registered(&self) -> Vec<RoutedEvent> {
        self.registered_routed_events.borrow().values().flat_map(|events| events.iter().copied()).collect()
    }

    /// Returns the routed events registered on a type.
    pub fn get_registered(&self, type_: &'static TypeInfo) -> Vec<RoutedEvent> {
        self.registered_routed_events.borrow().get(type_).cloned().unwrap_or_default()
    }

    /// Returns the routed events registered on class `TOwner`.
    pub fn get_registered_for<TOwner: StaticType>(&self) -> Vec<RoutedEvent> {
        self.get_registered(TOwner::TYPE)
    }
}
