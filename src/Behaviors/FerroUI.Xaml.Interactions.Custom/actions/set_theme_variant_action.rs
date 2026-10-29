//! Port of `Actions/SetThemeVariantAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::ThemeVariantScope;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Sets the requested theme variant of the [`target`](Self::target) theme variant scope (or of
/// the sender when it is one).
#[repr(C)]
pub struct SetThemeVariantAction {
    base: StyledElementAction,
}

ferro_class!(SetThemeVariantAction: StyledElementAction);
ferro_class_info!(SetThemeVariantAction {
    new: SetThemeVariantAction::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SetThemeVariantAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetThemeVariantAction {
        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn target_property() -> StyledProperty<Option<ElementRef<ThemeVariantScope>>> {
            ValueTypes::register_element_ref::<ThemeVariantScope>();
            FerroProperty::register::<SetThemeVariantAction, _>("Target", None)
        }

        /// Identifies the `ThemeVariant` property.
        pub fn theme_variant_property() -> StyledProperty<Option<ThemeVariant>> {
            FerroProperty::register::<SetThemeVariantAction, _>("ThemeVariant", None)
        }
    }
}

impl StyledElementActionImpl for SetThemeVariantAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let target = this
            .target()
            .or_else(|| sender.and_then(object_of).and_then(|sender| sender.cast::<ThemeVariantScope>()));
        let Some(target) = target else {
            return Some(Rc::new(false));
        };

        target.set_current_value(ThemeVariantScope::requested_theme_variant_property(), this.theme_variant());
        Some(Rc::new(true))
    }
}

impl SetThemeVariantAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target theme variant scope. If not set, the sender is used when it is a
    /// scope. Markup resolves a name given for it to the named element.
    pub fn target(&self) -> Option<Ref<ThemeVariantScope>> {
        ElementRef::resolve(&self.get_value(Self::target_property()))
    }

    pub fn set_target(&self, value: impl Into<Nullable<ThemeVariantScope>>) {
        self.set_value(Self::target_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the theme variant to set. This is a property of the object model.
    pub fn theme_variant(&self) -> Option<ThemeVariant> {
        self.get_value(Self::theme_variant_property())
    }

    pub fn set_theme_variant(&self, value: Option<ThemeVariant>) {
        self.set_value(Self::theme_variant_property(), value)
    }
}
