//! Port of `Contract/IBehaviorEventsHandler.cs`.

use super::InterfaceTable;
use ferroui_base::{FerroObject, ObjectType};

/// Interface implemented by base behaviors: the lifecycle callbacks of the host that a behavior
/// collection forwards to its behaviors.
pub trait IBehaviorEventsHandler {
    /// Called when element is attached to the visual tree.
    fn attached_to_visual_tree_event_handler(&self);

    /// Called when element is detached from the visual tree.
    fn detached_from_visual_tree_event_handler(&self);

    /// Called when element is attached to the logical tree.
    fn attached_to_logical_tree_event_handler(&self);

    /// Called when element is detached from the logical tree.
    fn detached_from_logical_tree_event_handler(&self);

    /// Called when element is loaded.
    fn loaded_event_handler(&self);

    /// Called when element is unloaded.
    fn unloaded_event_handler(&self);

    /// Called when element is initialized.
    fn initialized_event_handler(&self);

    /// Called when element DataContext changed.
    fn data_context_changed_event_handler(&self);

    /// Called when element Resources changed.
    fn resources_changed_event_handler(&self);

    /// Called when element ActualThemeVariant changed.
    fn actual_theme_variant_changed_event_handler(&self);
}

static TABLE: InterfaceTable<dyn IBehaviorEventsHandler> = InterfaceTable::new();

fn cast<'a, T: ObjectType + IBehaviorEventsHandler>(
    object: &'a FerroObject,
) -> Option<&'a (dyn IBehaviorEventsHandler + 'static)> {
    object.downcast_ref::<T>().map(|object| object as &(dyn IBehaviorEventsHandler + 'static))
}

impl dyn IBehaviorEventsHandler {
    /// States that the objects of class `T` (and of the classes deriving from it) handle the
    /// lifecycle callbacks of their host. Called from the static constructor of the class.
    pub fn register<T: ObjectType + IBehaviorEventsHandler>() {
        TABLE.register(T::TYPE, cast::<T>);
    }

    /// The object as a handler of the lifecycle callbacks (`object as IBehaviorEventsHandler`),
    /// if its class is one.
    pub fn of(object: &FerroObject) -> Option<&dyn IBehaviorEventsHandler> {
        TABLE.find(object)
    }
}
