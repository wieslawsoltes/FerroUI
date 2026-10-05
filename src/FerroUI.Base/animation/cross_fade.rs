use crate::animation::easings::Easing;
use crate::animation::{
    Animation, Cue, FillMode, IPageTransition, IProgressPageTransition, KeyFrame, PageTransitionItem, TimeSpan,
};
use crate::animation::i_page_transition::{start_async, when_all};
use crate::styling::Setter;
use crate::threading::{CancellationToken, DispatcherTask};
use crate::{Ref, Visual};

const SIDE_PEEK_OPACITY: f64 = 0.72;
const FAR_PEEK_OPACITY: f64 = 0.42;
const OUTGOING_DIP: f64 = 0.22;
const INCOMING_BOOST: f64 = 0.12;
const PASSIVE_DIP: f64 = 0.05;

/// Defines a cross-fade animation between two visuals.
pub struct CrossFade {
    fade_out_animation: Ref<Animation>,
    fade_in_animation: Ref<Animation>,
}

impl Default for CrossFade {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossFade {
    /// Creates a cross-fade with a zero duration.
    pub fn new() -> Self {
        Self::with_duration(TimeSpan::ZERO)
    }

    /// Creates a cross-fade with the given duration.
    pub fn with_duration(duration: TimeSpan) -> Self {
        let opacity_animation = |from: f64, to: f64| {
            let animation = Animation::new();
            animation.set_fill_mode(FillMode::Forward);
            animation
                .children()
                .add(KeyFrame::with_cue(Cue::new(0.0), [Setter::new(Visual::opacity_property(), from) as _]));
            animation
                .children()
                .add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(Visual::opacity_property(), to) as _]));
            animation.set_duration(duration);
            animation
        };

        Self { fade_out_animation: opacity_animation(1.0, 0.0), fade_in_animation: opacity_animation(0.0, 1.0) }
    }

    /// The duration of the animation.
    pub fn duration(&self) -> TimeSpan {
        self.fade_out_animation.duration()
    }

    pub fn set_duration(&self, value: TimeSpan) {
        self.fade_out_animation.set_duration(value);
        self.fade_in_animation.set_duration(value);
    }

    /// The easing function used to fade in.
    pub fn fade_in_easing(&self) -> Easing {
        self.fade_in_animation.easing()
    }

    pub fn set_fade_in_easing(&self, value: impl Into<Easing>) {
        self.fade_in_animation.set_easing(value)
    }

    /// The easing function used to fade out.
    pub fn fade_out_easing(&self) -> Easing {
        self.fade_out_animation.easing()
    }

    pub fn set_fade_out_easing(&self, value: impl Into<Easing>) {
        self.fade_out_animation.set_easing(value)
    }

    /// The fill mode.
    pub fn fill_mode(&self) -> FillMode {
        self.fade_out_animation.fill_mode()
    }

    pub fn set_fill_mode(&self, value: FillMode) {
        self.fade_out_animation.set_fill_mode(value);
        self.fade_in_animation.set_fill_mode(value);
    }

    /// Starts the animation: fades `from` out and `to` in. When it has
    /// ended without being cancelled, `from` is hidden.
    pub fn start_cross_fade(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        let fade_out_animation = self.fade_out_animation.clone();
        let fade_in_animation = self.fade_in_animation.clone();
        let from = from.cloned();
        let to = to.cloned();

        start_async(async move {
            if cancellation_token.is_cancellation_requested() {
                return;
            }

            let mut tasks = Vec::new();

            if let Some(from) = &from {
                tasks.push(fade_out_animation.run_async_with_clock(from, None, cancellation_token.clone()));
            }

            if let Some(to) = &to {
                to.set_is_visible(true);
                tasks.push(fade_in_animation.run_async_with_clock(to, None, cancellation_token.clone()));
            }

            when_all(tasks).await;

            if let Some(from) = &from {
                if !cancellation_token.is_cancellation_requested() {
                    from.set_is_visible(false);
                }
            }
        })
    }

    fn update_visible_items(
        progress: f64,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        visible_items: &[PageTransitionItem],
    ) {
        let emphasis = (progress.clamp(0.0, 1.0) * std::f64::consts::PI).sin();
        for item in visible_items {
            item.visual.set_is_visible(true);
            let mut opacity = Self::get_opacity_for_offset(item.viewport_center_offset);

            if from.is_some_and(|from| item.visual.ptr_eq(from)) {
                opacity = FAR_PEEK_OPACITY.max(opacity - (OUTGOING_DIP * emphasis));
            } else if to.is_some_and(|to| item.visual.ptr_eq(to)) {
                opacity = 1.0_f64.min(opacity + (INCOMING_BOOST * emphasis));
            } else {
                opacity = FAR_PEEK_OPACITY.max(opacity - (PASSIVE_DIP * emphasis));
            }

            item.visual.set_opacity(opacity);
        }
    }

    fn get_opacity_for_offset(offset_from_center: f64) -> f64 {
        let distance = offset_from_center.abs();

        if distance <= 1.0 {
            return lerp(1.0, SIDE_PEEK_OPACITY, distance);
        }

        if distance <= 2.0 {
            return lerp(SIDE_PEEK_OPACITY, FAR_PEEK_OPACITY, distance - 1.0);
        }

        FAR_PEEK_OPACITY
    }
}

fn lerp(from: f64, to: f64, t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    from + (to - from) * t
}

impl IPageTransition for CrossFade {
    fn start(
        &self,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        _forward: bool,
        cancellation_token: CancellationToken,
    ) -> DispatcherTask<()> {
        self.start_cross_fade(from, to, cancellation_token)
    }

    fn as_progress_page_transition(&self) -> Option<&dyn IProgressPageTransition> {
        Some(self)
    }
}

impl IProgressPageTransition for CrossFade {
    fn update(
        &self,
        progress: f64,
        from: Option<&Ref<Visual>>,
        to: Option<&Ref<Visual>>,
        _forward: bool,
        _page_length: f64,
        visible_items: &[PageTransitionItem],
    ) {
        if !visible_items.is_empty() {
            Self::update_visible_items(progress, from, to, visible_items);
            return;
        }

        if let Some(from) = from {
            from.set_opacity(1.0 - progress);
        }
        if let Some(to) = to {
            to.set_is_visible(true);
            to.set_opacity(progress);
        }
    }

    fn reset(&self, visual: &Ref<Visual>) {
        visual.set_opacity(1.0);
    }
}
