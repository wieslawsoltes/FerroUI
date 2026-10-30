//! Port of `ExecuteCommand/Core/ExecuteCommandOnKeyBehaviorBase.cs`.

use crate::{
    AttachedToVisualTreeBehaviorImpl, ExecuteCommandBehaviorBaseImpl, ExecuteCommandRoutedEventBehaviorBase,
};
use ferroui_base::input::{Key, KeyGesture};
use ferroui_base::{ferro_class, ferro_properties, FerroObjectImpl, FerroProperty, StyledElementImpl, StyledProperty};
use ferroui_xaml_interactivity::{DisposingBehaviorImpl, StyledElementBehaviorImpl};

/// The base class of the behaviors that execute a command on a key event: the key or the key
/// gesture that executes it.
///
/// The class is abstract.
#[repr(C)]
pub struct ExecuteCommandOnKeyBehaviorBase {
    base: ExecuteCommandRoutedEventBehaviorBase,
}

ferro_class!(ExecuteCommandOnKeyBehaviorBase: ExecuteCommandRoutedEventBehaviorBase);
ferroui_base::ferro_impl_classes!(
    ExecuteCommandOnKeyBehaviorBase: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
    AttachedToVisualTreeBehaviorImpl,
    ExecuteCommandBehaviorBaseImpl,
);

ferro_properties! {
    impl ExecuteCommandOnKeyBehaviorBase {
        /// Identifies the `Key` property.
        pub fn key_property() -> StyledProperty<Option<Key>> {
            FerroProperty::register::<ExecuteCommandOnKeyBehaviorBase, _>("Key", None)
        }

        /// Identifies the `Gesture` property.
        pub fn gesture_property() -> StyledProperty<Option<KeyGesture>> {
            FerroProperty::register::<ExecuteCommandOnKeyBehaviorBase, _>("Gesture", None)
        }
    }
}

impl ExecuteCommandOnKeyBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ExecuteCommandRoutedEventBehaviorBase::construct() }
    }

    /// Gets or sets the key that executes the command.
    pub fn key(&self) -> Option<Key> {
        self.get_value(Self::key_property())
    }

    pub fn set_key(&self, value: Option<Key>) {
        self.set_value(Self::key_property(), value)
    }

    /// Gets or sets the key gesture that executes the command.
    pub fn gesture(&self) -> Option<KeyGesture> {
        self.get_value(Self::gesture_property())
    }

    pub fn set_gesture(&self, value: Option<KeyGesture>) {
        self.set_value(Self::gesture_property(), value)
    }
}
