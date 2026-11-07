//! Port of `Pages/NavigationPage/Transitions/ParallaxSlideTransition.cs`.

use ferroui_base::animation::easings::{Easing, CubicEaseOut};
use ferroui_base::animation::{Animation, AnimationTask, Cue, FillMode, IPageTransition, KeyFrame, SignedDuration};
use ferroui_base::media::TranslateTransform;
use ferroui_base::styling::Setter;
use ferroui_base::threading::{CancellationToken, JoinError, JoinHandle};
use ferroui_base::{Ref, Visual};
use mini_mvvm::start_async_on_result;
use std::cell::{Cell, RefCell};

/// Example custom page transition: a parallax slide. The incoming page slides full-width from the right while the outgoing page shifts about 30% to the left with a subtle opacity fade, producing a depth-layered push effect.
pub struct ParallaxSlideTransition {
    duration: Cell<SignedDuration>,
    slide_easing: RefCell<Easing>,
}

impl Default for ParallaxSlideTransition {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallaxSlideTransition {
    /// `new ParallaxSlideTransition()`.
    pub fn new() -> Self {
        Self { duration: Cell::new(SignedDuration::from_milliseconds(350.0)), slide_easing: RefCell::new(CubicEaseOut.into()) }
    }

    /// `new ParallaxSlideTransition(duration)`.
    pub fn with_duration(duration: SignedDuration) -> Self {
        let result = Self::new();
        result.set_duration(duration);
        result
    }

    /// The duration of the transition.
    pub fn duration(&self) -> SignedDuration {
        self.duration.get()
    }

    pub fn set_duration(&self, value: SignedDuration) {
        self.duration.set(value)
    }

    /// The easing applied to both pages.
    pub fn slide_easing(&self) -> Easing {
        self.slide_easing.borrow().clone()
    }

    pub fn set_slide_easing(&self, value: impl Into<Easing>) {
        *self.slide_easing.borrow_mut() = value.into();
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
async fn when_all(tasks: Vec<AnimationTask>) -> Result<(), JoinError> {
    AnimationTask::when_all(tasks).await.map_err(|error| JoinError::Panicked(Box::new(error.to_string())))
}

impl IPageTransition for ParallaxSlideTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> JoinHandle<()> {
        let duration = self.duration();
        let slide_easing = self.slide_easing();
        let from = from.cloned();
        let to = to.cloned();

        start_async_on_result(async move {
            if cancellation_token.is_cancelled() {
                return Ok(());
            }

            let mut tasks = Vec::new();
            let parent = get_visual_parent(from.as_ref(), to.as_ref());
            let distance = if parent.bounds().width > 0.0 { parent.bounds().width } else { 500.0 };

            if let Some(from) = &from {
                let anim = Animation::new();
                anim.set_fill_mode(FillMode::Forward);
                anim.set_easing(slide_easing.clone());
                anim.set_duration(duration);
                anim.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(TranslateTransform::x_property(), 0.0) as _, Setter::new(Visual::opacity_property(), 1.0) as _]));
                anim.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(TranslateTransform::x_property(), if forward { -distance * 0.3 } else { distance }) as _, Setter::new(Visual::opacity_property(), if forward { 0.7 } else { 1.0 }) as _]));
                tasks.push(anim.run_async(from, cancellation_token.clone()));
            }

            if let Some(to) = &to {
                to.set_is_visible(true);

                let anim = Animation::new();
                anim.set_fill_mode(FillMode::Forward);
                anim.set_easing(slide_easing.clone());
                anim.set_duration(duration);
                anim.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(TranslateTransform::x_property(), if forward { distance } else { -distance * 0.3 }) as _, Setter::new(Visual::opacity_property(), if forward { 1.0 } else { 0.7 }) as _]));
                anim.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(TranslateTransform::x_property(), 0.0) as _, Setter::new(Visual::opacity_property(), 1.0) as _]));
                tasks.push(anim.run_async(to, cancellation_token.clone()));
            }

            when_all(tasks).await?;

            if let Some(from) = &from {
                if !cancellation_token.is_cancelled() {
                    from.set_is_visible(false);
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn defaults_and_the_constructor_with_a_duration() {
        assert_eq!(SignedDuration::from_milliseconds(350.0), ParallaxSlideTransition::new().duration());
        assert_eq!(SignedDuration::from_milliseconds(90.0), ParallaxSlideTransition::with_duration(SignedDuration::from_milliseconds(90.0)).duration());
    }
}
