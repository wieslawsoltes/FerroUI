//! Port of `Show/ShowOnKeyDownBehavior.cs`.

use crate::{AttachedToVisualTreeBehaviorImpl, ShowBehaviorBase};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyGesture};
use ferroui_base::interactivity::Interactive;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// A behavior that allows to show control on key down event: the [`key`](Self::key) or the
/// [`gesture`](Self::gesture) shows it.
#[repr(C)]
pub struct ShowOnKeyDownBehavior {
    base: ShowBehaviorBase,
}

ferro_class!(ShowOnKeyDownBehavior: ShowBehaviorBase);
ferro_class_info!(ShowOnKeyDownBehavior { new: ShowOnKeyDownBehavior::new });
ferroui_base::ferro_impl_classes!(
    ShowOnKeyDownBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl ShowOnKeyDownBehavior {
        /// Identifies the `Key` property.
        pub fn key_property() -> StyledProperty<Option<Key>> {
            FerroProperty::register::<ShowOnKeyDownBehavior, _>("Key", None)
        }

        /// Identifies the `Gesture` property: the key gesture used to trigger the behavior.
        pub fn gesture_property() -> StyledProperty<Option<KeyGesture>> {
            FerroProperty::register::<ShowOnKeyDownBehavior, _>("Gesture", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ShowOnKeyDownBehavior {
        /// Called when the behavior is attached to the visual tree.
        ///
        /// Returns a disposable that removes the event handler.
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let behavior = this.to_ref().downgrade();
            associated_object.add_disposable_handler_with(
                InputElement::key_down_event(),
                move |_sender: &Interactive, e: &KeyEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.associated_object_key_down(e);
                    }
                },
                this.event_routing_strategy(),
                false,
            )
        }
    }
}

impl ShowOnKeyDownBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ShowBehaviorBase::construct() }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the key. This is a property of the object model.
    pub fn key(&self) -> Option<Key> {
        self.get_value(Self::key_property())
    }

    pub fn set_key(&self, value: Option<Key>) {
        self.set_value(Self::key_property(), value)
    }

    /// Gets or sets the key gesture used to trigger the behavior.
    pub fn gesture(&self) -> Option<KeyGesture> {
        self.get_value(Self::gesture_property())
    }

    pub fn set_gesture(&self, value: Option<KeyGesture>) {
        self.set_value(Self::gesture_property(), value)
    }

    fn associated_object_key_down(&self, e: &KeyEventArgs) {
        let have_key = self.key().is_some_and(|key| e.key == key);
        let have_gesture = self.gesture().is_some_and(|gesture| gesture.matches(Some(e)));

        if !have_key && !have_gesture {
            return;
        }

        self.show();
    }
}
