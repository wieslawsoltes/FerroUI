//! Port of `Behaviors/ResourcesChangedMessageBehavior.cs` of the reference sample.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::TextBlock;
use ferroui_xaml_interactions_custom::{ResourcesChangedBehavior, ResourcesChangedBehaviorImpl};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Writes `Resources changed` to its target text block when the resources of its element change
/// (`ResourcesChangedMessageBehavior : ResourcesChangedBehavior<StyledElement>`).
#[repr(C)]
pub struct ResourcesChangedMessageBehavior {
    base: ResourcesChangedBehavior,
}

ferro_class!(ResourcesChangedMessageBehavior: ResourcesChangedBehavior);
ferro_class_info!(ResourcesChangedMessageBehavior {
    new: ResourcesChangedMessageBehavior::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    ResourcesChangedMessageBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl ResourcesChangedMessageBehavior {
        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors gallery): the behavior does not own the text block,
        /// so the value is an element reference.
        pub fn target_property() -> StyledProperty<Option<ElementRef<TextBlock>>> {
            ValueTypes::register_element_ref::<TextBlock>();
            FerroProperty::register::<ResourcesChangedMessageBehavior, _>("Target", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl ResourcesChangedBehaviorImpl for ResourcesChangedMessageBehavior {
        fn on_resources_changed_event_override(this: &Self) -> Rc<dyn IDisposable> {
            if let Some(target) = this.target() {
                target.set_text(Some("Resources changed"));
            }

            DisposableAction::empty()
        }
    }
}

impl ResourcesChangedMessageBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ResourcesChangedBehavior::construct(<ferroui_base::StyledElement as StaticType>::TYPE),
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
