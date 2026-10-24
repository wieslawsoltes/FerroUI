//! Port of `Behaviors/LoadedMessageBehavior.cs` of the reference sample.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::TextBlock;
use ferroui_xaml_interactions_custom::{LoadedBehavior, LoadedBehaviorImpl};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Writes `Loaded` to its target text block when its control is loaded
/// (`LoadedMessageBehavior : LoadedBehavior<Control>`).
#[repr(C)]
pub struct LoadedMessageBehavior {
    base: LoadedBehavior,
}

ferro_class!(LoadedMessageBehavior: LoadedBehavior);
ferro_class_info!(LoadedMessageBehavior {
    new: LoadedMessageBehavior::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    LoadedMessageBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl LoadedMessageBehavior {
        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors gallery): the behavior does not own the text block,
        /// so the value is an element reference.
        pub fn target_property() -> StyledProperty<Option<ElementRef<TextBlock>>> {
            ValueTypes::register_element_ref::<TextBlock>();
            FerroProperty::register::<LoadedMessageBehavior, _>("Target", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl LoadedBehaviorImpl for LoadedMessageBehavior {
        fn on_loaded_override(this: &Self) -> Rc<dyn IDisposable> {
            if let Some(target) = this.target() {
                target.set_text(Some("Loaded"));
            }

            DisposableAction::empty()
        }
    }
}

impl LoadedMessageBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: LoadedBehavior::construct(<ferroui_controls::Control as StaticType>::TYPE),
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
