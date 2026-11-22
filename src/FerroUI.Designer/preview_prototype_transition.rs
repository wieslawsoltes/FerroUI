//! Authored motion belongs to a stage set and travels with its interaction.
use crate::{PreviewPrototype, PreviewPrototypeOverlay, PreviewStageId};
use ferroui_designer_animation::Easing;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::time::Duration;

pub const MAX_PROTOTYPE_TRANSITION_MS: u32 = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewPrototypeEffect {
    Dissolve,
    SlideLeft,
    SlideRight,
    SlideUp,
    SlideDown,
}

impl PreviewPrototypeEffect {
    /// Direction of travel for an entering surface, in viewport coordinates.
    pub(crate) fn travel(self) -> (f64, f64) {
        match self {
            Self::Dissolve => (0.0, 0.0),
            Self::SlideLeft => (-1.0, 0.0),
            Self::SlideRight => (1.0, 0.0),
            Self::SlideUp => (0.0, -1.0),
            Self::SlideDown => (0.0, 1.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewPrototypeTransitionEasing {
    Linear,
    EaseIn,
    EaseOut,
    #[default]
    EaseInOut,
}

impl PreviewPrototypeTransitionEasing {
    fn curve(self) -> Easing {
        let (x1, y1, x2, y2) = match self {
            Self::Linear => return Easing::Linear,
            Self::EaseIn => (0.42, 0.0, 1.0, 1.0),
            Self::EaseOut => (0.0, 0.0, 0.58, 1.0),
            Self::EaseInOut => (0.42, 0.0, 0.58, 1.0),
        };
        Easing::CubicBezier { x1, y1, x2, y2 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewPrototypeTransition {
    pub effect: PreviewPrototypeEffect,
    pub duration_ms: u32,
    #[serde(default)]
    pub easing: PreviewPrototypeTransitionEasing,
}

impl PreviewPrototypeTransition {
    pub fn validate(self) -> Result<(), String> {
        if !(1..=MAX_PROTOTYPE_TRANSITION_MS).contains(&self.duration_ms) {
            return Err(format!(
                "Prototype transitions need a duration from 1 to {MAX_PROTOTYPE_TRANSITION_MS} milliseconds"
            ));
        }
        Ok(())
    }

    pub(crate) fn progress(self, elapsed: Duration, reverse: bool) -> f64 {
        let limit = Duration::from_millis(u64::from(self.duration_ms));
        let time = elapsed.min(limit).as_secs_f64() / limit.as_secs_f64();
        let curve = self.easing.curve();
        if reverse {
            1.0 - curve.sample(1.0 - time).expect("Preset prototype easing")
        } else {
            curve.sample(time).expect("Preset prototype easing")
        }
    }
}

/// An optional transition for one interaction in the graph's current order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewPrototypeLinkTransition {
    pub link: usize,
    pub transition: PreviewPrototypeTransition,
}

pub(crate) fn validate_link_transitions(
    transitions: &[PreviewPrototypeLinkTransition],
    link_count: usize,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for entry in transitions {
        if entry.link >= link_count || !seen.insert(entry.link) {
            return Err("Every transition must reference a distinct prototype interaction".into());
        }
        entry.transition.validate()?;
    }
    Ok(())
}

pub(crate) fn rebase_link_transitions(
    transitions: &[PreviewPrototypeLinkTransition],
    previous: &PreviewPrototype,
    next: &PreviewPrototype,
) -> Vec<PreviewPrototypeLinkTransition> {
    let mut rebased = transitions
        .iter()
        .filter_map(|entry| {
            let old = &previous.links[entry.link];
            let link = next.links.iter().position(|candidate| {
                candidate.source == old.source && candidate.trigger == old.trigger
            })?;
            Some(PreviewPrototypeLinkTransition { link, ..*entry })
        })
        .collect::<Vec<_>>();
    rebased.sort_unstable_by_key(|entry| entry.link);
    rebased
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MotionSurface {
    ScreenIn,
    ScreenOut,
    OverlayIn,
    OverlayOut,
    OverlaySwap,
}

/// A resolved transition. The player consumes it only after accepting the new
/// layout. Back or dismissal can reverse the animation that opened the layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewPrototypeMotion {
    pub(crate) transition: PreviewPrototypeTransition,
    pub(crate) surface: MotionSurface,
    pub(crate) reverse: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JourneyPosition {
    pub screen: PreviewStageId,
    pub history_depth: usize,
    pub overlays: Vec<PreviewPrototypeOverlay>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MotionHistory {
    screens: Vec<Option<PreviewPrototypeTransition>>,
    overlays: Vec<Option<PreviewPrototypeTransition>>,
}

impl MotionHistory {
    pub fn advance(
        &mut self,
        before: &JourneyPosition,
        after: &JourneyPosition,
        authored: Option<PreviewPrototypeTransition>,
    ) -> Option<PreviewPrototypeMotion> {
        let (surface, inherited) = if before.screen != after.screen {
            self.overlays.clear();
            if before.history_depth > after.history_depth {
                (MotionSurface::ScreenOut, self.screens.pop().flatten())
            } else {
                self.screens.push(authored);
                (MotionSurface::ScreenIn, None)
            }
        } else if after.overlays.starts_with(&before.overlays)
            && after.overlays.len() > before.overlays.len()
        {
            self.overlays.push(authored);
            (MotionSurface::OverlayIn, None)
        } else if before.overlays.starts_with(&after.overlays)
            && before.overlays.len() == after.overlays.len() + 1
        {
            (MotionSurface::OverlayOut, self.overlays.pop().flatten())
        } else if before.overlays.len() == after.overlays.len() && !after.overlays.is_empty() {
            *self.overlays.last_mut().expect("Overlay journey motion") = authored;
            (MotionSurface::OverlaySwap, None)
        } else {
            // A navigation action can also close an entire stack on this screen.
            self.overlays.truncate(after.overlays.len());
            (MotionSurface::ScreenIn, None)
        };
        authored
            .or(inherited)
            .map(|transition| PreviewPrototypeMotion {
                transition,
                surface,
                reverse: authored.is_none() && inherited.is_some(),
            })
    }
}

#[cfg(test)]
#[path = "preview_prototype_transition_tests.rs"]
mod tests;
