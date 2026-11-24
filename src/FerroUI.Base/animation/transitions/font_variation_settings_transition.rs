use crate::animation::{Transition, TransitionBase, TransitionObservableBase};
use crate::ferro_transition_class;
use crate::media::FontVariationSettings;
use crate::reactive::IObservable;
use std::rc::Rc;

ferro_transition_class!(
    /// Transition class that handles `FontVariations` properties (upstream dfee7f224b): per-axis
    /// linear interpolation when both values set the same axes, discrete at the midpoint
    /// otherwise ([`FontVariationSettings::interpolate`]).
    FontVariationSettingsTransition: Option<FontVariationSettings>
);

impl Transition<Option<FontVariationSettings>> for FontVariationSettingsTransition {
    fn do_transition(
        this: &Self,
        progress: Rc<dyn IObservable<f64>>,
        old_value: Option<FontVariationSettings>,
        new_value: Option<FontVariationSettings>,
    ) -> Rc<dyn IObservable<Option<FontVariationSettings>>> {
        TransitionObservableBase::new(progress, this.easing(), move |progress| {
            FontVariationSettings::interpolate(old_value.as_ref(), new_value.as_ref(), progress)
        })
    }
}
