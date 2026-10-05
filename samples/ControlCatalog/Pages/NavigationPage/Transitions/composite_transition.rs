//! Port of `Pages/NavigationPage/Transitions/CompositeTransition.cs`.

use ferroui_base::animation::easings::{Easing, LinearEasing};
use ferroui_base::animation::{Animation, AnimationTask, Cue, FillMode, IPageTransition, KeyFrame, TimeSpan};
use ferroui_base::media::TranslateTransform;
use ferroui_base::styling::Setter;
use ferroui_base::threading::{CancellationToken, DispatcherTask};
use ferroui_base::{Ref, Visual};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};

/// Example custom page transition: a combined slide and fade. Both pages slide and fade simultaneously.
pub struct CompositeTransition {
    duration: Cell<TimeSpan>,
    transition_easing: RefCell<Easing>,
}

impl Default for CompositeTransition {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositeTransition {
    /// `new CompositeTransition()`.
    pub fn new() -> Self {
        Self { duration: Cell::new(TimeSpan::from_milliseconds(300.0)), transition_easing: RefCell::new(LinearEasing.into()) }
    }

    /// `new CompositeTransition(duration)`.
    pub fn with_duration(duration: TimeSpan) -> Self {
        let result = Self::new();
        result.set_duration(duration);
        result
    }

    /// The duration of the transition.
    pub fn duration(&self) -> TimeSpan {
        self.duration.get()
    }

    pub fn set_duration(&self, value: TimeSpan) {
        self.duration.set(value)
    }

    /// The easing applied to the slide and the fade.
    pub fn transition_easing(&self) -> Easing {
        self.transition_easing.borrow().clone()
    }

    pub fn set_transition_easing(&self, value: impl Into<Easing>) {
        *self.transition_easing.borrow_mut() = value.into();
    }
}

/// The common visual parent of the two elements.
///
/// # Panics
/// Panics if the elements have different parents or no parent (the
/// argument exceptions of the original).
fn get_visual_parent(from: Option<&Ref<Visual>>, to: Option<&Ref<Visual>>) -> Ref<Visual> {
    let p1 = from.or(to).expect("a transition element").visual_parent();
    if let (Some(from), Some(to)) = (from, to) {
        if from.visual_parent() != to.visual_parent() {
            panic!("Transition elements have different parents.");
        }
    }
    p1.unwrap_or_else(|| panic!("Transition elements have no parent."))
}

/// `await Task.WhenAll(tasks)`: a failure to run one of the animations
/// fails the transition.
async fn when_all(tasks: Vec<AnimationTask>) {
    if let Err(error) = AnimationTask::when_all(tasks).await {
        panic!("{error}");
    }
}

impl IPageTransition for CompositeTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let duration = self.duration();
        let transition_easing = self.transition_easing();
        let from = from.cloned();
        let to = to.cloned();

        start_async(async move {
            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let mut tasks = Vec::new();
            let parent = get_visual_parent(from.as_ref(), to.as_ref());
            let distance = if parent.bounds().width > 0.0 { parent.bounds().width } else { 500.0 };

            if let Some(from) = &from {
                let anim = Animation::new();
                anim.set_fill_mode(FillMode::Forward);
                anim.set_easing(transition_easing.clone());
                anim.set_duration(duration);
                anim.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(TranslateTransform::x_property(), 0.0) as _, Setter::new(Visual::opacity_property(), 1.0) as _]));
                anim.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(TranslateTransform::x_property(), if forward { -distance } else { distance }) as _, Setter::new(Visual::opacity_property(), 0.0) as _]));
                tasks.push(anim.run_async(from, cancellation_token.clone()));
            }

            if let Some(to) = &to {
                to.set_is_visible(true);

                let anim = Animation::new();
                anim.set_fill_mode(FillMode::Forward);
                anim.set_easing(transition_easing.clone());
                anim.set_duration(duration);
                anim.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(TranslateTransform::x_property(), if forward { distance } else { -distance }) as _, Setter::new(Visual::opacity_property(), 0.0) as _]));
                anim.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(TranslateTransform::x_property(), 0.0) as _, Setter::new(Visual::opacity_property(), 1.0) as _]));
                tasks.push(anim.run_async(to, cancellation_token.clone()));
            }

            when_all(tasks).await;

            if let Some(from) = &from {
                if !cancellation_token.is_cancellation_requested() {
                    from.set_is_visible(false);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn defaults_and_the_constructor_with_a_duration() {
        assert_eq!(TimeSpan::from_milliseconds(300.0), CompositeTransition::new().duration());
        assert_eq!(TimeSpan::from_milliseconds(120.0), CompositeTransition::with_duration(TimeSpan::from_milliseconds(120.0)).duration());
    }
}
