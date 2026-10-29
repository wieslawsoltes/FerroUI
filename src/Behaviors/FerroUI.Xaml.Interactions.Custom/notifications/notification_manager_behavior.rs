//! Port of `Notifications/NotificationManagerBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Control, INotificationManager, TopLevel, WindowNotificationManager};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Provides a [`INotificationManager`] for the associated [`Control`].
#[repr(C)]
pub struct NotificationManagerBehavior {
    base: AttachedToVisualTreeBehavior,
}

ferro_class!(NotificationManagerBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(NotificationManagerBehavior { new: NotificationManagerBehavior::new });
ferroui_base::ferro_impl_classes!(
    NotificationManagerBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl NotificationManagerBehavior {
        /// Identifies the `NotificationManager` property.
        pub fn notification_manager_property() -> StyledProperty<Option<Rc<dyn INotificationManager>>> {
            FerroProperty::register::<NotificationManagerBehavior, _>("NotificationManager", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for NotificationManagerBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let associated_object = this.associated_object_as::<Control>();
            if let Some(top_level) = associated_object.as_ref().and_then(|control| control.cast::<TopLevel>()) {
                this.set_notification_manager(Some(
                    WindowNotificationManager::with_host(Some(&top_level)).as_notification_manager(),
                ));
            } else if let Some(visual_root_top_level) =
                TopLevel::get_top_level(associated_object.as_deref().map(crate::part_c::control_as_visual))
            {
                this.set_notification_manager(Some(
                    WindowNotificationManager::with_host(Some(&visual_root_top_level)).as_notification_manager(),
                ));
            }

            let behavior = this.to_ref().downgrade();
            DisposableAction::create(move || {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.set_notification_manager(None);
                }
            })
        }
    }
}

impl NotificationManagerBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the [`INotificationManager`] instance. This is a property of the object model.
    pub fn notification_manager(&self) -> Option<Rc<dyn INotificationManager>> {
        self.get_value(Self::notification_manager_property())
    }

    fn set_notification_manager(&self, value: Option<Rc<dyn INotificationManager>>) {
        self.set_value(Self::notification_manager_property(), value)
    }
}
