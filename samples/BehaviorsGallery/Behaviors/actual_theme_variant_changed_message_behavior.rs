//! Port of `Behaviors/ActualThemeVariantChangedMessageBehavior.cs` of the reference sample.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::TextBlock;
use ferroui_xaml_interactions_custom::{ActualThemeVariantChangedBehavior, ActualThemeVariantChangedBehaviorImpl};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Writes `Theme Changed` to its target text block when the actual theme variant of its theme variant scope changes
/// (`ActualThemeVariantChangedMessageBehavior : ActualThemeVariantChangedBehavior<ThemeVariantScope>`).
#[repr(C)]
pub struct ActualThemeVariantChangedMessageBehavior {
    base: ActualThemeVariantChangedBehavior,
}

ferro_class!(ActualThemeVariantChangedMessageBehavior: ActualThemeVariantChangedBehavior);
ferro_class_info!(ActualThemeVariantChangedMessageBehavior {
    new: ActualThemeVariantChangedMessageBehavior::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    ActualThemeVariantChangedMessageBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl ActualThemeVariantChangedMessageBehavior {
        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors gallery): the behavior does not own the text block,
        /// so the value is an element reference.
        pub fn target_property() -> StyledProperty<Option<ElementRef<TextBlock>>> {
            ValueTypes::register_element_ref::<TextBlock>();
            FerroProperty::register::<ActualThemeVariantChangedMessageBehavior, _>("Target", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl ActualThemeVariantChangedBehaviorImpl for ActualThemeVariantChangedMessageBehavior {
        fn on_actual_theme_variant_changed_event_override(this: &Self) -> Rc<dyn IDisposable> {
            if let Some(target) = this.target() {
                target.set_text(Some("Theme Changed"));
            }

            DisposableAction::empty()
        }
    }
}

impl ActualThemeVariantChangedMessageBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ActualThemeVariantChangedBehavior::construct(<ferroui_controls::ThemeVariantScope as StaticType>::TYPE),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `Target`: the text block the behavior writes to. Markup resolves a name given for it.
    pub fn target(&self) -> Option<Ref<TextBlock>> {
        ElementRef::resolve(&self.get_value(Self::target_property()))
    }

    pub fn set_target(&self, value: impl Into<Nullable<TextBlock>>) {
        self.set_value(Self::target_property(), ElementRef::from_nullable(value.into().0))
    }
}
