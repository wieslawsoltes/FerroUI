use super::{field, input, picker};
use crate::{
    PreviewPrototypeEffect, PreviewPrototypeTransition, PreviewPrototypeTransitionEasing,
    MAX_PROTOTYPE_TRANSITION_MS,
};
use ferroui_base::Ref;
use ferroui_controls::{ColumnDefinitions, ComboBox, Control, Grid, StackPanel, TextBox};
use std::rc::Rc;

const EFFECTS: [(PreviewPrototypeEffect, &str); 5] = [
    (PreviewPrototypeEffect::Dissolve, "Dissolve"),
    (PreviewPrototypeEffect::SlideLeft, "Slide left"),
    (PreviewPrototypeEffect::SlideRight, "Slide right"),
    (PreviewPrototypeEffect::SlideUp, "Slide up"),
    (PreviewPrototypeEffect::SlideDown, "Slide down"),
];
const EASINGS: [(PreviewPrototypeTransitionEasing, &str); 4] = [
    (PreviewPrototypeTransitionEasing::Linear, "Linear"),
    (PreviewPrototypeTransitionEasing::EaseIn, "Ease in"),
    (PreviewPrototypeTransitionEasing::EaseOut, "Ease out"),
    (
        PreviewPrototypeTransitionEasing::EaseInOut,
        "Ease in and out",
    ),
];

pub(super) struct TransitionEditor {
    effect: Ref<ComboBox>,
    duration: Ref<TextBox>,
    easing: Ref<ComboBox>,
}

impl TransitionEditor {
    pub fn new(body: &StackPanel, initial: Option<PreviewPrototypeTransition>) -> Self {
        let effect = picker("PrototypeTransitionEffect", "Transition effect");
        effect.items().add(Some(Rc::new("Instant".to_owned())));
        for (_, label) in EFFECTS {
            effect.items().add(Some(Rc::new(label.to_owned())));
        }
        effect.set_selected_index(
            initial
                .and_then(|value| {
                    EFFECTS
                        .iter()
                        .position(|(effect, _)| *effect == value.effect)
                })
                .map_or(0, |index| index as i32 + 1),
        );
        field(body, "Transition", &effect);

        let duration = input(
            "PrototypeTransitionDuration",
            "Duration in milliseconds",
            &initial.map_or(250, |value| value.duration_ms).to_string(),
        );
        let easing = picker("PrototypeTransitionEasing", "Transition easing");
        for (_, label) in EASINGS {
            easing.items().add(Some(Rc::new(label.to_owned())));
        }
        let easing_kind = initial.map_or_else(PreviewPrototypeTransitionEasing::default, |value| {
            value.easing
        });
        easing.set_selected_index(
            EASINGS
                .iter()
                .position(|(kind, _)| *kind == easing_kind)
                .unwrap() as i32,
        );
        let details = Grid::new();
        details.set_column_definitions(ColumnDefinitions::parse("*,*").unwrap());
        details.set_column_spacing(8.0);
        let timing = StackPanel::new();
        let interpolation = StackPanel::new();
        field(&timing, "Duration (ms)", &duration);
        field(&interpolation, "Easing", &easing);
        Grid::set_column(&interpolation, 1);
        details.children().add(timing.upcast::<Control>());
        details.children().add(interpolation.upcast::<Control>());
        details.set_is_visible(initial.is_some());
        let fields = details.downgrade();
        let selected = effect.downgrade();
        effect.selection_changed(move |_, _| {
            if let (Some(fields), Some(effect)) = (fields.upgrade(), selected.upgrade()) {
                fields.set_is_visible(effect.selected_index() > 0);
            }
        });
        body.children().add(details.upcast::<Control>());
        Self {
            effect,
            duration,
            easing,
        }
    }

    pub fn value(&self) -> Result<Option<PreviewPrototypeTransition>, String> {
        let index = self.effect.selected_index();
        if index == 0 {
            return Ok(None);
        }
        let effect = usize::try_from(index - 1)
            .ok()
            .and_then(|index| EFFECTS.get(index))
            .ok_or("Choose a prototype transition effect")?
            .0;
        let easing = usize::try_from(self.easing.selected_index())
            .ok()
            .and_then(|index| EASINGS.get(index))
            .ok_or("Choose a prototype transition easing")?
            .0;
        let duration_ms = self
            .duration
            .text()
            .unwrap_or_default()
            .trim()
            .parse()
            .map_err(|_| {
                format!(
                    "Enter a whole duration from 1 to {MAX_PROTOTYPE_TRANSITION_MS} milliseconds"
                )
            })?;
        let transition = PreviewPrototypeTransition {
            effect,
            duration_ms,
            easing,
        };
        transition.validate()?;
        Ok(Some(transition))
    }
}
