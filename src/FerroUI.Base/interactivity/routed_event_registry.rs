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
        crate::ferro_thread_local! {
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

    /// Finds the routed event named `name` that `owner` or one of its base
    /// classes registers, nearest class first: the lookup of the routed
    /// event behind an event name of the managed original (the static
    /// `<Name>Event` field of the class or of a base class, found by
    /// reflection).
    ///
    /// The comparison is case-sensitive. `name` is the name of the event
    /// (`"Click"`) or of the field that holds it (`"ClickEvent"`): on each
    /// class an event of exactly that name is preferred, then the name
    /// without the `Event` suffix.
    pub fn find_by_name(&self, owner: &'static TypeInfo, name: &str) -> Option<RoutedEvent> {
        let field_name = name.strip_suffix("Event").filter(|stripped| !stripped.is_empty());
        let events = self.registered_routed_events.borrow();
        let mut current = Some(owner);
        while let Some(type_) = current {
            if let Some(registered) = events.get(type_) {
                let exact = registered.iter().find(|event| event.name() == name);
                let found = exact.or_else(|| {
                    let field_name = field_name?;
                    registered.iter().find(|event| event.name() == field_name)
                });
                if let Some(found) = found {
                    return Some(*found);
                }
            }
            current = type_.base_type();
        }
        None
    }
}
