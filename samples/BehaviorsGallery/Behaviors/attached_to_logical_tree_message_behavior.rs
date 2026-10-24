//! Port of `Behaviors/AttachedToLogicalTreeMessageBehavior.cs` of the reference sample.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StaticType, StyledElementImpl, StyledProperty,
};
use ferroui_controls::TextBlock;
use ferroui_xaml_interactions_custom::{AttachedToLogicalTreeBehavior, AttachedToLogicalTreeBehaviorImpl};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Writes `Attached` to its target text block when its element is attached to the logical tree
/// (`AttachedToLogicalTreeMessageBehavior : AttachedToLogicalTreeBehavior<StyledElement>`).
#[repr(C)]
pub struct AttachedToLogicalTreeMessageBehavior {
    base: AttachedToLogicalTreeBehavior,
}

ferro_class!(AttachedToLogicalTreeMessageBehavior: AttachedToLogicalTreeBehavior);
ferro_class_info!(AttachedToLogicalTreeMessageBehavior {
    new: AttachedToLogicalTreeMessageBehavior::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    AttachedToLogicalTreeMessageBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl AttachedToLogicalTreeMessageBehavior {
        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors gallery): the behavior does not own the text block,
        /// so the value is an element reference.
        pub fn target_property() -> StyledProperty<Option<ElementRef<TextBlock>>> {
            ValueTypes::register_element_ref::<TextBlock>();
            FerroProperty::register::<AttachedToLogicalTreeMessageBehavior, _>("Target", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToLogicalTreeBehaviorImpl for AttachedToLogicalTreeMessageBehavior {
        fn on_attached_to_logical_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            if let Some(target) = this.target() {
                target.set_text(Some("Attached"));
            }

            DisposableAction::empty()
        }
    }
}

impl AttachedToLogicalTreeMessageBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: AttachedToLogicalTreeBehavior::construct(<ferroui_base::StyledElement as StaticType>::TYPE),
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
