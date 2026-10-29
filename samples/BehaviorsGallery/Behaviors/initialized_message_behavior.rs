//! Port of `Behaviors/InitializedMessageBehavior.cs` of the reference sample.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::TextBlock;
use ferroui_xaml_interactions_custom::{InitializedBehavior, InitializedBehaviorImpl};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Writes `Initialized` to its target text block when its element is initialized
/// (`InitializedMessageBehavior : InitializedBehavior<StyledElement>`).
#[repr(C)]
pub struct InitializedMessageBehavior {
    base: InitializedBehavior,
}

ferro_class!(InitializedMessageBehavior: InitializedBehavior);
ferro_class_info!(InitializedMessageBehavior {
    new: InitializedMessageBehavior::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    InitializedMessageBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl InitializedMessageBehavior {
        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors gallery): the behavior does not own the text block,
        /// so the value is an element reference.
        pub fn target_property() -> StyledProperty<Option<ElementRef<TextBlock>>> {
            ValueTypes::register_element_ref::<TextBlock>();
            FerroProperty::register::<InitializedMessageBehavior, _>("Target", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl InitializedBehaviorImpl for InitializedMessageBehavior {
        fn on_initialized_event_override(this: &Self) -> Rc<dyn IDisposable> {
            if let Some(target) = this.target() {
                target.set_text(Some("Initialized"));
            }

            DisposableAction::empty()
        }
    }
}

impl InitializedMessageBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: InitializedBehavior::construct(<ferroui_base::StyledElement as StaticType>::TYPE),
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
