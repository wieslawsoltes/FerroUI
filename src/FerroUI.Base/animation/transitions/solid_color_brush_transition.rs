use crate::animation::animators::ISolidColorBrushAnimator;
use crate::animation::easings::{Easing, LinearEasing};
use crate::animation::{AnimatorTransitionObservable, TimeSpan, Transition, TransitionBase, TransitionObservableBase};
use crate::ferro_transition_class;
use crate::media::immutable::ImmutableSolidColorBrush;
use crate::media::{Color, IBrush};
use crate::reactive::IObservable;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class of brush properties with the rules of WinUI's `BrushTransition`
    /// (`Microsoft.UI.Xaml.BrushTransition`, the `BackgroundTransition` of `Border`, `Panel`,
    /// `ContentPresenter` and `Grid`; `WUCBrushManager::SetUpBrushTransitionIfAllowedHelper` of
    /// microsoft-ui-xaml): a change to a solid colour brush from no brush (transparent) or from
    /// another solid colour brush is interpolated; any other change (to or from an acrylic,
    /// gradient or image brush, or to no brush) is applied at once, when the delay of the
    /// transition has passed.
    ///
    /// [`BrushTransition`](crate::animation::BrushTransition) also interpolates gradient brushes
    /// and switches a brush it cannot interpolate half-way through its duration, which shows the
    /// old brush (a transparent background, say) for half of the duration.
    SolidColorBrushTransition: Option<Rc<dyn IBrush>>
);

type Brush = Option<Rc<dyn IBrush>>;

impl Transition<Brush> for SolidColorBrushTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: Brush,
        new_value: Brush,
    ) -> Rc<dyn IObservable<Brush>> {
        let old_is_none_or_solid = old_value.as_ref().map_or(true, |brush| brush.as_solid_color_brush().is_some());
        let new_is_solid = new_value.as_ref().is_some_and(|brush| brush.as_solid_color_brush().is_some());
        if old_is_none_or_solid && new_is_solid {
            // No brush fades in from transparent: the transparent colour of the new brush.
            let from = old_value.or_else(|| {
                let brush = new_value.as_ref()?;
                let color = brush.as_solid_color_brush()?.color();
                let transparent: Rc<dyn IBrush> =
                    Rc::new(ImmutableSolidColorBrush::with_opacity(Color { a: 0, ..color }, brush.opacity()));
                Some(transparent)
            });
            return AnimatorTransitionObservable::create(
                ISolidColorBrushAnimator::interpolate_core,
                progress,
                this.easing(),
                from,
                new_value,
            );
        }

        // The progress stays 0 during the delay and is above 0 (1 for no duration) afterwards; the
        // linear easing keeps an easing that returns 0 past the start from holding the old brush.
        let immediate = this.delay() == TimeSpan::ZERO;
        TransitionObservableBase::new(progress, Easing::new(LinearEasing::new()), move |progress| {
            if immediate || progress > 0.0 {
                new_value.clone()
            } else {
                old_value.clone()
            }
        })
    }
}
