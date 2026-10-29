//! Port of `Animations/CustomStringAnimationBuilder.cs`.

use super::custom_string_animator::CustomStringAnimator;
use ferroui_base::animation::{
    Animation, Cue, IAnimationSetter, ICustomAnimator, IterationCount, KeyFrame, TimeSpan,
};
use ferroui_base::styling::Setter;
use ferroui_base::Ref;
use ferroui_controls::{Control, TextBlock};
use ferroui_xaml_behaviors_animations::IAnimationBuilder;
use std::rc::Rc;

/// Builds an animation that uses [`CustomStringAnimator`].
///
/// Deviation (DEVIATIONS.md, Behaviors gallery): the builder of the reference is an object of the
/// object model; here it is a plain type that implements the contract, which the view model hands
/// to the animation behaviors as their builder handle (`AnimationBuilderRef`).
#[derive(Default)]
pub struct CustomStringAnimationBuilder;

impl CustomStringAnimationBuilder {
    /// `CustomStringAnimationBuilder()`.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }

    /// `CreateSetter(string)`.
    fn create_setter(value: &str) -> Rc<dyn IAnimationSetter> {
        let setter: Rc<dyn IAnimationSetter> =
            Setter::new(TextBlock::text_property(), Some(value.to_string()));
        let animator: Rc<dyn ICustomAnimator> = Rc::new(CustomStringAnimator);
        Animation::set_animator(&setter, animator);
        setter
    }
}

impl IAnimationBuilder for CustomStringAnimationBuilder {
    /// `Build(Control)`.
    fn build(&self, _control: &Control) -> Option<Ref<Animation>> {
        let animation = Animation::new();
        animation.set_duration(TimeSpan::from_seconds(1.0));
        animation.set_iteration_count(IterationCount::INFINITE);
        animation
            .children()
            .add(KeyFrame::with_cue(Cue::new(0.0), [Self::create_setter("")]));
        animation.children().add(KeyFrame::with_cue(
            Cue::new(1.0),
            [Self::create_setter("0123456789")],
        ));
        Some(animation)
    }
}
