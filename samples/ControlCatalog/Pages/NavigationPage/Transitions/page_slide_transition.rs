//! Port of `Pages/NavigationPage/Transitions/PageSlideTransition.cs`.

use ferroui_base::animation::easings::{Easing, LinearEasing};
use ferroui_base::animation::{Animation, AnimationTask, Cue, FillMode, IPageTransition, KeyFrame, TimeSpan};
use ferroui_base::media::TranslateTransform;
use ferroui_base::styling::Setter;
use ferroui_base::threading::{CancellationToken, DispatcherTask};
use ferroui_base::{Ref, StyledProperty, Visual};
use mini_mvvm::start_async;
use std::cell::{Cell, RefCell};

/// `PageSlideTransition.SlideAxis`: the axis on which the slide occurs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PageSlideTransitionAxis {
    #[default]
    Horizontal = 0,
    Vertical = 1,
}

/// Transitions between two pages by sliding them horizontally or vertically.
pub struct PageSlideTransition {
    duration: Cell<TimeSpan>,
    axis: Cell<PageSlideTransitionAxis>,
    slide_easing: RefCell<Easing>,
}

impl Default for PageSlideTransition {
    fn default() -> Self {
        Self::new()
    }
}

impl PageSlideTransition {
    /// `new PageSlideTransition()`.
    pub fn new() -> Self {
        Self {
            duration: Cell::new(TimeSpan::from_milliseconds(300.0)),
            axis: Cell::new(PageSlideTransitionAxis::Horizontal),
            slide_easing: RefCell::new(LinearEasing.into()),
        }
    }

    /// `new PageSlideTransition(duration, axis)`.
    pub fn with_duration(duration: TimeSpan, axis: PageSlideTransitionAxis) -> Self {
        let result = Self::new();
        result.set_duration(duration);
        result.set_axis(axis);
        result
    }

    /// The axis on which the slide occurs.
    pub fn axis(&self) -> PageSlideTransitionAxis {
        self.axis.get()
    }

    pub fn set_axis(&self, value: PageSlideTransitionAxis) {
        self.axis.set(value)
    }

    /// The duration of the transition.
    pub fn duration(&self) -> TimeSpan {
        self.duration.get()
    }

    pub fn set_duration(&self, value: TimeSpan) {
        self.duration.set(value)
    }

    /// The easing applied to both pages.
    pub fn slide_easing(&self) -> Easing {
        self.slide_easing.borrow().clone()
    }

    pub fn set_slide_easing(&self, value: impl Into<Easing>) {
        *self.slide_easing.borrow_mut() = value.into();
    }

    fn start_axis(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
        prop: &'static StyledProperty<f64>,
        get_distance: impl FnOnce() -> f64 + 'static,
    ) -> DispatcherTask<()> {
        let duration = self.duration();
        let slide_easing = self.slide_easing();
        let from = from.cloned();
        let to = to.cloned();

        start_async(async move {
            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let mut tasks = Vec::new();
            let distance = get_distance();
            let distance = if distance > 0.0 { distance } else { 500.0 };

            if let Some(from) = &from {
                let anim = Animation::new();
                anim.set_fill_mode(FillMode::Forward);
                anim.set_easing(slide_easing.clone());
                anim.set_duration(duration);
                anim.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(prop, 0.0) as _]));
                anim.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(prop, if forward { -distance } else { distance }) as _]));
                tasks.push(anim.run_async(from, cancellation_token.clone()));
            }

            if let Some(to) = &to {
                to.set_is_visible(true);
                let anim = Animation::new();
                anim.set_fill_mode(FillMode::Forward);
                anim.set_easing(slide_easing.clone());
                anim.set_duration(duration);
                anim.children().add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(prop, if forward { distance } else { -distance }) as _]));
                anim.children().add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(prop, 0.0) as _]));
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

impl IPageTransition for PageSlideTransition {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let (from_element, to_element) = (from.cloned(), to.cloned());
        if self.axis() == PageSlideTransitionAxis::Horizontal {
            self.start_axis(from, to, forward, cancellation_token, TranslateTransform::x_property(), move || {
                get_visual_parent(from_element.as_ref(), to_element.as_ref()).bounds().width
            })
        } else {
            self.start_axis(from, to, forward, cancellation_token, TranslateTransform::y_property(), move || {
                get_visual_parent(from_element.as_ref(), to_element.as_ref()).bounds().height
            })
        }
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn defaults_and_the_constructor_with_a_duration_and_an_axis() {
        let transition = PageSlideTransition::new();
        assert_eq!(TimeSpan::from_milliseconds(300.0), transition.duration());
        assert_eq!(PageSlideTransitionAxis::Horizontal, transition.axis());

        let transition =
            PageSlideTransition::with_duration(TimeSpan::from_milliseconds(40.0), PageSlideTransitionAxis::Vertical);
        assert_eq!(TimeSpan::from_milliseconds(40.0), transition.duration());
        assert_eq!(PageSlideTransitionAxis::Vertical, transition.axis());
    }
}
