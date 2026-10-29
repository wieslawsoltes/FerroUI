//! Port of `Focus/FocusBehaviorBase.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::input::{KeyModifiers, NavigationMethod};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_properties, FerroObjectImpl, FerroProperty, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposingBehaviorImpl, StyledElementBehaviorImpl};

/// The base class of the behaviors that focus their associated control: the navigation method
/// and the key modifiers the focus is given with.
///
/// The class is abstract: a deriving class implements
/// [`AttachedToVisualTreeBehaviorImpl::on_attached_to_visual_tree_override`] and calls
/// [`focus`](FocusBehaviorBaseImpl::focus).
#[repr(C)]
pub struct FocusBehaviorBase {
    base: AttachedToVisualTreeBehavior,
}

ferro_class! {
    FocusBehaviorBase: AttachedToVisualTreeBehavior,
    virtuals FocusBehaviorBaseImpl: AttachedToVisualTreeBehaviorImpl {
        /// Focuses the associated control, in a job of the dispatcher.
        ///
        /// Returns true when the focus is requested; false when the behavior is disabled.
        fn focus(this) -> bool;
    }
}

ferroui_base::ferro_impl_classes!(
    FocusBehaviorBase: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
    AttachedToVisualTreeBehaviorImpl,
);

ferro_properties! {
    impl FocusBehaviorBase {
        /// Identifies the `NavigationMethod` property.
        pub fn navigation_method_property() -> StyledProperty<NavigationMethod> {
            FerroProperty::register::<FocusBehaviorBase, _>("NavigationMethod", NavigationMethod::Unspecified)
        }

        /// Identifies the `KeyModifiers` property.
        pub fn key_modifiers_property() -> StyledProperty<KeyModifiers> {
            FerroProperty::register::<FocusBehaviorBase, _>("KeyModifiers", KeyModifiers::NONE)
        }
    }
}

impl FocusBehaviorBaseImpl for FocusBehaviorBase {
    fn focus(this: &Self) -> bool {
        if !this.is_enabled() {
            return false;
        }

        let behavior = this.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(behavior) = behavior.upgrade() else {
                    return;
                };
                if let Some(associated_object) = behavior.associated_object_as::<Control>() {
                    associated_object.focus_with(behavior.navigation_method(), behavior.key_modifiers());
                }
            },
            DispatcherPriority::DEFAULT,
        );

        true
    }
}

impl FocusBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: AttachedToVisualTreeBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Gets or sets the navigation method the focus is given with. This is a property of the
    /// object model.
    pub fn navigation_method(&self) -> NavigationMethod {
        self.get_value(Self::navigation_method_property())
    }

    pub fn set_navigation_method(&self, value: NavigationMethod) {
        self.set_value(Self::navigation_method_property(), value)
    }

    /// Gets or sets the key modifiers the focus is given with. This is a property of the object
    /// model.
    pub fn key_modifiers(&self) -> KeyModifiers {
        self.get_value(Self::key_modifiers_property())
    }

    pub fn set_key_modifiers(&self, value: KeyModifiers) {
        self.set_value(Self::key_modifiers_property(), value)
    }
}
