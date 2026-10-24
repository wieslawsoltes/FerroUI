//! Port of `ThemeVariant/ThemeVariantTrigger.cs`.

use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElement, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{
    Interaction, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementTriggerOfT,
};
use std::rc::Rc;

/// Executes actions when the actual theme variant of the associated element matches the specified
/// [`theme_variant`](Self::theme_variant).
#[repr(C)]
pub struct ThemeVariantTrigger {
    base: StyledElementTriggerOfT,
}

ferro_class!(ThemeVariantTrigger: StyledElementTriggerOfT);
ferro_class_info!(ThemeVariantTrigger { new: ThemeVariantTrigger::new });
ferroui_base::ferro_impl_classes!(ThemeVariantTrigger: StyledElementImpl);

ferro_properties! {
    impl ThemeVariantTrigger {
        /// Identifies the `ThemeVariant` property.
        pub fn theme_variant_property() -> StyledProperty<Option<ThemeVariant>> {
            FerroProperty::register::<ThemeVariantTrigger, _>("ThemeVariant", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ThemeVariantTrigger {
        fn on_attached(this: &Self) {
            Self::parent_on_attached(this);
            this.evaluate();
        }

        fn on_actual_theme_variant_changed_event(this: &Self) {
            this.evaluate();
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ThemeVariantTrigger {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::theme_variant_property().as_property() {
                this.evaluate();
            }
        }
    }
}

impl ThemeVariantTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<StyledElement as StaticType>::TYPE) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the theme variant to watch for. This is a property of the object model.
    pub fn theme_variant(&self) -> Option<ThemeVariant> {
        self.get_value(Self::theme_variant_property())
    }

    pub fn set_theme_variant(&self, value: Option<ThemeVariant>) {
        self.set_value(Self::theme_variant_property(), value)
    }

    fn evaluate(&self) {
        let Some(associated_object) = self.associated_object_as::<StyledElement>() else {
            return;
        };
        if !self.is_enabled() {
            return;
        }

        if associated_object.actual_theme_variant() == self.theme_variant() {
            let trigger: Ref<ThemeVariantTrigger> = self.to_ref();
            Dispatcher::ui_thread().post_local(move || trigger.execute(None), DispatcherPriority::DEFAULT);
        }
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), parameter);
    }
}
