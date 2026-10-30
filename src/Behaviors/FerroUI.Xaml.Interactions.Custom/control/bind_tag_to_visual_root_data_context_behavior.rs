//! Port of `Control/BindTagToVisualRootDataContextBehavior.cs`.

use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, FerroObject, FerroObjectExtensions, FerroObjectImpl, Ref, StaticType,
    StyledElement, StyledElementImpl, Visual,
};
use ferroui_controls::{Control, TopLevel};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehavior, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::rc::Rc;

/// Binds the tag of the associated control to the data context of its top level.
#[repr(C)]
pub struct BindTagToVisualRootDataContextBehavior {
    base: DisposingBehavior,
}

ferro_class!(BindTagToVisualRootDataContextBehavior: DisposingBehavior);
ferro_class_info!(BindTagToVisualRootDataContextBehavior { new: BindTagToVisualRootDataContextBehavior::new });
ferroui_base::ferro_impl_classes!(
    BindTagToVisualRootDataContextBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl DisposingBehaviorImpl for BindTagToVisualRootDataContextBehavior {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let associated_object = this.associated_object_as::<Control>();
            let visual_root =
                TopLevel::get_top_level(associated_object.as_ref().map(|associated_object| {
                    let visual: &Visual = associated_object;
                    visual
                }));
            if let (Some(visual_root), Some(associated_object)) = (visual_root, associated_object) {
                return Self::bind_data_context_to_tag(&visual_root, &associated_object);
            }

            DisposableAction::empty()
        }
    }
}

impl BindTagToVisualRootDataContextBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingBehavior::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    // The reference throws when the source or the target is null; both are references here.
    fn bind_data_context_to_tag(source: &StyledElement, target: &Control) -> Rc<dyn IDisposable> {
        let source_object: &FerroObject = source;
        target.bind(
            Control::tag_property(),
            FerroObjectExtensions::get_observable(source_object, StyledElement::data_context_property()),
            BindingPriority::LocalValue,
        )
    }
}
