//! Port of `Button/ButtonHideFlyoutBehavior.cs`.

use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObject, FerroObjectExtensions,
    FerroObjectImpl, FerroProperty, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::Button;
use ferroui_xaml_interactivity::{DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Behavior that hides a button's flyout when [`is_flyout_open`](Self::is_flyout_open) becomes
/// false.
#[repr(C)]
pub struct ButtonHideFlyoutBehavior {
    base: DisposingBehavior,
}

ferro_class!(ButtonHideFlyoutBehavior: DisposingBehavior);
ferro_class_info!(ButtonHideFlyoutBehavior { new: ButtonHideFlyoutBehavior::new });
ferroui_base::ferro_impl_classes!(
    ButtonHideFlyoutBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
);

ferro_properties! {
    impl ButtonHideFlyoutBehavior {
        /// Gets or sets a value indicating whether the flyout is open.
        pub fn is_flyout_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<ButtonHideFlyoutBehavior, _>("IsFlyoutOpen", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for ButtonHideFlyoutBehavior {
        /// Subscribes to [`is_flyout_open`](Self::is_flyout_open) changes.
        ///
        /// Returns a disposable that removes the subscription.
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            // The behavior is held weakly: the subscription lives in the behavior.
            let behavior = this.to_ref().downgrade();
            let object: &FerroObject = this;
            FerroObjectExtensions::get_observable(object, Self::is_flyout_open_property()).subscribe_fn(
                move |is_open: bool| {
                    if is_open {
                        return;
                    }

                    let flyout = behavior
                        .upgrade()
                        .and_then(|behavior| behavior.associated_object_as::<Button>())
                        .and_then(|button| button.flyout());
                    if let Some(flyout) = flyout {
                        flyout.hide();
                    }
                },
            )
        }
    }
}

impl ButtonHideFlyoutBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingBehavior::construct(<Button as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Whether the flyout is open.
    pub fn is_flyout_open(&self) -> bool {
        self.get_value(Self::is_flyout_open_property())
    }

    pub fn set_is_flyout_open(&self, value: bool) {
        self.set_value(Self::is_flyout_open_property(), value)
    }
}
