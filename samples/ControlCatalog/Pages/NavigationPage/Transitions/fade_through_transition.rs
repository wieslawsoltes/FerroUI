//! Port of `Pages/NavigationPage/Transitions/FadeThroughTransition.cs`.

use ferroui_base::animation::easings::{Easing, CubicEaseOut};
use ferroui_base::animation::{Animation, AnimationTask, Cue, FillMode, IPageTransition, KeyFrame, TimeSpan};
use ferroui_base::media::{ITransform, ScaleTransform};
use ferroui_base::styling::Setter;
use ferroui_base::threading::{CancellationToken, DispatcherTask};
use ferroui_base::{Ref, RelativePoint, Visual};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Example custom page transition: a fade through with a scale. The outgoing page fades out while scaling down; the incoming page fades in while scaling up, producing a smooth zoom-like transition.
pub struct FadeThroughTransition {
    duration: Cell<TimeSpan>,
    fade_easing: RefCell<Easing>,
}

impl Default for FadeThroughTransition {
    fn default() -> Self {
        Self::new()
    }
}

impl FadeThroughTransition {
    /// `new FadeThroughTransition()`.
    pub fn new() -> Self {
        Self { duration: Cell::new(TimeSpan::from_milliseconds(300.0)), fade_easing: RefCell::new(CubicEaseOut.into()) }
    }

    /// `new FadeThroughTransition(duration)`.
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

    /// The easing applied to both pages.
    pub fn fade_easing(&self) -> Easing {
        self.fade_easing.borrow().clone()
    }

    pub fn set_fade_easing(&self, value: impl Into<Easing>) {
        *self.fade_easing.borrow_mut() = value.into();
    }
}

/// `new ScaleTransform(1, 1)` as the render transform of `visual`.
fn set_unit_scale(visual: &Ref<Visual>) {
    let transform: Rc<dyn ITransform> = (&ScaleTransform::with_scale(1.0, 1.0)).into();
    visual.set_render_transform(Some(transform));
}

/// `await Task.WhenAll(tasks)`: a failure to run one of the animations
/// fails the transition.
async fn when_all(tasks: Vec<AnimationTask>) {
    if let Err(error) = AnimationTask::when_all(tasks).await {
        panic!("{error}");
    }
}

impl IPageTransition for FadeThroughTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let duration = self.duration();
        let fade_easing = self.fade_easing();
        let from = from.cloned();
        let to = to.cloned();

        start_async(async move {
            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let mut tasks = Vec::new();

            if let Some(from) = &from {
                from.set_render_transform_origin(RelativePoint::CENTER);
                set_unit_scale(from);
            }

            if let Some(to) = &to {
                to.set_render_transform_origin(RelativePoint::CENTER);
                set_unit_scale(to);
                to.set_opacity(0.0);
            }

            if let Some(from) = &from {
                let scale = if forward { 0.92 } else { 1.08 };
                let out_anim = Animation::new();
                out_anim.set_fill_mode(FillMode::Forward);
                out_anim.set_easing(fade_easing.clone());
                out_anim.set_duration(duration);
                out_anim.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(Visual::opacity_property(), 1.0) as _, Setter::new(ScaleTransform::scale_x_property(), 1.0) as _, Setter::new(ScaleTransform::scale_y_property(), 1.0) as _]));
                out_anim.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(Visual::opacity_property(), 0.0) as _, Setter::new(ScaleTransform::scale_x_property(), scale) as _, Setter::new(ScaleTransform::scale_y_property(), scale) as _]));
                tasks.push(out_anim.run_async(from, cancellation_token.clone()));
            }

            if let Some(to) = &to {
                to.set_is_visible(true);

                let scale = if forward { 1.08 } else { 0.92 };
                let in_anim = Animation::new();
                in_anim.set_fill_mode(FillMode::Forward);
                in_anim.set_easing(fade_easing.clone());
                in_anim.set_duration(duration);
                in_anim.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(Visual::opacity_property(), 0.0) as _, Setter::new(ScaleTransform::scale_x_property(), scale) as _, Setter::new(ScaleTransform::scale_y_property(), scale) as _]));
                in_anim.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(Visual::opacity_property(), 1.0) as _, Setter::new(ScaleTransform::scale_x_property(), 1.0) as _, Setter::new(ScaleTransform::scale_y_property(), 1.0) as _]));
                tasks.push(in_anim.run_async(to, cancellation_token.clone()));
            }

            when_all(tasks).await;

            if let Some(to) = &to {
                if !cancellation_token.is_cancellation_requested() {
                    to.set_opacity(1.0);
                    to.set_render_transform(None);
                }
            }

            if let Some(from) = &from {
                if !cancellation_token.is_cancellation_requested() {
                    from.set_is_visible(false);
                }
                from.set_opacity(1.0);
                from.set_render_transform(None);
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
        assert_eq!(TimeSpan::from_milliseconds(300.0), FadeThroughTransition::new().duration());
        assert_eq!(TimeSpan::from_milliseconds(75.0), FadeThroughTransition::with_duration(TimeSpan::from_milliseconds(75.0)).duration());
    }
}
