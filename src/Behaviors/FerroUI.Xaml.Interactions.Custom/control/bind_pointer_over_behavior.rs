//! Port of `Control/BindPointerOverBehavior.cs`.

use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElement;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty, StyledPropertyOptions,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Binds [`is_pointer_over`](Self::is_pointer_over) to whether the pointer is over the associated
/// control.
#[repr(C)]
pub struct BindPointerOverBehavior {
    base: DisposingBehavior,
}

ferro_class!(BindPointerOverBehavior: DisposingBehavior);
ferro_class_info!(BindPointerOverBehavior { new: BindPointerOverBehavior::new });
ferroui_base::ferro_impl_classes!(BindPointerOverBehavior: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl BindPointerOverBehavior {
        /// Identifies the `IsPointerOver` property.
        pub fn is_pointer_over_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<BindPointerOverBehavior, _>(
                "IsPointerOver",
                StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay),
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for BindPointerOverBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(control) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let behavior = this.to_ref().downgrade();
            let subscription = control.property_changed(move |e: &FerroPropertyChangedEventArgs<'_>| {
                if e.property() == InputElement::is_pointer_over_property().as_property() {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.set_is_pointer_over(e.new_value().downcast_ref::<bool>() == Some(&true));
                    }
                }
            });

            let behavior = this.to_ref().downgrade();
            DisposableAction::create(move || {
                subscription.dispose();
                if let Some(behavior) = behavior.upgrade() {
                    behavior.set_is_pointer_over(false);
                }
            })
        }
    }
}

impl BindPointerOverBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets whether the pointer is over the associated control. This is a property of
    /// the object model, bound in both directions by default.
    pub fn is_pointer_over(&self) -> bool {
        self.get_value(Self::is_pointer_over_property())
    }

    pub fn set_is_pointer_over(&self, value: bool) {
        self.set_value(Self::is_pointer_over_property(), value)
    }
}
