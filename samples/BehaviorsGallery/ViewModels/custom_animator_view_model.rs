//! Port of `ViewModels/CustomAnimatorViewModel.cs`.

use super::ViewModelBase;
use crate::animations::custom_string_animation_builder::CustomStringAnimationBuilder;
use ferroui_base::ferro_markup_type;
use ferroui_reactiveui::reactive_object;
use ferroui_xaml_behaviors_animations::IAnimationBuilder;
use ferroui_xaml_interactions_custom::AnimationBuilderRef;
use std::rc::Rc;

/// The view model of the pages of the animation builder.
pub struct CustomAnimatorViewModel {
    base: ViewModelBase,
    animation_builder: AnimationBuilderRef,
}
reactive_object!(CustomAnimatorViewModel, base);

impl PartialEq for CustomAnimatorViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl CustomAnimatorViewModel {
    /// `CustomAnimatorViewModel()`.
    pub fn new() -> Rc<Self> {
        let builder: Rc<dyn IAnimationBuilder> = CustomStringAnimationBuilder::new();
        Rc::new(Self {
            base: ViewModelBase::new(),
            animation_builder: AnimationBuilderRef::new(builder),
        })
    }

    /// Gets the builder used by the sample to create the animation.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors gallery): the property is the builder handle the
    /// animation behaviors take (`AnimationBuilderRef`), not the type of the builder.
    pub fn animation_builder(&self) -> Option<AnimationBuilderRef> {
        Some(self.animation_builder.clone())
    }
}

type Vm = Rc<CustomAnimatorViewModel>;

ferro_markup_type!(class CustomAnimatorViewModel {
    this: Rc<CustomAnimatorViewModel>,
    handles: [CustomAnimatorViewModel, Rc<CustomAnimatorViewModel>, Option<Rc<CustomAnimatorViewModel>>],
    constructors: [() => CustomAnimatorViewModel::new],
    properties: [
        AnimationBuilder: Option<AnimationBuilderRef> { get: |this: &Vm| this.animation_builder() },
    ],
    notify_property_changed: CustomAnimatorViewModel,
});
