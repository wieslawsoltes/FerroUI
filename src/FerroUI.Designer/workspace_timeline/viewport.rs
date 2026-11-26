//! Studio Timeline presentation uses the same range as the native lanes.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Ruler {
    Tracks,
    Clips,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    Window,
    Fit,
    FitSelection,
}

#[derive(Deserialize)]
pub(crate) struct TimelineViewportChange {
    ruler: Ruler,
    program: Option<NodeId>,
    action: Action,
    start: Option<f64>,
    end: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TimelineViewport {
    ruler: Ruler,
    #[serde(skip_serializing_if = "Option::is_none")]
    program: Option<NodeId>,
    start: f64,
    end: f64,
    duration: f64,
    can_fit_selection: bool,
}

impl DesignerWorkspace {
    pub(crate) fn timeline_viewport(&self) -> Option<TimelineViewport> {
        self.timeline_visible()
            .then(|| self.timeline_ruler())
            .flatten()
    }

    pub(crate) fn change_timeline_viewport(
        &self,
        change: TimelineViewportChange,
    ) -> Result<(), String> {
        let current = self
            .timeline_ruler()
            .ok_or("Choose an animation or clip ruler in the Timeline.")?;
        if current.ruler != change.ruler || current.program != change.program {
            return Err(
                "The visible ruler or animation changed. Read the Timeline view again.".into(),
            );
        }
        let view = match change.action {
            Action::Fit => None,
            Action::FitSelection => Some(
                self.timeline_selection_window()
                    .ok_or("Select keys on the visible curve first.")?,
            ),
            Action::Window => {
                let time = change.start.zip(change.end).filter(|(start, end)| {
                    *start >= 0.0
                        && *end > *start
                        && *end <= current.duration
                        && end - start + f64::EPSILON * current.duration
                            >= current.duration.min(0.05)
                });
                Some(time.ok_or("Choose a window within the ruler of at least 0.05 seconds, or fit the entire ruler.")?)
            }
        };
        self.set_timeline_visible(true);
        self.set_timeline_view(view);
        Ok(())
    }

    fn timeline_ruler(&self) -> Option<TimelineViewport> {
        let duration = self.timeline_view_duration();
        if !duration.is_finite() || duration <= 0.0 {
            return None;
        }
        let ui = self.timeline_ui.borrow();
        let (start, end) = ui
            .view
            .filter(|(start, end)| *start >= 0.0 && end > start && *end <= duration)
            .unwrap_or((0.0, duration));
        let (ruler, program) = if ui.compare_clips {
            (Ruler::Clips, None)
        } else {
            (
                Ruler::Tracks,
                self.session.borrow().active_animation_program(),
            )
        };
        Some(TimelineViewport {
            ruler,
            program,
            start,
            end,
            duration,
            can_fit_selection: self.timeline_selection_window().is_some(),
        })
    }
}
