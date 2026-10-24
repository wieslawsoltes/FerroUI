//! Port of `Control/HideAttachedFlyoutBehavior.cs`.

use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroProperty, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::primitives::FlyoutBase;
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Hides the attached flyout of the associated control when
/// [`is_flyout_open`](Self::is_flyout_open) becomes false.
#[repr(C)]
pub struct HideAttachedFlyoutBehavior {
    base: DisposingBehavior,
}

ferro_class!(HideAttachedFlyoutBehavior: DisposingBehavior);
ferro_class_info!(HideAttachedFlyoutBehavior { new: HideAttachedFlyoutBehavior::new });
ferroui_base::ferro_impl_classes!(
    HideAttachedFlyoutBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
);

ferro_properties! {
    impl HideAttachedFlyoutBehavior {
        /// Identifies the `IsFlyoutOpen` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the reference registers this property with the
        /// button behavior that hides a flyout (`ButtonHideFlyoutBehavior`) as its owner; here it
        /// is registered on this class, which declares it (defect corrected).
        pub fn is_flyout_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<HideAttachedFlyoutBehavior, _>("IsFlyoutOpen", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for HideAttachedFlyoutBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let behavior = this.to_ref().downgrade();
            let this_object: &FerroObject = this;
            FerroObjectExtensions::get_observable(this_object, Self::is_flyout_open_property()).subscribe_fn(
                move |is_open: bool| {
                    if is_open {
                        return;
                    }
                    let Some(associated_object) =
                        behavior.upgrade().and_then(|behavior| behavior.associated_object_as::<Control>())
                    else {
                        return;
                    };
                    if let Some(flyout) = FlyoutBase::get_attached_flyout(&associated_object) {
                        flyout.hide();
                    }
                },
            )
        }
    }
}

impl HideAttachedFlyoutBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets whether the attached flyout is open; setting it to false hides the flyout.
    /// This is a property of the object model.
    pub fn is_flyout_open(&self) -> bool {
        self.get_value(Self::is_flyout_open_property())
    }

    pub fn set_is_flyout_open(&self, value: bool) {
        self.set_value(Self::is_flyout_open_property(), value)
    }
}
