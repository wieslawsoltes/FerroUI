//! ferroui-xaml-behaviors
//!
//! The behaviors, triggers and actions of FerroUI in one crate, as the `Xaml.Behaviors` bundle of
//! Xaml.Behaviors (see `NOTICE.md`) is one package: each crate of the port is a module of this one,
//! behind a feature, and [`register_types`] registers all that are enabled.
//!
//! | Module | Crate | Feature |
//! |---|---|---|
//! | [`interactivity`] | `ferroui-xaml-interactivity` (the object model: `Interaction.Behaviors`, behaviors, triggers, actions) | always |
//! | `animations` | `ferroui-xaml-behaviors-animations` | `animations` |
//! | `interactions` | `ferroui-xaml-interactions` (event, data and timer triggers, property, method and command actions, clipboard, pickers) | `interactions` |
//! | `custom` | `ferroui-xaml-interactions-custom` (about 330 behaviors for controls, input, focus, validation, windows, ...) | `custom` |
//! | `events` | `ferroui-xaml-interactions-events` (a trigger and a behavior per input routed event) | `events` |
//! | `draggable` | `ferroui-xaml-interactions-draggable` | `draggable` |
//! | `drag_and_drop` | `ferroui-xaml-interactions-draganddrop` | `drag-and-drop` |
//! | `responsive` | `ferroui-xaml-interactions-responsive` | `responsive` |
//! | `reactiveui` | `ferroui-xaml-interactions-reactiveui` | `reactiveui` (off by default) |
//!
//! The default features are the crates of the reference's bundle; ReactiveUI is a package of its
//! own there and a feature that is off here. All types are in the default XML namespace of the
//! framework once [`register_types`] ran, so documents name them without a prefix.

pub use ferroui_xaml_interactivity as interactivity;

#[cfg(feature = "animations")]
pub use ferroui_xaml_behaviors_animations as animations;
#[cfg(feature = "custom")]
pub use ferroui_xaml_interactions_custom as custom;
#[cfg(feature = "drag-and-drop")]
pub use ferroui_xaml_interactions_draganddrop as drag_and_drop;
#[cfg(feature = "draggable")]
pub use ferroui_xaml_interactions_draggable as draggable;
#[cfg(feature = "events")]
pub use ferroui_xaml_interactions_events as events;
#[cfg(feature = "interactions")]
pub use ferroui_xaml_interactions as interactions;
#[cfg(feature = "reactiveui")]
pub use ferroui_xaml_interactions_reactiveui as reactiveui;
#[cfg(feature = "responsive")]
pub use ferroui_xaml_interactions_responsive as responsive;

/// Registers the types of every enabled behavior crate for markup (and of the crates they are built
/// on). Cheap and idempotent; call it before loading or compiling documents that use behaviors.
pub fn register_types() {
    ferroui_xaml_interactivity::register_types();
    #[cfg(feature = "animations")]
    ferroui_xaml_behaviors_animations::register_types();
    #[cfg(feature = "interactions")]
    ferroui_xaml_interactions::register_types();
    #[cfg(feature = "custom")]
    ferroui_xaml_interactions_custom::register_types();
    #[cfg(feature = "events")]
    ferroui_xaml_interactions_events::register_types();
    #[cfg(feature = "draggable")]
    ferroui_xaml_interactions_draggable::register_types();
    #[cfg(feature = "drag-and-drop")]
    ferroui_xaml_interactions_draganddrop::register_types();
    #[cfg(feature = "responsive")]
    ferroui_xaml_interactions_responsive::register_types();
    #[cfg(feature = "reactiveui")]
    ferroui_xaml_interactions_reactiveui::register_types();
}

#[cfg(test)]
mod tests {
    // Not from the reference: the facade registers the types of its default crates.
    use ferroui_base::TypeInfo;

    #[test]
    #[cfg(all(feature = "interactions", feature = "custom"))]
    fn the_default_features_register_the_types_of_the_bundle() {
        super::register_types();

        assert!(TypeInfo::find("FerroUI.Xaml.Interactivity", "Interaction").is_some());
        assert!(TypeInfo::find("FerroUI.Xaml.Interactions.Core", "EventTriggerBehavior").is_some());
        assert!(TypeInfo::find("FerroUI.Xaml.Interactions.Custom", "AttachedToVisualTreeBehavior").is_some());
    }
}
