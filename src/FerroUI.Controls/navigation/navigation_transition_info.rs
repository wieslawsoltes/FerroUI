//! WinUI's `NavigationTransitionInfo` and its kinds (`EntranceNavigationTransitionInfo`,
//! `DrillInNavigationTransitionInfo`, `SuppressNavigationTransitionInfo` and
//! `SlideNavigationTransitionInfo` with its `Effect`), as the animation of the new page of a
//! [`Frame`](crate::Frame). The animations run on the render transform and the opacity of the
//! page, which the compositor draws on the GPU.

use ferroui_base::animation::easings::{Easing, SplineEasing};
use ferroui_base::animation::{
    Animation, Cue, FillMode, IAnimationSetter, IGlobalClock, KeyFrame, SignedDuration,
};
use ferroui_base::media::TranslateTransform;
use ferroui_base::styling::Setter;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{FerroLocator, LocatorExtensions, Ref, Vector, Visual};
use std::rc::Rc;

/// How far an entering page travels, in device-independent pixels.
const ENTRANCE_OFFSET: f64 = 140.0;
/// How far a sliding page travels.
const SLIDE_OFFSET: f64 = 200.0;
const DURATION_MS: f64 = 300.0;

/// How a [`Frame`](crate::Frame) brings in the page of a navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NavigationTransitionInfo {
    /// The page rises into place and fades in.
    Entrance,
    /// The page fades in. WinUI also scales it.
    DrillIn,
    /// No animation.
    Suppress,
    /// The page slides in from the right.
    SlideFromRight,
    /// The page slides in from the left.
    SlideFromLeft,
    /// The page slides in from the bottom.
    SlideFromBottom,
}

impl NavigationTransitionInfo {
    /// The transition of the way back: a page that came from the right returns from the left.
    pub fn reversed(self) -> Self {
        match self {
            Self::SlideFromRight => Self::SlideFromLeft,
            Self::SlideFromLeft => Self::SlideFromRight,
            other => other,
        }
    }

    /// Where the page starts, relative to its place.
    pub(crate) fn start_offset(self) -> Vector {
        match self {
            Self::Entrance => Vector::new(0.0, ENTRANCE_OFFSET),
            Self::SlideFromRight => Vector::new(SLIDE_OFFSET, 0.0),
            Self::SlideFromLeft => Vector::new(-SLIDE_OFFSET, 0.0),
            Self::SlideFromBottom => Vector::new(0.0, SLIDE_OFFSET),
            Self::DrillIn | Self::Suppress => Vector::new(0.0, 0.0),
        }
    }

    /// Runs the animation on `page`, until it ends or `cancellation_token` is cancelled. When it
    /// ends the page is at its place with its own opacity.
    pub(crate) fn run(self, page: &Ref<Visual>, cancellation_token: CancellationToken) {
        if self == Self::Suppress {
            return;
        }
        // Without a clock (no application is running) there is no time to animate on: the page
        // is shown in its place.
        if page.clock().is_none() && FerroLocator::current().get_service::<dyn IGlobalClock>().is_none() {
            return;
        }
        let offset = self.start_offset();
        let animation = Animation::new();
        animation.set_fill_mode(FillMode::None);
        animation.set_duration(SignedDuration::from_milliseconds(DURATION_MS));
        // WinUI's deceleration curve of page transitions.
        animation.set_easing(Easing::new(SplineEasing::with_points(0.1, 0.9, 0.2, 1.0)));
        let frame = |cue: f64, x: f64, y: f64, opacity: f64| {
            let mut setters: Vec<Rc<dyn IAnimationSetter>> = vec![Setter::new(Visual::opacity_property(), opacity)];
            if offset.x != 0.0 {
                setters.push(Setter::new(TranslateTransform::x_property(), x));
            }
            if offset.y != 0.0 {
                setters.push(Setter::new(TranslateTransform::y_property(), y));
            }
            KeyFrame::with_cue(Cue::new(cue), setters)
        };
        animation.children().add(frame(0.0, offset.x, offset.y, 0.0));
        animation.children().add(frame(1.0, 0.0, 0.0, 1.0));
        drop(animation.run_async_with_clock(page, None, cancellation_token));
    }
}
