//! Port of `ItemsControl/ItemsControlContainerEventsBehavior.cs`.

use ferroui_base::reactive::IDisposable;
use ferroui_base::{ferro_class, FerroObjectImpl, StaticType, StyledElementImpl};
use ferroui_controls::{
    ContainerClearingEventArgs, ContainerIndexChangedEventArgs, ContainerPreparedEventArgs, ItemsControl,
};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Base class that exposes container related events from an [`ItemsControl`].
///
/// The class is abstract: a deriving class overrides the members of
/// [`ItemsControlContainerEventsBehaviorImpl`] it needs.
#[repr(C)]
pub struct ItemsControlContainerEventsBehavior {
    base: DisposingBehavior,
}

// Deviation (DEVIATIONS.md, Behaviors): the events of the items control are plain events whose
// handlers receive only the args; the `object? sender` of the members is the items control the
// handlers are attached to (`&ItemsControl`).
ferro_class! {
    ItemsControlContainerEventsBehavior: DisposingBehavior,
    virtuals ItemsControlContainerEventsBehaviorImpl: DisposingBehaviorImpl {
        /// Called before the container is prepared.
        fn on_preparing_container(this, sender: &ItemsControl, e: &ContainerPreparedEventArgs);
        /// Called after the container has been prepared.
        fn on_container_prepared(this, sender: &ItemsControl, e: &ContainerPreparedEventArgs);
        /// Called when the index of a container has changed.
        fn on_container_index_changed(this, sender: &ItemsControl, e: &ContainerIndexChangedEventArgs);
        /// Called when a container is being cleared.
        fn on_container_clearing(this, sender: &ItemsControl, e: &ContainerClearingEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(
    ItemsControlContainerEventsBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
);

impl ItemsControlContainerEventsBehaviorImpl for ItemsControlContainerEventsBehavior {
    fn on_preparing_container(_this: &Self, _sender: &ItemsControl, _e: &ContainerPreparedEventArgs) {}

    fn on_container_prepared(_this: &Self, _sender: &ItemsControl, _e: &ContainerPreparedEventArgs) {}

    fn on_container_index_changed(_this: &Self, _sender: &ItemsControl, _e: &ContainerIndexChangedEventArgs) {}

    fn on_container_clearing(_this: &Self, _sender: &ItemsControl, _e: &ContainerClearingEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for ItemsControlContainerEventsBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(items_control) = this.associated_object_as::<ItemsControl>() else {
                return DisposableAction::empty();
            };

            let behavior = this.to_ref().downgrade();
            let control = items_control.downgrade();
            let preparing_container = items_control.preparing_container(move |e: &ContainerPreparedEventArgs| {
                if let (Some(behavior), Some(control)) = (behavior.upgrade(), control.upgrade()) {
                    behavior.on_preparing_container(&control, e);
                }
            });

            let behavior = this.to_ref().downgrade();
            let control = items_control.downgrade();
            let container_prepared = items_control.container_prepared(move |e: &ContainerPreparedEventArgs| {
                if let (Some(behavior), Some(control)) = (behavior.upgrade(), control.upgrade()) {
                    behavior.on_container_prepared(&control, e);
                }
            });

            let behavior = this.to_ref().downgrade();
            let control = items_control.downgrade();
            let container_index_changed =
                items_control.container_index_changed(move |e: &ContainerIndexChangedEventArgs| {
                    if let (Some(behavior), Some(control)) = (behavior.upgrade(), control.upgrade()) {
                        behavior.on_container_index_changed(&control, e);
                    }
                });

            let behavior = this.to_ref().downgrade();
            let control = items_control.downgrade();
            let container_clearing = items_control.container_clearing(move |e: &ContainerClearingEventArgs| {
                if let (Some(behavior), Some(control)) = (behavior.upgrade(), control.upgrade()) {
                    behavior.on_container_clearing(&control, e);
                }
            });

            // Deviation (DEVIATIONS.md, Behaviors): the reference subscribes its handler of
            // `PreparingContainer` again when it is disposed (`+=` where `-=` is meant); here
            // every handler is removed (defect corrected).
            DisposableAction::create(move || {
                preparing_container.dispose();
                container_prepared.dispose();
                container_index_changed.dispose();
                container_clearing.dispose();
            })
        }
    }
}

impl ItemsControlContainerEventsBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingBehavior::construct(<ItemsControl as StaticType>::TYPE) }
    }
}
