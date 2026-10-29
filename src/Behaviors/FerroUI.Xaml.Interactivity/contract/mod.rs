//! The contracts of the object model (`Contract/`): behaviors, triggers, actions, the host
//! lifecycle callbacks and the comparison kinds of conditions.
//!
//! Deviation (DEVIATIONS.md, Behaviors): the reference tests an object of a behavior collection
//! for a contract with a type test (`item is IBehavior`). An object of the class model is not a
//! trait object, so a class states the contracts it implements once, in its static constructor
//! (`<dyn IBehavior>::register::<MyBehavior>()`), and the contract finds the implementation of an
//! object through its runtime type and the types it derives from (`<dyn IBehavior>::of(object)`).
//! The base classes of this crate register theirs, so a class deriving from them implements the
//! contracts of its base class, as in the reference.

mod comparison_condition_type;
mod i_action;
mod i_behavior;
mod i_behavior_events_handler;
mod i_reversible_action;
mod i_trigger;

pub use comparison_condition_type::ComparisonConditionType;
pub use i_action::IAction;
pub use i_behavior::IBehavior;
pub use i_behavior_events_handler::IBehaviorEventsHandler;
pub use i_reversible_action::IReversibleAction;
pub use i_trigger::ITrigger;

use ferroui_base::{FerroObject, TypeInfo};
use std::sync::RwLock;

/// Finds the implementation of a contract `I` for an object of the class model: the cast of the
/// nearest class in the chain of base types of the object that registered the contract.
///
/// The table is process wide (runtime types are static data); the casts are plain functions.
pub(crate) struct InterfaceTable<I: ?Sized + 'static> {
    entries: RwLock<Vec<(&'static TypeInfo, for<'a> fn(&'a FerroObject) -> Option<&'a I>)>>,
}

impl<I: ?Sized + 'static> InterfaceTable<I> {
    /// An empty table.
    pub(crate) const fn new() -> Self {
        Self { entries: RwLock::new(Vec::new()) }
    }

    /// States that the objects of class `type_` (and of the classes deriving from it) implement
    /// the contract through `cast`. Registering a class again replaces its cast.
    pub(crate) fn register(&self, type_: &'static TypeInfo, cast: for<'a> fn(&'a FerroObject) -> Option<&'a I>) {
        let mut entries = self.entries.write().unwrap_or_else(|e| e.into_inner());
        match entries.iter_mut().find(|entry| std::ptr::eq(entry.0, type_)) {
            Some(entry) => entry.1 = cast,
            None => entries.push((type_, cast)),
        }
    }

    /// The implementation of the contract for `object`, if its class or a base class of it
    /// registered one.
    pub(crate) fn find<'a>(&self, object: &'a FerroObject) -> Option<&'a I> {
        let mut current = Some(object.get_type());
        while let Some(type_) = current {
            let cast = {
                let entries = self.entries.read().unwrap_or_else(|e| e.into_inner());
                entries.iter().find(|entry| std::ptr::eq(entry.0, type_)).map(|entry| entry.1)
            };
            if let Some(cast) = cast {
                return cast(object);
            }
            current = type_.base_type();
        }
        None
    }
}
